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
    /// The workspace file could not be prepared (its directory is not
    /// creatable, and the like).
    Io(std::io::Error),
    /// A referenced entity (Knowledge Base, Catalog Node, Document,
    /// revision, Project, Task) does not exist.
    NotFound(String),
    /// An optimistic write whose base `updated_at` no longer matches the
    /// row: someone else committed first; retry against a fresh base.
    StaleDocument,
    /// A catalog move that would create a cycle, cross books, or attach
    /// to a trashed parent.
    InvalidMove(String),
    /// A Checklist Item promotion whose editor body holds no Linked Task
    /// Card placeholder (or otherwise cannot commit).
    InvalidPromotion(String),
    /// A project-scoped operation whose precondition does not hold
    /// (creating a Document for an unbound Project, or into a trashed
    /// grouping node).
    InvalidOperation(String),
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(err) => write!(f, "SQLite error: {err}"),
            Self::InvalidDateTime(value) => write!(f, "invalid date/time value: {value}"),
            Self::Io(err) => write!(f, "workspace I/O error: {err}"),
            Self::NotFound(what) => write!(f, "not found: {what}"),
            Self::StaleDocument => write!(f, "the document changed since this edit began"),
            Self::InvalidMove(value) => write!(f, "invalid catalog move: {value}"),
            Self::InvalidPromotion(value) => write!(f, "invalid checklist promotion: {value}"),
            Self::InvalidOperation(value) => write!(f, "invalid operation: {value}"),
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

/// The atomic outcome of promoting a Document's Checklist Item into a
/// Task (ADR-0002): the created Task, the committed Document whose body
/// now holds the Linked Task Card, and the new reference index row — all
/// inside the one SQLite transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct Promotion {
    pub task_id: i64,
    pub document: Document,
    pub reference: DocumentReference,
}

/// A backlink: a reference in another Document pointing at a target
/// (Document or Task), for the right panel's Links & Tasks section.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentBacklink {
    pub reference: DocumentReference,
    pub source_document_id: i64,
    pub source_title: String,
    /// Whether the source Document sits in Trash: the backlink degrades
    /// (renders inactive) and recovers on restore.
    pub source_trashed: bool,
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

/// A partial edit of a Project. `Some(Some(v))` sets, `Some(None)`
/// clears, `None` leaves the field untouched — clearing the binding
/// columns is how a Project comes back to unbound (ADR-0001).
#[derive(Debug, Clone, Default)]
pub struct ProjectPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub emoji: Option<String>,
    pub linked_book_id: Option<Option<i64>>,
    pub linked_group_node_id: Option<Option<i64>>,
}

/// Where a Project's Documents live (ADR-0001): always inside one
/// Knowledge Base, optionally narrowed to one grouping Catalog Node.
/// `group_node_id` is `None` when the Project binds the whole book.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectDocsTarget {
    pub book_id: i64,
    pub group_node_id: Option<i64>,
}

/// How a new Project binds its Knowledge Base (ADR-0001): not at all, to
/// a whole existing book, to one grouping Catalog Node (its book is
/// resolved and stored alongside), or one-click to a brand-new
/// same-named Knowledge Base created atomically with the Project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectBinding {
    None,
    Book(i64),
    Node(i64),
    NewBook,
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

    /// Every row across all groups (unfiltered length).
    pub fn total(&self) -> usize {
        self.total_active() + self.done.len() + self.cancelled.len()
    }

    /// The rows matching `filter`, still grouped with mutual exclusion.
    pub fn filtered(&self, filter: &TaskFilter) -> TaskGroups {
        let keeps = |aggregate: &&TaskAggregate| filter.matches(aggregate);
        TaskGroups {
            overdue: self.overdue.iter().filter(keeps).cloned().collect(),
            today: self.today.iter().filter(keeps).cloned().collect(),
            upcoming: self.upcoming.iter().filter(keeps).cloned().collect(),
            inbox: self.inbox.iter().filter(keeps).cloned().collect(),
            unscheduled: self.unscheduled.iter().filter(keeps).cloned().collect(),
            done: self.done.iter().filter(keeps).cloned().collect(),
            cancelled: self.cancelled.iter().filter(keeps).cloned().collect(),
        }
    }

    /// The same groups with each group's rows re-ordered by `sort`
    /// (the storage order is the manual order).
    pub fn sorted(&self, sort: TaskSort) -> TaskGroups {
        let mut groups = self.clone();
        match sort {
            TaskSort::Manual => {}
            TaskSort::CreatedAt => {
                for rows in groups.lists_mut() {
                    rows.sort_by_key(|a| std::cmp::Reverse(a.task.created_at.clone()));
                }
            }
            TaskSort::DueDate => {
                for rows in groups.lists_mut() {
                    rows.sort_by_key(|a| due_key(&a.task));
                }
            }
            TaskSort::Priority => {
                for rows in groups.lists_mut() {
                    rows.sort_by_key(|a| std::cmp::Reverse(priority_rank(a.task.priority)));
                }
            }
        }
        groups
    }

    fn lists_mut(&mut self) -> [&mut Vec<TaskAggregate>; 7] {
        [
            &mut self.overdue,
            &mut self.today,
            &mut self.upcoming,
            &mut self.inbox,
            &mut self.unscheduled,
            &mut self.done,
            &mut self.cancelled,
        ]
    }

    /// Every group in display order (the task picker's candidate list).
    pub fn lists(&self) -> [&Vec<TaskAggregate>; 7] {
        [
            &self.overdue,
            &self.today,
            &self.upcoming,
            &self.inbox,
            &self.unscheduled,
            &self.done,
            &self.cancelled,
        ]
    }
}

