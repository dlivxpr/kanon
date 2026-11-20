//! The thin NSH store the workbench binds to (spec: a thin reactive
//! projection layer over `Workspace`).
//!
//! Views never hold their own copies of Task state: every mutation flows
//! through these methods into `Workspace`, and the `bump` revision signal
//! re-runs the bound queries against the fresh snapshot.

use nsh_core::prelude::*;

use crate::domain::{
    LocalDate, LocalDateTime, NewTask, TaskAggregate, TaskGroups, TaskPatch, Workspace,
};

use super::tabs::{Route, TabStrip};

/// The sidebar's persistence keys.
pub const PREF_SIDEBAR_COLLAPSED: &str = "sidebar_collapsed";
pub const PREF_PANEL_OPEN: &str = "context_panel_open";

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
}

impl Store {
    pub fn new(workspace: Workspace) -> Self {
        let today = workspace.today();
        let sidebar_collapsed =
            workspace.preference(PREF_SIDEBAR_COLLAPSED).ok().flatten().is_some_and(|v| v == "true");
        // The panel defaults to open on task views (docs/ui-design.md
        // §3.4); only an explicit "false" preference collapses it.
        let panel_open = workspace.preference(PREF_PANEL_OPEN).ok().flatten() != Some("false".to_owned());
        Self {
            workspace: stored(workspace),
            bump: signal(0),
            strip: signal(TabStrip::new(Route::Today, today)),
            sidebar_collapsed: signal(sidebar_collapsed),
            sidebar_temp_expand: signal(false),
            panel_open: signal(panel_open),
            selected_task: signal(None),
        }
    }

    // ------------------------------------------------------------------
    // Reads (bindings re-run on `bump`)
    // ------------------------------------------------------------------

    pub fn now(&self) -> LocalDateTime {
        self.workspace.with(|ws| ws.now())
    }

    pub fn tasks_for_date(&self, date: LocalDate) -> Vec<TaskAggregate> {
        self.workspace.with(|ws| ws.tasks_for_date(date).unwrap_or_default())
    }

    pub fn task(&self, id: i64) -> Option<TaskAggregate> {
        self.workspace.with(|ws| ws.task(id).unwrap_or_default())
    }

    pub fn task_groups(&self) -> TaskGroups {
        self.workspace.with(|ws| ws.task_groups().unwrap_or_default())
    }

    pub fn carryover_count(&self) -> usize {
        self.workspace.with(|ws| ws.carryover_count().unwrap_or_default())
    }

    // ------------------------------------------------------------------
    // Knowledge Base and catalog reads
    // ------------------------------------------------------------------

    pub fn knowledge_bases(&self) -> Vec<crate::domain::KnowledgeBase> {
        self.bump.get();
        self.workspace.with(|ws| ws.knowledge_bases().unwrap_or_default())
    }

    pub fn knowledge_base(&self, id: i64) -> Option<crate::domain::KnowledgeBase> {
        self.bump.get();
        self.workspace.with(|ws| ws.knowledge_base(id).unwrap_or_default())
    }

    pub fn book_stats(&self, id: i64) -> crate::domain::BookStats {
        self.bump.get();
        self.workspace.with(|ws| ws.book_stats(id).unwrap_or_default())
    }

    pub fn catalog_outline(&self, book_id: i64) -> Vec<crate::domain::CatalogEntry> {
        self.bump.get();
        self.workspace.with(|ws| ws.catalog_outline(book_id).unwrap_or_default())
    }

    /// The catalog sidebar's restore section rows.
    pub fn trashed_catalog_nodes(&self, book_id: i64) -> Vec<crate::domain::CatalogNode> {
        self.bump.get();
        self.workspace.with(|ws| ws.trashed_catalog_nodes(book_id).unwrap_or_default())
    }

    pub fn document(&self, id: i64) -> Option<crate::domain::Document> {
        self.bump.get();
        self.workspace.with(|ws| ws.document(id).unwrap_or_default())
    }

    pub fn catalog_node_of_document(&self, document_id: i64) -> Option<crate::domain::CatalogNode> {
        self.bump.get();
        self.workspace.with(|ws| ws.catalog_node_of_document(document_id).unwrap_or_default())
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
        let result = self
            .workspace
            .with(|ws| ws.create_task(&NewTask { title: title.to_owned(), ..Default::default() }));
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
        if let Err(err) = self.workspace.with(|ws| ws.add_subtask(task_id, title.trim())) {
            log::warn!("add subtask failed: {err}");
        }
        self.bump();
    }

