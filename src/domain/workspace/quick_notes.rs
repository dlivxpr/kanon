//! Quick Notes (小记, GLOSSARY): the capture surface for fragments that
//! are not Tasks or Documents yet — publish, pin, tag, filter, and the
//! practical conversions the note menu offers: into a Knowledge Base
//! Document, into an Inbox Task, and into Trash.
//!
//! Pure Rust over the workspace connection (ADR-0003): the schema
//! already holds `quick_notes`, `tags` and `quick_note_tags`; this
//! module owns the read models and the business rules on top of them.

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::domain::date::LocalDateTime;
use crate::domain::document::count_words;
use crate::domain::task::{Priority, Tag, TaskStatus};
use crate::domain::workspace::{Result, Workspace, WorkspaceError};

/// The persisted Quick Notes stream layout (docs/kanon.html 小记:
/// 时间流 / 网格).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotesLayout {
    /// The time-ordered single column (the default).
    #[default]
    Stream,
    /// The wrapped card grid.
    Grid,
}

impl NotesLayout {
    pub const ALL: [Self; 2] = [Self::Stream, Self::Grid];

    /// The preference value persisted in `preferences`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stream => "stream",
            Self::Grid => "grid",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "grid" => Self::Grid,
            _ => Self::Stream,
        }
    }

    /// The segmented control's label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Stream => "时间流",
            Self::Grid => "网格",
        }
    }
}

/// A Quick Note as the views list it: content, state, timestamps and
/// its Tags (the prototype card shows the date, the pin and the tags).
#[derive(Debug, Clone, PartialEq)]
pub struct QuickNoteRow {
    pub id: i64,
    pub content: String,
    pub pinned: bool,
    pub archived: bool,
    pub created_at: LocalDateTime,
    pub updated_at: LocalDateTime,
    pub tags: Vec<Tag>,
}

/// One display line of a Quick Note's content. A `#`–`####` prefix is a
/// heading, `- [ ]` / `- [x]` a toggleable checklist item, and `**` /
/// `` ` `` mark bold / code runs (docs/kanon.html `noteHTML`).
#[derive(Debug, Clone, PartialEq)]
pub enum NoteLine {
    Check { done: bool, label: String },
    Heading(Vec<NoteSegment>),
    Text(Vec<NoteSegment>),
}

/// An inline run of one display line: plain, `bold`, `code`, or both.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteSegment {
    pub text: String,
    pub bold: bool,
    pub code: bool,
}

/// Splits `content` into display lines (`\n`; CR stripped).
pub fn parse_note_content(content: &str) -> Vec<NoteLine> {
    content.split('\n').map(parse_note_line).collect()
}

/// Classifies one line and keeps its inline marks in display order.
pub fn parse_note_line(line: &str) -> NoteLine {
    let line = line.strip_suffix('\r').unwrap_or(line);
    if let Some((done, label)) = check_marker(line) {
        return NoteLine::Check {
            done,
            label: label.to_owned(),
        };
    }
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    if (1..=4).contains(&hashes) && line[hashes..].starts_with(' ') {
        return NoteLine::Heading(parse_inline(&line[hashes + 1..]));
    }
    NoteLine::Text(parse_inline(line))
}

/// The `- [ ] ` / `- [x] ` (or `* ` bullet) checklist prefix.
fn check_marker(line: &str) -> Option<(bool, &str)> {
    let rest = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))?;
    let rest = rest.strip_prefix('[')?;
    let mark = rest.get(..1)?;
    let label = rest.get(1..)?.strip_prefix("] ")?;
    match mark {
        " " => Some((false, label)),
        "x" | "X" => Some((true, label)),
        _ => None,
    }
}

