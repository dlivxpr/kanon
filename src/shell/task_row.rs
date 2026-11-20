//! The shared Task row of the grouped task lists (Today, Tasks,
//! Upcoming): circle toggle, title, overdue badge, 进行中 chip, and the
//! inline meta (scheduled time-of-day, Due indicator, Project dot,
//! subtask progress, priority, Tags). Clicking the title opens the
//! Task Detail in the right context panel.

use nsh_base::{button, theme};
use nsh_core::prelude::*;

use crate::domain::{Task, TaskAggregate, TaskStatus};
use crate::ui::controls::{chip, list_row, overdue_badge, task_circle};
use crate::ui::theme as tokens;

use super::store::Store;

/// One Task row of a grouped list; the list decides the grouping.
pub fn task_row(store: Store, aggregate: TaskAggregate) -> impl IntoElement {
    let id = aggregate.task.id;
    let row_title = aggregate.task.title.clone();
    let circle_title = aggregate.task.title.clone();
    let overdue = aggregate.task.is_overdue_at(store.now());
    let project = aggregate.project.clone();
    let tags = aggregate.tags.clone();
    let (done_subs, total_subs) = Task::subtask_progress(&aggregate.subtasks);
    let scheduled_time = aggregate.task.scheduled_time;
    let due_date = aggregate.task.due_date;
    let due_time = aggregate.task.due_time;
    let priority = aggregate.task.priority;
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
                                        .text_color(tokens::fg_primary())
                                        .number_of_lines(1),
                                ),
                        )
                        .child(when(move || overdue, overdue_badge))
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
                        .child(when(scheduled_time.is_some(), move || {
                            text(scheduled_time.map(|t| t.to_string()).unwrap_or_default())
                                .text_size(px(11.0))
                                .text_color(tokens::fg_muted())
                        }))
                        .child(when(
                            due_date.is_some(),
                            move || {
                                let due = due_date.map(|d| d.to_string()).unwrap_or_default();
                                let time = due_time.map(|t| format!(" {}", t)).unwrap_or_default();
                                text(format!("截止 {due}{time}"))
                                    .text_size(px(11.0))
                                    .text_color(move || {
                                        if overdue { tokens::accent() } else { tokens::fg_muted() }
                                    })
                            },
                        ))
                        .child(when(project.is_some(), move || {
                            let project = project.clone().expect("checked above");
                            view()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .child(view().size(px(7.0)).rounded_full().bg(theme().primary))
                                .child(text(project.name).text_size(px(11.0)).text_color(tokens::fg_muted()))
                        }))
                        .child(when(total_subs > 0, move || {
                            text(format!("子任务 {done_subs}/{total_subs}"))
                                .text_size(px(11.0))
                                .text_color(tokens::fg_muted())
                        }))
                        .child(when(priority == crate::domain::Priority::High, || {
                            text("高优先级").text_size(px(11.0)).text_color(tokens::warning())
                        }))
                        .children(
                            tags.iter()
                                .map(|tag| chip(format!("#{}", tag.name)).into_element())
                                .collect::<Vec<_>>(),
                        ),
                ),
        )
}
