//! Workbench tabs: routes, per-tab back/forward history, the
//! focus-or-reuse navigation rule (spec user story 3), and the drag
//! reorder rule (docs/ui-design.md §3.1).
//!
//! Pure Rust: the engine store owns the signals, this module owns the
//! rules, and `cargo test` covers them without an engine.

use crate::domain::LocalDate;
use crate::ui::chrome::Icon;

/// Every destination a workbench tab can show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Start,
    Today,
    Upcoming,
    Tasks,
    Calendar,
    QuickNotes,
    QuickNote(i64),
    Favorites,
    Tags,
    Tag(i64),
    KnowledgeBases,
    /// One open book: its home page plus the inner catalog sidebar.
    Book(i64),
    /// One open Document (docs/ui-design.md §2.2 `doc/:docId`): the
    /// payload is the document id. Equality is by id, so a Knowledge
    /// Base's catalog focuses the tab already showing this Document
    /// instead of opening a duplicate.
    Document(i64),
    /// One open Project (docs/ui-design.md §4.3): the payload is the
    /// project id. The sub-view and the open Document of its Linked
    /// Documents sub-view live per tab (see [`Tab::project_section`]).
    Project(i64),
    /// The Project editor: `None` creates a new Project (with its
    /// optional Knowledge Base / grouping-node binding), `Some(id)`
    /// edits an existing one (the rebind flow).
    ProjectEditor(Option<i64>),
    Trash,
    Settings,
}

/// A Project view's sub-view (docs/ui-design.md §4.3), remembered per
/// tab like the Today view's selected date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectSection {
    #[default]
    Overview,
    Tasks,
    Board,
    Documents,
}

impl ProjectSection {
    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "概览与里程碑",
            Self::Tasks => "任务列表",
            Self::Board => "看板",
            Self::Documents => "关联文档",
        }
    }
}

impl Route {
    /// The tab title (工作标签标题).
    pub fn title(&self) -> String {
        match self {
            Self::Start => "开始".to_owned(),
            Self::Today => "今天".to_owned(),
            Self::Upcoming => "即将到来".to_owned(),
            Self::Tasks => "待办".to_owned(),
            Self::Calendar => "日历".to_owned(),
            Self::QuickNotes | Self::QuickNote(_) => "小记".to_owned(),
            Self::Favorites => "收藏".to_owned(),
            Self::Tags | Self::Tag(_) => "标签".to_owned(),
            Self::KnowledgeBases => "知识库".to_owned(),
            Self::Book(_) => "知识库".to_owned(),
            Self::Document(_) => "文档".to_owned(),
            Self::Project(_) => "项目".to_owned(),
            Self::ProjectEditor(_) => "项目".to_owned(),
            Self::Trash => "回收站".to_owned(),
            Self::Settings => "设置".to_owned(),
        }
    }

    /// The app's shared route icon for the sidebar and work tabs.
    pub fn icon(&self) -> Icon {
        match self {
            Self::Start => Icon::Start,
            Self::Today => Icon::Sun,
            Self::Upcoming => Icon::Clock,
            Self::Tasks => Icon::Tasks,
            Self::Calendar => Icon::Calendar,
            Self::QuickNotes | Self::QuickNote(_) => Icon::Note,
            Self::Favorites => Icon::Star,
            Self::Tags | Self::Tag(_) => Icon::Tag,
            Self::KnowledgeBases => Icon::Books,
            Self::Book(_) => Icon::Book,
            Self::Document(_) => Icon::Document,
            Self::Project(_) | Self::ProjectEditor(_) => Icon::Project,
            Self::Trash => Icon::Trash,
            Self::Settings => Icon::Settings,
        }
    }
}

/// One workbench tab: its current destination, independent back/forward
/// history, pin state, the date the Today view has selected, and the
/// Project view's sub-view state.
#[derive(Debug, Clone, PartialEq)]
pub struct Tab {
    pub id: u64,
    pub current: Route,
    pub past: Vec<Route>,
    pub future: Vec<Route>,
    pub pinned: bool,
    pub selected_date: LocalDate,
    pub catalog_hidden: bool,
    /// Which Project sub-view shows (Overview / Tasks / Board /
    /// Documents).
    pub project_section: ProjectSection,
    /// The Document open inside the Linked Documents sub-view (None =
    /// the view's landing).
    pub project_doc: Option<i64>,
}

impl Tab {
    pub fn new(id: u64, route: Route, today: LocalDate) -> Self {
        Self {
            id,
            current: route,
            past: Vec::new(),
            future: Vec::new(),
            pinned: false,
            selected_date: today,
            catalog_hidden: false,
            project_section: ProjectSection::default(),
            project_doc: None,
        }
    }

