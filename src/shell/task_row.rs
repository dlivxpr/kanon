//! The shared Task row of the grouped task lists (Today, Tasks,
//! Upcoming): circle toggle, title, overdue badge, 进行中 chip, and the
//! inline meta (scheduled time-of-day, Due indicator, Project dot,
//! subtask progress, priority, Tags). Clicking the title opens the
//! Task Detail in the right context panel.

use nsh_base::{button, theme};
use nsh_core::prelude::*;

use crate::domain::{TaskAggregate, TaskStatus};
use crate::ui::controls::{chip, list_row, overdue_badge, task_circle};
use crate::ui::theme as tokens;

use super::store::Store;

/// One Task row of a grouped list; the list decides the grouping.
pub fn task_row(store: Store, aggregate: TaskAggregate) -> impl IntoElement {
    let id = aggregate.task.id;
    let row_title = aggregate.task.title.clone();
    let circle_title = aggregate.task.title.clone();
    // Live lookups: the row must follow every workspace change (circle
    // toggle, scheduling, triage) instead of the snapshot it was built
    // from — the surrounding list keeps rows by key.
    let live = move || store.task(id);
    let overdue = move || live().is_some_and(|a| a.task.is_overdue_at(store.now()));
    let scheduled_time = move || live().and_then(|a| a.task.scheduled_time);
    let due = move || live().map(|a| (a.task.due_date, a.task.due_time));
    let project = move || live().and_then(|a| a.project);
    let subtasks = move || live().map(|a| a.subtasks).unwrap_or_default();
    let high_priority =
        move || live().is_some_and(|a| a.task.priority == crate::domain::Priority::High);
    let tags = move || live().map(|a| a.tags).unwrap_or_default();
    list_row()
        .items_start()
        .pt_3()
        .child(
            view().pt(px(2.0)).child(task_circle(
                move || {
                    store.task(id).is_some_and(|aggregate| aggregate.task.status == TaskStatus::Done)
                },
                format!("完成 {circle_title}"),
                move || store.toggle_circle(id),
            )),
        )
        .child(
            view()
                .flex_1()
                .min_w(px(0.0))
                .gap_1()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(
                            button()
                                .on_press(move || store.open_task(id))
                                .accessibility_label(Some(format!("打开任务 {row_title}").into()))
                                .items_center()
                                .child(
                                    text(row_title.clone())
                                        .text_size(px(14.0))
                                        .text_color(move || tokens::fg_primary())
                                        .number_of_lines(1),
                                ),
                        )
                        .child(when(overdue, overdue_badge))
                        .child(when(
                            move || {
                                store.task(id).is_some_and(|a| a.task.status == TaskStatus::Doing)
                            },
                            || {
                                view()
                                    .px_1p5()
                                    .py(px(1.0))
                                    .rounded(px(6.0))
                                    .bg(tokens::info_soft())
                                    .child(text("进行中").text_size(px(11.0)).text_color(tokens::info()))
                            },
                        )),
                )
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .flex_wrap()
                        .child(when(move || scheduled_time().is_some(), move || {
                            text(move || {
                                scheduled_time().map(|t| t.to_string()).unwrap_or_default()
                            })
                            .text_size(px(11.0))
                            .text_color(tokens::fg_muted())
                        }))
                        .child(when(
                            move || due().is_some_and(|(date, _)| date.is_some()),
                            move || {
                                text(move || {
                                    let (date, time) = due().unwrap_or_default();
                                    let due = date.map(|d| d.to_string()).unwrap_or_default();
                                    let time = time.map(|t| format!(" {t}")).unwrap_or_default();
                                    format!("截止 {due}{time}")
                                })
                                .text_size(px(11.0))
                                .text_color(move || {
                                    if overdue() { tokens::accent() } else { tokens::fg_muted() }
                                })
                            },
                        ))
                        .child(when(move || project().is_some(), move || {
                            let name = project().map(|project| project.name).unwrap_or_default();
                            view()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .child(view().size(px(7.0)).rounded_full().bg(theme().primary))
                                .child(text(name).text_size(px(11.0)).text_color(tokens::fg_muted()))
                        }))
                        .child(when(move || !subtasks().is_empty(), move || {
                            text(move || {
                                let subtasks = subtasks();
                                let done = subtasks.iter().filter(|subtask| subtask.done).count();
                                format!("子任务 {done}/{}", subtasks.len())
                            })
                            .text_size(px(11.0))
                            .text_color(tokens::fg_muted())
                        }))
                        .child(when(high_priority, || {
                            text("高优先级").text_size(px(11.0)).text_color(tokens::warning())
                        }))
                        .child(dynamic(move || {
                            view().flex_row().items_center().gap_1().children(
                                tags()
                                    .into_iter()
                                    .map(|tag| chip(format!("#{}", tag.name)).into_element())
                                    .collect::<Vec<_>>(),
                            )
                        })),
                ),
        )
}
