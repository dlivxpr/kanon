//! Cross-cutting collection queries: the surfaces that aggregate every
//! module at once — 收藏 (Favorites), 标签 (Tags) and the 开始 (Start)
//! view's recent-documents list (docs/ui-design.md §2.2, §4.1;
//! docs/kanon.html `favorites` / `tags` / `startView`).
//!
//! 收藏 includes live Knowledge Bases, Documents and Projects. Tags
//! attach to Tasks, Documents and Quick Notes.

use rusqlite::{Row, params, params_from_iter};

use crate::domain::{Tag, TaskAggregate};

use super::{Document, QuickNote, Result, Workspace, WorkspaceError};

// The parent module's private row mapper: this module is a descendant
// of `workspace`, so its SELECT shape composes here.
use super::task_from_row;

/// Which module a 收藏 row comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FavoriteKind {
    /// A Knowledge Base (知识库).
    Book,
    /// A Document (文档).
    Document,
    /// A Project (项目).
    Project,
}

impl FavoriteKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Book => "知识库",
            Self::Document => "文档",
            Self::Project => "项目",
        }
    }
}

/// One live row of 收藏, with the owning book's name for Documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FavoriteEntry {
    pub kind: FavoriteKind,
    pub id: i64,
    pub title: String,
    /// The owning Knowledge Base's name (`FavoriteKind::Document` only).
    pub book_name: Option<String>,
    pub updated_at: String,
}

/// One 标签 cloud chip's data: the Tag plus how many live rows carry it
/// in each module it can attach to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagUsage {
    pub tag: Tag,
    pub tasks: usize,
    pub documents: usize,
    pub notes: usize,
}