    /// Navigates to a new destination, pushing the current one on the
    /// back stack and clearing the forward stack. Navigating to the
    /// current destination is a no-op (no history noise). The per-tab
    /// Project state belongs to the Project the tab was showing; any
    /// real navigation resets it (the focus-or-reuse rule keeps it when
    /// a sidebar click re-selects the same Project).
    pub fn navigate(&mut self, route: Route) {
        if route == self.current {
            return;
        }
        self.past.push(self.current.clone());
        self.current = route;
        self.future.clear();
        self.project_section = ProjectSection::default();
        self.project_doc = None;
    }

    /// Steps back in this tab's own history. Returns false at the oldest
    /// entry.
    pub fn back(&mut self) -> bool {
        match self.past.pop() {
            Some(route) => {
                self.future.push(self.current.clone());
                self.current = route;
                true
            }
            None => false,
        }
    }

    /// Steps forward in this tab's own history. Returns false at the
    /// newest entry.
    pub fn forward(&mut self) -> bool {
        match self.future.pop() {
            Some(route) => {
                self.past.push(self.current.clone());
                self.current = route;
                true
            }
            None => false,
        }
    }

    pub fn can_back(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_forward(&self) -> bool {
        !self.future.is_empty()
    }

    /// Drops every back/forward entry pointing at `route`. Trash and
    /// permanent-delete fallbacks navigate away from the doomed
    /// destination; without the purge, Back would happily reopen it
    /// (spec: a trashed Project cannot be opened).
    pub fn purge_route(&mut self, route: &Route) {
        self.past.retain(|entry| entry != route);
        self.future.retain(|entry| entry != route);
    }
}

/// The tab-strip rules: opening, closing, pinning, drag reorder, and the
/// sidebar's focus-or-reuse navigation.
#[derive(Debug, Clone, PartialEq)]
pub struct TabStrip {
    pub tabs: Vec<Tab>,
    pub active: usize,
    next_id: u64,
    closed: Vec<Tab>,
}

impl TabStrip {
    pub fn new(initial: Route, today: LocalDate) -> Self {
        Self {
            tabs: vec![Tab::new(1, initial, today)],
            active: 0,
            next_id: 2,
            closed: Vec::new(),
        }
    }

    fn active_tab(&self) -> &Tab {
        &self.tabs[self.active]
    }

    /// Opens a new tab at `route` and focuses it.
    pub fn open_tab(&mut self, route: Route, today: LocalDate) {
        let tab = Tab::new(self.next_id, route, today);
        self.next_id += 1;
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
    }

    /// The sidebar navigation rule (spec user story 3): focus the tab
    /// already showing `route`; otherwise reuse the active unpinned tab,
    /// or open a new tab when the active one is pinned.
    pub fn open_or_focus(&mut self, route: Route, today: LocalDate) {
        if let Some(index) = self.tabs.iter().position(|tab| tab.current == route) {
            self.active = index;
        } else if self.active_tab().pinned {
            self.open_tab(route, today);
        } else {
            self.navigate_active(route);
        }
    }

    /// Closes a tab. A pinned tab refuses to close (spec: Ctrl+W closes
    /// the current *unpinned* tab). Closing the active tab focuses its
    /// right neighbor (or the new last tab).
    pub fn close_tab(&mut self, id: u64) -> bool {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return false;
        };
        if self.tabs[index].pinned {
            return false;
        }
        let closed = self.tabs.remove(index);
        let date = closed.selected_date;
        self.closed.push(closed);
        if self.closed.len() > 32 {
            self.closed.remove(0);
        }
        if self.tabs.is_empty() {
            self.tabs.push(Tab::new(self.next_id, Route::Start, date));
            self.next_id += 1;
            self.active = 0;
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        } else if index < self.active {
            self.active -= 1;
        }
        true
    }

    pub fn navigate_active(&mut self, route: Route) {
        self.tabs[self.active].navigate(route);
    }

    /// Remembers the Project sub-view for the active tab.
    pub fn set_project_section(&mut self, section: ProjectSection) {
        self.tabs[self.active].project_section = section;
    }

    /// Opens (or closes, with `None`) a Document inside the active tab's
    /// Linked Documents sub-view, switching to that sub-view.
    pub fn set_project_document(&mut self, document: Option<i64>) {
        let tab = &mut self.tabs[self.active];
        tab.project_section = ProjectSection::Documents;
        tab.project_doc = document;
    }

