//! The right context panel (docs/ui-design.md §4.6): Task Overview by
//! default (overdue / today / inbox with a quick capture box), Task
//! Detail once a Task is opened, and the Document modes (大纲,
//! 关联与反链) on a Document route.
//!
//! The shell composes the panel chrome and switches modes
//! (docs/kanon.html `.ctx-head`); this module exposes one renderer per
//! mode — `overview`, `outline_panel`, `links_panel`, plus the
//! `task_detail` module — and `context_panel()` as the
//! selection-driven fallback until the shell wires its mode switch.

use nsh_base::{button, menu, menu_item, theme};
use nsh_core::prelude::*;

use crate::domain::{Priority, outline};
use crate::ui::chrome::{Icon, icon};
use crate::ui::controls::{ButtonKind, app_button, panel, section_heading, submit_field};
use crate::ui::theme as tokens;

use super::quick_notes::{quick_capture_box, recent_notes};
use super::store::{Store, use_store};
use super::tabs::Route;
use super::task_detail;
use super::topbar::mini_task_row;

/// The Task Overview's capture filter (docs/kanon.html `.inbox-filter`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum OverviewFilter {
    All,
    High,
    Done,
}

impl OverviewFilter {
    fn label(self) -> &'static str {
        match self {
            Self::All => "全部任务",
            Self::High => "高优先级",
            Self::Done => "已完成",
        }
    }

    fn all() -> [Self; 3] {
        [Self::All, Self::High, Self::Done]
    }
}

/// The capture box's mode (docs/kanon.html `.capture-switch`): 速记待办
/// creates Inbox Tasks, 速记小记 publishes Quick Notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum CaptureMode {
    Task,
    Note,
}

/// The panel for the shell's mode switch to compose: surface, heading,
/// body. The shell owns the width and the narrow-window overlay
/// (docs/kanon.html clamps the panel at 266–326 px).
pub(super) fn context_panel_modes(
    mode: ContextMode,
    filter: Signal<OverviewFilter>,
    capture: Signal<CaptureMode>,
) -> impl IntoElement {
    let store = use_store();
    let heading = match mode {
        ContextMode::Overview => overview_heading(filter, capture).into_element(),
        ContextMode::Detail(_) => mode_heading("任务详情").into_element(),
        ContextMode::Outline { .. } => mode_heading("大纲").into_element(),
        ContextMode::Links { .. } => mode_heading("关联与反链").into_element(),
    };
    let key = match mode {
        ContextMode::Overview => "overview".to_owned(),
        ContextMode::Detail(id) => format!("task-detail-{id}"),
        ContextMode::Outline { document_id, .. } => format!("outline-{document_id}"),
        ContextMode::Links { document_id, .. } => format!("links-{document_id}"),
    };
    let body = match mode {
        ContextMode::Overview => overview(store, filter, capture).into_element(),
        ContextMode::Detail(id) => task_detail::task_detail(store, id).into_element(),
        ContextMode::Outline {
            tab_id,
            document_id,
        } => outline_panel(store, tab_id, document_id).into_element(),
        ContextMode::Links { document_id, .. } => links_panel(store, document_id).into_element(),
    };
    panel()
        .key(key)
        .flex_col()
        .overflow_hidden()
        .child(heading)
        .child(body)
}

/// Which panel mode the shell shows (docs/ui-design.md §4.6). The
/// Document modes resolve against the active tab's route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ContextMode {
    /// 任务速览: overdue / today / inbox groups plus quick capture.
    Overview,
    /// 任务详情 for the selected Task.
    Detail(i64),
    /// 文档大纲 of the active tab's Document.
    Outline { tab_id: u64, document_id: i64 },
    /// 关联与反链 of the active tab's Document.
    Links { tab_id: u64, document_id: i64 },
}

