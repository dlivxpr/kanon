//! The complete local-first SQLite schema (ADR-0003).
//!
//! Built upfront so every cross-module invariant holds from day one:
//! Knowledge Bases, Catalog Nodes, Documents, revisions, references,
//! Projects, Milestones, Tasks, subtasks, Quick Notes, Tags and
//! preferences live in one database and commit atomically.

/// The schema version `user_version` is set to after migration.
pub const SCHEMA_VERSION: i32 = 5;
/// SQLite file identity for recognizing Kanon workspace backups.
pub const APPLICATION_ID: i32 = 0x4b41_4e4f; // KANO

/// The complete workspace schema, applied inside one transaction.
pub const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS knowledge_bases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    icon TEXT NOT NULL DEFAULT '📕',
    color TEXT NOT NULL DEFAULT '#547093',
    favorited INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    trashed_at TEXT
);

CREATE TABLE IF NOT EXISTS catalog_nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL REFERENCES knowledge_bases(id),
    parent_id INTEGER REFERENCES catalog_nodes(id),
    document_id INTEGER REFERENCES documents(id),
    title TEXT NOT NULL DEFAULT '',
    sort_order REAL NOT NULL DEFAULT 0,
    expanded INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    trashed_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_catalog_nodes_book ON catalog_nodes(book_id, parent_id);

CREATE TABLE IF NOT EXISTS documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL REFERENCES knowledge_bases(id),
    title TEXT NOT NULL,
    body TEXT NOT NULL DEFAULT '',
    word_count INTEGER NOT NULL DEFAULT 0,
    favorited INTEGER NOT NULL DEFAULT 0,
    version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    trashed_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_documents_book ON documents(book_id);

CREATE INDEX IF NOT EXISTS idx_catalog_nodes_document ON catalog_nodes(document_id);
CREATE TRIGGER IF NOT EXISTS sync_catalog_document_title
AFTER UPDATE OF title ON documents
WHEN OLD.title IS NOT NEW.title
BEGIN
    UPDATE catalog_nodes SET title = NEW.title WHERE document_id = NEW.id;
END;

CREATE TABLE IF NOT EXISTS document_visits (
    document_id INTEGER PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
    visit_order INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_document_visit_order ON document_visits(visit_order);

CREATE TABLE IF NOT EXISTS document_revisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id INTEGER NOT NULL REFERENCES documents(id),
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    name TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_document_revisions_doc ON document_revisions(document_id);

CREATE TABLE IF NOT EXISTS document_references (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id INTEGER NOT NULL REFERENCES documents(id),
    ref_type TEXT NOT NULL,
    target_kind TEXT NOT NULL,
    target_id INTEGER,
    target_label TEXT NOT NULL DEFAULT '',
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_document_references_doc ON document_references(document_id);
CREATE INDEX IF NOT EXISTS idx_document_references_target ON document_references(target_kind, target_id);

CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    color TEXT NOT NULL DEFAULT '#547093',
    emoji TEXT NOT NULL DEFAULT '📌',
    linked_book_id INTEGER REFERENCES knowledge_bases(id),
    linked_group_node_id INTEGER REFERENCES catalog_nodes(id),
    sort_order INTEGER NOT NULL DEFAULT 0,
    favorited INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    trashed_at TEXT
);

CREATE TABLE IF NOT EXISTS milestones (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    title TEXT NOT NULL,
    target_date TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_milestones_project ON milestones(project_id);

CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'todo' CHECK (status IN ('todo','doing','done','cancel')),
    priority TEXT NOT NULL DEFAULT 'none' CHECK (priority IN ('none','low','medium','high')),
    memo TEXT NOT NULL DEFAULT '',
    project_id INTEGER REFERENCES projects(id),
    milestone_id INTEGER REFERENCES milestones(id),
    scheduled_date TEXT,
    scheduled_time TEXT,
    due_date TEXT,
    due_time TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    trashed_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_tasks_scheduled ON tasks(scheduled_date);
CREATE INDEX IF NOT EXISTS idx_tasks_due ON tasks(due_date);
CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_id);

CREATE TABLE IF NOT EXISTS task_subtasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL REFERENCES tasks(id),
    title TEXT NOT NULL,
    done INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_task_subtasks_task ON task_subtasks(task_id);

CREATE TABLE IF NOT EXISTS quick_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL,
    pinned INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    trashed_at TEXT
);

CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL DEFAULT '#547093',
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS task_tags (
    task_id INTEGER NOT NULL REFERENCES tasks(id),
    tag_id INTEGER NOT NULL REFERENCES tags(id),
    PRIMARY KEY (task_id, tag_id)
);

CREATE TABLE IF NOT EXISTS document_tags (
    document_id INTEGER NOT NULL REFERENCES documents(id),
    tag_id INTEGER NOT NULL REFERENCES tags(id),
    PRIMARY KEY (document_id, tag_id)
);

CREATE TABLE IF NOT EXISTS quick_note_tags (
    quick_note_id INTEGER NOT NULL REFERENCES quick_notes(id),
    tag_id INTEGER NOT NULL REFERENCES tags(id),
    PRIMARY KEY (quick_note_id, tag_id)
);

CREATE TABLE IF NOT EXISTS preferences (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
";
