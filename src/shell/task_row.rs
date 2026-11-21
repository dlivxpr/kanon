//! The shared Task row of the grouped task lists (Today, Tasks,
//! Upcoming): circle toggle, title, overdue badge, 进行中 chip, and the
//! inline meta (Due indicator, Project dot, subtask progress, priority,
//! Tags) with the scheduled time-of-day in a trailing column
//! (docs/kanon.html `.task-row` / `.time`). Clicking the title opens the
//! Task Detail in the right context panel.
//!
//! `board_card` is the raised card the Upcoming 看板 columns compose
//! (docs/kanon.html `.board-card`); both share the same meta builders.

use nsh_base::{button, theme};
use nsh_core::prelude::*;

use crate::domain::{
    LocalDate, LocalTime, Priority, ProjectRef, Subtask, TaskAggregate, TaskStatus,
};
use crate::ui::controls::{chip, list_row, overdue_badge, task_circle};
use crate::ui::theme as tokens;

use super::store::Store;

/// One Task row of a grouped list; the list decides the grouping.
pub fn task_row(store: Store, id: i64) -> impl IntoElement {
    // A keyed row outlives its initial aggregate. Share one lookup among its bindings.
    let live = computed(move || store.task(id));
    let title = move || {
        live.with(|task| {
            task.as_ref()
                .map_or_else(String::new, |a| a.task.title.clone())
        })
    };
    let done = move || {
        live.with(|task| {
            task.as_ref()
                .is_some_and(|a| a.task.status == TaskStatus::Done)
        })
    };
    let overdue = move || {
        live.with(|task| {
            task.as_ref()
                .is_some_and(|a| a.task.is_overdue_at(store.now()))
        })
    };
    let scheduled_time =
        move || live.with(|task| task.as_ref().and_then(|a| a.task.scheduled_time));
    let due = move || live.with(|task| task.as_ref().map(|a| (a.task.due_date, a.task.due_time)));
    let project = move || live.with(|task| task.as_ref().and_then(|a| a.project.clone()));
    let subtasks = move || {
        live.with(|task| {
            task.as_ref()
                .map(|a| a.subtasks.clone())
                .unwrap_or_default()
        })
    };
    let high_priority = move || {
        live.with(|task| {
            task.as_ref()
                .is_some_and(|a| a.task.priority == Priority::High)
        })
    };
    let tags = move || live.with(|task| task.as_ref().map(|a| a.tags.clone()).unwrap_or_default());
    let document = computed(move || store.task_documents(id, Some(1)).into_iter().next());
    list_row()
        .items_start()
        .pt_3()
        .style_when(tokens::is_compact(), |s| s.pt_2())
        .child(view().pt(px(2.0)).child(task_circle(
            done,
            move || Some(SharedString::from(format!("完成 {}", title()))),
            move || store.toggle_circle(id),
        )))
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
                        .child(title_button(store, id, title, done))
                        .child(when(overdue, overdue_badge))
                        .child(when(
                            move || {
                                live.with(|task| {
                                    task.as_ref()
                                        .is_some_and(|a| a.task.status == TaskStatus::Doing)
                                })
                            },
                            || {
                                view()
                                    .px_1p5()
                                    .py(px(1.0))
                                    .rounded(px(6.0))
                                    .bg(tokens::info_soft())
                                    .child(
                                        text("进行中")
                                            .text_size(tokens::text_px(11.0))
                                            .text_color(tokens::info()),
                                    )
                            },
                        )),
                )
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .flex_wrap()
                        .child(when(
                            move || due().is_some_and(|(date, _)| date.is_some()),
                            move || due_chip(store, due, overdue),
                        ))
                        .child(when(
                            move || project().is_some(),
                            move || project_meta(project),
                        ))
                        .child(dynamic(move || {
                            document.get().map(|(id, title)| {
                                let label = format!("关联文档 {title}");
                                crate::ui::controls::app_button(
                                    crate::ui::controls::ButtonKind::Ghost,
                                    title,
                                    move || store.open_document(id),
                                )
                                .accessibility_label(label)
                                .h(px(22.0))
                                .px_2()
                            })
                        }))
                        .child(when(
                            move || !subtasks().is_empty(),
                            move || subtask_progress(subtasks),
                        ))
                        .child(when(high_priority, || {
                            text("高优先级")
                                .text_size(px(11.0))
                                .text_color(tokens::warning())
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
        .child(when(
            move || scheduled_time().is_some(),
            move || {
                text(move || {
                    scheduled_time()
                        .map(|time| store.format_time(time))
                        .unwrap_or_default()
                })
                .text_size(tokens::text_px(12.0))
                .text_color(tokens::fg_muted())
            },
        ))
}

/// The Upcoming 看板's card: the raised surface from
/// docs/kanon.html `.board-card` (raised background, hairline border,
/// compact meta), keeping the circle and the Due indicator
/// (docs/ui-design.md §4.2).
pub fn board_card(store: Store, aggregate: TaskAggregate) -> impl IntoElement {
    let id = aggregate.task.id;
    let live = computed(move || store.task(id));
    let title = move || {
        live.with(|task| {
            task.as_ref()
                .map_or_else(String::new, |a| a.task.title.clone())
        })
    };
    let done = move || {
        live.with(|task| {
            task.as_ref()
                .is_some_and(|a| a.task.status == TaskStatus::Done)
        })
    };
    let overdue = move || {
        live.with(|task| {
            task.as_ref()
                .is_some_and(|a| a.task.is_overdue_at(store.now()))
        })
    };
    let due = move || live.with(|task| task.as_ref().map(|a| (a.task.due_date, a.task.due_time)));
    let subtasks = move || {
        live.with(|task| {
            task.as_ref()
                .map(|a| a.subtasks.clone())
                .unwrap_or_default()
        })
    };
    let high_priority = move || {
        live.with(|task| {
            task.as_ref()
                .is_some_and(|a| a.task.priority == Priority::High)
        })
    };
    view()
        .flex_col()
        .gap_2()
        .px(px(10.0))
        .py(px(10.0))
        .rounded(tokens::radius_row())
        .bg(theme().card)
        .border_1()
        .border_color(tokens::border_subtle())
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(task_circle(
                    done,
                    move || Some(SharedString::from(format!("完成 {}", title()))),
                    move || store.toggle_circle(id),
                ))
                .child(title_button(store, id, title, done))
                .child(when(overdue, overdue_badge)),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .flex_wrap()
                .child(when(
                    move || due().is_some_and(|(date, _)| date.is_some()),
                    move || due_chip(store, due, overdue),
                ))
                .child(when(
                    move || !subtasks().is_empty(),
                    move || subtask_progress(subtasks),
                ))
                .child(when(high_priority, || {
                    text("高优先级")
                        .text_size(px(11.0))
                        .text_color(tokens::warning())
                })),
        )
}

/// The title button of a row or card: opens the Task Detail; a done
/// Task reads struck-through and muted (docs/kanon.html
/// `.task-row.done .task-name`).
fn title_button(
    store: Store,
    id: i64,
    title: impl Fn() -> String + Copy + 'static,
    done: impl Fn() -> bool + Copy + 'static,
) -> impl IntoElement {
    button()
        .on_press(move || store.open_task(id))
        .accessibility_label(move || Some(SharedString::from(format!("打开任务 {}", title()))))
        .items_center()
        .child(
            text(title)
                .text_size(tokens::text_px(14.0))
                .text_color(move || {
                    if done() {
                        tokens::fg_muted()
                    } else {
                        tokens::fg_primary()
                    }
                })
                .style_when(done, |s| s.line_through())
                .number_of_lines(1),
        )
}

/// The Due indicator (`截止 2026-11-22 18:30`), accent-colored while
/// overdue. The time-of-day follows the 12h/24h display preference.
fn due_chip(
    store: Store,
    due: impl Fn() -> Option<(Option<LocalDate>, Option<LocalTime>)> + Copy + 'static,
    overdue: impl Fn() -> bool + Copy + 'static,
) -> impl IntoElement {
    text(move || {
        let (date, time) = due().unwrap_or_default();
        let due = date.map(|d| d.to_string()).unwrap_or_default();
        let time = time
            .map(|t| format!(" {}", store.format_time(t)))
            .unwrap_or_default();
        format!("截止 {due}{time}")
    })
    .text_size(px(11.0))
    .text_color(move || {
        if overdue() {
            tokens::accent()
        } else {
            tokens::fg_muted()
        }
    })
}

/// The Project marker: the Project's own color dot plus its name
/// (docs/kanon.html `.task-meta` `dot(p.color)`); the dot falls back to
/// the info token when the stored color does not parse.
fn project_meta(project: impl Fn() -> Option<ProjectRef> + Copy + 'static) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_1()
        .child(view().size(px(7.0)).rounded_full().bg(move || {
            project()
                .and_then(|project| nsh_core::Hsla::parse_hex(&project.color).ok())
                .map(Into::into)
                .unwrap_or_else(tokens::info)
        }))
        .child(
            text(move || project().map(|project| project.name).unwrap_or_default())
                .text_size(px(11.0))
                .text_color(tokens::fg_muted()),
        )
}

/// The subtask progress note (`子任务 1/2`).
fn subtask_progress(subtasks: impl Fn() -> Vec<Subtask> + Copy + 'static) -> impl IntoElement {
    text(move || {
        let subtasks = subtasks();
        let done = subtasks.iter().filter(|subtask| subtask.done).count();
        format!("子任务 {done}/{}", subtasks.len())
    })
    .text_size(px(11.0))
    .text_color(tokens::fg_muted())
}
