use super::Store;
use crate::domain::{
    Document, FavoriteEntry, QuickNote, RecentDocument, RecentScope, TagUsage, TaskAggregate,
};

impl Store {
    pub fn favorites(&self) -> Vec<FavoriteEntry> {
        self.bump.get();
        self.workspace.with(|ws| ws.favorites().unwrap_or_default())
    }

    pub fn tag_usages(&self) -> Vec<TagUsage> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.tag_usages().unwrap_or_default())
    }

    pub(in crate::shell) fn tag_name(&self, id: i64) -> Option<String> {
        self.bump.get();
        self.workspace.with(|ws| ws.tag_name(id).ok().flatten())
    }

    pub(in crate::shell) fn rename_tag(&self, id: i64, new_name: &str) -> Option<i64> {
        match self.workspace.with(|ws| ws.rename_tag(id, new_name)) {
            Ok(survivor) => {
                self.bump();
                Some(survivor)
            }
            Err(error) => {
                log::warn!("rename tag failed: {error}");
                None
            }
        }
    }

    pub fn tasks_for_tag(&self, id: i64) -> Vec<TaskAggregate> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.tasks_for_tag(id).unwrap_or_default())
    }

    pub fn documents_for_tag(&self, id: i64) -> Vec<Document> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.documents_for_tag(id).unwrap_or_default())
    }

    pub fn quick_notes_for_tag(&self, id: i64) -> Vec<QuickNote> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.quick_notes_for_tag(id).unwrap_or_default())
    }

    pub fn recent_documents(
        &self,
        book: Option<i64>,
        scope: RecentScope,
        limit: usize,
    ) -> Vec<RecentDocument> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.recent_documents(book, scope, limit).unwrap_or_default())
    }

    pub fn record_document_visit(&self, id: i64) {
        match self.workspace.with(|ws| ws.record_document_visit(id)) {
            Ok(true) => self.bump(),
            Ok(false) => {}
            Err(err) => log::warn!("record document visit {id} failed: {err}"),
        }
    }

    pub fn search_documents(&self, needle: &str, limit: u32) -> Vec<(i64, String)> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.search_documents(needle, limit).unwrap_or_default())
    }

    pub fn document_is_favorite(&self, id: i64) -> bool {
        self.bump.get();
        self.workspace
            .with(|ws| ws.document_is_favorite(id).unwrap_or(false))
    }

    pub fn set_document_favorite(&self, id: i64, favorited: bool) {
        match self
            .workspace
            .with(|ws| ws.set_document_favorite(id, favorited))
        {
            Ok(()) => self.bump(),
            Err(err) => log::warn!("favorite document {id} failed: {err}"),
        }
    }

    pub(crate) fn project_is_favorite(&self, id: i64) -> bool {
        self.bump.get();
        self.workspace
            .with(|ws| ws.project_is_favorite(id).unwrap_or(false))
    }

    pub(crate) fn set_project_favorite(&self, id: i64, favorited: bool) {
        match self
            .workspace
            .with(|ws| ws.set_project_favorite(id, favorited))
        {
            Ok(()) => self.bump(),
            Err(err) => log::warn!("favorite project {id} failed: {err}"),
        }
    }
}