/// How the Tasks view narrows rows (spec: filter by status, priority
/// and Project). A `None` field keeps every value.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskFilter {
    pub status: Option<TaskStatus>,
    pub priority: Option<Priority>,
    pub project: ProjectFilter,
}

impl TaskFilter {
    pub fn matches(&self, aggregate: &TaskAggregate) -> bool {
        if self.status.is_some_and(|status| aggregate.task.status != status) {
            return false;
        }
        if self.priority.is_some_and(|priority| aggregate.task.priority != priority) {
            return false;
        }
        match self.project {
            ProjectFilter::All => true,
            ProjectFilter::Unassigned => aggregate.task.project_id.is_none(),
            ProjectFilter::Project(id) => aggregate.task.project_id == Some(id),
        }
    }
}

/// The Project half of a [`TaskFilter`]: every Project, Project-less
/// rows only, or one Project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectFilter {
    #[default]
    All,
    Unassigned,
    Project(i64),
}

/// How the Tasks view orders rows inside each group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskSort {
    /// The storage order (`sort_order`, then id).
    #[default]
    Manual,
    /// Newest creation first.
    CreatedAt,
    /// Due Date first (`None` last), then Due Time with a date-only row
    /// counting as the start of its day.
    DueDate,
    /// High > Medium > Low > None.
    Priority,
}

fn due_key(task: &Task) -> (bool, LocalDate, LocalTime) {
    match task.due_date {
        Some(date) => (false, date, task.due_time.unwrap_or(LocalTime::new(0, 0))),
        None => (true, LocalDate::new(1970, 1, 1), LocalTime::new(0, 0)),
    }
}

