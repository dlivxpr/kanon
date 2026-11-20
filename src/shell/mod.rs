//! The multi-tab desktop workbench shell (docs/ui-design.md §3):
//! top chrome with the tab strip, the collapsible outer sidebar, the
//! main panel, and the toggleable right context panel.

pub mod context;
pub mod sidebar;
pub mod store;
pub mod tabs;
pub mod task_detail;
pub mod task_row;
pub mod topbar;

use nsh_core::prelude::*;

use crate::ui::controls::empty_state;
use crate::ui::theme as tokens;

pub use store::Store;
use tabs::Route;

/// Builds the workbench. The root component provides the store.
pub fn workbench(store: Store) -> View {
    keyboard_shortcuts(store);
    view()
        .flex_1()
        .min_h(px(0.0))
        .flex_col()
        .bg(tokens::shell_bg())
        .text_color(tokens::shell_fg())
        .p_2()
        .gap_2()
        .child(topbar::top_bar())
        .child(
            view()
                .flex_1()
                .min_h(px(0.0))
                .flex_row()
                .gap_2()
                .child(sidebar::sidebar())
                .child(main_panel(store))
                .child(when(move || store.panel_open.get(), context::context_panel)),
        )
}

/// Global keyboard shortcuts (docs/ui-design.md §7).
fn keyboard_shortcuts(store: Store) {
    on_key("mod+t", move |_| store.open_new_tab());
    on_key("mod+w", move |_| {
        let strip = store.strip.get_untracked();
        if let Some(tab) = strip.tabs.get(strip.active) {
            store.close_tab(tab.id);
        }
    });
    for digit in 1..=9 {
        let keystroke = format!("mod+{digit}");
        on_key(&keystroke, move |_| store.activate_tab(digit as usize - 1));
    }
    on_key("mod+\\", move |_| store.toggle_sidebar());
    on_key("mod+.", move |_| store.toggle_panel());
}

/// The main canvas: the active tab's view.
fn main_panel(store: Store) -> impl IntoElement {
    let current = computed(move || {
        let strip = store.strip.get();
        let tab = &strip.tabs[strip.active];
        (tab.id, tab.current.clone())
    });
    crate::ui::controls::panel().child(dynamic(move || {
        let (tab_id, route) = current.get();
        view()
            .flex_1()
            .min_h(px(0.0))
            .child(untracked(move || match route {
                Route::Today => crate::today::today_view(store).into_element(),
                Route::Start => start_view(store).into_element(),
                Route::Tasks => crate::tasks::tasks_view(store).into_element(),
                Route::Upcoming => crate::upcoming::upcoming_view(store).into_element(),
                Route::KnowledgeBases => crate::kb::kb_shelf(store).into_element(),
                Route::Trash => crate::kb::trash_view(store).into_element(),
                Route::Book(book_id) => crate::kb::book_shell(store, book_id, crate::kb::CatalogFocus::Home)
                    .into_element(),
                Route::Document(doc_id) => match store.document(doc_id) {
                    Some(document) => crate::kb::book_shell(
                        store,
                        document.book_id,
                        crate::kb::CatalogFocus::Document(doc_id),
                    )
                    .into_element(),
                    None => crate::kb::kb_shelf(store).into_element(),
                },
                other => unreleased_view(other).into_element(),
            }))
            .key(format!("tab-{tab_id}"))
    }))
}

/// The new-tab landing view (开始): a calm workspace summary.
fn start_view(store: Store) -> impl IntoElement {
    let now = computed(move || {
        store.bump.get();
        store.now()
    });
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_3()
        .child(text("◇").text_size(px(40.0)).text_color(tokens::accent()))
        .child(text("Kanon").text_size(px(24.0)).font_weight(FontWeight::BOLD).text_color(tokens::fg_primary()))
        .child(
            text(move || format!("{} · 本地工作区已就绪", now.get().date.long_label()))
                .text_size(px(13.0))
                .text_color(tokens::fg_muted()),
        )
        .child(
            text("从左侧导航打开「今天」，或使用 ⌘/Ctrl+T 新建标签页")
                .text_size(px(12.0))
                .text_color(tokens::fg_muted()),
        )
}

/// An honest placeholder for views whose screens ship in later
/// milestones: the tab, navigation and shell behaviors around them are
/// fully real.
fn unreleased_view(route: Route) -> impl IntoElement {
    empty_state(
        route.title(),
        "该视图将在后续里程碑交付，标签与导航行为已可用",
    )
}
