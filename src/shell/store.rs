//! The thin NSH store the workbench binds to (spec: a thin reactive
//! projection layer over `Workspace`).
//!
//! Views never hold their own copies of Task state: every mutation flows
//! through these methods into `Workspace`, and the `bump` revision signal
//! re-runs the bound queries against the fresh snapshot.

use nsh_core::prelude::*;

use crate::domain::{
    Document, DocumentRevision, LocalDate, LocalDateTime, NewTask, TaskAggregate, TaskGroups,
    TaskPatch, Workspace, WorkspaceError,
};

use super::tabs::{ProjectSection, Route, TabStrip};

/// The sidebar's persistence keys.
pub const PREF_SIDEBAR_COLLAPSED: &str = "sidebar_collapsed";
pub const PREF_PANEL_OPEN: &str = "context_panel_open";

/// The single-editor lock (issue #6): at most one workbench tab holds
/// edit ownership at a time, for any Document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorLock {
    pub tab_id: u64,
    pub document_id: i64,
}

/// What answering an Edit request means for the requesting tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditOutcome {
    /// The lock is (or was already) this tab's: switch to editing.
    Granted,
    /// Another tab holds the lock: focus the editing tab instead.
    FocusOther(u64),
}

/// A reading tab's request to jump to one of the document's headings,
/// answered by whichever tab shows that document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutlineRequest {
    pub tab_id: u64,
    /// Which heading among the body's headings, in document order.
    pub ordinal: usize,
    /// Bumped per request so repeats of the same heading re-fire.
    pub nonce: u64,
}

/// The app-level store: one `Workspace` behind signals.
#[derive(Clone, Copy)]
pub struct Store {
    workspace: Stored<Workspace>,
    /// Bumped after every workspace mutation so bound queries re-run.
    pub bump: Signal<u64>,
    /// The workbench tab strip (routes, per-tab history, pin state).
    pub strip: Signal<TabStrip>,
    pub sidebar_collapsed: Signal<bool>,
    /// The manual temporary expand while the responsive rules
    /// auto-collapse the outer sidebar (docs/ui-design.md §3.4).
    pub sidebar_temp_expand: Signal<bool>,
    pub panel_open: Signal<bool>,
    /// The Task shown in the right context panel's Task Detail.
    pub selected_task: Signal<Option<i64>>,
    /// Which tab (if any) currently owns the single editor lock, and for
    /// which Document.
    pub editor_lock: Signal<Option<EditorLock>>,
    /// The reading tab's latest outline-jump request.
    pub outline_request: Signal<Option<OutlineRequest>>,
}

impl Store {
    pub fn new(workspace: Workspace) -> Self {
        let today = workspace.today();
        let sidebar_collapsed = workspace
            .preference(PREF_SIDEBAR_COLLAPSED)
            .ok()
            .flatten()
            .is_some_and(|v| v == "true");
        // The panel defaults to open on task views (docs/ui-design.md
        // §3.4); only an explicit "false" preference collapses it.
        let panel_open =
            workspace.preference(PREF_PANEL_OPEN).ok().flatten() != Some("false".to_owned());
        Self {
            workspace: stored(workspace),
            bump: signal(0),
            strip: signal(TabStrip::new(Route::Today, today)),
            sidebar_collapsed: signal(sidebar_collapsed),
            sidebar_temp_expand: signal(false),
            panel_open: signal(panel_open),
            selected_task: signal(None),
            editor_lock: signal(None),
            outline_request: signal(None),
        }
    }

    // ------------------------------------------------------------------
    // Reads (bindings re-run on `bump`)
    // ------------------------------------------------------------------

    pub fn now(&self) -> LocalDateTime {
        self.workspace.with(|ws| ws.now())
    }

    pub fn tasks_for_date(&self, date: LocalDate) -> Vec<TaskAggregate> {
        self.workspace
            .with(|ws| ws.tasks_for_date(date).unwrap_or_default())
    }

    pub fn task(&self, id: i64) -> Option<TaskAggregate> {
        self.workspace.with(|ws| ws.task(id).unwrap_or_default())
    }

    pub fn task_groups(&self) -> TaskGroups {
        self.workspace
            .with(|ws| ws.task_groups().unwrap_or_default())
    }

    pub fn inbox_count(&self) -> usize {
        self.workspace
            .with(|ws| ws.inbox_count().unwrap_or_default())
    }

    pub fn upcoming_tasks(&self) -> Vec<TaskAggregate> {
        self.workspace
            .with(|ws| ws.upcoming_tasks().unwrap_or_default())
    }

    pub fn projects(&self) -> Vec<crate::domain::Project> {
        self.workspace.with(|ws| ws.projects().unwrap_or_default())
    }

    /// One Project regardless of its Trash state (route titles, the
    /// Trash screen, the Linked Documents notice).
    pub fn project(&self, id: i64) -> Option<crate::domain::Project> {
        self.bump.get();
        self.workspace.with(|ws| ws.project(id).unwrap_or_default())
    }