/// The selection-driven panel: Task Detail once a Task is open, the
/// combined Document context on a Document route, Task Overview
/// otherwise. Used until the shell's topbar mode switch lands.
pub fn context_panel() -> impl IntoElement {
    let store = use_store();
    let filter = signal(OverviewFilter::All);
    let capture = signal(CaptureMode::Task);
    panel()
        .flex_col()
        .overflow_hidden()
        .child(dynamic(move || {
            let selected = store.selected_task();
            let document = active_document(store);
            untracked(move || match (selected, document) {
                (Some(_), _) => mode_heading("任务详情").into_element(),
                (None, Some(_)) => mode_heading("文档大纲与关联").into_element(),
                (None, None) => overview_heading(filter, capture).into_element(),
            })
        }))
        .child(dynamic(move || {
            // Only the selection selects the shape; building the content
            // reads more signals (the detail's aggregate computed evaluates
            // eagerly), which must not re-run this region.
            let selected = store.selected_task();
            let document = active_document(store);
            untracked(move || match (selected, document) {
                (Some(id), _) => task_detail::task_detail(store, id).into_element(),
                (None, Some((tab_id, doc_id))) => {
                    document_context(store, tab_id, doc_id).into_element()
                }
                (None, None) => overview(store, filter, capture).into_element(),
            })
        }))
}

/// The active tab's Document route, if any: (tab id, document id).
pub(super) fn active_document(store: Store) -> Option<(u64, i64)> {
    store
        .strip
        .with(|strip| match &strip.tabs[strip.active].current {
            super::tabs::Route::Document(id) => Some((strip.tabs[strip.active].id, *id)),
            _ => None,
        })
}

/// A panel mode's heading (docs/kanon.html `.ctx-head` /
/// `.ctx-heading`): 21 px bold title, chrome the shell lays out.
pub(super) fn mode_heading(title: &'static str) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .px(px(17.0))
        .pt(px(18.0))
        .pb(px(12.0))
        .child(
            text(title)
                .text_size(px(21.0))
                .font_weight(FontWeight::BOLD)
                .text_color(tokens::fg_primary()),
        )
}

