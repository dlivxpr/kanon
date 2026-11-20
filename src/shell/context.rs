//! The right context panel (docs/ui-design.md §4.6): Task Overview by
//! default (overdue / today / inbox with a quick capture box), Task
//! Detail once a Task is opened.

use nsh_core::prelude::*;

use crate::domain::outline;
use crate::ui::controls::{panel, section_heading, submit_field};
use crate::ui::theme as tokens;

use super::store::{use_store, Store};
use super::task_detail;
use super::topbar::mini_task_row;

pub fn context_panel() -> impl IntoElement {
    let store = use_store();
    panel()
        .w(px(300.0))
        .flex_col()
        .overflow_hidden()
        .child(panel_header(store))
        .child(dynamic(move || {
            // Only the selection selects the shape; building the content
            // reads more signals (the detail's aggregate computed evaluates
            // eagerly), which must not re-run this region.
            let selected = store.selected_task.get();
            let document = store.strip.with(|strip| match &strip.tabs[strip.active].current {
                super::tabs::Route::Document(id) => Some((strip.tabs[strip.active].id, *id)),
                _ => None,
            });
            untracked(move || match (selected, document) {
                (Some(id), _) => task_detail::task_detail(store, id).into_element(),
                (None, Some((tab_id, doc_id))) => document_context(store, tab_id, doc_id).into_element(),
                (None, None) => overview(store).into_element(),
            })
        }))
}

fn panel_header(store: Store) -> impl IntoElement {
    view()
        .h(px(44.0))
        .flex_row()
        .items_center()
        .px_3()
        .border_b_1()
        .border_color(tokens::border_subtle())
        .child(text(move || {
            if store.selected_task.get().is_some() {
                "任务详情".to_owned()
            } else if store.strip.with(|strip| matches!(strip.tabs[strip.active].current, super::tabs::Route::Document(_))) {
                "文档大纲与关联".to_owned()
            } else {
                "任务速览".to_owned()
            }
        })
        .text_size(px(14.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(tokens::fg_primary()))
}

/// The Document outline (docs/ui-design.md §4.6): the committed body's
/// H1–H3 headings; pressing one asks the tab showing the document to
/// reveal that heading.
fn document_outline(store: Store, tab_id: u64, document_id: i64) -> impl IntoElement {
    let entries = move || {
        store.bump.get();
        let body = store.document(document_id).map(|doc| doc.body).unwrap_or_default();
        outline(&body, 3)
    };
    scroll().flex_1().child(
        view().flex_col().p_2().gap_1().child(each(
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
                            .child(text(text_label).text_size(px(12.0)).text_color(tokens::fg_body())),
                    )
            },
        )),
    )
}

/// The Document context (docs/ui-design.md §4.6 关联与反链): the heading
/// outline, this Document's Linked Task Cards (live, directly
/// toggle-able) and the backlinks of Documents mentioning it — degrading
/// while a source Document sits in Trash and recovering on restore.
fn document_context(store: Store, tab_id: u64, document_id: i64) -> impl IntoElement {
    scroll().flex_1().child(
        view()
            .flex_col()
            .p_2()
            .gap_3()
            .child(document_outline(store, tab_id, document_id))
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
    view().flex_col().gap_1().child(section_heading("关联任务")).child(when(
        move || cards().is_empty(),
        move || {
            text("正文中的关联任务卡片会出现在这里")
                .text_size(px(11.0))
                .text_color(tokens::fg_muted())
        },
    )).child(dynamic(move || {
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
                        live().map(|a| a.task.title.clone()).unwrap_or(snapshot.clone())
                    };
                    let trashed = move || live().is_none_or(|a| a.task.trashed_at.is_some());
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .rounded(tokens::radius_row())
                        .child(when(
                            move || !trashed(),
                            {
                                let title = title.clone();
                                move || task_circle_row(store, task_id, title()).into_element()
                            },
                        ))
                        .child(when(
                            trashed,
                            {
                                let title = title.clone();
                                move || {
                                    text(format!("任务已移除 · {}", title()))
                                        .text_size(px(12.0))
                                        .text_color(tokens::fg_muted())
                                        .flex_1()
                                        .number_of_lines(1)
                                        .into_element()
                                }
                            },
                        ))
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
    view().flex_col().gap_1().child(section_heading("反向链接")).child(when(
        move || links().is_empty(),
        move || {
            text("其它文档用 @ 提及本文档后会出现在这里")
                .text_size(px(11.0))
                .text_color(tokens::fg_muted())
        },
    )).child(dynamic(move || {
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
                        .child(when(
                            move || !trashed,
                            {
                                let title = title.clone();
                                move || {
                                    nsh_base::button()
                                        .on_press(move || jump.open_document(source))
                                        .accessibility_label(Some(
                                            nsh_core::element::SharedString::from(format!("打开文档 {title}")),
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
                            },
                        ))
                        .child(when(
                            move || trashed,
                            {
                                let title = title.clone();
                                move || {
                                    text(format!("{title}（源文档已删除）"))
                                        .text_size(px(12.0))
                                        .text_color(tokens::fg_muted())
                                        .number_of_lines(1)
                                        .into_element()
                                }
                            },
                        ))
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
        store.task(task_id).is_some_and(|a| a.task.status == crate::domain::TaskStatus::Done)
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

/// Task Overview: mutually-exclusive groups 逾期 / 今天 / 收件箱 plus a
/// quick capture box that creates Inbox tasks.
fn overview(store: Store) -> impl IntoElement {
    let groups = computed(move || {
        store.bump.get();
        store.task_groups()
    });
    let capture = signal(String::new());
    view()
        .flex_1()
        .min_h(px(0.0))
        .child(
            scroll().flex_1().child(
                view()
                    .flex_col()
                    .gap_4()
                    .p_3()
                    .child(
                        view()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(submit_field(
                                "速记待办，回车保存",
                                move || capture.get(),
                                move |text: &str| capture.set(text.to_owned()),
                                move |text: &str| {
                                    store.create_inbox_task(text);
                                    capture.set(String::new());
                                },
                            ))
                            .child(when(
                                move || !capture.get().trim().is_empty(),
                                move || {
                                    crate::ui::controls::app_button(
                                        crate::ui::controls::ButtonKind::Positive,
                                        "添加任务",
                                        move || {
                                            store.create_inbox_task(&capture.get());
                                            capture.set(String::new());
                                        },
                                    )
                                },
                            )),
                    )
                    .child(overview_group(store, "逾期", move || groups.get().overdue))
                    .child(overview_group(store, "今天", move || groups.get().today))
                    .child(overview_group(store, "收件箱", move || groups.get().inbox)),
            ),
        )
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
                    .child(section_heading(title))
                    .child(
                        text(move || format!("{}", items().len()))
                            .text_size(px(11.0))
                            .text_color(tokens::fg_muted()),
                    )
            },
        ))
        .child(dynamic(move || {
            view().flex_col().gap(px(2.0)).children(
                items()
                    .into_iter()
                    .map(|aggregate| mini_task_row(store, aggregate).into_element())
                    .collect::<Vec<_>>(),
            )
        }))
}
