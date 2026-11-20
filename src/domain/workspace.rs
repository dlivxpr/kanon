//! The single deep domain module: `Workspace` owns every business rule and
//! the SQLite persistence behind them (ADR-0003).
//!
//! It is pure Rust with zero UI-crate coupling: it accepts only a database
//! location (a file path, or in-memory for tests) and a [`Clock`]. All
//! cross-object writes commit atomically in single transactions.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::clock::{Clock, SystemClock};
use super::date::{LocalDate, LocalDateTime, LocalTime};
use super::schema::{SCHEMA, SCHEMA_VERSION};
use super::task::{
    NewTask, Priority, ProjectRef, Subtask, Tag, Task, TaskAggregate, TaskPatch, TaskStatus,
};

/// What can go wrong inside the domain.
#[derive(Debug)]
pub enum WorkspaceError {
    Sqlite(rusqlite::Error),
    /// A date/time string in storage or input that is not well-formed.
    InvalidDateTime(String),
    /// An optimistic write whose base `updated_at` no longer matches the
    /// row: someone else committed first; retry against a fresh base.
    StaleDocument,
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(err) => write!(f, "SQLite error: {err}"),
            Self::InvalidDateTime(value) => write!(f, "invalid date/time value: {value}"),
            Self::StaleDocument => write!(f, "the document changed since this edit began"),
        }
    }
}

impl std::error::Error for WorkspaceError {}

impl From<rusqlite::Error> for WorkspaceError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Sqlite(err)
    }
}

pub type Result<T, E = WorkspaceError> = std::result::Result<T, E>;

/// A Knowledge Base (书目式容器; GLOSSARY).
#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeBase {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub color: String,
}

/// A Catalog Node: a pure grouping node, or a node carrying a Document
/// that nests child Documents (GLOSSARY).
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogNode {
    pub id: i64,
    pub book_id: i64,
    pub parent_id: Option<i64>,
    pub document_id: Option<i64>,
    pub title: String,
    pub expanded: bool,
}

/// A Document, owned strictly by one Knowledge Base (ADR-0001).
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub id: i64,
    pub book_id: i64,
    pub title: String,
    pub body: String,
    /// The commit stamp of the last body write.
    pub updated_at: String,
    /// `count_words` of `body`, persisted with every write.
    pub word_count: u64,
    /// Bumped on every body write: the optimistic-locking base of the
    /// single editor's auto-save.
    pub version: i64,
}

/// A saved Document revision: `name = None` is an automatic snapshot,
/// `Some` a user-named version.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentRevision {
    pub id: i64,
    pub document_id: i64,
    pub title: String,
    pub body: String,
    pub name: Option<String>,
    /// When the revision was recorded (the workspace clock's stamp).
    pub created_at: String,
}

/// An indexed projection of a Linked Task Card or an inline `@` mention
/// inside a Document, for O(1) backlink queries (ADR-0002).
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentReference {
    pub id: i64,
    pub document_id: i64,
    pub ref_type: String,
    pub target_kind: String,
    pub target_id: Option<i64>,
    pub target_label: String,
    /// Where in the body the reference anchors, in UTF-16 code units as
    /// the editor counts; a restore re-anchors it (see
    /// [`Workspace::restore_document_revision`]).
    pub position: i64,
}

/// A Project with its optional `(linked_book_id, linked_group_node_id)`
/// binding (ADR-0001).
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub color: String,
    pub emoji: String,
    pub linked_book_id: Option<i64>,
    pub linked_group_node_id: Option<i64>,
}

/// A Milestone inside a Project.
#[derive(Debug, Clone, PartialEq)]
pub struct Milestone {
    pub id: i64,
    pub project_id: i64,
    pub title: String,
    pub target_date: Option<LocalDate>,
}

/// A Quick Note (GLOSSARY).
#[derive(Debug, Clone, PartialEq)]
pub struct QuickNote {
    pub id: i64,
    pub content: String,
    pub pinned: bool,
    pub archived: bool,
}