/// The Task Overview heading: title plus the capture filter menu
/// (docs/kanon.html `.ctx-head` with `.inbox-filter`).
pub(super) fn overview_heading(
    filter: Signal<OverviewFilter>,
    capture: Signal<CaptureMode>,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .px(px(17.0))
        .pt(px(18.0))
        .pb(px(12.0))
        .child(
            view()
                .flex_row()
                .items_baseline()
                .gap_1()
                .min_w(px(0.0))
                .flex_1()
                .child(
                    text("任务速览")
                        .text_size(px(21.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(tokens::fg_primary()),
                )
                .child(
                    text("Inbox")
                        .text_size(px(15.0))
                        .text_color(tokens::fg_muted()),
                ),
        )
        // docs/kanon.html shows `.inbox-filter` only for 速记待办.
        .child(when(
            move || capture.get() == CaptureMode::Task,
            move || filter_menu(filter).into_element(),
        ))
}

/// The capture filter dropdown (docs/kanon.html `.inbox-filter`).
fn filter_menu(filter: Signal<OverviewFilter>) -> impl IntoElement {
    menu()
        .offset(px(4.0))
        .popup_style(|s| {
            s.p_1()
                .rounded(tokens::radius_card())
                .bg(theme().popover)
                .shadow_md()
                .min_w(px(150.0))
        })
        .trigger(
            nsh_base::button()
                .accessibility_label("筛选收件箱任务")
                .h(px(26.0))
                .flex_row()
                .flex_shrink(0.0)
                .px_2()
                .items_center()
                .gap_1()
                .rounded_full()
                .bg(tokens::control_bg)
                .text_color(tokens::fg_body)
                .hover(|s| s.bg(tokens::control_hover()))
                .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                .child(text(move || filter.get().label()).text_size(px(12.0)))
                .child(text("▾").text_size(px(10.0)).text_color(tokens::fg_muted())),
        )
        .child(each(
            move || Vec::from(OverviewFilter::all()),
            |option: &OverviewFilter| *option,
            move |option, _index| {
                let value = option.with(|option| *option);
                let selected = move || filter.get() == value;
                menu_item()
                    .on_select(move || filter.set(value))
                    .accessibility_label(move || Some(format!("筛选 {}", value.label()).into()))
                    .px_3()
                    .py_2()
                    .rounded(tokens::radius_row())
                    .hover(|s| s.bg(tokens::control_hover()))
                    .child(
                        view()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(
                                text(value.label())
                                    .text_size(px(13.0))
                                    .text_color(move || {
                                        if selected() {
                                            tokens::accent()
                                        } else {
                                            tokens::fg_primary()
                                        }
                                    })
                                    .flex_1(),
                            )
                            .child(when(selected, || {
                                text("✓").text_size(px(12.0)).text_color(tokens::accent())
                            })),
                    )
                    .into_element()
            },
        ))
}

/// The Document 大纲 mode (docs/ui-design.md §4.6): the committed
/// body's H1–H3 headings; pressing one asks the tab showing the
/// document to reveal that heading.
pub(super) fn outline_panel(store: Store, tab_id: u64, document_id: i64) -> impl IntoElement {
    let entries = move || {
        store.bump.get();
        let body = store
            .document(document_id)
            .map(|doc| doc.body)
            .unwrap_or_default();
        outline(&body, 3)
    };
    scroll().flex_1().child(
        view()
            .flex_col()
            .p_2()
            .gap_1()
            .child(when(
                move || entries().is_empty(),
                move || {
                    text("添加标题后会显示大纲。")
                        .text_size(px(11.0))
                        .text_color(tokens::fg_muted())
                },
            ))
            .child(each(
                entries,
                |entry: &crate::domain::OutlineEntry| entry.ordinal,
                move |entry, _index| {
                    let ordinal = entry.with(|entry| entry.ordinal);
                    let level = entry.with(|entry| entry.level);
                    let text_label = entry.with(|entry| entry.text.clone());
                    let jump = store;
                    nsh_base::button()
                        .on_press(move || jump.request_outline_jump(tab_id, ordinal))
                        .accessibility_label(text_label.clone())
                        .items_start()
                        .px_2()
                        .py_1()
                        .rounded(tokens::radius_row())
                        .bg(move || nsh_base::theme().background.opacity(0.0))
                        .hover(|s| s.bg(tokens::control_bg()))
                        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .pl(px((level.saturating_sub(1)) as f32 * 12.0))
                                .child(
                                    text(text_label)
                                        .text_size(px(12.0))
                                        .text_color(tokens::fg_body()),
                                ),
                        )
                },
            )),
    )
}

/// The Document 关联与反链 mode (docs/ui-design.md §4.6): this
/// Document's Linked Task Cards (live, directly toggle-able) and the
/// backlinks of Documents mentioning it — degrading while a source
/// Document sits in Trash and recovering on restore.
pub(super) fn links_panel(store: Store, document_id: i64) -> impl IntoElement {
    scroll().flex_1().child(
        view()
            .flex_col()
            .p_2()
            .gap_3()
            .child(linked_tasks_section(store, document_id))
            .child(backlinks_section(store, document_id)),
    )
}

/// The combined Document context the fallback panel renders until the
/// shell's mode switch lands: outline, Linked Task Cards, backlinks.
fn document_context(store: Store, tab_id: u64, document_id: i64) -> impl IntoElement {
    scroll().flex_1().child(
        view()
            .flex_col()
            .p_2()
            .gap_3()
            .child(outline_panel(store, tab_id, document_id))
            .child(linked_tasks_section(store, document_id))
            .child(backlinks_section(store, document_id)),
    )
}

/// 本文档关联的任务卡片: one live row per indexed Linked Task Card.
fn linked_tasks_section(store: Store, document_id: i64) -> impl IntoElement {
    let cards = move || {
        store.bump.get();
        store
            .document_references(document_id)
            .into_iter()
            .filter(|reference| reference.ref_type == "task_card")
            .collect::<Vec<_>>()
    };
    view()
        .flex_col()
        .gap_1()
        .child(section_heading("关联任务"))
        .child(when(
            move || cards().is_empty(),
            move || {
                text("正文中的关联任务卡片会出现在这里")
                    .text_size(px(11.0))
                    .text_color(tokens::fg_muted())
            },
        ))
        .child(dynamic(move || {
            view().flex_col().gap(px(2.0)).children(
                cards()
                    .into_iter()
                    .map(|reference| {
                        let task_id = reference.target_id.unwrap_or(0);
                        let snapshot = reference.target_label.clone();
                        let live = move || {
                            store.bump.get();
                            store.task(task_id)
                        };
                        let title = move || {
                            live()
                                .map(|a| a.task.title.clone())
                                .unwrap_or(snapshot.clone())
                        };
                        let trashed = move || live().is_none_or(|a| a.task.trashed_at.is_some());
                        view()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .px_2()
                            .py_1()
                            .rounded(tokens::radius_row())
                            .child(when(move || !trashed(), {
                                let title = title.clone();
                                move || task_circle_row(store, task_id, title()).into_element()
                            }))
                            .child(when(trashed, {
                                let title = title.clone();
                                move || {
                                    text(format!("任务已移除 · {}", title()))
                                        .text_size(px(12.0))
                                        .text_color(tokens::fg_muted())
                                        .flex_1()
                                        .number_of_lines(1)
                                        .into_element()
                                }
                            }))
                            .into_element()
                    })
                    .collect::<Vec<_>>(),
            )
        }))
}

/// 引用本文档的反向链接: every Document referencing this one. A source
/// in Trash renders inactive (degraded) and recovers on restore.
fn backlinks_section(store: Store, document_id: i64) -> impl IntoElement {
    let links = move || store.document_backlinks(document_id);
    view()
        .flex_col()
        .gap_1()
        .child(section_heading("反向链接"))
        .child(when(
            move || links().is_empty(),
            move || {
                text("其它文档用 @ 提及本文档后会出现在这里")
                    .text_size(px(11.0))
                    .text_color(tokens::fg_muted())
            },
        ))
        .child(dynamic(move || {
            view().flex_col().gap(px(2.0)).children(
                links()
                    .into_iter()
                    .map(|backlink| {
                        let source = backlink.source_document_id;
                        let title = backlink.source_title.clone();
                        let trashed = backlink.source_trashed;
                        let jump = store;
                        view()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .px_2()
                            .py_1()
                            .rounded(tokens::radius_row())
                            .child(when(move || !trashed, {
                                let title = title.clone();
                                move || {
                                    nsh_base::button()
                                        .on_press(move || jump.open_document(source))
                                        .accessibility_label(Some(
                                            nsh_core::element::SharedString::from(format!(
                                                "打开文档 {title}"
                                            )),
                                        ))
                                        .items_center()
                                        .px_1()
                                        .py(px(1.0))
                                        .rounded(tokens::radius_row())
                                        .bg(move || nsh_base::theme().background.opacity(0.0))
                                        .hover(|s| s.bg(tokens::control_bg()))
                                        .child(
                                            text(title.clone())
                                                .text_size(px(12.0))
                                                .text_color(tokens::fg_body())
                                                .number_of_lines(1),
                                        )
                                        .into_element()
                                }
                            }))
                            .child(when(move || trashed, {
                                let title = title.clone();
                                move || {
                                    text(format!("{title}（源文档已删除）"))
                                        .text_size(px(12.0))
                                        .text_color(tokens::fg_muted())
                                        .number_of_lines(1)
                                        .into_element()
                                }
                            }))
                            .into_element()
                    })
                    .collect::<Vec<_>>(),
            )
        }))
}

/// One live Linked Task Card row of the 关联任务 section: the quick
/// circle plus the title button. The labels say 关联任务 so they never
/// collide with the in-document card's or the task lists' circles.
fn task_circle_row(store: Store, task_id: i64, title: String) -> impl IntoElement {
    let done = move || {
        store.bump.get();
        store
            .task(task_id)
            .is_some_and(|a| a.task.status == crate::domain::TaskStatus::Done)
    };
    let label = title.clone();
    let open = store;
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .flex_1()
        .min_w(px(0.0))
        .child(crate::ui::controls::task_circle(
            done,
            format!("完成关联任务 {label}"),
            move || store.toggle_circle(task_id),
        ))
        .child(
            nsh_base::button()
                .on_press(move || open.open_task(task_id))
                .accessibility_label(Some(format!("打开关联任务 {label}").into()))
                .items_center()
                .child(
                    text(label.clone())
                        .text_size(px(13.0))
                        .text_color(tokens::fg_body())
                        .number_of_lines(1),
                ),
        )
}

/// Task Overview (任务速览): the 速记待办 / 速记小记 capture switch
/// (docs/kanon.html `.capture-switch`), then either the Inbox capture
/// box with the mutually-exclusive groups 逾期 / 今天 / 收件箱, or the
/// Quick Note capture with the latest three notes.
pub(super) fn overview(
    store: Store,
    filter: Signal<OverviewFilter>,
    capture: Signal<CaptureMode>,
) -> impl IntoElement {
    let groups = computed(move || {
        store.bump.get();
        store.task_groups()
    });
    let draft = signal(String::new());
    view().flex_1().min_h(px(0.0)).child(
        scroll().flex_1().child(
            view()
                .flex_col()
                .gap_4()
                .p_3()
                .child(capture_switch(capture))
                .child(when(
                    move || capture.get() == CaptureMode::Note,
                    move || {
                        view()
                            .flex_col()
                            .gap_3()
                            .child(quick_capture_box(store))
                            .child(
                                view()
                                    .flex_row()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        text("最近小记")
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(tokens::fg_body()),
                                    )
                                    .child(
                                        nsh_base::button()
                                            .on_press(move || {
                                                store.open_or_focus(Route::QuickNotes);
                                            })
                                            .accessibility_label("查看全部小记")
                                            .items_center()
                                            .px_1()
                                            .py(px(1.0))
                                            .rounded(tokens::radius_row())
                                            .bg(move || theme().background.opacity(0.0))
                                            .text_color(tokens::fg_muted())
                                            .hover(|s| {
                                                s.bg(tokens::control_bg())
                                                    .text_color(tokens::fg_primary())
                                            })
                                            .focus_visible(|s| {
                                                s.outline(px(2.0)).outline_color(tokens::accent())
                                            })
                                            .child(text("查看全部").text_size(px(12.0))),
                                    ),
                            )
                            .child(recent_notes(store, 3))
                            .into_element()
                    },
                ))
                .child(when(
                    move || capture.get() == CaptureMode::Task,
                    move || {
                        view()
                            .flex_col()
                            .gap_4()
                            .child(capture_box(store, draft))
                            .child(dynamic(move || match filter.get() {
                                OverviewFilter::Done => {
                                    overview_group(store, "已完成", move || groups.get().done)
                                        .into_element()
                                }
                                option => view()
                                    .flex_col()
                                    .gap_4()
                                    .child(overview_group(store, "逾期", move || {
                                        filter_priority(groups.get().overdue, option)
                                    }))
                                    .child(overview_group(store, "今天", move || {
                                        filter_priority(groups.get().today, option)
                                    }))
                                    .child(overview_group(store, "收件箱", move || {
                                        filter_priority(groups.get().inbox, option)
                                    }))
                                    .into_element(),
                            }))
                            .child(when(
                                move || overview_empty(groups, filter),
                                || {
                                    text("暂无符合条件的任务。")
                                        .text_size(px(13.0))
                                        .text_color(tokens::fg_muted())
                                },
                            ))
                            .into_element()
                    },
                )),
        ),
    )
}

