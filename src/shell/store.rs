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

use super::tabs::{Route, TabStrip};

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

    /// Opens a Document in the workbench (the catalog tree's click): focus
    /// the tab already showing it, else reuse or open per the sidebar rule.
    pub fn open_document(&self, id: i64) {
        self.open_or_focus(Route::Document(id));
    }

    /// Opens a Document in a new tab on purpose: how one Document is read
    /// concurrently from several tabs.
    pub fn open_document_in_new_tab(&self, id: i64) {
        let today = self.now().date;
        self.strip.update(|strip| strip.open_tab(Route::Document(id), today));
    }

    /// The committed Document, if it exists.
    pub fn document(&self, id: i64) -> Option<Document> {
        self.workspace.with(|ws| ws.document(id).unwrap_or_default())
    }

    /// The Document's history, newest first.
    pub fn document_revisions(&self, id: i64) -> Vec<DocumentRevision> {
        self.workspace.with(|ws| ws.document_revisions(id).unwrap_or_default())
    }

    /// The Knowledge Base a breadcrumb names.
    pub fn knowledge_base(&self, id: i64) -> Option<crate::domain::KnowledgeBase> {
        self.workspace.with(|ws| ws.knowledge_base(id).unwrap_or_default())
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
                self.editor_lock.set(Some(EditorLock { tab_id, document_id }));
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
        let Some(lock) = self.editor_lock.get_untracked() else { return };
        if lock.tab_id != tab_id {
            return;
        }
        if snapshot {
            self.take_snapshot(lock.document_id);
        }
        self.editor_lock.set(None);
    }

    /// Records an automatic history snapshot of the committed Document.
    pub fn take_snapshot(&self, document_id: i64) {
        if let Err(err) = self.workspace.with(|ws| ws.save_document_revision(document_id, None)) {
            log::warn!("snapshot document {document_id} failed: {err}");
        }
    }

    /// Saves the committed state as a user-named version.
    pub fn save_named_version(&self, document_id: i64, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() {
            return false;
        }
        match self.workspace.with(|ws| ws.save_document_revision(document_id, Some(name))) {
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
        let result =
            self.workspace.with(|ws| ws.update_document_body(document_id, title, body, Some(base_version)));
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
            let revision = ws
                .document_revision(revision_id)?
                .ok_or_else(|| WorkspaceError::InvalidDateTime(format!("no revision {revision_id}")))?;
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
        self.outline_request.set(Some(OutlineRequest { tab_id, ordinal, nonce }));
    }

    // ------------------------------------------------------------------
    // Shell commands
    // ------------------------------------------------------------------

    pub fn toggle_sidebar(&self) {
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