    pub fn back(&mut self) -> bool {
        self.tabs[self.active].back()
    }

    pub fn forward(&mut self) -> bool {
        self.tabs[self.active].forward()
    }

    pub fn cycle(&mut self) {
        self.active = (self.active + 1) % self.tabs.len();
    }

    pub fn reopen_last(&mut self, is_live: impl Fn(&Route) -> bool) {
        while let Some(mut tab) = self.closed.pop() {
            if is_live(&tab.current) {
                tab.past.retain(&is_live);
                tab.future.retain(&is_live);
                self.tabs.push(tab);
                self.active = self.tabs.len() - 1;
                return;
            }
        }
    }

    pub fn set_active(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
    }

    pub fn toggle_pin(&mut self, id: u64) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) {
            tab.pinned = !tab.pinned;
        }
    }

    /// Moves the tab `id` onto the tab `target`, occupying the target's
    /// prior index (docs/kanon.html's tab drag: `splice(j, 0,
    /// splice(i, 1))` — the dragged tab takes the slot the target
    /// occupied, every tab between the two shifting one place). A drop
    /// on the tab itself, or on an unknown tab, is a no-op, as is a
    /// cancelled release. The active tab keeps its identity: `active`
    /// follows the same tab id to its new index, so reordering never
    /// activates a different tab.
    pub fn reorder(&mut self, id: u64, target: u64) -> bool {
        if id == target {
            return false;
        }
        let Some(from) = self.tabs.iter().position(|tab| tab.id == id) else {
            return false;
        };
        let Some(to) = self.tabs.iter().position(|tab| tab.id == target) else {
            return false;
        };
        let active_id = self.tabs[self.active].id;
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        self.active = self
            .tabs
            .iter()
            .position(|tab| tab.id == active_id)
            .unwrap_or(0);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> LocalDate {
        LocalDate::new(2026, 11, 20)
    }

    #[test]
    fn new_tab_starts_on_today_with_empty_history() {
        let strip = TabStrip::new(Route::Today, today());
        assert_eq!(strip.tabs.len(), 1);
        assert_eq!(strip.active_tab().current, Route::Today);
        assert!(!strip.active_tab().can_back());
        assert!(!strip.active_tab().can_forward());
    }

    #[test]
    fn navigation_pushes_history_and_clears_forward() {
        let mut strip = TabStrip::new(Route::Today, today());
        strip.navigate_active(Route::Start);
        strip.back();
        assert_eq!(strip.active_tab().current, Route::Today);
        assert!(strip.active_tab().can_forward());
        strip.navigate_active(Route::Tasks);
        assert!(
            !strip.active_tab().can_forward(),
            "new navigation clears the forward stack"
        );
        assert_eq!(strip.active_tab().current, Route::Tasks);
    }

    #[test]
    fn back_and_forward_walk_the_tab_own_history() {
        let mut strip = TabStrip::new(Route::Today, today());
        strip.navigate_active(Route::Start);
        strip.navigate_active(Route::Tasks);
        assert!(strip.back());
        assert_eq!(strip.active_tab().current, Route::Start);
        assert!(strip.back());
        assert_eq!(strip.active_tab().current, Route::Today);
        assert!(!strip.back(), "oldest entry refuses to go back");
        assert!(strip.forward());
        assert_eq!(strip.active_tab().current, Route::Start);
        assert!(strip.forward());
        assert_eq!(strip.active_tab().current, Route::Tasks);
        assert!(!strip.forward(), "newest entry refuses to go forward");
    }

    #[test]
    fn navigating_to_the_current_route_is_a_no_op() {
        let mut strip = TabStrip::new(Route::Today, today());
        strip.navigate_active(Route::Today);
        assert!(!strip.active_tab().can_back());
    }

    #[test]
    fn sidebar_click_focuses_an_open_tab() {
        let mut strip = TabStrip::new(Route::Today, today());
        strip.open_tab(Route::Tasks, today());
        assert_eq!(strip.active, 1);
        strip.open_or_focus(Route::Today, today());
        assert_eq!(strip.active, 0, "clicking 今天 focuses its existing tab");
        assert_eq!(strip.tabs.len(), 2, "no new tab is opened");
    }

    #[test]
    fn sidebar_click_reuses_the_active_unpinned_tab() {
        let mut strip = TabStrip::new(Route::Today, today());
        strip.open_or_focus(Route::Tasks, today());
        assert_eq!(
            strip.tabs.len(),
            1,
            "the unpinned tab navigates instead of opening a new tab"
        );
        assert_eq!(strip.active_tab().current, Route::Tasks);
        assert!(strip.active_tab().can_back(), "reuse still records history");
    }

    #[test]
    fn sidebar_click_opens_a_new_tab_when_the_active_one_is_pinned() {
        let mut strip = TabStrip::new(Route::Today, today());
        let pinned_id = strip.tabs[0].id;
        strip.toggle_pin(pinned_id);
        strip.open_or_focus(Route::Tasks, today());
        assert_eq!(strip.tabs.len(), 2, "a pinned active tab forces a new tab");
        assert_eq!(strip.active, 1);
        assert_eq!(
            strip.tabs[0].current,
            Route::Today,
            "the pinned tab keeps its place"
        );
    }

    #[test]
    fn pinned_tabs_refuse_to_close_and_a_tab_always_remains() {
        let mut strip = TabStrip::new(Route::Today, today());
        let id = strip.tabs[0].id;
        strip.toggle_pin(id);
        assert!(!strip.close_tab(id), "pinned tabs do not close");
        strip.toggle_pin(id);
        assert!(strip.close_tab(id));
        assert_eq!(
            strip.tabs.len(),
            1,
            "closing the last tab opens a fresh Start tab"
        );
        assert_eq!(strip.active_tab().current, Route::Start);
    }

    #[test]
    fn closing_the_active_tab_focuses_the_right_neighbor() {
        let mut strip = TabStrip::new(Route::Today, today());
        strip.open_tab(Route::Tasks, today());
        strip.open_tab(Route::Calendar, today());
        let middle = strip.tabs[1].id;
        assert!(strip.close_tab(middle));
        assert_eq!(strip.active, 1, "the right neighbor takes focus");
        let last = strip.tabs[1].id;
        assert!(strip.close_tab(last));
        assert_eq!(strip.active, 0);
    }

    #[test]
    fn project_sub_view_state_is_per_tab_and_opens_documents_inside_it() {
        let mut strip = TabStrip::new(Route::Project(3), today());
        assert_eq!(strip.active_tab().project_section, ProjectSection::Overview);
        strip.set_project_section(ProjectSection::Board);
        strip.set_project_document(Some(9));
        assert_eq!(
            strip.active_tab().project_section,
            ProjectSection::Documents
        );
        assert_eq!(strip.active_tab().project_doc, Some(9));

        // Another tab keeps its own project state.
        strip.open_tab(Route::Project(4), today());
        assert_eq!(strip.active_tab().project_section, ProjectSection::Overview);
        assert_eq!(strip.active_tab().project_doc, None);

        // Focusing the first project tab restores its state.
        strip.open_or_focus(Route::Project(3), today());
        assert_eq!(strip.active, 0);
        assert_eq!(
            strip.active_tab().project_section,
            ProjectSection::Documents
        );
        assert_eq!(strip.active_tab().project_doc, Some(9));

        // Navigating away (or to another Project) resets the per-tab
        // state: it belonged to the Project the tab was showing.
        strip.navigate_active(Route::Project(7));
        assert_eq!(strip.active_tab().project_section, ProjectSection::Overview);
        assert_eq!(strip.active_tab().project_doc, None);
    }

    #[test]
    fn document_routes_focus_by_document_id_and_read_concurrently_in_new_tabs() {
        let mut strip = TabStrip::new(Route::Document(7), today());
        strip.open_tab(Route::Document(9), today());
        strip.open_or_focus(Route::Document(7), today());
        assert_eq!(
            strip.active, 0,
            "opening a Document focuses the tab already showing it"
        );
        assert_eq!(
            strip.tabs.len(),
            2,
            "no duplicate tab for the same document"
        );
        strip.open_tab(Route::Document(7), today());
        assert_eq!(
            strip
                .tabs
                .iter()
                .filter(|tab| tab.current == Route::Document(7))
                .count(),
            2,
            "a second tab on the same document is how concurrent reading happens"
        );
        strip.open_or_focus(Route::Tasks, today());
        assert_eq!(
            strip.active_tab().current,
            Route::Tasks,
            "other routes keep navigating by reuse"
        );
    }

    #[test]
    fn drag_a_right_tab_onto_a_left_tab_takes_the_targets_prior_index() {
        let mut strip = TabStrip::new(Route::Today, today());
        strip.open_tab(Route::Tasks, today());
        strip.open_tab(Route::Calendar, today());
        let today_id = strip.tabs[0].id;
        let tasks_id = strip.tabs[1].id;
        let calendar_id = strip.tabs[2].id;
        assert!(
            strip.reorder(calendar_id, today_id),
            "a real drop onto another tab reorders"
        );
        let order = |strip: &TabStrip| strip.tabs.iter().map(|tab| tab.id).collect::<Vec<_>>();
        assert_eq!(
            order(&strip),
            vec![calendar_id, today_id, tasks_id],
            "the dragged tab occupies the target's prior index; the target and the tabs it jumped over shift one place"
        );
        assert!(
            !strip.reorder(tasks_id, 999),
            "dropping on a tab that does not exist is an invalid drop: order stays"
        );
        assert_eq!(order(&strip), vec![calendar_id, today_id, tasks_id]);
    }

    #[test]
    fn drag_a_left_tab_onto_a_right_tab_moves_everything_between() {
        let mut strip = TabStrip::new(Route::Today, today());
        strip.open_tab(Route::Tasks, today());
        strip.open_tab(Route::Calendar, today());
        let today_id = strip.tabs[0].id;
        let tasks_id = strip.tabs[1].id;
        let calendar_id = strip.tabs[2].id;
        assert!(strip.reorder(today_id, calendar_id));
        let order = || strip.tabs.iter().map(|tab| tab.id).collect::<Vec<_>>();
        assert_eq!(order(), vec![tasks_id, calendar_id, today_id]);
    }

    #[test]
    fn drop_onto_the_tab_itself_leaves_the_order() {
        let mut strip = TabStrip::new(Route::Today, today());
        strip.open_tab(Route::Tasks, today());
        let (first, second) = (strip.tabs[0].id, strip.tabs[1].id);
        assert!(
            !strip.reorder(second, second),
            "dropping a tab on itself is a no-op"
        );
        assert_eq!(
            strip.tabs.iter().map(|tab| tab.id).collect::<Vec<_>>(),
            vec![first, second]
        );
    }

    #[test]
    fn reorder_preserves_the_active_tabs_identity_not_its_index() {
        // The active tab is a bystander: its index follows it.
        let mut strip = TabStrip::new(Route::Today, today());
        strip.open_tab(Route::Tasks, today());
        strip.open_tab(Route::Calendar, today());
        strip.set_active(1);
        let active = strip.tabs[strip.active].id;
        let rightmost = strip.tabs[2].id;
        let leftmost = strip.tabs[0].id;
        assert!(strip.reorder(rightmost, leftmost));
        assert_eq!(
            strip.tabs[strip.active].id, active,
            "the same tab is still active after others moved around it"
        );
        assert_eq!(
            strip.active, 2,
            "jumping the rightmost tab left pushed the bystander one slot right"
        );

        // The active tab is the dragged one: it stays active from its new
        // index, and no other tab takes focus.
        let mut strip = TabStrip::new(Route::Today, today());
        strip.open_tab(Route::Tasks, today());
        strip.open_tab(Route::Calendar, today());
        let rightmost = strip.tabs[2].id;
        let leftmost = strip.tabs[0].id;
        assert!(strip.reorder(rightmost, leftmost));
        assert_eq!(
            strip.active, 0,
            "the dragged tab lands at the target's index"
        );
        assert_eq!(
            strip.tabs[strip.active].id, rightmost,
            "reordering never activates a different tab"
        );
    }

    #[test]
    fn reorder_carries_history_pin_date_and_project_state_with_the_tab() {
        let mut strip = TabStrip::new(Route::Today, today());
        strip.open_tab(Route::Project(3), today());
        strip.open_tab(Route::Calendar, today());
        // Give the middle tab a history, a pin, and per-tab state.
        strip.set_active(1);
        strip.navigate_active(Route::Project(5));
        let middle = strip.tabs[1].id;
        strip.toggle_pin(middle);
        strip.set_project_document(Some(9));
        let leftmost = strip.tabs[0].id;

        assert!(strip.reorder(middle, leftmost));
        assert_eq!(
            strip.tabs[0].id, middle,
            "the tab moved to the target's prior index"
        );
        let moved = &strip.tabs[0];
        assert!(moved.pinned, "the pin state travels with the tab");
        assert!(moved.can_back(), "its back history travels with it");
        assert_eq!(moved.project_section, ProjectSection::Documents);
        assert_eq!(moved.project_doc, Some(9));
        assert!(!strip.tabs[1].pinned, "the tab it jumped over is untouched");
        assert!(
            strip.reorder(strip.tabs[0].id, strip.tabs[2].id),
            "reordering a pinned tab is allowed; only closing refuses them"
        );
        assert_eq!(
            strip.tabs[2].id, middle,
            "the pinned tab took the right target's prior index"
        );
    }
}