/// A single left-to-right pass over `**bold**` and `` `code` `` runs;
/// a backtick inside code only closes, a `**` inside code stays literal.
fn parse_inline(text: &str) -> Vec<NoteSegment> {
    fn flush(current: &mut String, segments: &mut Vec<NoteSegment>, bold: bool, code: bool) {
        if !current.is_empty() {
            segments.push(NoteSegment {
                text: std::mem::take(current),
                bold,
                code,
            });
        }
    }
    let mut segments: Vec<NoteSegment> = Vec::new();
    let mut current = String::new();
    let mut bold = false;
    let mut code = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '`' && (code || chars.peek().is_none_or(|&next| next != '`')) {
            flush(&mut current, &mut segments, bold, code);
            code = !code;
        } else if c == '*' && !code && chars.peek() == Some(&'*') {
            chars.next();
            flush(&mut current, &mut segments, bold, code);
            bold = !bold;
        } else {
            current.push(c);
        }
    }
    flush(&mut current, &mut segments, bold, code);
    if segments.is_empty() {
        segments.push(NoteSegment {
            text: String::new(),
            bold: false,
            code: false,
        });
    }
    segments
}

/// Parses a composer tags field (`#a, b，c` — comma, 顿号 or whitespace
/// separated) into clean Tag names, first use wins, order kept.
pub fn parse_tag_names(input: &str) -> Vec<String> {
    let mut names = Vec::new();
    for part in input.split([',', '，', '、', ' ']) {
        let name = part.trim().trim_start_matches('#');
        if !name.is_empty() && !names.iter().any(|existing: &String| existing == name) {
            names.push(name.to_owned());
        }
    }
    names
}

