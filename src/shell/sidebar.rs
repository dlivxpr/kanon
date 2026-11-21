//! The themed outer sidebar (docs/ui-design.md §3.3):
//! main navigation, collapsible to a 64 px icon rail.

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::ui::chrome::{Icon, chrome_button, hint, icon};
use crate::ui::theme as tokens;

use super::store::{Store, use_store};
use super::tabs::{ProjectSection, Route};

/// The primary routes and their stable visibility preference keys.
pub(super) const NAV: &[(Route, &str)] = &[
    (Route::Start, "show_nav_start"),
    (Route::Today, "show_nav_today"),
    (Route::Upcoming, "show_nav_upcoming"),
    (Route::Tasks, "show_nav_tasks"),
    (Route::Calendar, "show_nav_calendar"),
    (Route::QuickNotes, "show_nav_notes"),
    (Route::Favorites, "show_nav_favorites"),
    (Route::Tags, "show_nav_tags"),
];

const BOTTOM: &[Route] = &[Route::Trash, Route::Settings];

/// Whether the responsive rules (docs/ui-design.md §3.4) currently
/// force the outer sidebar into its icon rail: a window under
/// 1440 px showing an inner-catalog destination. The strip read is
/// tracked: bindings that ask re-run when the active tab changes.
/// A Project counts only while its Linked Documents sub-view shows.
pub(crate) fn sidebar_auto_collapsed(store: Store) -> bool {
    let strip = store.strip.get();
    let width = window().width.0;
    width < 960.0
        || width < 1440.0
            && strip
                .tabs
                .get(strip.active)
                .is_some_and(|tab| match &tab.current {
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

pub(super) fn sidebar(open_book_form: impl Fn() + Copy + 'static) -> impl IntoElement {
    let store = use_store();
    // The window width subscribes the binding: resizing across the
    // 1440 px breakpoint re-applies the auto-collapse rule
    // 188 pt at intermediate widths, like the prototype.
    let width = move || {
        let width = window().width.0;
        if sidebar_effective_collapsed(store) {
            px(56.0)
        } else if width <= 1200.0 {
            px(188.0)
        } else {
            px(224.0)
        }
    };
    view()
        .w(width)
        .h_full()
        .flex_shrink(0.0)
        .flex_col()
        .px_2()
        .min_h(px(0.0))
        .gap_1()
        .child(brand(store))
        .child(
            scroll().flex_1().min_h(px(0.0)).child(
                view()
                    .gap_2()
                    .child(primary_nav(store))
                    .child(projects_section(store))
                    .child(books_section(store, open_book_form)),
            ),
        )
        .child(
            view()
                .pt_1()
                .border_t_1()
                .border_color(tokens::shell_border)
                .child(nav_list(store, BOTTOM)),
        )
}

fn brand(store: Store) -> impl IntoElement {
    let collapsed = move || sidebar_effective_collapsed(store);
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .px_1()
        .pt_2()
        .pb_4()
        .style_when(collapsed, |s| s.justify_center())
        .child(
            view()
                .size(px(32.0))
                .flex_shrink(0.0)
                .items_center()
                .justify_center()
                .rounded(tokens::radius_row())
                .bg(tokens::brand_bg)
                .text_color(tokens::brand_fg)
                .child(icon(|| Icon::Brand, px(21.0))),
        )
        .child(when(
            move || !collapsed(),
            || {
                view()
                    .child(
                        text("Kanon")
                            .font_family("Georgia")
                            .text_size(px(21.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(tokens::shell_fg),
                    )
                    .child(
                        text("个人工作区")
                            .text_size(px(11.0))
                            .text_color(tokens::shell_muted),
                    )
            },
        ))
}

fn books_section(store: Store, open_book_form: impl Fn() + Copy + 'static) -> impl IntoElement {
    let expanded = signal(true);
    let collapsed = move || sidebar_effective_collapsed(store);
    view()
        .gap_1()
        .child(when(
            move || !collapsed(),
            move || {
                view()
                    .flex_row()
                    .items_center()
                    .px_2()
                    .gap_1()
                    .child(section_toggle("知识库", expanded).flex_1())
                    .child(
                        chrome_button(|| Icon::Books, || "全部知识库".into(), true)
                            .test_id("nav-知识库")
                            .on_press(move || store.open_or_focus(Route::KnowledgeBases)),
                    )
                    .child(
                        chrome_button(|| Icon::Plus, || "新建知识库".into(), true)
                            .test_id("nav-new-book")
                            .on_press(open_book_form),
                    )
            },
        ))
        .child(when(collapsed, move || {
            chrome_button(|| Icon::Books, || "全部知识库".into(), false)
                .self_center()
                .test_id("nav-知识库")
                .on_press(move || store.open_or_focus(Route::KnowledgeBases))
        }))
        .child(when(
            move || expanded.get() && !collapsed(),
            move || {
                view().gap_1().child(each(
                    move || store.knowledge_bases(),
                    |book: &crate::domain::KnowledgeBase| book.id,
                    move |book, _| {
                        nsh_base::button()
                            .h(px(32.0))
                            .flex_row()
                            .items_center()
                            .justify_start()
                            .px_2()
                            .gap_3()
                            .rounded(tokens::radius_row())
                            .hover(|s| s.bg(tokens::shell_surface))
                            .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
                            .accessibility_label(move || {
                                Some(book.with(|book| book.name.clone()).into())
                            })
                            .selected(move || {
                                store.strip.with(|strip| {
                                    strip.tabs[strip.active].current
                                        == Route::Book(book.with(|b| b.id))
                                })
                            })
                            .selected_style(|s| s.bg(tokens::shell_surface_active))
                            .on_press(move || {
                                store.open_sidebar_route(Route::Book(book.with(|b| b.id)))
                            })
                            .child(icon(|| Icon::Book, px(16.0)).text_color(tokens::info))
                            .child(
                                text(move || book.with(|book| book.name.clone()))
                                    .text_size(px(13.0))
                                    .number_of_lines(1),
                            )
                    },
                ))
            },
        ))
}

fn section_toggle(label: &'static str, expanded: Signal<bool>) -> nsh_base::ButtonElement {
    nsh_base::button()
        .h(px(26.0))
        .flex_row()
        .items_center()
        .gap_1()
        .justify_start()
        .rounded(tokens::radius_row())
        .text_color(tokens::shell_muted)
        .hover(|s| s.bg(tokens::shell_surface))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
        .selected(expanded)
        .accessibility_label(label)
        .on_press(move || expanded.update(|value| *value = !*value))
        .child(icon(
            move || {
                if expanded.get() {
                    Icon::Down
                } else {
                    Icon::Forward
                }
            },
            px(13.0),
        ))
        .child(text(label).text_size(px(11.0)))
}

/// The collapsible 项目 section (docs/ui-design.md §3.3): the active
/// projects with their emoji/color and the open-task badge, plus a `+`
/// that quick-creates a Project. Rows collapse to icons in the rail.
fn projects_section(store: Store) -> impl IntoElement {
    let collapsed = move || sidebar_effective_collapsed(store);
    let expanded = signal(true);
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
                    move || section_toggle("项目", expanded).flex_1(),
                ))
                .child(when(collapsed, move || {
                    view()
                        .items_center()
                        .child(icon(|| Icon::Project, px(16.0)).text_color(tokens::shell_muted))
                }))
                .child(when(
                    move || !collapsed(),
                    move || {
                        chrome_button(|| Icon::Plus, || "新建项目".into(), true)
                            .test_id("project-new")
                            .on_press(move || store.open_project_editor(None))
                    },
                )),
        )
        .child(when(
            move || expanded.get() || collapsed(),
            move || {
                view().child(each(
                    projects,
                    |project: &crate::domain::Project| project.id,
                    move |project, _index| project_row(store, project, collapsed),
                ))
            },
        ))
}

fn project_row(
    store: Store,
    project: ReadSignal<crate::domain::Project>,
    collapsed: impl Fn() -> bool + Copy + 'static,
) -> impl IntoElement {
    let is_active = move || {
        let strip = store.strip.get();
        strip
            .tabs
            .get(strip.active)
            .is_some_and(|tab| tab.current == Route::Project(project.with(|p| p.id)))
    };
    let open_count = move || {
        store.bump.get();
        store.project_open_count(project.with(|p| p.id))
    };
    nsh_base::button()
        .on_press(move || store.open_sidebar_route(Route::Project(project.with(|p| p.id))))
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

fn primary_nav(store: Store) -> impl IntoElement {
    view().gap(px(2.0)).children(NAV.iter().map(|(route, key)| {
        when(
            move || store.preference_enabled(key),
            move || nav_row(store, route),
        )
    }))
}

fn nav_list(store: Store, routes: &'static [Route]) -> impl IntoElement {
    view()
        .flex_col()
        .gap(px(2.0))
        .children(routes.iter().map(|route| nav_row(store, route)))
}

fn nav_row(store: Store, route: &'static Route) -> impl IntoElement {
    let matches = move |current: &Route| {
        current == route
            || matches!(
                (route, current),
                (Route::Tags, Route::Tag(_)) | (Route::QuickNotes, Route::QuickNote(_))
            )
    };
    let is_open = move || {
        store
            .strip
            .get()
            .tabs
            .iter()
            .any(|tab| matches(&tab.current))
    };
    let is_active = move || {
        let strip = store.strip.get();
        strip
            .tabs
            .get(strip.active)
            .is_some_and(|tab| matches(&tab.current))
    };
    let collapsed = move || sidebar_effective_collapsed(store);
    let label = route.title();
    let glyph = route.icon();
    nsh_base::button()
        .on_press(move || store.open_or_focus(route.clone()))
        .accessibility_label(Some(label.clone().into()))
        .test_id(&format!("nav-{}", route.title()))
        .h(tokens::side_row_h())
        .flex_row()
        .items_center()
        .gap_2()
        .px_2()
        .rounded(tokens::radius_row())
        .justify_start()
        .style_when(collapsed, |s| s.justify_center())
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
        .child(icon(move || glyph, px(16.0)).text_color(move || {
            if is_active() {
                tokens::accent_ink()
            } else if is_open() {
                tokens::shell_fg()
            } else {
                tokens::shell_muted()
            }
        }))
        .child(when(
            move || !collapsed(),
            move || {
                text(label.clone())
                    .text_size(tokens::text_px(13.0))
                    .number_of_lines(1)
                    .text_color(move || {
                        if is_open() {
                            tokens::shell_fg()
                        } else {
                            tokens::shell_muted()
                        }
                    })
            },
        ))
        .child(hint(route.title()))
}