    pub fn set_subtask_done(&self, subtask_id: i64, done: bool) {
        if let Err(err) = self.workspace.with(|ws| ws.set_subtask_done(subtask_id, done)) {
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
        if let Err(err) = self.workspace.with(|ws| ws.update_knowledge_base(id, patch)) {
            log::warn!("update knowledge base {id} failed: {err}");
        }
        self.bump();
    }

    pub fn set_kb_favorite(&self, id: i64, favorited: bool) {
        if let Err(err) = self.workspace.with(|ws| ws.set_knowledge_base_favorite(id, favorited)) {
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
        self.workspace.with(|ws| ws.trashed_knowledge_bases().unwrap_or_default())
    }

    /// Permanently deletes a trashed Knowledge Base (not undoable).
    pub fn delete_knowledge_base_permanent(&self, id: i64) {
        if let Err(err) = self.workspace.with(|ws| ws.delete_knowledge_base_permanent(id)) {
            log::warn!("delete knowledge base {id} failed: {err}");
        }
        self.bump();
    }

    /// Creates a Document at the top of its level (the catalog's top `+`);
    /// `parent` nests it under that node. Returns the new document id.
    pub fn create_document_in(&self, book_id: i64, parent: Option<i64>, title: &str) -> Option<i64> {
        let title = title.trim();
        if title.is_empty() {
            return None;
        }
        let result = self.workspace.with(|ws| ws.create_document(book_id, parent, title));
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
        let result = self.workspace.with(|ws| ws.create_catalog_node(book_id, parent, title));
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
        let result = self.workspace.with(|ws| ws.move_catalog_node(id, parent, index));
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
        let outline = self.workspace.with(|ws| ws.catalog_outline(book_id).unwrap_or_default());
        let siblings = outline
            .iter()
            .filter(|entry| entry.node.parent_id == parent)
            .map(|entry| entry.node.id)
            .collect::<Vec<_>>();
        let Some(position) = siblings.iter().position(|sibling| *sibling == id) else { return };
        let at = (position as isize + delta).clamp(0, siblings.len() as isize - 1) as usize;
        self.move_catalog_node(id, parent, at);
    }

    /// The catalog row menu's outdent: move the node to the top of the
    /// book's root level.
    pub fn move_catalog_node_to_top(&self, id: i64) {
        self.move_catalog_node(id, None, 0);
    }

    pub fn set_node_expanded(&self, id: i64, expanded: bool) {
        if let Err(err) = self.workspace.with(|ws| ws.set_catalog_node_expanded(id, expanded)) {
            log::warn!("set node {id} expanded failed: {err}");
        }
        self.bump();
    }

    /// The catalog sidebar's expand/collapse-all.
    pub fn set_catalog_expanded_all(&self, book_id: i64, expanded: bool) {
        if let Err(err) = self.workspace.with(|ws| ws.set_catalog_expanded_all(book_id, expanded)) {
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
                        self.workspace.with(|ws| ws.catalog_outline(book_id).ok()).map(|outline| {
                            outline.iter().any(|entry| entry.node.document_id == Some(*doc_id))
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
        self.strip.update(|strip| strip.open_tab(Route::Start, today));
    }

    pub fn open_or_focus(&self, route: Route) {
        let today = self.now().date;
        self.strip.update(|strip| strip.open_or_focus(route, today));
    }

    pub fn close_tab(&self, id: u64) {
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
    // Shell commands
    // ------------------------------------------------------------------

    /// Whether the responsive rules (docs/ui-design.md §3.4) currently
    /// force the outer sidebar into its icon rail: a window under
    /// 1440 px showing an inner-catalog destination. The strip read is
    /// tracked: bindings that ask re-run when the active tab changes.
    pub fn sidebar_auto_collapsed(&self) -> bool {
        let strip = self.strip.get();
        nsh_core::prelude::window().width.0 < 1440.0
            && strip
                .tabs
                .get(strip.active)
                .is_some_and(|tab| tab.current.has_inner_catalog())
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
            self.sidebar_temp_expand.update(|expanded| *expanded = !*expanded);
            return;
        }
        let collapsed = !self.sidebar_collapsed.get_untracked();
        self.sidebar_collapsed.set(collapsed);
        let result =
            self.workspace.with(|ws| ws.set_preference(PREF_SIDEBAR_COLLAPSED, if collapsed { "true" } else { "false" }));
        if let Err(err) = result {
            log::warn!("persist sidebar preference failed: {err}");
        }
    }

    pub fn toggle_panel(&self) {
        let open = !self.panel_open.get_untracked();
        self.panel_open.set(open);
        let result =
            self.workspace.with(|ws| ws.set_preference(PREF_PANEL_OPEN, if open { "true" } else { "false" }));
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
