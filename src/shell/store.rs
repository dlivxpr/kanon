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

    pub fn inbox_count(&self) -> usize {
        self.workspace.with(|ws| ws.inbox_count().unwrap_or_default())
    }

    pub fn upcoming_tasks(&self) -> Vec<TaskAggregate> {
        self.workspace.with(|ws| ws.upcoming_tasks().unwrap_or_default())
    }

    pub fn projects(&self) -> Vec<crate::domain::Project> {
        self.workspace.with(|ws| ws.projects().unwrap_or_default())
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