impl Workspace {
    /// The Quick Notes stream (not in Trash, not archived): pinned
    /// first, then newest first, each with its Tags.
    pub fn quick_notes(&self) -> Result<Vec<QuickNoteRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, pinned, archived, created_at, updated_at FROM quick_notes \
             WHERE trashed_at IS NULL AND archived = 0 \
             ORDER BY pinned DESC, created_at DESC, id DESC",
        )?;
        let notes = stmt
            .query_map([], quick_note_row_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        notes
            .into_iter()
            .map(|note| self.with_note_tags(note))
            .collect()
    }

    pub(crate) fn recent_quick_notes(&self, limit: u32) -> Result<Vec<QuickNoteRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, pinned, archived, created_at, updated_at FROM quick_notes \
             WHERE trashed_at IS NULL AND archived = 0 \
             ORDER BY created_at DESC, id DESC LIMIT ?1",
        )?;
        stmt.query_map([limit], quick_note_row_from_row)?
            .map(|row| self.with_note_tags(row?))
            .collect()
    }

    /// The archived notes (converted ones; the archive page reads this),
    /// newest touch first.
    pub fn archived_quick_notes(&self) -> Result<Vec<QuickNoteRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, pinned, archived, created_at, updated_at FROM quick_notes \
             WHERE trashed_at IS NULL AND archived = 1 ORDER BY updated_at DESC, id DESC",
        )?;
        let notes = stmt
            .query_map([], quick_note_row_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        notes
            .into_iter()
            .map(|note| self.with_note_tags(note))
            .collect()
    }

    pub fn trashed_quick_notes(&self) -> Result<Vec<QuickNoteRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, pinned, archived, created_at, updated_at FROM quick_notes
             WHERE trashed_at IS NOT NULL ORDER BY trashed_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], quick_note_row_from_row)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter()
            .map(|note| self.with_note_tags(note))
            .collect()
    }

    /// One Quick Note regardless of its Trash state (menus, conversions).
    pub fn quick_note(&self, id: i64) -> Result<Option<QuickNoteRow>> {
        self.quick_note_by_id(id, true)
    }

    /// A note that may be opened from the workbench, including an archived note.
    pub(crate) fn live_quick_note(&self, id: i64) -> Result<Option<QuickNoteRow>> {
        self.quick_note_by_id(id, false)
    }

    fn quick_note_by_id(&self, id: i64, include_trash: bool) -> Result<Option<QuickNoteRow>> {
        let Some(mut note) = self
            .conn
            .query_row(
                "SELECT id, content, pinned, archived, created_at, updated_at FROM quick_notes \
                 WHERE id = ?1 AND (?2 OR trashed_at IS NULL)",
                params![id, include_trash],
                quick_note_row_from_row,
            )
            .optional()?
        else {
            return Ok(None);
        };
        note.tags = note_tags(&self.conn, id)?;
        Ok(Some(note))
    }

    /// Publishes a Quick Note with its Tags (one transaction).
    pub fn publish_quick_note(&mut self, content: &str, tags: &[String]) -> Result<QuickNoteRow> {
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO quick_notes (content, created_at, updated_at) VALUES (?, ?, ?)",
            params![content, now, now],
        )?;
        let id = tx.last_insert_rowid();
        reconcile_note_tags(&tx, id, tags, &now)?;
        tx.commit()?;
        Ok(self.quick_note(id)?.expect("note exists after publish"))
    }

    /// Rewrites the content and the Tag set of a live note (one
    /// transaction; the composer edit and the tag field land together).
    pub fn update_quick_note(
        &mut self,
        id: i64,
        content: &str,
        tags: &[String],
    ) -> Result<QuickNoteRow> {
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        let changed = tx.execute(
            "UPDATE quick_notes SET content = ?, updated_at = ? \
             WHERE id = ? AND trashed_at IS NULL AND archived = 0",
            params![content, now, id],
        )?;
        if changed == 0 {
            return Err(WorkspaceError::NotFound(format!("quick note {id}")));
        }
        reconcile_note_tags(&tx, id, tags, &now)?;
        tx.commit()?;
        Ok(self.quick_note(id)?.expect("note exists after update"))
    }

    /// Pins or unpins a Quick Note (pinned notes head the stream).
    pub fn set_quick_note_pinned(&mut self, id: i64, pinned: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE quick_notes SET pinned = ?, updated_at = ? WHERE id = ?",
            params![pinned as i64, self.stamp(), id],
        )?;
        Ok(())
    }

    /// Moves a Quick Note to Trash (the note menu's 移入回收站).
    pub fn trash_quick_note(&mut self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE quick_notes SET trashed_at = ?, updated_at = ? \
             WHERE id = ? AND trashed_at IS NULL",
            params![self.stamp(), self.stamp(), id],
        )?;
        Ok(())
    }

    /// Restores a Quick Note from Trash (the Trash screen's restore).
    pub fn restore_quick_note(&mut self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE quick_notes SET trashed_at = NULL, updated_at = ? WHERE id = ?",
            params![self.stamp(), id],
        )?;
        Ok(())
    }

    /// Permanently deletes a Quick Note (the Trash screen's purge; the
    /// note must sit in Trash — purging a live or archived note is
    /// refused).
    pub fn delete_quick_note_permanent(&mut self, id: i64) -> Result<()> {
        let trashed = self
            .conn
            .query_row(
                "SELECT trashed_at IS NOT NULL FROM quick_notes WHERE id = ?",
                params![id],
                |row| row.get::<_, bool>(0),
            )
            .optional()?;
        if trashed != Some(true) {
            return Err(WorkspaceError::InvalidOperation(format!(
                "quick note {id} is not in Trash"
            )));
        }
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM quick_note_tags WHERE quick_note_id = ?",
            params![id],
        )?;
        tx.execute("DELETE FROM quick_notes WHERE id = ?", params![id])?;
        tx.commit()?;
        Ok(())
    }

    /// Flips the `- [ ]` ↔ `- [x]` marker of content line `line`
    /// (0-based, the same indexing `parse_note_content` uses).
    pub fn toggle_quick_note_check(&mut self, id: i64, line: usize) -> Result<()> {
        let note = self.require_active_note(id)?;
        let mut lines: Vec<String> = note.content.split('\n').map(str::to_owned).collect();
        let Some(text) = lines.get_mut(line) else {
            return Err(WorkspaceError::InvalidOperation(format!(
                "quick note {id} has no line {line}"
            )));
        };
        let Some((done, label)) = check_marker(text) else {
            return Err(WorkspaceError::InvalidOperation(format!(
                "quick note {id} line {line} is not a checklist item"
            )));
        };
        *text = format!("- [{}] {}", if done { " " } else { "x" }, label);
        self.conn.execute(
            "UPDATE quick_notes SET content = ?, updated_at = ? WHERE id = ?",
            params![lines.join("\n"), self.stamp(), id],
        )?;
        Ok(())
    }

    /// Converts a Quick Note into a Document of `book_id` (one
    /// transaction): the document lands at the top of the book's root
    /// catalog, titled by the note's first line (42 chars max) with the
    /// content as its body, the Tags carry over, and the note is
    /// archived — not trashed — so it stays findable on the archive
    /// page (ADR-0003). Returns the new document's id.
    pub fn convert_quick_note_to_document(&mut self, id: i64, book_id: i64) -> Result<i64> {
        let note = self.require_active_note(id)?;
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        let book_live = tx
            .query_row(
                "SELECT trashed_at IS NULL FROM knowledge_bases WHERE id = ?",
                params![book_id],
                |row| row.get::<_, bool>(0),
            )
            .optional()?;
        if book_live != Some(true) {
            return Err(WorkspaceError::NotFound(format!(
                "knowledge base {book_id}"
            )));
        }
        let title = note_title(&note);
        let words = count_words(&note.content) as i64;
        tx.execute(
            "INSERT INTO documents (book_id, title, body, word_count, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            params![book_id, title, note.content, words, now, now],
        )?;
        let document_id = tx.last_insert_rowid();
        let sort_order = super::top_sort_order(&tx, book_id, None)?;
        tx.execute(
            "INSERT INTO catalog_nodes \
             (book_id, parent_id, document_id, title, sort_order, expanded, created_at, updated_at) \
             VALUES (?, NULL, ?, ?, ?, 1, ?, ?)",
            params![book_id, document_id, title, sort_order, now, now],
        )?;
        for tag in &note.tags {
            tx.execute(
                "INSERT OR IGNORE INTO document_tags (document_id, tag_id) VALUES (?, ?)",
                params![document_id, tag.id],
            )?;
        }
        tx.execute(
            "UPDATE quick_notes SET archived = 1, updated_at = ? WHERE id = ?",
            params![now, id],
        )?;
        tx.commit()?;
        Ok(document_id)
    }

    /// Extracts a Quick Note's first line into an Inbox Task (no
    /// Project, no dates): the full content becomes the memo and the
    /// Tags carry over; the note stays — extraction copies one
    /// actionable item out of a note that may hold more. The task row,
    /// its memo and its tags commit in ONE transaction (ADR-0003).
    /// Returns the task id.
    pub fn extract_quick_note_task(&mut self, id: i64) -> Result<i64> {
        let note = self.require_active_note(id)?;
        let title = note
            .content
            .split('\n')
            .find(|line| !line.trim().is_empty())
            .map(str::trim)
            .unwrap_or("")
            .to_owned();
        if title.is_empty() {
            return Err(WorkspaceError::InvalidOperation(format!(
                "quick note {id} has no extractable title"
            )));
        }
        let now = self.stamp();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO tasks (title, status, priority, memo, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                title,
                TaskStatus::Todo.as_str(),
                Priority::None.as_str(),
                note.content,
                now,
                now
            ],
        )?;
        let task_id = tx.last_insert_rowid();
        for tag in &note.tags {
            tx.execute(
                "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?, ?)",
                params![task_id, tag.id],
            )?;
        }
        tx.commit()?;
        Ok(task_id)
    }
}

