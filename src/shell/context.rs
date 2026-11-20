//! The right context panel (docs/ui-design.md §4.6): Task Overview by
//! default (overdue / today / inbox with a quick capture box), Task
//! Detail once a Task is opened.

use nsh_core::prelude::*;

use crate::ui::controls::{input_field, panel, section_heading};
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
            untracked(move || match selected {
                Some(id) => task_detail::task_detail(store, id).into_element(),
                None => overview(store).into_element(),
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
            } else {
                "任务速览".to_owned()
            }
        })
        .text_size(px(14.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(tokens::fg_primary()))
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
                            .child(input_field(
                                "速记待办，回车保存",
                                move || capture.get(),
                                move |text: &str| capture.set(text.to_owned()),
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
