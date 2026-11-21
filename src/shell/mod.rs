//! The multi-tab desktop workbench shell (docs/ui-design.md §3):
//! top chrome with the tab strip, the collapsible outer sidebar, the
//! main panel, and the toggleable right context panel.

mod appearance;
mod calendar;
mod collections;
pub mod context;
mod create;
mod quick_notes;
mod reminders;
mod search;
pub mod sidebar;
mod start;
pub mod store;
pub mod tabs;
pub mod task_detail;
pub mod task_row;
pub mod topbar;

use nsh_core::prelude::*;

use crate::ui::theme as tokens;

pub use store::Store;
use tabs::Route;

/// Builds the workbench. The root component provides the store.
pub fn workbench(store: Store) -> View {
    let accent = computed(move || store.preference(appearance::PREF_ACCENT));
    effect(move || {
        accent.with(|key| {
            if let Some(key) = key {
                untracked(|| tokens::apply_accent(Some(key)));
            }
        });
    });
    let presentation = computed(move || {
        (
            store.preference(appearance::PREF_UI_FONT),
            store.preference(appearance::PREF_TEXT_SIZE),
            store.preference(appearance::PREF_DENSITY),
        )
    });
    effect(move || {
        presentation.with(|(font, size, density)| {
            untracked(|| {
                tokens::apply_ui_font(font.as_deref());
                tokens::apply_text_size(size.as_deref());
                tokens::apply_density(density.as_deref());
            })
        });
    });
    reminders::watch(store);
    let search_open = signal(false);
    let create_open = signal(None);
    let open_book_form = move || create_open.set(Some(create::Create::Book));
    let overview_filter = signal(context::OverviewFilter::All);
    let overview_capture = signal(context::CaptureMode::Task);
    let active_document = computed(move || {
        store.strip.with(|strip| {
            let tab = &strip.tabs[strip.active];
            match tab.current {
                Route::Document(id) => Some(id),
                Route::Project(_) if tab.project_section == tabs::ProjectSection::Documents => {
                    tab.project_doc
                }
                _ => None,
            }
        })
    });
    effect(move || {
        if let Some(id) = active_document.get() {
            untracked(|| store.record_document_visit(id));
        }
    });
    keyboard_shortcuts(store);
    on_key("mod+k", move |_| search_open.set(true));
    on_key("mod+j", move |_| search_open.set(true));
    view()
        .flex_1()
        .min_h(px(0.0))
        .flex_col()
        .bg(tokens::shell_bg)
        .text_color(tokens::shell_fg)
        .child(topbar::top_bar(search_open, create_open))
        .child(
            view()
                .flex_1()
                .relative()
                .min_h(px(0.0))
                .flex_row()
                .px_2()
                .pb_2()
                .gap_2()
                .child(when(
                    move || window().width.0 >= 680.0,
                    move || sidebar::sidebar(open_book_form),
                ))
                .child(main_panel(store, create_open))
                .child(when(
                    move || store.panel_visible(),
                    move || {
                        view()
                            .test_id("context-pane")
                            .w(|| {
                                let width = window().width.0;
                                px(if width < 960.0 {
                                    300.0
                                } else if width <= 1200.0 {
                                    266.0
                                } else {
                                    (width * 0.227).clamp(292.0, 326.0)
                                })
                            })
                            .flex_shrink(0.0)
                            .style_when(
                                || window().width.0 < 960.0,
                                |s| {
                                    s.absolute()
                                        .right_2()
                                        .top_0()
                                        .bottom_2()
                                        .shadow_lg()
                                        .z_index(2)
                                },
                            )
                            .child(dynamic(move || {
                                context_body(
                                    store,
                                    store.panel_choice(),
                                    overview_filter,
                                    overview_capture,
                                )
                            }))
                    },
                ))
                .child(when(
                    move || window().width.0 < 680.0 && store.sidebar_temp_expand.get(),
                    move || {
                        view()
                            .absolute()
                            .inset_0()
                            .z_index(3)
                            .flex_row()
                            .child(
                                view()
                                    .bg(tokens::shell_bg)
                                    .shadow_lg()
                                    .child(sidebar::sidebar(open_book_form)),
                            )
                            .child(
                                nsh_base::button()
                                    .flex_1()
                                    .accessibility_label("关闭主导航")
                                    .on_press(move || store.sidebar_temp_expand.set(false)),
                            )
                    },
                )),
        )
        .child(when(search_open, move || {
            search::command_palette(store, search_open, create_open)
        }))
        .child(dynamic(move || {
            create_open
                .get()
                .map(|screen| untracked(|| create::create_dialog(store, create_open, screen)))
        }))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ContextChoice {
    Auto,
    Inbox,
    Calendar,
    Outline,
    Links,
}

fn context_body(
    store: Store,
    choice: ContextChoice,
    filter: Signal<context::OverviewFilter>,
    capture: Signal<context::CaptureMode>,
) -> AnyElement {
    use context::ContextMode;
    let selected = store.selected_task();
    let document = context::active_document(store);
    untracked(move || {
        let mode = match (selected, choice, document) {
            (Some(id), _, _) => ContextMode::Detail(id),
            (None, ContextChoice::Outline, Some((tab_id, document_id))) => ContextMode::Outline {
                tab_id,
                document_id,
            },
            (None, ContextChoice::Links, Some((tab_id, document_id))) => ContextMode::Links {
                tab_id,
                document_id,
            },
            (None, ContextChoice::Calendar, None) => {
                return crate::ui::controls::panel()
                    .child(context::mode_heading("迷你日历"))
                    .child(calendar::mini_calendar(store))
                    .into_element();
            }
            (None, ContextChoice::Auto, Some(_)) => return context::context_panel().into_element(),
            _ => ContextMode::Overview,
        };
        context::context_panel_modes(mode, filter, capture).into_element()
    })
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
    on_key("mod+shift+t", move |_| store.reopen_tab());
    on_key("mod+tab", move |_| store.cycle_tab());
    for digit in 1..=9 {
        let keystroke = format!("mod+{digit}");
        on_key(&keystroke, move |_| store.activate_tab(digit as usize - 1));
    }
    on_key("mod+\\", move |_| store.toggle_sidebar());
    on_key("mod+.", move |_| store.toggle_panel());
}

/// The main canvas: the active tab's view.
fn main_panel(store: Store, create_open: Signal<Option<create::Create>>) -> impl IntoElement {
    let current = computed(move || {
        let strip = store.strip.get();
        let tab = &strip.tabs[strip.active];
        (tab.id, tab.current.clone())
    });
    crate::ui::controls::panel().child(dynamic(move || {
        let (tab_id, route) = current.get();
        let reveal = crate::ui::motion::reveal();
        view()
            .bind_motion(&reveal)
            .flex_1()
            .min_h(px(0.0))
            .child(untracked(move || match route {
                Route::Today => crate::today::today_view(store).into_element(),
                Route::Start => start::start_view(store, create_open).into_element(),
                Route::Tasks => crate::tasks::tasks_view(store, move || {
                    create_open.set(Some(create::Create::Task))
                })
                .into_element(),
                Route::Upcoming => crate::upcoming::upcoming_view(store).into_element(),
                Route::Calendar => calendar::calendar_view(store).into_element(),
                Route::Favorites => collections::favorites_view(store).into_element(),
                Route::Tags => collections::tags_view(store, None).into_element(),
                Route::Tag(id) => collections::tags_view(store, Some(id)).into_element(),
                Route::KnowledgeBases => crate::kb::kb_shelf(store).into_element(),
                Route::Trash => crate::kb::trash_view(store).into_element(),
                Route::Settings => appearance::settings_view(store).into_element(),
                Route::Book(book_id) => crate::kb::book_shell(
                    store,
                    tab_id,
                    book_id,
                    crate::kb::CatalogFocus::Home,
                    crate::kb::CatalogNav::Book,
                )
                .into_element(),
                Route::Document(doc_id) => match store.document(doc_id) {
                    Some(document) => crate::kb::book_shell(
                        store,
                        tab_id,
                        document.book_id,
                        crate::kb::CatalogFocus::Document(doc_id),
                        crate::kb::CatalogNav::Book,
                    )
                    .into_element(),
                    None => crate::kb::kb_shelf(store).into_element(),
                },
                Route::Project(project_id) => {
                    crate::projects::project_view(store, tab_id, project_id).into_element()
                }
                Route::ProjectEditor(editing) => {
                    crate::projects::project_editor_view(store, editing).into_element()
                }
                Route::QuickNotes => quick_notes::quick_notes_view(store, None).into_element(),
                Route::QuickNote(id) => {
                    quick_notes::quick_notes_view(store, Some(id)).into_element()
                }
            }))
            .key(format!("tab-{tab_id}"))
    }))
}