impl Workspace {
    /// The note exists, is not in Trash and is not archived: the
    /// precondition of the stream mutations and the conversions.
    fn require_active_note(&self, id: i64) -> Result<QuickNoteRow> {
        let Some(note) = self.quick_note(id)? else {
            return Err(WorkspaceError::NotFound(format!("quick note {id}")));
        };
        let live = self.conn.query_row(
            "SELECT trashed_at IS NULL FROM quick_notes WHERE id = ?",
            params![id],
            |row| row.get::<_, bool>(0),
        )?;
        if !live {
            return Err(WorkspaceError::NotFound(format!(
                "quick note {id} is in Trash"
            )));
        }
        if note.archived {
            return Err(WorkspaceError::InvalidOperation(format!(
                "quick note {id} is archived"
            )));
        }
        Ok(note)
    }
}

/// The document title of a conversion: the first line, 42 chars max,
/// never empty.
fn note_title(note: &QuickNoteRow) -> String {
    let first = note.content.split('\n').next().unwrap_or("").trim();
    let title: String = first.chars().take(42).collect();
    if title.is_empty() {
        "未命名小记".to_owned()
    } else {
        title
    }
}

fn quick_note_row_from_row(row: &Row<'_>) -> std::result::Result<QuickNoteRow, rusqlite::Error> {
    let created = row.get::<_, String>("created_at")?;
    let updated = row.get::<_, String>("updated_at")?;
    let stamp = |value: String| LocalDateTime::parse(&value).ok_or(rusqlite::Error::InvalidQuery);
    Ok(QuickNoteRow {
        id: row.get("id")?,
        content: row.get("content")?,
        pinned: row.get::<_, i64>("pinned")? != 0,
        archived: row.get::<_, i64>("archived")? != 0,
        created_at: stamp(created)?,
        updated_at: stamp(updated)?,
        tags: Vec::new(),
    })
}

