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
                (None, Some((tab_id, doc_id))) => document_outline(store, tab_id, doc_id).into_element(),
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
                "文档大纲".to_owned()
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