/// Tasks grouped with strict mutual exclusion (spec:
/// `Overdue > Today > Upcoming > Inbox > Unscheduled`, plus `Done` and
/// `Cancelled`): a single Task appears in at most one group.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskGroups {
    pub overdue: Vec<TaskAggregate>,
    pub today: Vec<TaskAggregate>,
    pub upcoming: Vec<TaskAggregate>,
    pub inbox: Vec<TaskAggregate>,
    pub unscheduled: Vec<TaskAggregate>,
    pub done: Vec<TaskAggregate>,
    pub cancelled: Vec<TaskAggregate>,
}

impl TaskGroups {
    pub fn total_active(&self) -> usize {
        self.overdue.len() + self.today.len() + self.upcoming.len() + self.inbox.len() + self.unscheduled.len()
    }
}

/// The local-first workspace facade: one SQLite database, one clock.
pub struct Workspace {
    conn: Connection,
    clock: Arc<dyn Clock>,
}

impl Workspace {
    /// Opens (creating if needed) the workspace at `path` and migrates it.
    pub fn open(path: &Path, clock: Arc<dyn Clock>) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|err| {
                WorkspaceError::InvalidDateTime(format!("cannot create {}: {err}", parent.display()))
            })?;
        }
        let mut workspace = Self { conn: Connection::open(path)?, clock };
        workspace.migrate()?;
        Ok(workspace)
    }

    /// An in-memory workspace (tests and the ephemeral default).
    pub fn open_in_memory() -> Result<Self> {
        Self::open_in_memory_with_clock(Arc::new(SystemClock))
    }

    /// An in-memory workspace with an explicit clock.
    pub fn open_in_memory_with_clock(clock: Arc<dyn Clock>) -> Result<Self> {
        let mut workspace = Self { conn: Connection::open_in_memory()?, clock };
        workspace.migrate()?;
        Ok(workspace)
    }

    /// Initializes the full schema in one transaction; safe to re-run.
    pub fn migrate(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute_batch(SCHEMA)?;
        let version: i32 = tx.query_row("SELECT user_version FROM pragma_user_version", [], |row| row.get(0))?;
        if version < 2 && !table_has_column(&tx, "documents", "version")? {
            // Version 2 adds the optimistic-locking counter of the
            // single-editor auto-save.
            tx.execute_batch("ALTER TABLE documents ADD COLUMN version INTEGER NOT NULL DEFAULT 0;")?;
        }
        if version < SCHEMA_VERSION {
            tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// The clock's current local date and time.
    pub fn now(&self) -> LocalDateTime {
        self.clock.now()
    }

    /// The clock's current local date.
    pub fn today(&self) -> LocalDate {
        self.clock.now().date
    }

    fn stamp(&self) -> String {
        let now = self.clock.now();
        format!("{} {:02}:{:02}", now.date, now.time.hour, now.time.minute)
    }

    // ------------------------------------------------------------------
    // Preferences
    // ------------------------------------------------------------------

    /// Reads a preference value (e.g. `sidebar_collapsed`).
    pub fn preference(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM preferences WHERE key = ?")?;
        Ok(stmt.query_row(params![key], |row| row.get(0)).optional()?)
    }

    /// Sets a preference value, replacing any previous one.
    pub fn set_preference(&mut self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO preferences (key, value, updated_at) VALUES (?, ?, ?) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, self.stamp()],
        )?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Tasks
    // ------------------------------------------------------------------

    /// Creates a Task (its cross-object effects, tags for instance, commit
    /// atomically with the row).
    pub fn create_task(&mut self, new: &NewTask) -> Result<TaskAggregate> {
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO tasks (title, status, priority, project_id, milestone_id, \
                                scheduled_date, scheduled_time, due_date, due_time, \
                                created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                new.title,
                new.status.unwrap_or_default().as_str(),
                new.priority.unwrap_or_default().as_str(),
                new.project_id,
                None::<i64>,
                new.scheduled_date.map(|d| d.to_string()),
                new.scheduled_time.map(|t| t.to_string()),
                new.due_date.map(|d| d.to_string()),
                new.due_time.map(|t| t.to_string()),
                now,
                now,
            ],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        self.task(id)?.ok_or_else(|| WorkspaceError::InvalidDateTime(format!("task {id} missing after insert")))
    }

    /// Loads a Task with its tags, subtasks and project.
    pub fn task(&self, id: i64) -> Result<Option<TaskAggregate>> {
        let task = self
            .conn
            .query_row("SELECT * FROM tasks WHERE id = ?", params![id], task_from_row)
            .optional()?;
        task.map(|task| self.with_details(task)).transpose()
    }

    /// All non-trashed Tasks scheduled for `date` (any status but
    /// `cancel`), each carrying its overdue flag's inputs.
    pub fn tasks_for_date(&self, date: LocalDate) -> Result<Vec<TaskAggregate>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM tasks WHERE trashed_at IS NULL AND status != 'cancel' \
             AND scheduled_date = ? ORDER BY scheduled_time IS NULL, scheduled_time, sort_order, id",
        )?;
        let tasks = stmt
            .query_map(params![date.to_string()], task_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        tasks.into_iter().map(|task| self.with_details(task)).collect()
    }

    /// Applies a partial edit. `Some(Some(v))` sets a field, `Some(None)`
    /// clears it, `None` leaves it untouched.
    pub fn update_task(&mut self, id: i64, patch: &TaskPatch) -> Result<Task> {
        let mut sets: Vec<&str> = Vec::new();
        let mut values: Vec<Option<String>> = Vec::new();
        if let Some(title) = &patch.title {
            sets.push("title = ?");
            values.push(Some(title.clone()));
        }
        if let Some(status) = patch.status {
            sets.push("status = ?");
            values.push(Some(status.as_str().to_owned()));
            sets.push("completed_at = ?");
            values.push(if status == TaskStatus::Done { Some(self.stamp()) } else { None });
        }
        if let Some(priority) = patch.priority {
            sets.push("priority = ?");
            values.push(Some(priority.as_str().to_owned()));
        }
        if let Some(memo) = &patch.memo {
            sets.push("memo = ?");
            values.push(Some(memo.clone()));
        }
        if let Some(project_id) = &patch.project_id {
            sets.push("project_id = ?");
            values.push(project_id.map(|id| id.to_string()));
        }
        if let Some(scheduled_date) = &patch.scheduled_date {
            sets.push("scheduled_date = ?");
            values.push(scheduled_date.map(|d| d.to_string()));
        }
        if let Some(scheduled_time) = &patch.scheduled_time {
            sets.push("scheduled_time = ?");
            values.push(scheduled_time.map(|t| t.to_string()));
        }
        if let Some(due_date) = &patch.due_date {
            sets.push("due_date = ?");
            values.push(due_date.map(|d| d.to_string()));
        }
        if let Some(due_time) = &patch.due_time {
            sets.push("due_time = ?");
            values.push(due_time.map(|t| t.to_string()));
        }
        sets.push("updated_at = ?");
        values.push(Some(self.stamp()));

        let mut refs: Vec<&dyn rusqlite::ToSql> =
            values.iter().map(|value| value as &dyn rusqlite::ToSql).collect();
        let id_param: &dyn rusqlite::ToSql = &id;
        refs.push(id_param);
        let sql = format!("UPDATE tasks SET {} WHERE id = ?", sets.join(", "));
        let changed = self.conn.execute(&sql, &refs[..])?;
        if changed == 0 {
            return Err(WorkspaceError::InvalidDateTime(format!("no task {id}")));
        }
        Ok(self.task(id)?.expect("task exists after update").task)
    }

    /// The quick circle control: toggles strictly between `done` and
    /// `todo` (spec); `doing`/`cancel` go to `done`, `done` goes to `todo`.
    pub fn toggle_task_circle(&mut self, id: i64) -> Result<Task> {
        let current = self
            .task(id)?
            .ok_or_else(|| WorkspaceError::InvalidDateTime(format!("no task {id}")))?
            .task;
        self.update_task(id, &TaskPatch { status: Some(current.status.circle_toggled()), ..Default::default() })
    }

    /// How many tasks Carryover would move today (the badge on its
    /// button).
    pub fn carryover_count(&self) -> Result<usize> {
        let today = self.today().to_string();
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE trashed_at IS NULL AND status IN ('todo', 'doing') \
             AND scheduled_date IS NOT NULL AND scheduled_date < ?",
            params![today],
            |row| row.get::<_, i64>(0),
        )? as usize)
    }

    /// Carryover (GLOSSARY): moves every active task scheduled strictly
    /// before today onto today's Scheduled Date, clearing the stale
    /// scheduled time-of-day and leaving Due Date/Time untouched.
    /// Returns how many tasks moved.
    pub fn carryover(&mut self) -> Result<usize> {
        let today = self.today().to_string();
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE tasks SET scheduled_date = ?, scheduled_time = NULL, updated_at = ? \
             WHERE trashed_at IS NULL AND status IN ('todo', 'doing') \
             AND scheduled_date IS NOT NULL AND scheduled_date < ?",
            params![today, now, today],
        )?;
        let moved = tx.changes() as usize;
        tx.commit()?;
        Ok(moved)
    }

    /// The Inbox: active tasks with no Project, no Scheduled Date and no
    /// Due Date (GLOSSARY).
    pub fn inbox_tasks(&self) -> Result<Vec<TaskAggregate>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM tasks WHERE trashed_at IS NULL AND status IN ('todo', 'doing') \
             AND project_id IS NULL AND scheduled_date IS NULL AND due_date IS NULL \
             ORDER BY sort_order, id",
        )?;
        let tasks = stmt
            .query_map([], task_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        tasks.into_iter().map(|task| self.with_details(task)).collect()
    }

    /// All non-trashed tasks, grouped with strict mutual exclusion
    /// (`Overdue > Today > Upcoming > Inbox > Unscheduled`, plus `Done`
    /// and `Cancelled`).
    pub fn task_groups(&self) -> Result<TaskGroups> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM tasks WHERE trashed_at IS NULL ORDER BY sort_order, id")?;
        let tasks = stmt
            .query_map([], task_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let now = self.now();
        let today = now.date;
        let mut groups = TaskGroups::default();
        for task in tasks {
            let overdue = task.is_overdue_at(now);
            let aggregate = self.with_details(task)?;
            let task = &aggregate.task;
            if !task.status.is_active() {
                if task.status == TaskStatus::Done {
                    groups.done.push(aggregate);
                } else {
                    groups.cancelled.push(aggregate);
                }
            } else if overdue {
                groups.overdue.push(aggregate);
            } else if task.scheduled_date == Some(today) {
                groups.today.push(aggregate);
            } else if task.scheduled_date.is_some_and(|date| date > today) {
                groups.upcoming.push(aggregate);
            } else if task.is_inbox_shape() {
                groups.inbox.push(aggregate);
            } else {
                groups.unscheduled.push(aggregate);
            }
        }
        Ok(groups)
    }

    /// Moves a Task to Trash (its views stop listing it; rows referencing
    /// it keep their position, per ADR-0002).
    pub fn move_task_to_trash(&mut self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE tasks SET trashed_at = ?, updated_at = ? WHERE id = ? AND trashed_at IS NULL",
            params![self.stamp(), self.stamp(), id],
        )?;
        Ok(())
    }

    /// Restores a Task from Trash.
    pub fn restore_task(&mut self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE tasks SET trashed_at = NULL, updated_at = ? WHERE id = ?",
            params![self.stamp(), id],
        )?;
        Ok(())
    }

    /// Permanently deletes a Task.
    pub fn delete_task_permanent(&mut self, id: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM task_subtasks WHERE task_id = ?", params![id])?;
        tx.execute("DELETE FROM task_tags WHERE task_id = ?", params![id])?;
        tx.execute("DELETE FROM tasks WHERE id = ?", params![id])?;
        tx.commit()?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Subtasks and tags
    // ------------------------------------------------------------------

    /// Adds a subtask to a Task.
    pub fn add_subtask(&mut self, task_id: i64, title: &str) -> Result<Subtask> {
        self.conn.execute(
            "INSERT INTO task_subtasks (task_id, title, created_at) VALUES (?, ?, ?)",
            params![task_id, title, self.stamp()],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(Subtask { id, task_id, title: title.to_owned(), done: false })
    }

    /// Toggles or sets a subtask's done flag.
    pub fn set_subtask_done(&mut self, subtask_id: i64, done: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE task_subtasks SET done = ? WHERE id = ?",
            params![done as i64, subtask_id],
        )?;
        Ok(())
    }

    /// Removes a subtask.
    pub fn remove_subtask(&mut self, subtask_id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM task_subtasks WHERE id = ?", params![subtask_id])?;
        Ok(())
    }

    /// Finds or creates a Tag by name.
    pub fn ensure_tag(&mut self, name: &str) -> Result<Tag> {
        if let Some(tag) = self
            .conn
            .query_row("SELECT id, name, color FROM tags WHERE name = ?", params![name], tag_from_row)
            .optional()?
        {
            return Ok(tag);
        }
        self.conn.execute(
            "INSERT INTO tags (name, created_at) VALUES (?, ?)",
            params![name, self.stamp()],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(Tag { id, name: name.to_owned(), color: "#547093".to_owned() })
    }

    /// Attaches a Tag to a Task (idempotent).
    pub fn attach_tag(&mut self, task_id: i64, tag_id: i64) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?, ?)",
            params![task_id, tag_id],
        )?;
        Ok(())
    }

    /// Detaches a Tag from a Task.
    pub fn detach_tag(&mut self, task_id: i64, tag_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM task_tags WHERE task_id = ? AND tag_id = ?",
            params![task_id, tag_id],
        )?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Knowledge Bases, Catalog, Documents (schema and ownership rules)
    // ------------------------------------------------------------------

    /// Creates a Knowledge Base.
    pub fn create_knowledge_base(&mut self, name: &str) -> Result<KnowledgeBase> {
        let now = self.stamp();
        self.conn.execute(
            "INSERT INTO knowledge_bases (name, created_at, updated_at) VALUES (?, ?, ?)",
            params![name, now, now],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(KnowledgeBase {
            id,
            name: name.to_owned(),
            description: String::new(),
            icon: "📕".to_owned(),
            color: "#547093".to_owned(),
        })
    }

    /// Loads a Knowledge Base (the document breadcrumb shows its name).
    pub fn knowledge_base(&self, id: i64) -> Result<Option<KnowledgeBase>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, name, description, icon, color FROM knowledge_bases WHERE id = ?",
                params![id],
                |row| {
                    Ok(KnowledgeBase {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        icon: row.get(3)?,
                        color: row.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    /// Creates a Document strictly inside one Knowledge Base
    /// (ADR-0001), optionally under a parent Catalog Node, and links it
    /// into the catalog tree.
    pub fn create_document(
        &mut self,
        book_id: i64,
        parent_node_id: Option<i64>,
        title: &str,
    ) -> Result<Document> {
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO documents (book_id, title, created_at, updated_at) VALUES (?, ?, ?, ?)",
            params![book_id, title, now, now],
        )?;
        let document_id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO catalog_nodes (book_id, parent_id, document_id, title, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            params![book_id, parent_node_id, document_id, title, now, now],
        )?;
        tx.commit()?;
        Ok(Document {
            id: document_id,
            book_id,
            title: title.to_owned(),
            body: String::new(),
            updated_at: now.clone(),
            word_count: 0,
            version: 0,
        })
    }

    /// Creates a pure grouping Catalog Node (or a child position).
    pub fn create_catalog_node(
        &mut self,
        book_id: i64,
        parent_id: Option<i64>,
        title: &str,
    ) -> Result<CatalogNode> {
        let now = self.stamp();
        self.conn.execute(
            "INSERT INTO catalog_nodes (book_id, parent_id, title, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?)",
            params![book_id, parent_id, title, now, now],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(CatalogNode { id, book_id, parent_id, document_id: None, title: title.to_owned(), expanded: true })
    }

    /// Saves a Document revision; `name = None` records an automatic
    /// snapshot, `Some` a user-named version.
    pub fn save_document_revision(
        &mut self,
        document_id: i64,
        name: Option<&str>,
    ) -> Result<DocumentRevision> {
        let document = self
            .document(document_id)?
            .ok_or_else(|| WorkspaceError::InvalidDateTime(format!("no document {document_id}")))?;
        let created_at = self.stamp();
        self.conn.execute(
            "INSERT INTO document_revisions (document_id, title, body, name, created_at) \
             VALUES (?, ?, ?, ?, ?)",
            params![document_id, document.title, document.body, name, created_at],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(DocumentRevision {
            id,
            document_id,
            title: document.title,
            body: document.body,
            name: name.map(str::to_owned),
            created_at,
        })
    }

    /// Indexes a Linked Task Card or `@` mention inside a Document.
    #[allow(clippy::too_many_arguments)]
    pub fn add_document_reference(
        &mut self,
        document_id: i64,
        ref_type: &str,
        target_kind: &str,
        target_id: Option<i64>,
        target_label: &str,
        position: i64,
    ) -> Result<DocumentReference> {
        self.conn.execute(
            "INSERT INTO document_references \
             (document_id, ref_type, target_kind, target_id, target_label, position, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![document_id, ref_type, target_kind, target_id, target_label, position, self.stamp()],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(DocumentReference {
            id,
            document_id,
            ref_type: ref_type.to_owned(),
            target_kind: target_kind.to_owned(),
            target_id,
            target_label: target_label.to_owned(),
            position,
        })
    }

    /// Loads a Document.
    pub fn document(&self, id: i64) -> Result<Option<Document>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, book_id, title, body, updated_at, word_count, version \
                 FROM documents WHERE id = ?",
                params![id],
                |row| {
                    Ok(Document {
                        id: row.get(0)?,
                        book_id: row.get(1)?,
                        title: row.get(2)?,
                        body: row.get(3)?,
                        updated_at: row.get(4)?,
                        word_count: row.get::<_, i64>(5)? as u64,
                        version: row.get(6)?,
                    })
                },
            )
            .optional()?)
    }

    /// Commits a body write: title, body and the derived word count land
    /// in one statement. With `base_version` the write is optimistic — it
    /// refuses with [`WorkspaceError::StaleDocument`] when another tab or
    /// process committed first, so a draft is never silently overwritten;
    /// re-read the document and retry with its fresh version.
    pub fn update_document_body(
        &mut self,
        id: i64,
        title: &str,
        body: &str,
        base_version: Option<i64>,
    ) -> Result<Document> {
        let now = self.stamp();
        let words = super::document::count_words(body) as i64;
        let changed = match base_version {
            Some(base) => self.conn.execute(
                "UPDATE documents SET title = ?, body = ?, word_count = ?, updated_at = ?, \
                 version = version + 1 WHERE id = ? AND version = ?",
                params![title, body, words, now, id, base],
            )?,
            None => self.conn.execute(
                "UPDATE documents SET title = ?, body = ?, word_count = ?, updated_at = ?, \
                 version = version + 1 WHERE id = ?",
                params![title, body, words, now, id],
            )?,
        };
        if changed == 0 {
            if base_version.is_some() && self.document(id)?.is_some() {
                return Err(WorkspaceError::StaleDocument);
            }
            return Err(WorkspaceError::InvalidDateTime(format!("no document {id}")));
        }
        Ok(self.document(id)?.expect("document exists after update"))
    }

    /// Every revision of a Document, newest first for the history panel.
    pub fn document_revisions(&self, document_id: i64) -> Result<Vec<DocumentRevision>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, document_id, title, body, name, created_at FROM document_revisions \
             WHERE document_id = ? ORDER BY created_at DESC, id DESC",
        )?;
        Ok(stmt
            .query_map(params![document_id], |row| {
                Ok(DocumentRevision {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    title: row.get(2)?,
                    body: row.get(3)?,
                    name: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Every indexed reference of a Document, oldest first (their anchor
    /// order in the body).
    pub fn document_references(&self, document_id: i64) -> Result<Vec<DocumentReference>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, document_id, ref_type, target_kind, target_id, target_label, position \
             FROM document_references WHERE document_id = ? ORDER BY position, id",
        )?;
        Ok(stmt
            .query_map(params![document_id], |row| {
                Ok(DocumentReference {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    ref_type: row.get(2)?,
                    target_kind: row.get(3)?,
                    target_id: row.get(4)?,
                    target_label: row.get(5)?,
                    position: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Restores a saved revision: the Document's title, body and word
    /// count are rewritten, and its references are re-anchored to the
    /// restored text (the first occurrence of each reference's label, in
    /// UTF-16 offsets as the editor counts). References whose label the
    /// restored body does not contain keep their position. No Task — and
    /// no other object — is ever touched (ADR-0002).
    pub fn restore_document_revision(&mut self, revision_id: i64) -> Result<Document> {
        let revision = self
            .document_revision(revision_id)?
            .ok_or_else(|| WorkspaceError::InvalidDateTime(format!("no revision {revision_id}")))?;
        let restored = self.update_document_body(
            revision.document_id,
            &revision.title,
            &revision.body,
            None,
        )?;
        let references = self.document_references(revision.document_id)?;
        for reference in references {
            if let Some(position) = utf16_position_of(&restored.body, &reference.target_label) {
                self.conn.execute(
                    "UPDATE document_references SET position = ? WHERE id = ?",
                    params![position as i64, reference.id],
                )?;
            }
        }
        Ok(restored)
    }

    /// Loads a single revision.
    pub fn document_revision(&self, revision_id: i64) -> Result<Option<DocumentRevision>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, document_id, title, body, name, created_at FROM document_revisions WHERE id = ?",
                params![revision_id],
                |row| {
                    Ok(DocumentRevision {
                        id: row.get(0)?,
                        document_id: row.get(1)?,
                        title: row.get(2)?,
                        body: row.get(3)?,
                        name: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                },
            )
            .optional()?)
    }

    // ------------------------------------------------------------------
    // Projects and Milestones
    // ------------------------------------------------------------------

    /// Creates a Project with its optional Knowledge Base / grouping-node
    /// binding (ADR-0001).
    pub fn create_project(
        &mut self,
        name: &str,
        color: &str,
        emoji: &str,
        linked_book_id: Option<i64>,
        linked_group_node_id: Option<i64>,
    ) -> Result<Project> {
        let now = self.stamp();
        self.conn.execute(
            "INSERT INTO projects (name, color, emoji, linked_book_id, linked_group_node_id, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![name, color, emoji, linked_book_id, linked_group_node_id, now, now],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(Project {
            id,
            name: name.to_owned(),
            description: String::new(),
            color: color.to_owned(),
            emoji: emoji.to_owned(),
            linked_book_id,
            linked_group_node_id,
        })
    }

    /// All non-trashed Projects.
    pub fn projects(&self) -> Result<Vec<Project>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM projects WHERE trashed_at IS NULL ORDER BY sort_order, id")?;
        let projects = stmt
            .query_map([], project_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(projects)
    }

    /// Adds a Milestone to a Project.
    pub fn create_milestone(
        &mut self,
        project_id: i64,
        title: &str,
        target_date: Option<LocalDate>,
    ) -> Result<Milestone> {
        let now = self.stamp();
        self.conn.execute(
            "INSERT INTO milestones (project_id, title, target_date, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?)",
            params![project_id, title, target_date.map(|d| d.to_string()), now, now],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(Milestone { id, project_id, title: title.to_owned(), target_date })
    }

    /// Deletes a Milestone, clearing the milestone reference of its Tasks
    /// without deleting them (ADR-0001).
    pub fn delete_milestone(&mut self, milestone_id: i64) -> Result<()> {
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE tasks SET milestone_id = NULL, updated_at = ? WHERE milestone_id = ?",
            params![now, milestone_id],
        )?;
        tx.execute("DELETE FROM milestones WHERE id = ?", params![milestone_id])?;
        tx.commit()?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Quick Notes
    // ------------------------------------------------------------------

    /// Captures a Quick Note.
    pub fn create_quick_note(&mut self, content: &str) -> Result<QuickNote> {
        let now = self.stamp();
        self.conn.execute(
            "INSERT INTO quick_notes (content, created_at, updated_at) VALUES (?, ?, ?)",
            params![content, now, now],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(QuickNote { id, content: content.to_owned(), pinned: false, archived: false })
    }

    // ------------------------------------------------------------------
    // Internals
    // ------------------------------------------------------------------

    fn with_details(&self, task: Task) -> Result<TaskAggregate> {
        let tags = self.tags_of_task(task.id)?;
        let subtasks = self.subtasks_of_task(task.id)?;
        let project = match task.project_id {
            Some(id) => self
                .conn
                .query_row(
                    "SELECT id, name, color FROM projects WHERE id = ?",
                    params![id],
                    |row| {
                        Ok(ProjectRef { id: row.get(0)?, name: row.get(1)?, color: row.get(2)? })
                    },
                )
                .optional()?,
            None => None,
        };
        Ok(TaskAggregate { task, tags, subtasks, project })
    }

    fn tags_of_task(&self, task_id: i64) -> Result<Vec<Tag>> {
        let mut stmt = self.conn.prepare(
            "SELECT tags.id, tags.name, tags.color FROM tags \
             JOIN task_tags ON task_tags.tag_id = tags.id \
             WHERE task_tags.task_id = ? ORDER BY tags.name",
        )?;
        Ok(stmt.query_map(params![task_id], tag_from_row)?.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    fn subtasks_of_task(&self, task_id: i64) -> Result<Vec<Subtask>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, title, done FROM task_subtasks WHERE task_id = ? ORDER BY sort_order, id",
        )?;
        Ok(stmt
            .query_map(params![task_id], |row| {
                Ok(Subtask {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    title: row.get(2)?,
                    done: row.get::<_, i64>(3)? != 0,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }
}

fn opt_date(value: Option<String>) -> std::result::Result<Option<LocalDate>, rusqlite::Error> {
    value
        .map(|text| LocalDate::parse(&text).ok_or_else(|| rusqlite::Error::InvalidQuery))
        .transpose()
}

fn opt_time(value: Option<String>) -> std::result::Result<Option<LocalTime>, rusqlite::Error> {
    value
        .map(|text| LocalTime::parse(&text).ok_or_else(|| rusqlite::Error::InvalidQuery))
        .transpose()
}

/// Whether `table` already has `column` (a migration guard: `ALTER TABLE
/// ADD COLUMN` fails on databases that were created with it).
fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT name FROM pragma_table_info(?)")?;
    let columns = stmt
        .query_map(params![table], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(columns.iter().any(|name| name == column))
}

/// Where `label` first occurs in `body`, counted in UTF-16 code units as
/// the editor counts positions; `None` when absent.
fn utf16_position_of(body: &str, label: &str) -> Option<usize> {
    let byte = body.find(label)?;
    Some(body[..byte].chars().map(char::len_utf16).sum())
}

fn task_from_row(row: &Row<'_>) -> std::result::Result<Task, rusqlite::Error> {
    Ok(Task {
        id: row.get("id")?,
        title: row.get("title")?,
        status: TaskStatus::parse(&row.get::<_, String>("status")?)
            .ok_or(rusqlite::Error::InvalidQuery)?,
        priority: Priority::parse(&row.get::<_, String>("priority")?)
            .ok_or(rusqlite::Error::InvalidQuery)?,
        memo: row.get("memo")?,
        project_id: row.get("project_id")?,
        milestone_id: row.get("milestone_id")?,
        scheduled_date: opt_date(row.get("scheduled_date")?)?,
        scheduled_time: opt_time(row.get("scheduled_time")?)?,
        due_date: opt_date(row.get("due_date")?)?,
        due_time: opt_time(row.get("due_time")?)?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        completed_at: row.get("completed_at")?,
    })
}

fn tag_from_row(row: &Row<'_>) -> std::result::Result<Tag, rusqlite::Error> {
    Ok(Tag { id: row.get(0)?, name: row.get(1)?, color: row.get(2)? })
}

fn project_from_row(row: &Row<'_>) -> std::result::Result<Project, rusqlite::Error> {
    Ok(Project {
        id: row.get("id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        color: row.get("color")?,
        emoji: row.get("emoji")?,
        linked_book_id: row.get("linked_book_id")?,
        linked_group_node_id: row.get("linked_group_node_id")?,
    })
}
