//! Quick Note commands keep SQLite as the only source of record.

use super::Store;
use crate::domain::{NotesLayout, QuickNoteRow, Workspace, WorkspaceError};

impl Store {
    pub fn quick_notes(&self) -> Vec<QuickNoteRow> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.quick_notes().unwrap_or_default())
    }

    pub(in crate::shell) fn recent_quick_notes(&self, limit: u32) -> Vec<QuickNoteRow> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.recent_quick_notes(limit).unwrap_or_default())
    }

    pub(in crate::shell) fn live_quick_note(&self, id: i64) -> Option<QuickNoteRow> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.live_quick_note(id).ok().flatten())
    }

    pub fn archived_quick_notes(&self) -> Vec<QuickNoteRow> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.archived_quick_notes().unwrap_or_default())
    }

    pub fn trashed_quick_notes(&self) -> Vec<QuickNoteRow> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.trashed_quick_notes().unwrap_or_default())
    }

    pub fn restore_quick_note(&self, id: i64) {
        self.write_note(|ws| ws.restore_quick_note(id));
    }

    pub fn delete_quick_note_permanent(&self, id: i64) -> bool {
        self.write_note(|ws| ws.delete_quick_note_permanent(id))
            .is_some()
    }

    pub fn publish_quick_note(&self, content: &str, tags: &[String]) -> Option<i64> {
        if content.trim().is_empty() {
            return None;
        }
        self.write_note(|ws| ws.publish_quick_note(content, tags))
            .map(|row| row.id)
    }

    pub fn set_quick_note_pinned(&self, id: i64, pinned: bool) {
        self.write_note(|ws| ws.set_quick_note_pinned(id, pinned));
    }

    pub fn trash_quick_note(&self, id: i64) {
        self.write_note(|ws| ws.trash_quick_note(id));
    }

    pub fn toggle_quick_note_check(&self, id: i64, line: usize) -> bool {
        self.write_note(|ws| ws.toggle_quick_note_check(id, line))
            .is_some()
    }

    pub fn convert_quick_note_to_document(&self, id: i64, book_id: i64) -> Option<i64> {
        self.write_note(|ws| ws.convert_quick_note_to_document(id, book_id))
    }

    pub fn extract_quick_note_task(&self, id: i64) -> Option<i64> {
        self.write_note(|ws| ws.extract_quick_note_task(id))
    }

    pub fn quick_notes_layout(&self) -> NotesLayout {
        match self.preference("quick_notes_layout").as_deref() {
            Some("grid") => NotesLayout::Grid,
            _ => NotesLayout::Stream,
        }
    }

    pub fn set_quick_notes_layout(&self, layout: NotesLayout) {
        self.set_preference("quick_notes_layout", layout.as_str());
    }

    fn write_note<T>(
        &self,
        action: impl FnOnce(&mut Workspace) -> Result<T, WorkspaceError>,
    ) -> Option<T> {
        match self.workspace.with(action) {
            Ok(outcome) => {
                self.bump();
                Some(outcome)
            }
            Err(err) => {
                log::warn!("quick note command failed: {err}");
                None
            }
        }
    }
}