fn note_tags(conn: &Connection, note_id: i64) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT tags.id, tags.name, tags.color FROM tags \
         JOIN quick_note_tags ON quick_note_tags.tag_id = tags.id \
         WHERE quick_note_tags.quick_note_id = ? ORDER BY tags.name",
    )?;
    Ok(stmt
        .query_map(params![note_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

impl Workspace {
    /// Fills the Tags of one stream row (the list query reads notes and
    /// tags in two steps, like `with_details` does for Tasks).
    fn with_note_tags(&self, mut note: QuickNoteRow) -> Result<QuickNoteRow> {
        note.tags = note_tags(&self.conn, note.id)?;
        Ok(note)
    }
}

/// Replaces a note's Tag set with exactly `names` (creating Tags on
/// first use); runs on the caller's transaction.
fn reconcile_note_tags(conn: &Connection, note_id: i64, names: &[String], now: &str) -> Result<()> {
    let mut wanted: Vec<&str> = Vec::new();
    for name in names {
        let name = name.trim().trim_start_matches('#');
        if !name.is_empty() && !wanted.contains(&name) {
            wanted.push(name);
        }
    }
    let tag_ids = wanted
        .iter()
        .map(|name| -> Result<i64> {
            if let Some(id) = conn
                .query_row("SELECT id FROM tags WHERE name = ?", params![name], |row| {
                    row.get(0)
                })
                .optional()?
            {
                return Ok(id);
            }
            conn.execute(
                "INSERT INTO tags (name, created_at) VALUES (?, ?)",
                params![name, now],
            )?;
            Ok(conn.last_insert_rowid())
        })
        .collect::<Result<Vec<_>>>()?;
    conn.execute(
        "DELETE FROM quick_note_tags WHERE quick_note_id = ?",
        params![note_id],
    )?;
    for tag_id in tag_ids {
        conn.execute(
            "INSERT OR IGNORE INTO quick_note_tags (quick_note_id, tag_id) VALUES (?, ?)",
            params![note_id, tag_id],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_display_lines_and_inline_marks() {
        let lines = parse_note_content(
            "# 标题\n- [x] 已完成\n- [ ] 待办\n有 `代码` 和 **重点** 混排\n- [ ]not a check",
        );
        assert_eq!(
            lines[0],
            NoteLine::Heading(vec![NoteSegment {
                text: "标题".into(),
                bold: false,
                code: false
            }])
        );
        assert_eq!(
            lines[1],
            NoteLine::Check {
                done: true,
                label: "已完成".into()
            }
        );
        assert_eq!(
            lines[2],
            NoteLine::Check {
                done: false,
                label: "待办".into()
            }
        );
        assert_eq!(
            lines[3],
            NoteLine::Text(vec![
                NoteSegment {
                    text: "有 ".into(),
                    bold: false,
                    code: false
                },
                NoteSegment {
                    text: "代码".into(),
                    bold: false,
                    code: true
                },
                NoteSegment {
                    text: " 和 ".into(),
                    bold: false,
                    code: false
                },
                NoteSegment {
                    text: "重点".into(),
                    bold: true,
                    code: false
                },
                NoteSegment {
                    text: " 混排".into(),
                    bold: false,
                    code: false
                },
            ])
        );
        assert!(
            matches!(&lines[4], NoteLine::Text(_)),
            "no space after ] is not a checklist"
        );
    }

    #[test]
    fn parses_tag_names_from_the_composer_field() {
        assert_eq!(
            parse_tag_names("#设计, 田野 ，写作、阅读"),
            vec!["设计", "田野", "写作", "阅读"]
        );
        assert!(parse_tag_names("# ,，").is_empty());
    }
}