/// The 速记待办 / 速记小记 switch above the capture box
/// (docs/kanon.html `.capture-switch`): a two-option segmented track.
fn capture_switch(capture: Signal<CaptureMode>) -> impl IntoElement {
    view()
        .flex_row()
        .gap(px(3.0))
        .p(px(3.0))
        .rounded(tokens::radius_row())
        .bg(tokens::control_bg)
        .child(capture_option(
            Icon::Tasks,
            "速记待办",
            CaptureMode::Task,
            capture,
        ))
        .child(capture_option(
            Icon::Note,
            "速记小记",
            CaptureMode::Note,
            capture,
        ))
}

/// One option of the capture switch: icon + label, the selected mode
/// standing on the raised surface.
fn capture_option(
    glyph: Icon,
    label: &'static str,
    value: CaptureMode,
    capture: Signal<CaptureMode>,
) -> impl IntoElement {
    let selected = move || capture.get() == value;
    button()
        .on_press(move |_: &()| capture.set(value))
        .accessibility_label(Some(label.into()))
        .selected(selected)
        .h(px(29.0))
        .flex_1()
        .flex_row()
        .items_center()
        .justify_center()
        .gap_1()
        .rounded(move || theme().radius)
        .bg(move || {
            if selected() {
                theme().card
            } else {
                theme().background.opacity(0.0)
            }
        })
        .text_color(move || {
            if selected() {
                tokens::fg_primary()
            } else {
                tokens::fg_muted()
            }
        })
        .hover(|s| s.bg(tokens::control_hover()))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(icon(move || glyph, px(14.0)))
        .child(text(label).text_size(px(12.0)))
}

