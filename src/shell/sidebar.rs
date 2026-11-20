//! The outer Woday-style dark sidebar (docs/ui-design.md §3.3):
//! main navigation, collapsible to a 64 px icon rail.

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::ui::theme as tokens;

use super::store::{use_store, Store};
use super::tabs::{ProjectSection, Route};

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

/// Whether the responsive rules (docs/ui-design.md §3.4) currently
/// force the outer sidebar into its icon rail: a window under
/// 1440 px showing an inner-catalog destination. The strip read is
/// tracked: bindings that ask re-run when the active tab changes.
/// A Project counts only while its Linked Documents sub-view shows.
pub(crate) fn sidebar_auto_collapsed(store: Store) -> bool {
    let strip = store.strip.get();
    window().width.0 < 1440.0
        && strip.tabs.get(strip.active).is_some_and(|tab| match &tab.current {
            Route::Book(_) | Route::Document(_) => true,
            Route::Project(_) => tab.project_section == ProjectSection::Documents,
            _ => false,
        })
}

/// The sidebar's effective width state for the current window and
/// tab: the auto rule wins unless manually expanded for a while.
pub(crate) fn sidebar_effective_collapsed(store: Store) -> bool {
    store.bump.get();
    if sidebar_auto_collapsed(store) {
        !store.sidebar_temp_expand.get()
    } else {
        store.sidebar_collapsed.get()
    }
}

pub fn sidebar() -> impl IntoElement {
    let store = use_store();
    // The window width subscribes the binding: resizing across the
    // 1440 px breakpoint re-applies the auto-collapse rule
    // (docs/ui-design.md §3.4).
    let width = move || {
        let _ = window();
        if sidebar_effective_collapsed(store) { px(64.0) } else { px(232.0) }
    };
    view()
        .w(width)
        .flex_col()
        .gap_1()
        .py_2()
        .px_1()
        .rounded(tokens::radius_panel())
        .min_h(px(0.0))
        .child(nav_list(store, NAV))
        .child(projects_section(store))
        .child(view().flex_1().min_h(px(0.0)))
        .child(nav_list(store, BOTTOM))
}

/// The collapsible 项目 section (docs/ui-design.md §3.3): the active
/// projects with their emoji/color and the open-task badge, plus a `+`
/// that quick-creates a Project. Rows collapse to icons in the rail.
fn projects_section(store: Store) -> impl IntoElement {
    let collapsed = move || sidebar_effective_collapsed(store);
    let projects = move || {
        store.bump.get();
        store.projects()
    };
    view()
        .flex_col()
        .gap(px(2.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_1()
                .px_2()
                .h(px(26.0))
                .child(when(
                    move || !collapsed(),
                    move || {
                        text("项目")
                            .text_size(px(11.0))
                            .text_color(tokens::shell_muted())
                            .flex_1()
                            .number_of_lines(1)
                    },
                ))
                .child(when(
                    collapsed,
                    move || text("🗂").text_size(px(13.0)).text_color(tokens::shell_muted()).w(px(20.0)).text_center(),
                ))
                .child(when(
                    move || !collapsed(),
                    move || {
                        nsh_base::button()
                            .on_press(move || store.open_project_editor(None))
                            .accessibility_label("新建项目")
                            .test_id("project-new")
                            .size(px(20.0))
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .text_color(tokens::shell_muted())
                            .hover(|s| s.bg(tokens::shell_surface_active()).text_color(tokens::shell_fg()))
                            .child(text("＋").text_size(px(12.0)))
                    },
                )),
        )
        .child(each(
            projects,
            |project: &crate::domain::Project| project.id,
            move |project, _index| project_row(store, project, collapsed),
        ))
}

fn project_row(
    store: Store,
    project: ReadSignal<crate::domain::Project>,
    collapsed: impl Fn() -> bool + Copy + 'static,
) -> impl IntoElement {
    let is_active = move || {
        let strip = store.strip.get();
        strip.tabs.get(strip.active).is_some_and(|tab| tab.current == Route::Project(project.with(|p| p.id)))
    };
    let open_count = move || {
        store.bump.get();
        store.project_open_count(project.with(|p| p.id))
    };
    nsh_base::button()
        .on_press(move || store.open_project(project.with(|p| p.id)))
        .accessibility_label(move || Some(project.with(|p| p.name.clone()).into()))
        .test_id("sidebar-project")
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
        .child(
            text(move || project.with(|p| p.emoji.clone()))
                .text_size(px(15.0))
                .w(px(20.0))
                .text_center(),
        )
        .child(when(
            move || !collapsed(),
            move || {
                text(move || project.with(|p| p.name.clone()))
                    .text_size(px(13.0))
                    .number_of_lines(1)
                    .flex_1()
                    .text_color(tokens::shell_fg())
            },
        ))
        .child(when(
            move || !collapsed() && open_count() > 0,
            move || {
                text(move || format!("{}", open_count()))
                    .text_size(px(10.0))
                    .text_color(tokens::shell_muted())
                    .px_1()
                    .rounded_full()
                    .bg(tokens::shell_surface_active())
            },
        ))
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
    let collapsed = move || sidebar_effective_collapsed(store);
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