    /// A Project's Milestones in display order.
    pub fn milestones(&self, project_id: i64) -> Vec<crate::domain::Milestone> {
        self.workspace
            .with(|ws| ws.milestones(project_id).unwrap_or_default())
    }

    /// Every non-trashed Task assigned to the Project, in status order.
    pub fn project_tasks(&self, project_id: i64) -> Vec<TaskAggregate> {
        self.workspace
            .with(|ws| ws.project_tasks(project_id).unwrap_or_default())
    }

    /// The sidebar badge: active (`todo`/`doing`) tasks of the Project.
    pub fn project_open_count(&self, project_id: i64) -> usize {
        self.project_tasks(project_id)
            .iter()
            .filter(|aggregate| aggregate.task.status.is_active())
            .count()
    }

    /// Where the Project's Documents live (ADR-0001); `None` = unbound.
    pub fn project_document_target(
        &self,
        project_id: i64,
    ) -> Option<crate::domain::ProjectDocsTarget> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.project_document_target(project_id).unwrap_or_default())
    }

    /// Whether the Project's linked grouping node currently sits in
    /// Trash (the recoverable-binding notice reads this).
    pub fn project_linked_node_trashed(&self, project_id: i64) -> bool {
        let Some(project) = self.project(project_id) else { return false };
        let Some(node_id) = project.linked_group_node_id else { return false };
        self.catalog_node_is_trashed(node_id)
    }

    pub fn catalog_node_is_trashed(&self, id: i64) -> bool {
        self.bump.get();
        self.workspace.with(|ws| ws.catalog_node_is_trashed(id).unwrap_or(false))
    }

    /// One Catalog Node regardless of its Trash state (labels and
    /// breadcrumbs).
    pub fn catalog_node(&self, id: i64) -> Option<crate::domain::CatalogNode> {
        self.bump.get();
        self.workspace.with(|ws| ws.catalog_node(id).unwrap_or_default())
    }

    /// The Trash screen's project rows.
    pub fn trashed_projects(&self) -> Vec<crate::domain::Project> {
        self.bump.get();
        self.workspace.with(|ws| ws.trashed_projects().unwrap_or_default())
    }

    /// The Trash screen's catalog node rows (subtree roots, all books).
    pub fn trashed_catalog_nodes_all(&self) -> Vec<crate::domain::CatalogNode> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.trashed_catalog_nodes_all().unwrap_or_default())
    }

    /// The Overview's 关联文档 metric: live documents in the linked scope.
    pub fn project_document_count(&self, project_id: i64) -> usize {
        self.bump.get();
        self.workspace
            .with(|ws| ws.project_document_count(project_id).unwrap_or_default())
    }

    /// One-click binding: creates the same-named Knowledge Base and binds
    /// the Project to it (ADR-0001).
    pub fn bind_project_new_book(&self, project_id: i64) -> bool {
        match self.workspace.with(|ws| ws.bind_new_book(project_id)) {
            Ok(_) => {
                self.bump();
                true
            }
            Err(err) => {
                log::warn!("bind new book to project {project_id} failed: {err}");
                false
            }
        }
    }

    pub fn carryover_count(&self) -> usize {
        self.workspace
            .with(|ws| ws.carryover_count().unwrap_or_default())
    }

    // ------------------------------------------------------------------
    // Knowledge Base and catalog reads
    // ------------------------------------------------------------------

    pub fn knowledge_bases(&self) -> Vec<crate::domain::KnowledgeBase> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.knowledge_bases().unwrap_or_default())
    }

    pub fn knowledge_base(&self, id: i64) -> Option<crate::domain::KnowledgeBase> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.knowledge_base(id).unwrap_or_default())
    }

    pub fn book_stats(&self, id: i64) -> crate::domain::BookStats {
        self.bump.get();
        self.workspace
            .with(|ws| ws.book_stats(id).unwrap_or_default())
    }

    pub fn catalog_outline(&self, book_id: i64) -> Vec<crate::domain::CatalogEntry> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.catalog_outline(book_id).unwrap_or_default())
    }

    /// The catalog sidebar's restore section rows.
    pub fn trashed_catalog_nodes(&self, book_id: i64) -> Vec<crate::domain::CatalogNode> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.trashed_catalog_nodes(book_id).unwrap_or_default())
    }

    pub fn document(&self, id: i64) -> Option<crate::domain::Document> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.document(id).unwrap_or_default())
    }

    pub fn catalog_node_of_document(&self, document_id: i64) -> Option<crate::domain::CatalogNode> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.catalog_node_of_document(document_id).unwrap_or_default())
    }

    /// The tab/sidebar label for a route: book and document routes take
    /// their live names, everything else its static title.
    pub fn route_title(&self, route: &Route) -> String {
        self.bump.get();
        match route {
            Route::Book(id) => self
                .workspace
                .with(|ws| ws.knowledge_base(*id).ok().flatten())
                .map(|kb| kb.name)
                .unwrap_or_else(|| route.title()),
            Route::Document(id) => self
                .workspace
                .with(|ws| ws.document(*id).ok().flatten())
                .map(|doc| doc.title)
                .unwrap_or_else(|| route.title()),
            Route::Project(id) => self
                .workspace
                .with(|ws| ws.project(*id).ok().flatten())
                .map(|project| project.name)
                .unwrap_or_else(|| route.title()),
            Route::ProjectEditor(Some(id)) => self
                .workspace
                .with(|ws| ws.project(*id).ok().flatten())
                .map(|project| format!("编辑 {}", project.name))
                .unwrap_or_else(|| route.title()),
            other => other.title(),
        }
    }

    // ------------------------------------------------------------------
    // Task commands
    // ------------------------------------------------------------------

    /// Creates a Task scheduled for `date` (the quick-add row of Today).
    pub fn create_task_on(&self, date: LocalDate, title: &str) -> Option<i64> {
        let title = title.trim();
        if title.is_empty() {
            return None;
        }
        let result = self.workspace.with(|ws| {
            ws.create_task(&NewTask {
                title: title.to_owned(),
                scheduled_date: Some(date),
                ..Default::default()
            })
        });
        match result {
            Ok(aggregate) => {
                self.bump();
                Some(aggregate.task.id)
            }
            Err(err) => {
                log::warn!("create task failed: {err}");
                None
            }
        }
    }

    /// Creates an Inbox task (no Project, no dates).
    pub fn create_inbox_task(&self, title: &str) -> Option<i64> {
        let title = title.trim();
        if title.is_empty() {
            return None;
        }
        let result = self.workspace.with(|ws| {
            ws.create_task(&NewTask {
                title: title.to_owned(),
                ..Default::default()
            })
        });
        match result {
            Ok(aggregate) => {
                self.bump();
                Some(aggregate.task.id)
            }
            Err(err) => {
                log::warn!("create inbox task failed: {err}");
                None
            }
        }
    }

    /// The quick circle: strictly `done ↔ todo`.
    pub fn toggle_circle(&self, id: i64) {
        if let Err(err) = self.workspace.with(|ws| ws.toggle_task_circle(id)) {
            log::warn!("toggle task {id} failed: {err}");
        }
        self.bump();
    }

    pub fn update_task(&self, id: i64, patch: &TaskPatch) {
        if let Err(err) = self.workspace.with(|ws| ws.update_task(id, patch)) {
            log::warn!("update task {id} failed: {err}");
        }
        self.bump();
    }

    /// Moves a Task to Trash (with its detail panel closed).
    pub fn trash_task(&self, id: i64) {
        if let Err(err) = self.workspace.with(|ws| ws.move_task_to_trash(id)) {
            log::warn!("trash task {id} failed: {err}");
        }
        if self.selected_task.get_untracked() == Some(id) {
            self.selected_task.set(None);
        }
        self.bump();
    }

    /// Carryover (GLOSSARY): past active tasks move onto today, keeping
    /// their Due Date/Time. Returns how many moved.
    pub fn carryover(&self) -> usize {
        let moved = self.workspace.with(|ws| ws.carryover().unwrap_or_default());
        if moved > 0 {
            self.bump();
        }
        moved
    }

    pub fn add_subtask(&self, task_id: i64, title: &str) {
        if title.trim().is_empty() {
            return;
        }
        if let Err(err) = self
            .workspace
            .with(|ws| ws.add_subtask(task_id, title.trim()))
        {
            log::warn!("add subtask failed: {err}");
        }
        self.bump();
    }

    pub fn set_subtask_done(&self, subtask_id: i64, done: bool) {
        if let Err(err) = self
            .workspace
            .with(|ws| ws.set_subtask_done(subtask_id, done))
        {
            log::warn!("set subtask {subtask_id} failed: {err}");
        }
        self.bump();
    }

    pub fn remove_subtask(&self, subtask_id: i64) {
        if let Err(err) = self.workspace.with(|ws| ws.remove_subtask(subtask_id)) {
            log::warn!("remove subtask {subtask_id} failed: {err}");
        }
        self.bump();
    }

    /// Attaches a Tag (creating it on first use) to a Task.
    pub fn add_tag(&self, task_id: i64, name: &str) {
        let name = name.trim().trim_start_matches('#');
        if name.is_empty() {
            return;
        }
        let result = self.workspace.with(|ws| {
            let tag = ws.ensure_tag(name)?;
            ws.attach_tag(task_id, tag.id)
        });
        if let Err(err) = result {
            log::warn!("add tag {name} failed: {err}");
        }
        self.bump();
    }

    pub fn remove_tag(&self, task_id: i64, tag_id: i64) {
        if let Err(err) = self.workspace.with(|ws| ws.detach_tag(task_id, tag_id)) {
            log::warn!("remove tag failed: {err}");
        }
        self.bump();
    }

    // ------------------------------------------------------------------
    // Knowledge Base and catalog commands
    // ------------------------------------------------------------------

    /// Creates a Knowledge Base and lands on its book home. Returns the id.
    pub fn create_knowledge_base(&self, name: &str) -> Option<i64> {
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        let result = self.workspace.with(|ws| ws.create_knowledge_base(name));
        match result {
            Ok(kb) => {
                self.bump();
                Some(kb.id)
            }
            Err(err) => {
                log::warn!("create knowledge base failed: {err}");
                None
            }
        }
    }

    pub fn update_knowledge_base(&self, id: i64, patch: &crate::domain::KnowledgeBasePatch) {
        if let Err(err) = self
            .workspace
            .with(|ws| ws.update_knowledge_base(id, patch))
        {
            log::warn!("update knowledge base {id} failed: {err}");
        }
        self.bump();
    }

    pub fn set_kb_favorite(&self, id: i64, favorited: bool) {
        if let Err(err) = self
            .workspace
            .with(|ws| ws.set_knowledge_base_favorite(id, favorited))
        {
            log::warn!("favorite knowledge base {id} failed: {err}");
        }
        self.bump();
    }

    /// Trashes a book's whole subtree and closes the tabs that showed it
    /// (they fall back to the shelf).
    pub fn trash_knowledge_base(&self, id: i64) {
        if let Err(err) = self.workspace.with(|ws| ws.trash_knowledge_base(id)) {
            log::warn!("trash knowledge base {id} failed: {err}");
            return;
        }
        self.strip.update(|strip| {
            for tab in &mut strip.tabs {
                let stale = match &tab.current {
                    Route::Book(book) => *book == id,
                    Route::Document(doc) => self
                        .workspace
                        .with(|ws| ws.document(*doc).ok().flatten())
                        .is_some_and(|doc| doc.book_id == id),
                    _ => false,
                };
                if stale {
                    tab.navigate(Route::KnowledgeBases);
                }
            }
        });
        self.bump();
    }

    pub fn restore_knowledge_base(&self, id: i64) {
        if let Err(err) = self.workspace.with(|ws| ws.restore_knowledge_base(id)) {
            log::warn!("restore knowledge base {id} failed: {err}");
        }
        self.bump();
    }

    /// The Trash screen's rows.
    pub fn trashed_knowledge_bases(&self) -> Vec<crate::domain::KnowledgeBase> {
        self.bump.get();
        self.workspace
            .with(|ws| ws.trashed_knowledge_bases().unwrap_or_default())
    }

    /// Permanently deletes a trashed Knowledge Base (not undoable).
    pub fn delete_knowledge_base_permanent(&self, id: i64) {
        if let Err(err) = self
            .workspace
            .with(|ws| ws.delete_knowledge_base_permanent(id))
        {
            log::warn!("delete knowledge base {id} failed: {err}");
        }
        self.bump();
    }

    /// Creates a Document at the top of its level (the catalog's top `+`);
    /// `parent` nests it under that node. Returns the new document id.
    pub fn create_document_in(
        &self,
        book_id: i64,
        parent: Option<i64>,
        title: &str,
    ) -> Option<i64> {
        let title = title.trim();
        if title.is_empty() {
            return None;
        }
        let result = self
            .workspace
            .with(|ws| ws.create_document(book_id, parent, title));
        match result {
            Ok(doc) => {
                self.bump();
                Some(doc.id)
            }
            Err(err) => {
                log::warn!("create document failed: {err}");
                None
            }
        }
    }

    /// Creates a grouping Catalog Node at the top of its level. Returns
    /// the new node id.
    pub fn create_group_in(&self, book_id: i64, parent: Option<i64>, title: &str) -> Option<i64> {
        let title = title.trim();
        if title.is_empty() {
            return None;
        }
        let result = self
            .workspace
            .with(|ws| ws.create_catalog_node(book_id, parent, title));
        match result {
            Ok(node) => {
                self.bump();
                Some(node.id)
            }
            Err(err) => {
                log::warn!("create catalog node failed: {err}");
                None
            }
        }
    }

    /// Drag/move reparenting. Returns false (and changes nothing) when
    /// the move would create a cycle or cross books.
    pub fn move_catalog_node(&self, id: i64, parent: Option<i64>, index: usize) -> bool {
        let result = self
            .workspace
            .with(|ws| ws.move_catalog_node(id, parent, index));
        match result {
            Ok(()) => {
                self.bump();
                true
            }
            Err(err) => {
                log::warn!("move catalog node {id} failed: {err}");
                false
            }
        }
    }

    /// The catalog row menu's reorder: `delta` of -1/1 moves the node
    /// one slot within its level.
    pub fn move_catalog_node_rel(&self, id: i64, delta: isize) {
        let (book_id, parent) = {
            let node = self.workspace.with(|ws| ws.catalog_node(id).ok().flatten());
            match node {
                Some(node) => (node.book_id, node.parent_id),
                None => return,
            }
        };
        let outline = self
            .workspace
            .with(|ws| ws.catalog_outline(book_id).unwrap_or_default());
        let siblings = outline
            .iter()
            .filter(|entry| entry.node.parent_id == parent)
            .map(|entry| entry.node.id)
            .collect::<Vec<_>>();
        let Some(position) = siblings.iter().position(|sibling| *sibling == id) else {
            return;
        };
        let at = (position as isize + delta).clamp(0, siblings.len() as isize - 1) as usize;
        self.move_catalog_node(id, parent, at);
    }

    /// The catalog row menu's outdent: move the node to the top of the
    /// book's root level.
    pub fn move_catalog_node_to_top(&self, id: i64) {
        self.move_catalog_node(id, None, 0);
    }

    pub fn set_node_expanded(&self, id: i64, expanded: bool) {
        if let Err(err) = self
            .workspace
            .with(|ws| ws.set_catalog_node_expanded(id, expanded))
        {
            log::warn!("set node {id} expanded failed: {err}");
        }
        self.bump();
    }

    /// The catalog sidebar's expand/collapse-all.
    pub fn set_catalog_expanded_all(&self, book_id: i64, expanded: bool) {
        if let Err(err) = self
            .workspace
            .with(|ws| ws.set_catalog_expanded_all(book_id, expanded))
        {
            log::warn!("set catalog expanded-all failed: {err}");
        }
        self.bump();
    }

    pub fn trash_catalog_node(&self, id: i64) {
        if let Err(err) = self.workspace.with(|ws| ws.trash_catalog_node(id)) {
            log::warn!("trash catalog node {id} failed: {err}");
            return;
        }
        // A document tab whose tree row just disappeared falls back to
        // its book home.
        self.strip.update(|strip| {
            for tab in &mut strip.tabs {
                if let Route::Document(doc_id) = &tab.current {
                    let book = self
                        .workspace
                        .with(|ws| ws.document(*doc_id).ok().flatten())
                        .map(|doc| doc.book_id);
                    let visible = book.and_then(|book_id| {
                        self.workspace
                            .with(|ws| ws.catalog_outline(book_id).ok())
                            .map(|outline| {
                                outline
                                    .iter()
                                    .any(|entry| entry.node.document_id == Some(*doc_id))
                            })
                    });
                    if visible == Some(false)
                        && let Some(book_id) = book
                    {
                        tab.navigate(Route::Book(book_id));
                    }
                }
            }
        });
        self.bump();
    }

    pub fn restore_catalog_node(&self, id: i64) {
        if let Err(err) = self.workspace.with(|ws| ws.restore_catalog_node(id)) {
            log::warn!("restore catalog node {id} failed: {err}");
        }
        self.bump();
    }

    /// Permanently deletes a trashed Catalog Node subtree (not undoable).
    /// Projects bound to any node of the subtree reset to unbound
    /// (ADR-0001); document tabs showing the removed documents fall back
    /// to their book home.
    pub fn delete_catalog_node_permanent(&self, id: i64) {
        // Document tabs showing a removed document fall back to the
        // subtree's book home; the book id must be read before the delete.
        let book_id = self
            .workspace
            .with(|ws| ws.catalog_node(id).ok().flatten())
            .map(|node| node.book_id);
        let open_docs = self.strip.with_untracked(|strip| {
            strip
                .tabs
                .iter()
                .filter_map(|tab| match &tab.current {
                    Route::Document(doc_id) => Some((tab.id, *doc_id)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        });
        if let Err(err) = self
            .workspace
            .with(|ws| ws.delete_catalog_node_permanent(id))
        {
            log::warn!("delete catalog node {id} failed: {err}");
            return;
        }
        let fallback = |doc_id: i64| {
            self.workspace.with(|ws| ws.document(doc_id).ok().flatten()).is_none()
        };
        self.strip.update(|strip| {
            for (tab_id, doc_id) in open_docs {
                if fallback(doc_id)
                    && let Some(book_id) = book_id
                    && let Some(tab) = strip.tabs.iter_mut().find(|tab| tab.id == tab_id)
                {
                    tab.navigate(Route::Book(book_id));
                }
            }
        });
        self.bump();
    }

    // ------------------------------------------------------------------
    // Project commands
    // ------------------------------------------------------------------

    /// Opens a Project (focus-or-reuse, like every sidebar entry).
    pub fn open_project(&self, id: i64) {
        self.open_or_focus(Route::Project(id));
    }

    /// Creates a Project with its binding choice; returns the new id.
    pub fn create_project_with_binding(
        &self,
        name: &str,
        color: &str,
        emoji: &str,
        binding: crate::domain::ProjectBinding,
    ) -> Option<i64> {
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        let result = self.workspace.with(|ws| {
            ws.create_project_with_binding(name, color, emoji, binding)
        });
        match result {
            Ok(project) => {
                self.bump();
                Some(project.id)
            }
            Err(err) => {
                log::warn!("create project failed: {err}");
                None
            }
        }
    }

    pub fn update_project(&self, id: i64, patch: &crate::domain::ProjectPatch) -> bool {
        let result = self.workspace.with(|ws| ws.update_project(id, patch));
        match result {
            Ok(_) => {
                self.bump();
                true
            }
            Err(err) => {
                log::warn!("update project {id} failed: {err}");
                false
            }
        }
    }

    /// Moves a Project into Trash; tabs showing it fall back to 待办.
    /// The Tasks keep their `project_id` (ADR-0001).
    pub fn trash_project(&self, id: i64) {
        if let Err(err) = self.workspace.with(|ws| ws.trash_project(id)) {
            log::warn!("trash project {id} failed: {err}");
            return;
        }
        self.strip.update(|strip| {
            for tab in &mut strip.tabs {
                let stale = matches!(tab.current, Route::Project(project) if project == id)
                    || matches!(tab.current, Route::ProjectEditor(Some(project)) if project == id);
                if stale {
                    tab.navigate(Route::Tasks);
                }
            }
        });
        self.bump();
    }

    pub fn restore_project(&self, id: i64) {
        if let Err(err) = self.workspace.with(|ws| ws.restore_project(id)) {
            log::warn!("restore project {id} failed: {err}");
        }
        self.bump();
    }

    /// Permanently deletes a trashed Project: its Tasks are detached and
    /// its Milestones deleted, while Knowledge Bases and Documents stay
    /// (ADR-0001).
    pub fn delete_project_permanent(&self, id: i64) {
        if let Err(err) = self
            .workspace
            .with(|ws| ws.delete_project_permanent(id))
        {
            log::warn!("delete project {id} failed: {err}");
        }
        self.bump();
    }

    /// The Task List sub-view's quick-add: creates a `todo` Task in the
    /// Project. Returns the new task id.
    pub fn create_project_task(&self, project_id: i64, title: &str) -> Option<i64> {
        let title = title.trim();
        if title.is_empty() {
            return None;
        }
        let result = self.workspace.with(|ws| {
            ws.create_task(&NewTask {
                title: title.to_owned(),
                project_id: Some(project_id),
                ..Default::default()
            })
        });
        match result {
            Ok(aggregate) => {
                self.bump();
                Some(aggregate.task.id)
            }
            Err(err) => {
                log::warn!("create project task failed: {err}");
                None
            }
        }
    }

    /// The kanban card's move: sets a Task's status (recording the
    /// completion time on `done`).
    pub fn set_task_status(&self, id: i64, status: crate::domain::TaskStatus) {
        if let Err(err) = self.workspace.with(|ws| {
            ws.update_task(id, &TaskPatch { status: Some(status), ..Default::default() })
        }) {
            log::warn!("set task {id} status failed: {err}");
        }
        self.bump();
    }

    pub fn create_milestone(
        &self,
        project_id: i64,
        title: &str,
        target_date: Option<LocalDate>,
    ) -> bool {
        let title = title.trim();
        if title.is_empty() {
            return false;
        }
        match self
            .workspace
            .with(|ws| ws.create_milestone(project_id, title, target_date))
        {
            Ok(_) => {
                self.bump();
                true
            }
            Err(err) => {
                log::warn!("create milestone failed: {err}");
                false
            }
        }
    }

    pub fn delete_milestone(&self, milestone_id: i64) {
        if let Err(err) = self.workspace.with(|ws| ws.delete_milestone(milestone_id)) {
            log::warn!("delete milestone {milestone_id} failed: {err}");
        }
        self.bump();
    }

    /// The Linked Documents view's creation: places the new Document
    /// under the linked grouping node, or at the top of the linked
    /// book's root (ADR-0001). Returns the new document id.
    pub fn create_document_for_project(&self, project_id: i64, title: &str) -> Option<i64> {
        let title = title.trim();
        if title.is_empty() {
            return None;
        }
        let result = self
            .workspace
            .with(|ws| ws.create_document_for_project(project_id, title));
        match result {
            Ok(doc) => {
                self.bump();
                Some(doc.id)
            }
            Err(err) => {
                log::warn!("create project document failed: {err}");
                None
            }
        }
    }

    /// The Linked Documents view remembers which Document is open, per
    /// tab (like the Today view's selected date).
    pub fn set_project_section(&self, section: super::tabs::ProjectSection) {
        self.strip.update(|strip| strip.set_project_section(section));
    }

    pub fn set_project_document(&self, document: Option<i64>) {
        self.strip.update(|strip| strip.set_project_document(document));
    }

    /// The Project sub-view one tab currently shows.
    pub fn project_section_of(&self, tab_id: u64) -> ProjectSection {
        self.strip
            .with(|strip| {
                strip
                    .tabs
                    .iter()
                    .find(|tab| tab.id == tab_id)
                    .map(|tab| tab.project_section)
            })
            .unwrap_or_default()
    }

    /// The Document open inside the tab's Linked Documents sub-view.
    pub fn project_document_of(&self, tab_id: u64) -> Option<i64> {
        self.strip.with(|strip| {
            strip
                .tabs
                .iter()
                .find(|tab| tab.id == tab_id)
                .and_then(|tab| tab.project_doc)
        })
    }

    /// Opens the Project editor (create when `editing` is None).
    pub fn open_project_editor(&self, editing: Option<i64>) {
        self.open_or_focus(Route::ProjectEditor(editing));
    }

    /// Opens a book's home (focus-or-reuse, like every sidebar entry).
    pub fn open_book(&self, id: i64) {
        self.open_or_focus(Route::Book(id));
    }

    /// Opens a document's reading view.
    pub fn open_document(&self, id: i64) {
        self.open_or_focus(Route::Document(id));
    }

    // ------------------------------------------------------------------
    // Tab commands
    // ------------------------------------------------------------------

    pub fn open_new_tab(&self) {
        let today = self.now().date;
        self.strip
            .update(|strip| strip.open_tab(Route::Start, today));
    }

    pub fn open_or_focus(&self, route: Route) {
        let today = self.now().date;
        self.strip.update(|strip| strip.open_or_focus(route, today));
    }

    pub fn close_tab(&self, id: u64) {
        // Closing ends the tab's edit session first: the finish-to-read
        // snapshot lands and the single-editor lock frees.
        self.release_editor(id, true);
        self.strip.update(|strip| {
            strip.close_tab(id);
        });
    }

    pub fn toggle_pin(&self, id: u64) {
        self.strip.update(|strip| strip.toggle_pin(id));
    }

    pub fn activate_tab(&self, index: usize) {
        self.strip.update(|strip| strip.set_active(index));
    }

    /// Focuses the tab with `id` (a tab chip press).
    pub fn activate_tab_by_id(&self, id: u64) {
        self.strip.update(|strip| {
            if let Some(index) = strip.tabs.iter().position(|tab| tab.id == id) {
                strip.set_active(index);
            }
        });
    }

    pub fn back(&self) {
        self.strip.update(|strip| {
            strip.back();
        });
    }

    pub fn forward(&self) {
        self.strip.update(|strip| {
            strip.forward();
        });
    }

    /// The Today view's selected date for the active tab.
    pub fn set_selected_date(&self, date: LocalDate) {
        self.strip.update(|strip| {
            strip.tabs[strip.active].selected_date = date;
        });
    }

    // ------------------------------------------------------------------
    // Documents: navigation, the single-editor lock, writes
    // ------------------------------------------------------------------

    /// Opens a Document in a new tab on purpose: how one Document is read
    /// concurrently from several tabs.
    pub fn open_document_in_new_tab(&self, id: i64) {
        let today = self.now().date;
        self.strip
            .update(|strip| strip.open_tab(Route::Document(id), today));
    }

    /// The Document's history, newest first.
    pub fn document_revisions(&self, id: i64) -> Vec<DocumentRevision> {
        self.workspace
            .with(|ws| ws.document_revisions(id).unwrap_or_default())
    }

    /// Answers a tab's Edit request under the single-editor invariant:
    /// grant the lock when it is free or already this tab's, otherwise
    /// name the tab the requester must focus.
    pub fn request_edit(&self, tab_id: u64, document_id: i64) -> EditOutcome {
        match self.editor_lock.get_untracked() {
            Some(lock) if lock.tab_id == tab_id && lock.document_id == document_id => {
                EditOutcome::Granted
            }
            Some(lock) => EditOutcome::FocusOther(lock.tab_id),
            None => {
                self.editor_lock.set(Some(EditorLock {
                    tab_id,
                    document_id,
                }));
                EditOutcome::Granted
            }
        }
    }

    /// Whether any tab is editing right now (reading tabs lock their
    /// Checklist Items while this holds).
    pub fn editing_locked(&self) -> bool {
        self.editor_lock.get_untracked().is_some()
    }

    /// Ends the tab's edit session: records the finish-to-read transition
    /// as an automatic history snapshot, then frees the lock. No-op when
    /// the tab does not hold the lock.
    pub fn release_editor(&self, tab_id: u64, snapshot: bool) {
        let Some(lock) = self.editor_lock.get_untracked() else {
            return;
        };
        if lock.tab_id != tab_id {
            return;
        }
        if snapshot {
            self.take_snapshot(lock.document_id);
        }
        self.editor_lock.set(None);
    }

    /// The unmount cleanup of `editing_view`: the edit session ends only
    /// when its tab leaves the Document (navigates or closes). Switching
    /// to another tab unmounts the view too, but the lock belongs to the
    /// tab — it must survive so the editor keeps ownership.
    pub fn release_editor_if_route_left(&self, tab_id: u64, snapshot: bool) {
        let Some(lock) = self.editor_lock.get_untracked() else {
            return;
        };
        if lock.tab_id != tab_id {
            return;
        }
        let still_on_document = self.strip.with_untracked(|strip| {
            strip.tabs.iter().any(|tab| {
                tab.id == tab_id
                    && matches!(tab.current, Route::Document(id) if id == lock.document_id)
            })
        });
        if still_on_document {
            return;
        }
        self.release_editor(tab_id, snapshot);
    }

    /// Records an automatic history snapshot of the committed Document.
    pub fn take_snapshot(&self, document_id: i64) {
        if let Err(err) = self
            .workspace
            .with(|ws| ws.save_document_revision(document_id, None))
        {
            log::warn!("snapshot document {document_id} failed: {err}");
        }
    }

    /// Saves the committed state as a user-named version.
    pub fn save_named_version(&self, document_id: i64, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() {
            return false;
        }
        match self
            .workspace
            .with(|ws| ws.save_document_revision(document_id, Some(name)))
        {
            Ok(_) => {
                self.bump();
                true
            }
            Err(err) => {
                log::warn!("named version failed: {err}");
                false
            }
        }
    }

    /// The editing tab's auto-save: optimistic commit of title+body.
    /// `Ok` carries the committed document (its `version` becomes the
    /// next save's base); `Err(StaleDocument)` means another writer
    /// committed first and the caller must keep the draft and offer a
    /// retry against a fresh base.
    pub fn save_document_body(
        &self,
        document_id: i64,
        title: &str,
        body: &str,
        base_version: i64,
    ) -> Result<Document, WorkspaceError> {
        let result = self
            .workspace
            .with(|ws| ws.update_document_body(document_id, title, body, Some(base_version)));
        if result.is_ok() {
            self.bump();
        }
        result
    }

    /// A reading tab's Checklist toggle when no edit session is active:
    /// a body-only rewrite that never creates Tasks (ADR-0002).
    pub fn toggle_reading_checklist(&self, document_id: i64, new_body: &str) -> bool {
        let document = match self.document(document_id) {
            Some(document) => document,
            None => return false,
        };
        let result = self
            .workspace
            .with(|ws| ws.update_document_body(document_id, &document.title, new_body, None));
        match result {
            Ok(_) => {
                self.bump();
                true
            }
            Err(err) => {
                log::warn!("reading checklist toggle failed: {err}");
                false
            }
        }
    }

    /// Restores a history version: the current state is archived as an
    /// automatic snapshot first, then the revision's title/body come back
    /// and references re-anchor — never touching any Task.
    pub fn restore_revision(&self, revision_id: i64) -> bool {
        let result = self.workspace.with(|ws| {
            let revision = ws.document_revision(revision_id)?.ok_or_else(|| {
                WorkspaceError::InvalidDateTime(format!("no revision {revision_id}"))
            })?;
            ws.save_document_revision(revision.document_id, None)?;
            ws.restore_document_revision(revision_id)
        });
        match result {
            Ok(_) => {
                self.bump();
                true
            }
            Err(err) => {
                log::warn!("restore revision {revision_id} failed: {err}");
                false
            }
        }
    }

    /// A reading tab asks the tab showing the document to reveal one of
    /// its headings.
    pub fn request_outline_jump(&self, tab_id: u64, ordinal: usize) {
        let nonce = self
            .outline_request
            .get_untracked()
            .map(|request| request.nonce + 1)
            .unwrap_or(1);
        self.outline_request.set(Some(OutlineRequest {
            tab_id,
            ordinal,
            nonce,
        }));
    }

    // ------------------------------------------------------------------
    // Shell commands
    // ------------------------------------------------------------------

    /// Whether the responsive rules (docs/ui-design.md §3.4) currently
    /// force the outer sidebar into its icon rail: a window under
    /// 1440 px showing an inner-catalog destination. The strip read is
    /// tracked: bindings that ask re-run when the active tab changes.
    /// A Project counts only while its Linked Documents sub-view shows.
    pub fn sidebar_auto_collapsed(&self) -> bool {
        let strip = self.strip.get();
        nsh_core::prelude::window().width.0 < 1440.0
            && strip.tabs.get(strip.active).is_some_and(|tab| match &tab.current {
                Route::Book(_) | Route::Document(_) => true,
                Route::Project(_) => tab.project_section == ProjectSection::Documents,
                _ => false,
            })
    }

    /// The sidebar's effective width state for the current window and
    /// tab: the auto rule wins unless manually expanded for a while.
    pub fn sidebar_effective_collapsed(&self) -> bool {
        self.bump.get();
        if self.sidebar_auto_collapsed() {
            !self.sidebar_temp_expand.get()
        } else {
            self.sidebar_collapsed.get()
        }
    }

    pub fn toggle_sidebar(&self) {
        if self.sidebar_auto_collapsed() {
            self.sidebar_temp_expand
                .update(|expanded| *expanded = !*expanded);
            return;
        }
        let collapsed = !self.sidebar_collapsed.get_untracked();
        self.sidebar_collapsed.set(collapsed);
        let result = self.workspace.with(|ws| {
            ws.set_preference(
                PREF_SIDEBAR_COLLAPSED,
                if collapsed { "true" } else { "false" },
            )
        });
        if let Err(err) = result {
            log::warn!("persist sidebar preference failed: {err}");
        }
    }

    pub fn toggle_panel(&self) {
        let open = !self.panel_open.get_untracked();
        self.panel_open.set(open);
        let result = self
            .workspace
            .with(|ws| ws.set_preference(PREF_PANEL_OPEN, if open { "true" } else { "false" }));
        if let Err(err) = result {
            log::warn!("persist panel preference failed: {err}");
        }
    }

    pub fn open_task(&self, id: i64) {
        self.selected_task.set(Some(id));
        if !self.panel_open.get_untracked() {
            self.toggle_panel();
        }
    }

    pub fn close_task(&self) {
        self.selected_task.set(None);
    }

    fn bump(&self) {
        self.bump.update(|value| *value += 1);
    }
}

/// The store the root provided.
pub fn use_store() -> Store {
    use_context::<Store>().expect("the root provides the workbench store")
}