/// The capture box (docs/kanon.html `.capture-box`): the quick entry
/// plus the hint naming where the Task lands and its commit button.
fn capture_box(store: Store, capture: Signal<String>) -> impl IntoElement {
    view()
        .flex_col()
        .gap_2()
        .px(px(12.0))
        .py(px(10.0))
        .rounded(tokens::radius_card())
        .bg(move || theme().card)
        .border_1()
        .border_color(tokens::border_default())
        .child(submit_field(
            "速记待办，回车保存",
            move || capture.get(),
            move |text: &str| capture.set(text.to_owned()),
            move |text: &str| {
                if store.create_inbox_task(text).is_some() {
                    capture.set(String::new());
                }
            },
        ))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .pt_2()
                .border_t_1()
                .border_color(tokens::border_subtle())
                .child(
                    text("未排期 · 无项目")
                        .text_size(px(11.0))
                        .text_color(tokens::fg_muted()),
                )
                .child(app_button(ButtonKind::Primary, "添加任务", move || {
                    if store.create_inbox_task(&capture.get()).is_some() {
                        capture.set(String::new());
                    }
                })),
        )
}

/// Applies the 高优先级 filter to one group's items; 全部任务 passes
/// through and 已完成 never reaches the grouped sections.
fn filter_priority(
    items: Vec<crate::domain::TaskAggregate>,
    filter: OverviewFilter,
) -> Vec<crate::domain::TaskAggregate> {
    match filter {
        OverviewFilter::High => items
            .into_iter()
            .filter(|aggregate| aggregate.task.priority == Priority::High)
            .collect(),
        _ => items,
    }
}

