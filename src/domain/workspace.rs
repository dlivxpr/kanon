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
    /// A catalog move that would create a cycle, cross books, or attach
    /// to a trashed parent.
    InvalidMove(String),
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(err) => write!(f, "SQLite error: {err}"),
            Self::InvalidDateTime(value) => write!(f, "invalid date/time value: {value}"),
            Self::InvalidMove(value) => write!(f, "invalid catalog move: {value}"),
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
    pub favorited: bool,
}

/// A partial edit of a Knowledge Base: `Some` replaces, `None` leaves.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KnowledgeBasePatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
}

/// Live statistics of one Knowledge Base (its book-style home and the
/// shelf cards read these; docs/words exclude trashed content).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BookStats {
    pub documents: usize,
    pub words: usize,
}

/// One flattened row of a book's catalog in display order (pre-order),
/// carrying its nesting depth for indentation.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogEntry {
    pub node: CatalogNode,
    pub depth: usize,
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
    /// Local timestamp of the last structural or content change (the
    /// dot-leader catalog on the book home renders it after the title).
    pub updated_at: String,
}

/// A Document, owned strictly by one Knowledge Base (ADR-0001).
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub id: i64,
    pub book_id: i64,
    pub title: String,
    pub body: String,
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

    /// Creates a Knowledge Base (appended after the existing ones; the
    /// shelf reorders by favorite, not by creation).
    pub fn create_knowledge_base(&mut self, name: &str) -> Result<KnowledgeBase> {
        let now = self.stamp();
        self.conn.execute(
            "INSERT INTO knowledge_bases (name, created_at, updated_at, sort_order) \
             VALUES (?, ?, ?, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM knowledge_bases))",
            params![name, now, now],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(KnowledgeBase {
            id,
            name: name.to_owned(),
            description: String::new(),
            icon: "📕".to_owned(),
            color: "#547093".to_owned(),
            favorited: false,
        })
    }

    /// All non-trashed Knowledge Bases: favorites first, then shelf
    /// order (the grid/list and the book switcher read this).
    pub fn knowledge_bases(&self) -> Result<Vec<KnowledgeBase>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, icon, color, favorited FROM knowledge_bases \
             WHERE trashed_at IS NULL ORDER BY favorited DESC, sort_order, id",
        )?;
        Ok(stmt.query_map([], kb_from_row)?.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Loads a Knowledge Base regardless of its Trash state (the Trash
    /// screen and the restore flow need the trashed rows).
    pub fn knowledge_base(&self, id: i64) -> Result<Option<KnowledgeBase>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, name, description, icon, color, favorited FROM knowledge_bases WHERE id = ?",
                params![id],
                kb_from_row,
            )
            .optional()?)
    }

    /// Applies a partial edit to a Knowledge Base.
    pub fn update_knowledge_base(
        &mut self,
        id: i64,
        patch: &KnowledgeBasePatch,
    ) -> Result<KnowledgeBase> {
        let mut sets: Vec<&str> = Vec::new();
        let mut values: Vec<String> = Vec::new();
        if let Some(name) = &patch.name {
            sets.push("name = ?");
            values.push(name.clone());
        }
        if let Some(description) = &patch.description {
            sets.push("description = ?");
            values.push(description.clone());
        }
        if let Some(icon) = &patch.icon {
            sets.push("icon = ?");
            values.push(icon.clone());
        }
        if let Some(color) = &patch.color {
            sets.push("color = ?");
            values.push(color.clone());
        }
        if !sets.is_empty() {
            sets.push("updated_at = ?");
            values.push(self.stamp());
            let mut refs: Vec<&dyn rusqlite::ToSql> =
                values.iter().map(|value| value as &dyn rusqlite::ToSql).collect();
            let id_param: &dyn rusqlite::ToSql = &id;
            refs.push(id_param);
            let sql = format!("UPDATE knowledge_bases SET {} WHERE id = ?", sets.join(", "));
            let changed = self.conn.execute(&sql, &refs[..])?;
            if changed == 0 {
                return Err(WorkspaceError::InvalidDateTime(format!("no knowledge base {id}")));
            }
        }
        self.knowledge_base(id)?.ok_or_else(|| WorkspaceError::InvalidDateTime(format!("no knowledge base {id}")))
    }

    /// Pins or unpins a Knowledge Base on the shelf.
    pub fn set_knowledge_base_favorite(&mut self, id: i64, favorited: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE knowledge_bases SET favorited = ?, updated_at = ? WHERE id = ?",
            params![favorited as i64, self.stamp(), id],
        )?;
        Ok(())
    }

    /// Live document/word counts of one book (trashed content excluded):
    /// the shelf cards and the book-style home page display these.
    pub fn book_stats(&self, book_id: i64) -> Result<BookStats> {
        let (documents, words) = self.conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(word_count), 0) FROM documents \
             WHERE book_id = ? AND trashed_at IS NULL",
            params![book_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;
        Ok(BookStats { documents: documents as usize, words: words as usize })
    }

    /// Moves a Knowledge Base and its whole descendant subtree — every
    /// Catalog Node and Document of the book — into Trash, atomically
    /// (ADR-0001). Project bindings stay untouched so a restore brings
    /// the book back exactly as it was.
    pub fn trash_knowledge_base(&mut self, id: i64) -> Result<()> {
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE knowledge_bases SET trashed_at = ?, updated_at = ? WHERE id = ? AND trashed_at IS NULL",
            params![now, now, id],
        )?;
        tx.execute(
            "UPDATE catalog_nodes SET trashed_at = ?, updated_at = ? WHERE book_id = ? AND trashed_at IS NULL",
            params![now, now, id],
        )?;
        tx.execute(
            "UPDATE documents SET trashed_at = ?, updated_at = ? WHERE book_id = ? AND trashed_at IS NULL",
            params![now, now, id],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Restores a Knowledge Base and its whole subtree from Trash,
    /// atomically.
    pub fn restore_knowledge_base(&mut self, id: i64) -> Result<()> {
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE knowledge_bases SET trashed_at = NULL, updated_at = ? WHERE id = ?",
            params![now, id],
        )?;
        tx.execute(
            "UPDATE catalog_nodes SET trashed_at = NULL, updated_at = ? WHERE book_id = ?",
            params![now, id],
        )?;
        tx.execute(
            "UPDATE documents SET trashed_at = NULL, updated_at = ? WHERE book_id = ?",
            params![now, id],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Permanently deletes a Knowledge Base, its Catalog Nodes and its
    /// Documents (with revisions, references and tag links). Projects
    /// bound to the book or to one of its nodes fall back to unbound
    /// (ADR-0001). One transaction; not undoable.
    pub fn delete_knowledge_base_permanent(&mut self, id: i64) -> Result<()> {
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE projects SET linked_book_id = NULL, linked_group_node_id = NULL, updated_at = ? \
             WHERE linked_book_id = ?",
            params![now, id],
        )?;
        tx.execute(
            "UPDATE projects SET linked_group_node_id = NULL, updated_at = ? \
             WHERE linked_group_node_id IN (SELECT id FROM catalog_nodes WHERE book_id = ?)",
            params![now, id],
        )?;
        tx.execute(
            "DELETE FROM document_references WHERE document_id IN \
             (SELECT id FROM documents WHERE book_id = ?)",
            params![id],
        )?;
        tx.execute(
            "DELETE FROM document_revisions WHERE document_id IN \
             (SELECT id FROM documents WHERE book_id = ?)",
            params![id],
        )?;
        tx.execute(
            "DELETE FROM document_tags WHERE document_id IN (SELECT id FROM documents WHERE book_id = ?)",
            params![id],
        )?;
        // Catalog nodes reference their documents; delete the tree first.
        tx.execute("DELETE FROM catalog_nodes WHERE book_id = ?", params![id])?;
        tx.execute("DELETE FROM documents WHERE book_id = ?", params![id])?;
        tx.execute("DELETE FROM knowledge_bases WHERE id = ?", params![id])?;
        tx.commit()?;
        Ok(())
    }

    /// Creates a Document strictly inside one Knowledge Base
    /// (ADR-0001), optionally under a parent Catalog Node, and links it
    /// into the catalog tree at the TOP of its level (spec: the top `+`
    /// inserts at the top of the catalog; a node-level `+` nests the new
    /// Document as a child at the top of that level). First-level nodes
    /// start expanded, deeper nodes collapsed (the first-open rule).
    pub fn create_document(
        &mut self,
        book_id: i64,
        parent_node_id: Option<i64>,
        title: &str,
    ) -> Result<Document> {
        let expanded = self.validate_parent(book_id, parent_node_id)?;
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO documents (book_id, title, created_at, updated_at) VALUES (?, ?, ?, ?)",
            params![book_id, title, now, now],
        )?;
        let document_id = tx.last_insert_rowid();
        let sort_order = top_sort_order(&tx, book_id, parent_node_id)?;
        tx.execute(
            "INSERT INTO catalog_nodes \
             (book_id, parent_id, document_id, title, sort_order, expanded, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![book_id, parent_node_id, document_id, title, sort_order, expanded as i64, now, now],
        )?;
        tx.commit()?;
        Ok(Document { id: document_id, book_id, title: title.to_owned(), body: String::new() })
    }

    /// Creates a pure grouping Catalog Node at the TOP of its level
    /// (see [`Self::create_document`] for the insertion rule).
    pub fn create_catalog_node(
        &mut self,
        book_id: i64,
        parent_id: Option<i64>,
        title: &str,
    ) -> Result<CatalogNode> {
        let expanded = self.validate_parent(book_id, parent_id)?;
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        let sort_order = top_sort_order(&tx, book_id, parent_id)?;
        tx.execute(
            "INSERT INTO catalog_nodes \
             (book_id, parent_id, title, sort_order, expanded, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![book_id, parent_id, title, sort_order, expanded as i64, now, now],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(CatalogNode {
            id,
            book_id,
            parent_id,
            document_id: None,
            title: title.to_owned(),
            expanded,
            updated_at: now,
        })
    }

    /// The Catalog Node carrying a Document (locate-current in the inner
    /// catalog sidebar resolves a document tab back to its tree row).
    pub fn catalog_node_of_document(&self, document_id: i64) -> Result<Option<CatalogNode>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, book_id, parent_id, document_id, title, expanded, updated_at \
                 FROM catalog_nodes WHERE document_id = ?",
                params![document_id],
                catalog_node_from_row,
            )
            .optional()?)
    }

    /// Loads one Catalog Node (regardless of Trash state).
    pub fn catalog_node(&self, id: i64) -> Result<Option<CatalogNode>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, book_id, parent_id, document_id, title, expanded, updated_at \
                 FROM catalog_nodes WHERE id = ?",
                params![id],
                catalog_node_from_row,
            )
            .optional()?)
    }

    /// The flattened catalog of one book in display order (pre-order),
    /// depths included. Trashed nodes disappear; a node whose parent is
    /// still trashed renders at top level until its parent comes back
    /// (its stored parent link never changes).
    pub fn catalog_outline(&self, book_id: i64) -> Result<Vec<CatalogEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, book_id, parent_id, document_id, title, expanded, updated_at FROM catalog_nodes \
             WHERE book_id = ? AND trashed_at IS NULL ORDER BY sort_order, id",
        )?;
        let nodes = stmt
            .query_map(params![book_id], catalog_node_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(flatten_catalog(nodes))
    }

    /// Persists one node's expand/collapse state (the inner catalog
    /// sidebar re-reads it on every open).
    pub fn set_catalog_node_expanded(&mut self, id: i64, expanded: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE catalog_nodes SET expanded = ?, updated_at = ? WHERE id = ?",
            params![expanded as i64, self.stamp(), id],
        )?;
        Ok(())
    }

    /// The catalog sidebar's expand/collapse-all: one persisted write
    /// for every node of the book.
    pub fn set_catalog_expanded_all(&mut self, book_id: i64, expanded: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE catalog_nodes SET expanded = ?, updated_at = ? WHERE book_id = ? AND trashed_at IS NULL",
            params![expanded as i64, self.stamp(), book_id],
        )?;
        Ok(())
    }

    /// The catalog sidebar's restore section: this book's trashed nodes
    /// (whole subtrees listed by their root row), newest first.
    pub fn trashed_catalog_nodes(&self, book_id: i64) -> Result<Vec<CatalogNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, book_id, parent_id, document_id, title, expanded, updated_at \
             FROM catalog_nodes \
             WHERE book_id = ? AND trashed_at IS NOT NULL \
             AND (parent_id IS NULL OR parent_id NOT IN \
                  (SELECT id FROM catalog_nodes WHERE trashed_at IS NOT NULL)) \
             ORDER BY updated_at DESC, id",
        )?;
        Ok(stmt
            .query_map(params![book_id], catalog_node_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// The Trash screen: every trashed Knowledge Base, newest first.
    pub fn trashed_knowledge_bases(&self) -> Result<Vec<KnowledgeBase>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, icon, color, favorited FROM knowledge_bases \
             WHERE trashed_at IS NOT NULL ORDER BY updated_at DESC, id",
        )?;
        Ok(stmt.query_map([], kb_from_row)?.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Moves a Catalog Node to `new_parent_id` (its own id's level when
    /// `None`) at `index` among the target level's children, renumbering
    /// both affected levels. Rejects a move into the node itself or any
    /// descendant (a cycle), across books, or under a trashed parent
    /// (spec: drag/move reparenting stays a tree).
    pub fn move_catalog_node(
        &mut self,
        id: i64,
        new_parent_id: Option<i64>,
        index: usize,
    ) -> Result<()> {
        let node = self
            .catalog_node(id)?
            .ok_or_else(|| WorkspaceError::InvalidDateTime(format!("no catalog node {id}")))?;
        let _ = self.validate_parent(node.book_id, new_parent_id)?;
        if let Some(parent) = new_parent_id {
            if parent == id {
                return Err(WorkspaceError::InvalidMove(format!("cannot move node {id} into itself")));
            }
            let mut cursor = Some(parent);
            while let Some(current) = cursor {
                if current == id {
                    return Err(WorkspaceError::InvalidMove(format!(
                        "cannot move node {id} into its own descendant {parent}"
                    )));
                }
                cursor = self.catalog_node(current)?.and_then(|ancestor| ancestor.parent_id);
            }
        }
        if node.parent_id == new_parent_id {
            let siblings = self.children_of(node.book_id, new_parent_id)?;
            let mut ids: Vec<i64> =
                siblings.iter().filter(|sibling| sibling.id != id).map(|sibling| sibling.id).collect();
            let at = index.min(ids.len());
            ids.insert(at, id);
            self.renumber_level(node.book_id, new_parent_id, &ids)?;
        } else {
            let old_ids: Vec<i64> = self
                .children_of(node.book_id, node.parent_id)?
                .iter()
                .filter(|sibling| sibling.id != id)
                .map(|sibling| sibling.id)
                .collect();
            let mut new_ids: Vec<i64> = self
                .children_of(node.book_id, new_parent_id)?
                .iter()
                .filter(|sibling| sibling.id != id)
                .map(|sibling| sibling.id)
                .collect();
            let at = index.min(new_ids.len());
            new_ids.insert(at, id);
            let now = self.stamp();
            let tx = self.conn.transaction()?;
            for (parent, ids) in [(node.parent_id, old_ids), (new_parent_id, new_ids)] {
                for (order, child) in ids.iter().enumerate() {
                    tx.execute(
                        "UPDATE catalog_nodes SET parent_id = ?, sort_order = ?, updated_at = ? WHERE id = ?",
                        params![parent, order as f64, now, child],
                    )?;
                }
            }
            tx.commit()?;
        }
        Ok(())
    }

    /// Moves a Catalog Node and its whole descendant subtree into
    /// Trash, atomically (the subtree restores together).
    pub fn trash_catalog_node(&mut self, id: i64) -> Result<()> {
        self.set_catalog_subtree_trashed(id, true)
    }

    /// Restores a Catalog Node and its whole descendant subtree from
    /// Trash, atomically.
    pub fn restore_catalog_node(&mut self, id: i64) -> Result<()> {
        self.set_catalog_subtree_trashed(id, false)
    }

    fn set_catalog_subtree_trashed(&mut self, id: i64, trashed: bool) -> Result<()> {
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        let mut stmt = tx.prepare(
            "WITH RECURSIVE subtree(id) AS ( \
                SELECT id FROM catalog_nodes WHERE id = :id \
                UNION ALL \
                SELECT child.id FROM catalog_nodes child JOIN subtree ON child.parent_id = subtree.id \
            ) \
            SELECT id FROM subtree",
        )?;
        let ids = stmt
            .query_map(&[(":id", &id)], |row| row.get::<_, i64>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(stmt);
        for node_id in &ids {
            if trashed {
                tx.execute(
                    "UPDATE catalog_nodes SET trashed_at = ?, updated_at = ? WHERE id = ? AND trashed_at IS NULL",
                    params![now, now, node_id],
                )?;
            } else {
                tx.execute(
                    "UPDATE catalog_nodes SET trashed_at = NULL, updated_at = ? WHERE id = ?",
                    params![now, node_id],
                )?;
            }
        }
        // The subtree's Documents move with it, atomically.
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let trashed_value: Option<String> = if trashed { Some(now.clone()) } else { None };
        let trashed_clause = if trashed { "trashed_at IS NULL" } else { "trashed_at IS NOT NULL" };
        tx.execute(
            &format!(
                "UPDATE documents SET trashed_at = ?, updated_at = ? \
                 WHERE id IN (SELECT document_id FROM catalog_nodes WHERE id IN ({placeholders})) \
                 AND {trashed_clause}",
            ),
            rusqlite::params_from_iter(
                std::iter::once(Box::new(trashed_value) as Box<dyn rusqlite::ToSql>)
                    .chain(std::iter::once(Box::new(now) as Box<dyn rusqlite::ToSql>))
                    .chain(ids.iter().map(|id| Box::new(*id) as Box<dyn rusqlite::ToSql>))
                    .map(|param| param as Box<dyn rusqlite::ToSql>),
            ),
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Saves a Document's title and body, refreshing its word count
    /// (CJK characters count individually, other alphanumeric runs as
    /// words). The editor milestone (#6) drives this after every edit.
    pub fn save_document(&mut self, id: i64, title: &str, body: &str) -> Result<Document> {
        let words = count_words(body);
        let changed = self.conn.execute(
            "UPDATE documents SET title = ?, body = ?, word_count = ?, updated_at = ? WHERE id = ?",
            params![title, body, words, self.stamp(), id],
        )?;
        if changed == 0 {
            return Err(WorkspaceError::InvalidDateTime(format!("no document {id}")));
        }
        self.document(id)?.ok_or_else(|| WorkspaceError::InvalidDateTime(format!("no document {id}")))
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
        self.conn.execute(
            "INSERT INTO document_revisions (document_id, title, body, name, created_at) \
             VALUES (?, ?, ?, ?, ?)",
            params![document_id, document.title, document.body, name, self.stamp()],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(DocumentRevision { id, document_id, title: document.title, body: document.body, name: name.map(str::to_owned) })
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
        })
    }

    /// Loads a Document.
    pub fn document(&self, id: i64) -> Result<Option<Document>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, book_id, title, body FROM documents WHERE id = ?",
                params![id],
                |row| {
                    Ok(Document {
                        id: row.get(0)?,
                        book_id: row.get(1)?,
                        title: row.get(2)?,
                        body: row.get(3)?,
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

    /// A parent must exist in the same book and not be trashed; returns
    /// the new child's initial expand state (first level expands, deeper
    /// levels collapse — the first-open default).
    fn validate_parent(&self, book_id: i64, parent_id: Option<i64>) -> Result<bool> {
        match parent_id {
            None => Ok(true),
            Some(parent) => {
                let parent = self.catalog_node(parent)?.ok_or_else(|| {
                    WorkspaceError::InvalidMove(format!("no parent catalog node {parent}"))
                })?;
                if parent.book_id != book_id {
                    return Err(WorkspaceError::InvalidMove(format!(
                        "parent {} belongs to book {}, not {book_id}",
                        parent.id, parent.book_id
                    )));
                }
                let trashed = self.conn.query_row(
                    "SELECT trashed_at IS NOT NULL FROM catalog_nodes WHERE id = ?",
                    params![parent.id],
                    |row| row.get::<_, bool>(0),
                )?;
                if trashed {
                    return Err(WorkspaceError::InvalidMove(format!(
                        "parent {} is in Trash",
                        parent.id
                    )));
                }
                Ok(false)
            }
        }
    }

    /// The children of one level (direct children of `parent_id` in
    /// `book_id`), in display order.
    fn children_of(&self, book_id: i64, parent_id: Option<i64>) -> Result<Vec<CatalogNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, book_id, parent_id, document_id, title, expanded, updated_at FROM catalog_nodes \
             WHERE book_id = ? AND parent_id IS ? AND trashed_at IS NULL ORDER BY sort_order, id",
        )?;
        Ok(stmt
            .query_map(params![book_id, parent_id], catalog_node_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Renumbers one level's children 0..n in display order and points
    /// them at `parent_id` (a single transaction).
    fn renumber_level(&mut self, book_id: i64, parent_id: Option<i64>, ids: &[i64]) -> Result<()> {
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        for (order, child) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE catalog_nodes SET parent_id = ?, sort_order = ?, updated_at = ? \
                 WHERE id = ? AND book_id = ?",
                params![parent_id, order as f64, now, child, book_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

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

fn kb_from_row(row: &Row<'_>) -> std::result::Result<KnowledgeBase, rusqlite::Error> {
    Ok(KnowledgeBase {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        icon: row.get(3)?,
        color: row.get(4)?,
        favorited: row.get::<_, i64>(5)? != 0,
    })
}

fn catalog_node_from_row(row: &Row<'_>) -> std::result::Result<CatalogNode, rusqlite::Error> {
    Ok(CatalogNode {
        id: row.get(0)?,
        book_id: row.get(1)?,
        parent_id: row.get(2)?,
        document_id: row.get(3)?,
        title: row.get(4)?,
        expanded: row.get::<_, i64>(5)? != 0,
        updated_at: row.get(6)?,
    })
}

/// The `sort_order` that puts a new sibling at the TOP of its level:
/// one step ahead of the current first entry.
fn top_sort_order(
    tx: &rusqlite::Transaction<'_>,
    book_id: i64,
    parent_id: Option<i64>,
) -> Result<f64> {
    let min: Option<f64> = tx.query_row(
        "SELECT MIN(sort_order) FROM catalog_nodes \
         WHERE book_id = ? AND parent_id IS ? AND trashed_at IS NULL",
        params![book_id, parent_id],
        |row| row.get(0),
    )?;
    Ok(min.map(|min| min - 1.0).unwrap_or(0.0))
}

/// Groups flat nodes into a pre-order outline with depths. A node whose
/// parent is not in the set (a trashed ancestor) renders at top level;
/// every visible chain is walked from there.
fn flatten_catalog(nodes: Vec<CatalogNode>) -> Vec<CatalogEntry> {
    use std::collections::HashMap;
    let ids: HashMap<i64, ()> = nodes.iter().map(|node| (node.id, ())).collect();
    let mut children: HashMap<Option<i64>, Vec<&CatalogNode>> = HashMap::new();
    for node in &nodes {
        children.entry(node.parent_id).or_default().push(node);
    }
    let mut outline = Vec::new();
    fn walk(
        children: &HashMap<Option<i64>, Vec<&CatalogNode>>,
        parent: Option<i64>,
        depth: usize,
        outline: &mut Vec<CatalogEntry>,
    ) {
        let Some(level) = children.get(&parent) else { return };
        for node in level {
            outline.push(CatalogEntry { node: (*node).clone(), depth });
            walk(children, Some(node.id), depth + 1, outline);
        }
    }
    for node in &nodes {
        let visible_root = match node.parent_id {
            None => true,
            Some(parent) => !ids.contains_key(&parent),
        };
        if visible_root {
            outline.push(CatalogEntry { node: (*node).clone(), depth: 0 });
            walk(&children, Some(node.id), 1, &mut outline);
        }
    }
    outline
}

/// Yuque-style word count: every CJK character counts as one word, each
/// maximal run of other alphanumeric characters counts as one.
pub fn count_words(body: &str) -> i64 {
    let cjk = |ch: char| {
        ('\u{4e00}'..='\u{9fff}').contains(&ch)
            || ('\u{3400}'..='\u{4dbf}').contains(&ch)
            || ('\u{f900}'..='\u{faff}').contains(&ch)
            || ('\u{3040}'..='\u{30ff}').contains(&ch)
            || ('\u{ac00}'..='\u{d7af}').contains(&ch)
    };
    let mut count = 0_i64;
    let mut in_latin_run = false;
    for ch in body.chars() {
        if cjk(ch) {
            count += 1;
            in_latin_run = false;
        } else if ch.is_alphanumeric() {
            if !in_latin_run {
                count += 1;
            }
            in_latin_run = true;
        } else {
            in_latin_run = false;
        }
    }
    count
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
