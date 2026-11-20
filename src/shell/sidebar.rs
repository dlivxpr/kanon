//! The outer Woday-style dark sidebar (docs/ui-design.md §3.3):
//! main navigation, collapsible to a 64 px icon rail.

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::ui::theme as tokens;

use super::store::{use_store, Store};
use super::tabs::Route;

/// The main navigation destinations, in display order.
const NAV: &[Route] = &[
    Route::Start,
    Route::Today,
    Route::Upcoming,
    Route::Tasks,
    Route::Calendar,
    Route::QuickNotes,
    Route::Favorites,
    Route::Tags,
];

const BOTTOM: &[Route] = &[Route::KnowledgeBases, Route::Trash, Route::Settings];

pub fn sidebar() -> impl IntoElement {
    let store = use_store();
    view()
        .w(move || if store.sidebar_collapsed.get() { px(64.0) } else { px(232.0) })
        .flex_col()
        .gap_1()
        .py_2()
        .px_1()
        .rounded(tokens::radius_panel())
        .min_h(px(0.0))
        .child(nav_list(store, NAV))
        .child(view().flex_1().min_h(px(0.0)))
        .child(nav_list(store, BOTTOM))
}

fn nav_list(store: Store, routes: &'static [Route]) -> impl IntoElement {
    view().flex_col().gap(px(2.0)).children(routes.iter().map(|route| nav_row(store, route)))
}

fn nav_row(store: Store, route: &'static Route) -> impl IntoElement {
    let is_open = move || store.strip.get().tabs.iter().any(|tab| tab.current == *route);
    let is_active = move || {
        let strip = store.strip.get();
        strip.tabs.get(strip.active).is_some_and(|tab| tab.current == *route)
    };
    let collapsed = move || store.sidebar_collapsed.get();
    let label = route.title();
    let glyph = route.glyph();
    nsh_base::button()
        .on_press(move || store.open_or_focus(route.clone()))
        .accessibility_label(Some(label.clone().into()))
        .test_id(&format!("nav-{}", route.title()))
        .h(px(34.0))
        .flex_row()
        .items_center()
        .gap_2()
        .px_2()
        .rounded(tokens::radius_row())
        .justify_start()
        .bg(move || {
            if is_active() {
                tokens::shell_surface_active()
            } else {
                theme().background.opacity(0.0)
            }
        })
        .hover(|s| s.bg(tokens::shell_surface_active()))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(
            text(glyph)
                .text_size(px(15.0))
                .text_color(move || if is_open() { tokens::shell_fg() } else { tokens::shell_muted() })
                .w(px(20.0))
                .text_center(),
        )
        .child(when(
            move || !collapsed(),
            move || {
                text(label.clone())
                    .text_size(px(13.0))
                    .number_of_lines(1)
                    .text_color(move || if is_open() { tokens::shell_fg() } else { tokens::shell_muted() })
            },
        ))
}