/// Whether every group is empty under the current filter
/// (docs/kanon.html `.ctx-list` empty state).
fn overview_empty(groups: Memo<crate::domain::TaskGroups>, filter: Signal<OverviewFilter>) -> bool {
    let groups = groups.get();
    match filter.get() {
        OverviewFilter::Done => groups.done.is_empty(),
        option => {
            filter_priority(groups.overdue.clone(), option).is_empty()
                && filter_priority(groups.today.clone(), option).is_empty()
                && filter_priority(groups.inbox.clone(), option).is_empty()
        }
    }
}

fn overview_group(
    store: Store,
    title: &'static str,
    items: impl Fn() -> Vec<crate::domain::TaskAggregate> + Copy + 'static,
) -> impl IntoElement {
    view()
        .flex_col()
        .gap_1()
        .child(when(
            move || !items().is_empty(),
            move || {
                view()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(
                        text(title)
                            .text_size(px(13.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(tokens::fg_body()),
                    )
                    .child(
                        text(move || format!("{}", items().len()))
                            .text_size(px(11.0))
                            .text_color(tokens::fg_muted()),
                    )
            },
        ))
        .child(view().flex_col().gap(px(2.0)).child(each(
            items,
            |row: &crate::domain::TaskAggregate| row.task.id,
            move |row, _| mini_task_row(store, row),
        )))
}