fn priority_rank(priority: Priority) -> u8 {
    match priority {
        Priority::None => 0,
        Priority::Low => 1,
        Priority::Medium => 2,
        Priority::High => 3,
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
            std::fs::create_dir_all(parent).map_err(WorkspaceError::Io)?;
        }
        let mut workspace = Self::connect(Connection::open(path)?, clock)?;
        workspace.migrate()?;
        Ok(workspace)
    }

    /// An in-memory workspace (tests and the ephemeral default).
    pub fn open_in_memory() -> Result<Self> {
        Self::open_in_memory_with_clock(Arc::new(SystemClock))
    }

    /// An in-memory workspace with an explicit clock.
    pub fn open_in_memory_with_clock(clock: Arc<dyn Clock>) -> Result<Self> {
        let mut workspace = Self::connect(Connection::open_in_memory()?, clock)?;
        workspace.migrate()?;
        Ok(workspace)
    }

    /// A workspace over an open connection, with the enforcement
    /// pragmas applied (ADR-0001: Documents belong strictly to one
    /// Knowledge Base — the schema's foreign keys actually reject
    /// orphans).
    fn connect(conn: Connection, clock: Arc<dyn Clock>) -> Result<Self> {
        conn.pragma_update(None, "foreign_keys", true)?;
        Ok(Self { conn, clock })
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
        self.task(id)?.ok_or_else(|| WorkspaceError::NotFound(format!("task {id} missing after insert")))
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
        if let Some(milestone_id) = &patch.milestone_id {
            sets.push("milestone_id = ?");
            values.push(milestone_id.map(|id| id.to_string()));
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
            return Err(WorkspaceError::NotFound(format!("task {id}")));
        }
        Ok(self.task(id)?.expect("task exists after update").task)
    }

    /// The quick circle control: toggles strictly between `done` and
    /// `todo` (spec); `doing`/`cancel` go to `done`, `done` goes to `todo`.
    pub fn toggle_task_circle(&mut self, id: i64) -> Result<Task> {
        let current = self
            .task(id)?
            .ok_or_else(|| WorkspaceError::NotFound(format!("task {id}")))?
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

    /// How many tasks sit in the Inbox right now (the top-bar badge
    /// reads this; GLOSSARY: the badge counts Inbox tasks only).
    pub fn inbox_count(&self) -> Result<usize> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE trashed_at IS NULL AND status IN ('todo', 'doing') \
             AND project_id IS NULL AND scheduled_date IS NULL AND due_date IS NULL",
            [],
            |row| row.get::<_, i64>(0),
        )? as usize)
    }

    /// Every non-trashed, non-cancelled Task scheduled strictly after
    /// today, in upcoming order (the Upcoming view's list and board).
    pub fn upcoming_tasks(&self) -> Result<Vec<TaskAggregate>> {
        let today = self.today().to_string();
        let mut stmt = self.conn.prepare(
            "SELECT * FROM tasks WHERE trashed_at IS NULL AND status != 'cancel' \
             AND scheduled_date IS NOT NULL AND scheduled_date > ? \
             ORDER BY scheduled_date, scheduled_time IS NULL, scheduled_time, sort_order, id",
        )?;
        let tasks = stmt
            .query_map(params![today], task_from_row)?
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
                return Err(WorkspaceError::NotFound(format!("knowledge base {id}")));
            }
        }
        self.knowledge_base(id)?.ok_or_else(|| WorkspaceError::NotFound(format!("knowledge base {id}")))
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

    /// Every trashed Catalog Node subtree root across all books (the
    /// Trash screen's rows), newest first — the same root-only filter as
    /// the per-book restore section.
    pub fn trashed_catalog_nodes_all(&self) -> Result<Vec<CatalogNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, book_id, parent_id, document_id, title, expanded, updated_at \
             FROM catalog_nodes \
             WHERE trashed_at IS NOT NULL \
             AND (parent_id IS NULL OR parent_id NOT IN \
                  (SELECT id FROM catalog_nodes WHERE trashed_at IS NOT NULL)) \
             ORDER BY updated_at DESC, id",
        )?;
        Ok(stmt
            .query_map([], catalog_node_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Whether a Catalog Node currently sits in Trash (the Linked
    /// Documents view's recoverable-binding notice reads this).
    pub fn catalog_node_is_trashed(&self, id: i64) -> Result<bool> {
        Ok(self.conn.query_row(
            "SELECT trashed_at IS NOT NULL FROM catalog_nodes WHERE id = ?",
            params![id],
            |row| row.get::<_, bool>(0),
        )?)
    }

    /// Permanently deletes a Catalog Node and its whole subtree — every
    /// descendant node and their Documents (with revisions, references
    /// and tag links) — and resets every Project bound to any node of
    /// the subtree cleanly to unbound, never widening it to the whole
    /// Knowledge Base (ADR-0001). One transaction; not undoable.
    pub fn delete_catalog_node_permanent(&mut self, id: i64) -> Result<()> {
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
        if ids.is_empty() {
            return Err(WorkspaceError::InvalidMove(format!("no catalog node {id}")));
        }
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        // The subtree's documents go with it (their ids are collected
        // before the tree rows go, which the FK ordering demands).
        let doc_ids = tx
            .prepare(&format!(
                "SELECT document_id FROM catalog_nodes WHERE id IN ({placeholders}) AND document_id IS NOT NULL"
            ))?
            .query_map(rusqlite::params_from_iter(ids.iter()), |row| row.get::<_, i64>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let doc_placeholders = doc_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        if !doc_ids.is_empty() {
            for table in ["document_references", "document_revisions", "document_tags"] {
                tx.execute(
                    &format!("DELETE FROM {table} WHERE document_id IN ({doc_placeholders})"),
                    rusqlite::params_from_iter(doc_ids.iter()),
                )?;
            }
        }
        // Projects bound to any node of the subtree reset to unbound.
        for node_id in &ids {
            tx.execute(
                "UPDATE projects SET linked_book_id = NULL, linked_group_node_id = NULL, updated_at = ? \
                 WHERE linked_group_node_id = ?",
                params![now, node_id],
            )?;
        }
        tx.execute(
            &format!("DELETE FROM catalog_nodes WHERE id IN ({placeholders})"),
            rusqlite::params_from_iter(ids.iter()),
        )?;
        if !doc_ids.is_empty() {
            tx.execute(
                &format!("DELETE FROM documents WHERE id IN ({doc_placeholders})"),
                rusqlite::params_from_iter(doc_ids.iter()),
            )?;
        }
        tx.commit()?;
        Ok(())
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
            .ok_or_else(|| WorkspaceError::NotFound(format!("catalog node {id}")))?;
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

    /// Saves a Document's title and body, refreshing its word count and
    /// advancing the optimistic-locking version (the editor milestone
    /// (#6) drives this after every edit).
    pub fn save_document(&mut self, id: i64, title: &str, body: &str) -> Result<Document> {
        let words = super::document::count_words(body) as i64;
        let changed = self.conn.execute(
            "UPDATE documents SET title = ?, body = ?, word_count = ?, updated_at = ?, \
             version = version + 1 WHERE id = ?",
            params![title, body, words, self.stamp(), id],
        )?;
        if changed == 0 {
            return Err(WorkspaceError::NotFound(format!("document {id}")));
        }
        self.document(id)?.ok_or_else(|| WorkspaceError::NotFound(format!("document {id}")))
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
            .ok_or_else(|| WorkspaceError::NotFound(format!("document {document_id}")))?;
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
            return Err(WorkspaceError::NotFound(format!("document {id}")));
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
    /// count are rewritten and its reference index is rebuilt from the
    /// restored body's Linked Task Card and `@` mention markers, all in
    /// one transaction — the index is a projection of the body, so
    /// rollback brings the cards, mentions and their anchor positions
    /// back exactly (and drops what the restored body does not contain).
    /// No Task — and no other object — is ever touched (ADR-0002).
    pub fn restore_document_revision(&mut self, revision_id: i64) -> Result<Document> {
        let revision = self
            .document_revision(revision_id)?
            .ok_or_else(|| WorkspaceError::NotFound(format!("revision {revision_id}")))?;
        let now = self.stamp();
        let words = super::document::count_words(&revision.body) as i64;
        let tx = self.conn.transaction()?;
        let changed = tx.execute(
            "UPDATE documents SET title = ?, body = ?, word_count = ?, updated_at = ?, \
             version = version + 1 WHERE id = ?",
            params![revision.title, revision.body, words, now, revision.document_id],
        )?;
        if changed == 0 {
            return Err(WorkspaceError::NotFound(format!("document {}", revision.document_id)));
        }
        reconcile_references(&tx, revision.document_id, &revision.body, &now)?;
        tx.commit()?;
        Ok(self.document(revision.document_id)?.expect("document exists after restore"))
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
    // Checklist promotion, Linked Task Cards, `@` mentions (ADR-0002)
    // ------------------------------------------------------------------

    /// Promotes one of a Document's Checklist Items into a real Task:
    /// the Task row, the Document body holding the Linked Task Card
    /// (the editor committed `@task(0)`, substituted here with the real
    /// id), and the `document_references` index row commit inside ONE
    /// SQLite transaction — they never split. `base_version` drives the
    /// single editor's optimistic locking (see
    /// [`Self::update_document_body`]); `project_id = None` lands the
    /// Task in the Inbox.
    pub fn promote_checklist_item(
        &mut self,
        document_id: i64,
        base_version: Option<i64>,
        editor_body: &str,
        title: &str,
        project_id: Option<i64>,
    ) -> Result<Promotion> {
        let title = title.trim();
        if title.is_empty() {
            return Err(WorkspaceError::InvalidPromotion("empty title".to_owned()));
        }
        if !editor_body.contains(super::document::TASK_CARD_PLACEHOLDER_MARKER) {
            return Err(WorkspaceError::InvalidPromotion(
                "the editor body holds no Linked Task Card placeholder".to_owned(),
            ));
        }
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO tasks (title, status, priority, project_id, milestone_id, \
                                scheduled_date, scheduled_time, due_date, due_time, \
                                created_at, updated_at) \
             VALUES (?, 'todo', 'none', ?, NULL, NULL, NULL, NULL, NULL, ?, ?)",
            params![title, project_id, now, now],
        )?;
        let task_id = tx.last_insert_rowid();
        // The placeholder becomes the real card marker in the committed
        // body; nothing else in the body may change.
        let final_body = editor_body.replacen(
            super::document::TASK_CARD_PLACEHOLDER_MARKER,
            &super::document::task_card_marker(task_id),
            1,
        );
        let words = super::document::count_words(&final_body) as i64;
        let changed = match base_version {
            Some(base) => tx.execute(
                "UPDATE documents SET body = ?, word_count = ?, updated_at = ?, \
                 version = version + 1 WHERE id = ? AND version = ?",
                params![final_body, words, now, document_id, base],
            )?,
            None => tx.execute(
                "UPDATE documents SET body = ?, word_count = ?, updated_at = ?, \
                 version = version + 1 WHERE id = ?",
                params![final_body, words, now, document_id],
            )?,
        };
        if changed == 0 {
            if base_version.is_some() && document_exists(&tx, document_id)? {
                return Err(WorkspaceError::StaleDocument);
            }
            return Err(WorkspaceError::NotFound(format!("document {document_id}")));
        }
        reconcile_references(&tx, document_id, &final_body, &now)?;
        tx.commit()?;

        let document = self.document(document_id)?.expect("document exists after promotion");
        let reference = self
            .conn
            .query_row(
                "SELECT id, document_id, ref_type, target_kind, target_id, target_label, position \
                 FROM document_references \
                 WHERE document_id = ? AND ref_type = 'task_card' AND target_id = ?",
                params![document_id, task_id],
                reference_from_row,
            )
            .optional()?
            .ok_or_else(|| {
                WorkspaceError::InvalidPromotion(format!("reference for task {task_id} missing"))
            })?;
        Ok(Promotion { task_id, document, reference })
    }

    /// The editing tab's auto-save: like [`Self::update_document_body`],
    /// plus reconciliation of the `document_references` index against the
    /// committed body's Linked Task Card and `@` mention markers, in the
    /// same transaction. Editor Undo after a promotion therefore reverts
    /// the Document content AND the reference index while the created
    /// Task survives (ADR-0002).
    pub fn update_document_body_and_references(
        &mut self,
        id: i64,
        title: &str,
        body: &str,
        base_version: Option<i64>,
    ) -> Result<Document> {
        let now = self.stamp();
        let words = super::document::count_words(body) as i64;
        let tx = self.conn.transaction()?;
        let changed = match base_version {
            Some(base) => tx.execute(
                "UPDATE documents SET title = ?, body = ?, word_count = ?, updated_at = ?, \
                 version = version + 1 WHERE id = ? AND version = ?",
                params![title, body, words, now, id, base],
            )?,
            None => tx.execute(
                "UPDATE documents SET title = ?, body = ?, word_count = ?, updated_at = ?, \
                 version = version + 1 WHERE id = ?",
                params![title, body, words, now, id],
            )?,
        };
        if changed == 0 {
            if base_version.is_some() && document_exists(&tx, id)? {
                return Err(WorkspaceError::StaleDocument);
            }
            return Err(WorkspaceError::NotFound(format!("document {id}")));
        }
        reconcile_references(&tx, id, body, &now)?;
        tx.commit()?;
        Ok(self.document(id)?.expect("document exists after update"))
    }

    /// Every reference across the workspace pointing at
    /// (`target_kind`, `target_id`) — the backlinks of a Document or a
    /// Task. Trashed source Documents still report (degraded), so the
    /// panel can render the invalid state and recover on restore.
    pub fn documents_referencing(
        &self,
        target_kind: &str,
        target_id: i64,
    ) -> Result<Vec<DocumentBacklink>> {
        let mut stmt = self.conn.prepare(
            "SELECT r.id, r.document_id, r.ref_type, r.target_kind, r.target_id, r.target_label, \
                    r.position, d.title, \
                    (d.trashed_at IS NOT NULL OR NOT EXISTS ( \
                        SELECT 1 FROM catalog_nodes cn \
                        WHERE cn.document_id = d.id AND cn.trashed_at IS NULL \
                    )) \
             FROM document_references r JOIN documents d ON d.id = r.document_id \
             WHERE r.target_kind = ? AND r.target_id = ? \
             ORDER BY d.updated_at DESC, r.id",
        )?;
        Ok(stmt
            .query_map(params![target_kind, target_id], |row| {
                Ok(DocumentBacklink {
                    reference: DocumentReference {
                        id: row.get(0)?,
                        document_id: row.get(1)?,
                        ref_type: row.get(2)?,
                        target_kind: row.get(3)?,
                        target_id: row.get(4)?,
                        target_label: row.get(5)?,
                        position: row.get(6)?,
                    },
                    source_document_id: row.get(1)?,
                    source_title: row.get(7)?,
                    source_trashed: row.get::<_, i64>(8)? != 0,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Documents whose title matches `query` (the `@` mention menu);
    /// trashed documents never surface as mention candidates.
    pub fn document_titles_matching(&self, query: &str, limit: usize) -> Result<Vec<(i64, String)>> {
        let needle = format!("%{}%", query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
        let mut stmt = self.conn.prepare(
            "SELECT id, title FROM documents WHERE trashed_at IS NULL AND title LIKE ? ESCAPE '\\' \
             ORDER BY updated_at DESC, id LIMIT ?",
        )?;
        Ok(stmt
            .query_map(params![needle, limit as i64], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// The Trash screen's task section: every trashed Task, newest first.
    pub fn trashed_tasks(&self) -> Result<Vec<TaskAggregate>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM tasks WHERE trashed_at IS NOT NULL ORDER BY updated_at DESC, id",
        )?;
        let tasks = stmt
            .query_map([], task_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        tasks.into_iter().map(|task| self.with_details(task)).collect()
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

    /// Creates a Project together with its binding choice, atomically
    /// (ADR-0001). `ProjectBinding::Node` resolves and stores the node's
    /// book; `NewBook` creates the same-named Knowledge Base first, so a
    /// failure cannot leave an orphaned book behind.
    pub fn create_project_with_binding(
        &mut self,
        name: &str,
        color: &str,
        emoji: &str,
        binding: ProjectBinding,
    ) -> Result<Project> {
        let (linked_book_id, linked_group_node_id) = match binding {
            ProjectBinding::None => (None, None),
            ProjectBinding::Book(book_id) => {
                let trashed: Option<bool> = self
                    .conn
                    .query_row(
                        "SELECT trashed_at IS NOT NULL FROM knowledge_bases WHERE id = ?",
                        params![book_id],
                        |row| row.get(0),
                    )
                    .optional()?;
                match trashed {
                    None => {
                        return Err(WorkspaceError::InvalidOperation(format!(
                            "no knowledge base {book_id}"
                        )));
                    }
                    Some(true) => {
                        return Err(WorkspaceError::InvalidOperation(format!(
                            "knowledge base {book_id} is in Trash"
                        )));
                    }
                    Some(false) => {}
                }
                (Some(book_id), None)
            }
            ProjectBinding::Node(node_id) => {
                let node = self.catalog_node(node_id)?.ok_or_else(|| {
                    WorkspaceError::InvalidOperation(format!("no catalog node {node_id}"))
                })?;
                if self.catalog_node_is_trashed(node_id)? {
                    return Err(WorkspaceError::InvalidOperation(format!(
                        "catalog node {node_id} is in Trash"
                    )));
                }
                (Some(node.book_id), Some(node_id))
            }
            ProjectBinding::NewBook => {
                // The same-named Knowledge Base and the Project binding
                // commit atomically: a failure cannot leave an orphaned
                // book behind (ADR-0003's cross-object atomicity).
                let now = self.stamp();
                let tx = self.conn.transaction()?;
                tx.execute(
                    "INSERT INTO knowledge_bases (name, created_at, updated_at, sort_order) \
                     VALUES (?, ?, ?, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM knowledge_bases))",
                    params![name, now, now],
                )?;
                let book_id = tx.last_insert_rowid();
                tx.execute(
                    "INSERT INTO projects (name, color, emoji, linked_book_id, linked_group_node_id, \
                                           created_at, updated_at) \
                     VALUES (?, ?, ?, ?, NULL, ?, ?)",
                    params![name, color, emoji, book_id, now, now],
                )?;
                let project_id = tx.last_insert_rowid();
                tx.commit()?;
                return self
                    .project(project_id)?
                    .ok_or_else(|| WorkspaceError::NotFound(format!("project {project_id}")));
            }
        };
        self.create_project(name, color, emoji, linked_book_id, linked_group_node_id)
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

    /// Every trashed Project (the Trash screen's restore rows), newest
    /// first.
    pub fn trashed_projects(&self) -> Result<Vec<Project>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM projects WHERE trashed_at IS NOT NULL ORDER BY updated_at DESC, id")?;
        let projects = stmt
            .query_map([], project_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(projects)
    }

    /// Loads one Project regardless of its Trash state (route titles and
    /// the Trash screen need the trashed rows too).
    pub fn project(&self, id: i64) -> Result<Option<Project>> {
        Ok(self
            .conn
            .query_row("SELECT * FROM projects WHERE id = ?", params![id], project_from_row)
            .optional()?)
    }

    /// Applies a partial edit to a Project. Setting a binding to `None`
    /// is how a Project becomes (or returns to) unbound; the Knowledge
    /// Base and its Documents are never touched by a rebind (ADR-0001).
    pub fn update_project(&mut self, id: i64, patch: &ProjectPatch) -> Result<Project> {
        let mut sets: Vec<&str> = Vec::new();
        let mut values: Vec<Option<String>> = Vec::new();
        if let Some(name) = &patch.name {
            sets.push("name = ?");
            values.push(Some(name.clone()));
        }
        if let Some(description) = &patch.description {
            sets.push("description = ?");
            values.push(Some(description.clone()));
        }
        if let Some(color) = &patch.color {
            sets.push("color = ?");
            values.push(Some(color.clone()));
        }
        if let Some(emoji) = &patch.emoji {
            sets.push("emoji = ?");
            values.push(Some(emoji.clone()));
        }
        if let Some(linked_book_id) = &patch.linked_book_id {
            sets.push("linked_book_id = ?");
            values.push(linked_book_id.map(|book| book.to_string()));
        }
        if let Some(linked_group_node_id) = &patch.linked_group_node_id {
            sets.push("linked_group_node_id = ?");
            values.push(linked_group_node_id.map(|node| node.to_string()));
        }
        if !sets.is_empty() {
            sets.push("updated_at = ?");
            values.push(Some(self.stamp()));
            let mut refs: Vec<&dyn rusqlite::ToSql> =
                values.iter().map(|value| value as &dyn rusqlite::ToSql).collect();
            let id_param: &dyn rusqlite::ToSql = &id;
            refs.push(id_param);
            let sql = format!("UPDATE projects SET {} WHERE id = ?", sets.join(", "));
            let changed = self.conn.execute(&sql, &refs[..])?;
            if changed == 0 {
                return Err(WorkspaceError::NotFound(format!("project {id}")));
            }
        }
        self.project(id)?.ok_or_else(|| WorkspaceError::NotFound(format!("project {id}")))
    }

    /// Every non-trashed Task assigned to the Project, in status order
    /// (`todo > doing > done > cancel`), then shelf order.
    pub fn project_tasks(&self, project_id: i64) -> Result<Vec<TaskAggregate>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM tasks WHERE trashed_at IS NULL AND project_id = ? \
             ORDER BY CASE status WHEN 'todo' THEN 0 WHEN 'doing' THEN 1 WHEN 'done' THEN 2 ELSE 3 END, \
             sort_order, id",
        )?;
        let tasks = stmt
            .query_map(params![project_id], task_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        tasks.into_iter().map(|task| self.with_details(task)).collect()
    }

    /// Moves a Project into Trash. Its Tasks KEEP their `project_id` so
    /// they stay visible in the regular task views and a restore is
    /// lossless (ADR-0001); nothing else moves.
    pub fn trash_project(&mut self, id: i64) -> Result<()> {
        let now = self.stamp();
        self.conn.execute(
            "UPDATE projects SET trashed_at = ?, updated_at = ? WHERE id = ? AND trashed_at IS NULL",
            params![now, now, id],
        )?;
        Ok(())
    }

    /// Restores a Project from Trash. The Tasks never lost their
    /// assignment, so nothing has to be re-linked (ADR-0001).
    pub fn restore_project(&mut self, id: i64) -> Result<()> {
        let now = self.stamp();
        self.conn.execute(
            "UPDATE projects SET trashed_at = NULL, updated_at = ? WHERE id = ?",
            params![now, id],
        )?;
        Ok(())
    }

    /// Permanently deletes a Project: its Tasks are detached (never
    /// deleted) and its Milestones go away with it, while every Knowledge
    /// Base and Document is preserved (ADR-0001). One transaction; not
    /// undoable.
    pub fn delete_project_permanent(&mut self, id: i64) -> Result<()> {
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE tasks SET project_id = NULL, milestone_id = NULL, updated_at = ? \
             WHERE project_id = ?",
            params![now, id],
        )?;
        tx.execute("DELETE FROM milestones WHERE project_id = ?", params![id])?;
        tx.execute("DELETE FROM projects WHERE id = ?", params![id])?;
        tx.commit()?;
        Ok(())
    }

    /// Binds an existing Project to a brand-new same-named Knowledge
    /// Base, atomically (ADR-0001's one-click binding).
    pub fn bind_new_book(&mut self, project_id: i64) -> Result<Project> {
        let project = self
            .project(project_id)?
            .ok_or_else(|| WorkspaceError::NotFound(format!("project {project_id}")))?;
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO knowledge_bases (name, created_at, updated_at, sort_order) \
             VALUES (?, ?, ?, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM knowledge_bases))",
            params![project.name, now, now],
        )?;
        let book_id = tx.last_insert_rowid();
        tx.execute(
            "UPDATE projects SET linked_book_id = ?, linked_group_node_id = NULL, updated_at = ? WHERE id = ?",
            params![book_id, now, project_id],
        )?;
        tx.commit()?;
        self.project(project_id)?.ok_or_else(|| WorkspaceError::NotFound(format!("project {project_id}")))
    }

    /// How many non-trashed Documents live in the Project's linked scope
    /// (the whole book, or the grouping node's subtree) — the Overview's
    /// 关联文档 metric.
    pub fn project_document_count(&self, project_id: i64) -> Result<usize> {
        let Some(target) = self.project_document_target(project_id)? else { return Ok(0) };
        let count = match target.group_node_id {
            Some(node_id) => self.conn.query_row(
                "WITH RECURSIVE subtree(id) AS ( \
                    SELECT id FROM catalog_nodes WHERE id = :id \
                    UNION ALL \
                    SELECT child.id FROM catalog_nodes child JOIN subtree ON child.parent_id = subtree.id \
                 ) \
                 SELECT COUNT(*) FROM documents \
                 WHERE trashed_at IS NULL AND id IN \
                 (SELECT document_id FROM catalog_nodes WHERE id IN (SELECT id FROM subtree) AND document_id IS NOT NULL)",
                &[(":id", &node_id)],
                |row| row.get::<_, i64>(0),
            )?,
            None => self.conn.query_row(
                "SELECT COUNT(*) FROM documents WHERE book_id = ? AND trashed_at IS NULL",
                params![target.book_id],
                |row| row.get::<_, i64>(0),
            )?,
        };
        Ok(count as usize)
    }

    /// Resolves where a Project's Documents live (ADR-0001): the linked
    /// grouping Catalog Node when set, otherwise the whole linked book.
    /// `Ok(None)` means unbound. The target is returned even while the
    /// linked node sits in Trash — the binding survives, and the caller
    /// surfaces the recoverable notice instead of a silent fallback.
    pub fn project_document_target(&self, project_id: i64) -> Result<Option<ProjectDocsTarget>> {
        let project = self
            .project(project_id)?
            .ok_or_else(|| WorkspaceError::NotFound(format!("project {project_id}")))?;
        if let Some(node_id) = project.linked_group_node_id {
            let node = self.catalog_node(node_id)?.ok_or_else(|| {
                WorkspaceError::InvalidOperation(format!("linked grouping node {node_id} is gone"))
            })?;
            return Ok(Some(ProjectDocsTarget { book_id: node.book_id, group_node_id: Some(node_id) }));
        }
        Ok(project.linked_book_id.map(|book_id| ProjectDocsTarget { book_id, group_node_id: None }))
    }

    /// Creates a Document for a Project's Linked Documents view (ADR-0001):
    /// under the linked grouping node, or at the top of the linked book's
    /// root when bound to the whole book. Refused while the linked node
    /// sits in Trash (the UI offers restore or rebind first) and for
    /// unbound Projects.
    pub fn create_document_for_project(&mut self, project_id: i64, title: &str) -> Result<Document> {
        let Some(target) = self.project_document_target(project_id)? else {
            return Err(WorkspaceError::InvalidOperation(format!(
                "project {project_id} has no Knowledge Base binding"
            )));
        };
        if let Some(node_id) = target.group_node_id {
            if self.catalog_node_is_trashed(node_id)? {
                return Err(WorkspaceError::InvalidOperation(format!(
                    "linked grouping node {node_id} is in Trash"
                )));
            }
            self.create_document(target.book_id, Some(node_id), title)
        } else {
            self.create_document(target.book_id, None, title)
        }
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

    /// A Project's Milestones in display order (the Overview's list).
    pub fn milestones(&self, project_id: i64) -> Result<Vec<Milestone>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, title, target_date FROM milestones \
             WHERE project_id = ? ORDER BY sort_order, id",
        )?;
        Ok(stmt
            .query_map(params![project_id], |row| {
                Ok(Milestone {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    title: row.get(2)?,
                    target_date: opt_date(row.get(3)?)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?)
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
    /// levels collapse — the first-open default). With no parent the
    /// book itself must exist (ADR-0001: a Document never lands outside
    /// a Knowledge Base).
    fn validate_parent(&self, book_id: i64, parent_id: Option<i64>) -> Result<bool> {
        match parent_id {
            None => {
                let exists = self
                    .conn
                    .query_row("SELECT 1 FROM knowledge_bases WHERE id = ?", params![book_id], |_| Ok(()))
                    .optional()?
                    .is_some();
                if !exists {
                    return Err(WorkspaceError::NotFound(format!("knowledge base {book_id}")));
                }
                Ok(true)
            }
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

/// Whether `table` already has `column` (a migration guard: `ALTER TABLE
/// ADD COLUMN` fails on databases that were created with it).
fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT name FROM pragma_table_info(?)")?;
    let columns = stmt
        .query_map(params![table], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(columns.iter().any(|name| name == column))
}

/// Whether a Document row exists (any Trash state).
fn document_exists(conn: &Connection, document_id: i64) -> Result<bool> {
    Ok(conn
        .query_row("SELECT 1 FROM documents WHERE id = ?", params![document_id], |_| Ok(()))
        .optional()?
        .is_some())
}

fn reference_from_row(row: &Row<'_>) -> std::result::Result<DocumentReference, rusqlite::Error> {
    Ok(DocumentReference {
        id: row.get(0)?,
        document_id: row.get(1)?,
        ref_type: row.get(2)?,
        target_kind: row.get(3)?,
        target_id: row.get(4)?,
        target_label: row.get(5)?,
        position: row.get(6)?,
    })
}

/// Reconciles a Document's `task_card` and `mention` reference rows with
/// the markers actually anchored in `body`: rows whose target vanished
/// from the body are deleted, new markers are inserted, survivors move
/// to their fresh position and label. Runs on `conn` so the caller
/// commits it inside the same transaction as the body write.
fn reconcile_references(conn: &Connection, document_id: i64, body: &str, now: &str) -> Result<()> {
    use super::document::{scan_mention_markers, scan_task_card_markers};
    let cards = scan_task_card_markers(body);
    let mentions = scan_mention_markers(body);

    let mut stmt = conn.prepare(
        "SELECT id, document_id, ref_type, target_kind, target_id, target_label, position \
         FROM document_references \
         WHERE document_id = ? AND ref_type IN ('task_card', 'mention')",
    )?;
    let existing = stmt
        .query_map(params![document_id], reference_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    // Drop rows whose anchor disappeared from the body (an Undo, a
    // restore, or an edit removed the card or mention).
    for row in &existing {
        let alive = match row.ref_type.as_str() {
            "task_card" => row
                .target_id
                .is_some_and(|id| cards.iter().any(|(card_id, _)| *card_id == id)),
            "mention" => match row.target_kind.as_str() {
                "task" | "project" | "document" => row.target_id.is_some_and(|id| {
                    mentions.iter().any(|m| m.kind == row.target_kind && m.target_id == Some(id))
                }),
                "date" => mentions.iter().any(|m| m.kind == "date" && m.label == row.target_label),
                _ => true,
            },
            _ => true,
        };
        if !alive {
            conn.execute("DELETE FROM document_references WHERE id = ?", params![row.id])?;
        }
    }

    // Upsert every marker still anchored. Card labels denormalize the
    // Task's current title; a permanently deleted Task keeps the row
    // (the invalid placeholder) with its last known label.
    for (task_id, position) in &cards {
        let title = conn
            .query_row("SELECT title FROM tasks WHERE id = ?", params![task_id], |row| {
                row.get::<_, String>(0)
            })
            .unwrap_or_default();
        upsert_reference(
            conn,
            &existing,
            document_id,
            "task_card",
            "task",
            Some(*task_id),
            &title,
            *position,
            now,
        )?;
    }
    for marker in &mentions {
        upsert_reference(
            conn,
            &existing,
            document_id,
            "mention",
            &marker.kind,
            marker.target_id,
            &marker.label,
            marker.position,
            now,
        )?;
    }
    Ok(())
}

/// Inserts or refreshes one reference row of the committed body.
#[allow(clippy::too_many_arguments)]
fn upsert_reference(
    conn: &Connection,
    existing: &[DocumentReference],
    document_id: i64,
    ref_type: &str,
    target_kind: &str,
    target_id: Option<i64>,
    label: &str,
    position: usize,
    now: &str,
) -> Result<()> {
    let found = existing.iter().find(|row| {
        row.ref_type == ref_type && row.target_kind == target_kind && row.target_id == target_id
    });
    match found {
        Some(row) => {
            // A target gone from the database (permanently deleted Task)
            // keeps the row's last known label.
            let label = if label.is_empty() { row.target_label.as_str() } else { label };
            conn.execute(
                "UPDATE document_references SET position = ?, target_label = ? WHERE id = ?",
                params![position as i64, label, row.id],
            )?;
        }
        None => {
            conn.execute(
                "INSERT INTO document_references \
                 (document_id, ref_type, target_kind, target_id, target_label, position, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![document_id, ref_type, target_kind, target_id, label, position as i64, now],
            )?;
        }
    }
    Ok(())
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
        trashed_at: row.get("trashed_at")?,
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