impl TagUsage {
    /// Live rows across all three modules (rows in Trash never count).
    pub fn total(&self) -> usize {
        self.tasks + self.documents + self.notes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecentScope {
    Edited,
    Viewed,
    Favorites,
}

/// One 开始-recent row (docs/ui-design.md §4.1): a live Document with
/// the owning Knowledge Base's name and its favorite flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentDocument {
    pub id: i64,
    pub title: String,
    pub book_id: i64,
    pub book_name: String,
    pub updated_at: String,
    pub favorited: bool,
}

impl Workspace {
    /// Live favorites, most recently touched first. A document whose
    /// owning book is in Trash is also omitted.
    pub fn favorites(&self) -> Result<Vec<FavoriteEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT 'book' AS kind, id, name, NULL, updated_at FROM knowledge_bases \
             WHERE favorited = 1 AND trashed_at IS NULL \
             UNION ALL \
             SELECT 'doc' AS kind, documents.id, documents.title, knowledge_bases.name, \
                    documents.updated_at \
             FROM documents JOIN knowledge_bases ON knowledge_bases.id = documents.book_id \
             WHERE documents.favorited = 1 AND documents.trashed_at IS NULL \
               AND knowledge_bases.trashed_at IS NULL \
             UNION ALL \
             SELECT 'project' AS kind, id, name, NULL, updated_at FROM projects \
             WHERE favorited = 1 AND trashed_at IS NULL \
             ORDER BY updated_at DESC, kind, id",
        )?;
        let rows = stmt.query_map([], |row: &Row<'_>| {
            let kind = match row.get_ref(0)?.as_str()? {
                "doc" => FavoriteKind::Document,
                "project" => FavoriteKind::Project,
                _ => FavoriteKind::Book,
            };
            Ok(FavoriteEntry {
                kind,
                id: row.get(1)?,
                title: row.get(2)?,
                book_name: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub(crate) fn project_is_favorite(&self, id: i64) -> Result<bool> {
        use rusqlite::OptionalExtension;
        Ok(self
            .conn
            .query_row(
                "SELECT favorited FROM projects WHERE id = ?1 AND trashed_at IS NULL",
                [id],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(false))
    }

    pub(crate) fn set_project_favorite(&mut self, id: i64, favorited: bool) -> Result<()> {
        let changed = self.conn.execute(
            "UPDATE projects SET favorited = ?1 WHERE id = ?2 AND trashed_at IS NULL",
            params![favorited, id],
        )?;
        if changed == 0 {
            return Err(WorkspaceError::NotFound(format!("project {id}")));
        }
        Ok(())
    }

    pub fn search_documents(&self, needle: &str, limit: u32) -> Result<Vec<(i64, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT d.id, d.title FROM documents d JOIN knowledge_bases b ON b.id = d.book_id
             WHERE d.trashed_at IS NULL AND b.trashed_at IS NULL AND (
                 instr(lower(d.title), ?1) > 0 OR instr(lower(d.body), ?1) > 0 OR EXISTS (
                     SELECT 1 FROM document_tags dt JOIN tags t ON t.id = dt.tag_id
                     WHERE dt.document_id = d.id AND instr(lower(t.name), trim(?1, '#')) > 0
                 )
             ) ORDER BY (instr(lower(d.title), ?1) > 0) DESC, d.updated_at DESC, d.id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![needle, limit], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn document_is_favorite(&self, id: i64) -> Result<bool> {
        use rusqlite::OptionalExtension;
        Ok(self.conn.query_row("SELECT d.favorited FROM documents d JOIN knowledge_bases b ON b.id = d.book_id WHERE d.id = ? AND d.trashed_at IS NULL AND b.trashed_at IS NULL", [id], |row| row.get(0)).optional()?.unwrap_or(false))
    }

    /// The 收藏 star's write on Document rows. Refuses (with `NotFound`)
    /// when the Document is gone, sits in Trash, or its Knowledge Base
    /// sits in Trash — a trashed row is restored through the Trash
    /// screen, never edited. The content `updated_at` is left alone:
    /// favoriting is not a content write, and the 最近编辑 list sorts
    /// by it.
    pub fn set_document_favorite(&mut self, id: i64, favorited: bool) -> Result<()> {
        let visible = self.conn.query_row(
            "SELECT COUNT(*) FROM documents \
             JOIN knowledge_bases ON knowledge_bases.id = documents.book_id \
             WHERE documents.id = ? AND documents.trashed_at IS NULL \
               AND knowledge_bases.trashed_at IS NULL",
            params![id],
            |row| row.get::<_, i64>(0),
        )? > 0;
        if !visible {
            return Err(WorkspaceError::NotFound(format!("document {id}")));
        }
        self.conn.execute(
            "UPDATE documents SET favorited = ? WHERE id = ?",
            params![favorited as i64, id],
        )?;
        Ok(())
    }

    pub(crate) fn tag_name(&self, id: i64) -> Result<Option<String>> {
        use rusqlite::OptionalExtension;
        Ok(self
            .conn
            .query_row("SELECT name FROM tags WHERE id = ?", [id], |row| row.get(0))
            .optional()?)
    }

    /// Rename a Tag in place or merge into an existing one. Return the surviving id.
    pub(crate) fn rename_tag(&mut self, old_id: i64, new_name: &str) -> Result<i64> {
        use rusqlite::OptionalExtension;

        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err(WorkspaceError::InvalidOperation("tag name is empty".into()));
        }
        let tx = self.conn.transaction()?;
        let find = |name: &str| -> Result<Option<i64>> {
            Ok(tx
                .query_row("SELECT id FROM tags WHERE name = ?", [name], |row| {
                    row.get(0)
                })
                .optional()?)
        };
        if !tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE id = ?)",
            [old_id],
            |row| row.get::<_, bool>(0),
        )? {
            return Err(WorkspaceError::NotFound(format!("tag {old_id}")));
        }
        let target = find(new_name)?;
        match target {
            None => {
                tx.execute(
                    "UPDATE tags SET name = ? WHERE id = ?",
                    params![new_name, old_id],
                )?;
            }
            Some(id) if id != old_id => {
                for (copy, remove) in [
                    (
                        "INSERT OR IGNORE INTO task_tags SELECT task_id, ?1 FROM task_tags WHERE tag_id = ?2",
                        "DELETE FROM task_tags WHERE tag_id = ?",
                    ),
                    (
                        "INSERT OR IGNORE INTO document_tags SELECT document_id, ?1 FROM document_tags WHERE tag_id = ?2",
                        "DELETE FROM document_tags WHERE tag_id = ?",
                    ),
                    (
                        "INSERT OR IGNORE INTO quick_note_tags SELECT quick_note_id, ?1 FROM quick_note_tags WHERE tag_id = ?2",
                        "DELETE FROM quick_note_tags WHERE tag_id = ?",
                    ),
                ] {
                    tx.execute(copy, params![id, old_id])?;
                    tx.execute(remove, [old_id])?;
                }
                tx.execute("DELETE FROM tags WHERE id = ?", [old_id])?;
            }
            Some(_) => {}
        }
        tx.commit()?;
        Ok(target.unwrap_or(old_id))
    }

    /// Every Tag with its live usage counts, name order (the cloud).
    /// A Tag whose rows all went to Trash keeps its chip with zero
    /// counts; selecting it shows the honest empty bucket.
    pub fn tag_usages(&self) -> Result<Vec<TagUsage>> {
        let mut stmt = self.conn.prepare(
            "SELECT tags.id, tags.name, tags.color, \
                (SELECT COUNT(*) FROM task_tags \
                   JOIN tasks ON tasks.id = task_tags.task_id \
                  WHERE task_tags.tag_id = tags.id AND tasks.trashed_at IS NULL), \
                (SELECT COUNT(*) FROM document_tags \
                   JOIN documents ON documents.id = document_tags.document_id \
                   JOIN knowledge_bases ON knowledge_bases.id = documents.book_id \
                  WHERE document_tags.tag_id = tags.id AND documents.trashed_at IS NULL \
                    AND knowledge_bases.trashed_at IS NULL), \
                (SELECT COUNT(*) FROM quick_note_tags \
                   JOIN quick_notes ON quick_notes.id = quick_note_tags.quick_note_id \
                  WHERE quick_note_tags.tag_id = tags.id AND quick_notes.trashed_at IS NULL) \
             FROM tags ORDER BY tags.name",
        )?;
        let rows = stmt.query_map([], |row: &Row<'_>| {
            Ok(TagUsage {
                tag: Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                },
                tasks: row.get::<_, i64>(3)? as usize,
                documents: row.get::<_, i64>(4)? as usize,
                notes: row.get::<_, i64>(5)? as usize,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// The live Tasks carrying the Tag, in the canonical task order.
    pub fn tasks_for_tag(&self, tag_id: i64) -> Result<Vec<TaskAggregate>> {
        let mut stmt = self.conn.prepare(
            "SELECT tasks.* FROM tasks JOIN task_tags ON task_tags.task_id = tasks.id \
             WHERE task_tags.tag_id = ? AND tasks.trashed_at IS NULL \
             ORDER BY tasks.sort_order, tasks.id",
        )?;
        let tasks = stmt
            .query_map(params![tag_id], task_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        tasks
            .into_iter()
            .map(|task| self.with_details(task))
            .collect()
    }

    /// The live Documents carrying the Tag, most recently updated first.
    pub fn documents_for_tag(&self, tag_id: i64) -> Result<Vec<Document>> {
        let mut stmt = self.conn.prepare(
            "SELECT documents.id, documents.book_id, documents.title, documents.body, \
                    documents.updated_at, documents.word_count, documents.version \
             FROM documents JOIN document_tags ON document_tags.document_id = documents.id \
             JOIN knowledge_bases ON knowledge_bases.id = documents.book_id \
             WHERE document_tags.tag_id = ? AND documents.trashed_at IS NULL \
               AND knowledge_bases.trashed_at IS NULL \
             ORDER BY documents.updated_at DESC, documents.id DESC",
        )?;
        let rows = stmt.query_map(params![tag_id], document_from_row)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// The live Quick Notes carrying the Tag, newest first. Archived
    /// notes are still tagged content and appear (the usage counts and
    /// the bucket stay consistent); trashed notes never do.
    pub fn quick_notes_for_tag(&self, tag_id: i64) -> Result<Vec<QuickNote>> {
        let mut stmt = self.conn.prepare(
            "SELECT quick_notes.id, quick_notes.content, quick_notes.pinned, quick_notes.archived \
             FROM quick_notes JOIN quick_note_tags ON quick_note_tags.quick_note_id = quick_notes.id \
             WHERE quick_note_tags.tag_id = ? AND quick_notes.trashed_at IS NULL \
             ORDER BY quick_notes.created_at DESC, quick_notes.id DESC",
        )?;
        let rows = stmt.query_map(params![tag_id], |row: &Row<'_>| {
            Ok(QuickNote {
                id: row.get(0)?,
                content: row.get(1)?,
                pinned: row.get::<_, i64>(2)? != 0,
                archived: row.get::<_, i64>(3)? != 0,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Records a visit independently of edits. A sequence preserves order
    /// even when several documents are opened in the same clock minute.
    pub fn record_document_visit(&self, id: i64) -> Result<bool> {
        Ok(self.conn.execute(
            "INSERT INTO document_visits(document_id, visit_order)
             SELECT d.id, (SELECT COALESCE(MAX(visit_order), 0) + 1 FROM document_visits)
             FROM documents d JOIN knowledge_bases b ON b.id = d.book_id
             WHERE d.id = ?1 AND d.trashed_at IS NULL AND b.trashed_at IS NULL
             ON CONFLICT(document_id) DO UPDATE SET visit_order = excluded.visit_order",
            [id],
        )? != 0)
    }

    /// Returns live documents ordered by edit or visit recency, optionally
    /// restricted to a book. Unvisited documents never appear under Viewed.
    pub fn recent_documents(
        &self,
        book: Option<i64>,
        scope: RecentScope,
        limit: usize,
    ) -> Result<Vec<RecentDocument>> {
        let mut sql = String::from(
            "SELECT documents.id, documents.title, documents.book_id, knowledge_bases.name, \
                    documents.updated_at, documents.favorited \
             FROM documents JOIN knowledge_bases ON knowledge_bases.id = documents.book_id",
        );
        if scope == RecentScope::Viewed {
            sql.push_str(" JOIN document_visits ON document_visits.document_id = documents.id");
        }
        sql.push_str(" WHERE documents.trashed_at IS NULL AND knowledge_bases.trashed_at IS NULL");
        let mut values: Vec<i64> = Vec::new();
        if let Some(book_id) = book {
            sql.push_str(" AND documents.book_id = ?");
            values.push(book_id);
        }
        if scope == RecentScope::Favorites {
            sql.push_str(" AND documents.favorited = 1");
        }
        sql.push_str(match scope {
            RecentScope::Viewed => " ORDER BY document_visits.visit_order DESC LIMIT ?",
            _ => " ORDER BY documents.updated_at DESC, documents.id DESC LIMIT ?",
        });
        values.push(limit.min(i64::MAX as usize) as i64);
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(values), |row: &Row<'_>| {
            Ok(RecentDocument {
                id: row.get(0)?,
                title: row.get(1)?,
                book_id: row.get(2)?,
                book_name: row.get(3)?,
                updated_at: row.get(4)?,
                favorited: row.get::<_, i64>(5)? != 0,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
}

/// The `documents` SELECT column order shared by the per-tag and
/// recent-list queries (the same shape `Workspace::document` maps).
fn document_from_row(row: &Row<'_>) -> std::result::Result<Document, rusqlite::Error> {
    Ok(Document {
        id: row.get(0)?,
        book_id: row.get(1)?,
        title: row.get(2)?,
        body: row.get(3)?,
        updated_at: row.get(4)?,
        word_count: row.get::<_, i64>(5)? as u64,
        version: row.get(6)?,
    })
}
