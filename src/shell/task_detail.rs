//! The Task Detail editor shown in the right context panel
//! (docs/ui-design.md §4.6): title, status, priority, independent
//! Scheduled and Due date/times, project, subtasks, tags, memo, and
//! trash — every write flows through the store into `Workspace`.
//!
//! One aggregate `computed` per open task keeps every field live: text
//! inputs stay mounted (and keep their focus) across commits, and only
//! list-shaped regions (subtasks, tags) rebuild with the data.

use nsh_base::{button, input, theme};
use nsh_core::prelude::*;

use crate::domain::{LocalDate, LocalTime, Priority, TaskAggregate, TaskPatch, TaskStatus};
use crate::ui::controls::{
    app_button, panel_icon_button, section_heading, task_circle, ButtonKind,
};
use crate::ui::theme as tokens;

use super::store::Store;

type Detail = Memo<Option<TaskAggregate>>;

/// The detail editor for the Task `id`. The surrounding panel remakes it
/// when the selection changes; data flows in through `aggregate`.
pub fn task_detail(store: Store, id: i64) -> impl IntoElement {
    let aggregate: Detail = computed(move || {
        store.bump.get();
        store.task(id)
    });
    let title = store.task(id).map(|a| a.task.title).unwrap_or_default();

    scroll().flex_1().child(
        view()
            .flex_col()
            .gap_4()
            .p_3()
            .child(detail_header(store, id, aggregate, title))
            .child(title_field(store, id, aggregate))
            .child(status_segmented(store, id, aggregate))
            .child(priority_segmented(store, id, aggregate))
            .child(scheduling_section(store, id, aggregate))
            .child(project_section(store, id, aggregate))
            .child(milestone_section(store, id, aggregate))
            .child(subtasks_section(store, id, aggregate))
            .child(tags_section(store, id, aggregate))
            .child(memo_field(store, id, aggregate))
            .child(trash_row(store, id)),
    )
}

fn detail_header(store: Store, id: i64, aggregate: Detail, title: String) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(task_circle(
            move || aggregate.get().is_some_and(|a| a.task.status == TaskStatus::Done),
            format!("完成 {title}"),
            move || store.toggle_circle(id),
        ))
        .child(
            text(move || {
                aggregate
                    .get()
                    .map(|aggregate| aggregate.task.status.label().to_owned())
                    .unwrap_or_default()
            })
            .text_size(px(12.0))
            .text_color(tokens::fg_muted()),
        )
        .child(view().flex_1())
        .child(panel_icon_button("✕", "关闭任务详情", move || store.close_task()))
}

fn title_field(store: Store, id: i64, aggregate: Detail) -> impl IntoElement {
    input()
        .value(move || aggregate.get().map(|a| a.task.title).unwrap_or_default())
        .on_change(move |text: &str| {
            store.update_task(id, &TaskPatch { title: Some(text.to_owned()), ..Default::default() });
        })
        .placeholder("任务标题")
        .placeholder_color(|| color(theme().muted_foreground))
        .text_size(px(16.0))
        .font_weight(FontWeight::SEMIBOLD)
        .h(px(34.0))
        .px_2()
        .rounded(tokens::radius_row())
        .bg(tokens::control_bg())
        .border_1()
        .border_color(move || theme().background.opacity(0.0))
        .focus(|s| s.border_color(tokens::accent()))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .label("任务标题")
}

fn status_segmented(store: Store, id: i64, aggregate: Detail) -> impl IntoElement {
    dynamic(move || {
        let current = aggregate.get().map(|a| a.task.status).unwrap_or_default();
        segmented(
            "状态",
            vec![
                (TaskStatus::Todo.label(), current == TaskStatus::Todo),
                (TaskStatus::Doing.label(), current == TaskStatus::Doing),
                (TaskStatus::Done.label(), current == TaskStatus::Done),
                (TaskStatus::Cancelled.label(), current == TaskStatus::Cancelled),
            ],
            move |index| {
                let status =
                    [TaskStatus::Todo, TaskStatus::Doing, TaskStatus::Done, TaskStatus::Cancelled][index];
                store.update_task(id, &TaskPatch { status: Some(status), ..Default::default() });
            },
        )
        .into_element()
    })
}

fn priority_segmented(store: Store, id: i64, aggregate: Detail) -> impl IntoElement {
    dynamic(move || {
        let current = aggregate.get().map(|a| a.task.priority).unwrap_or_default();
        segmented(
            "优先级",
            vec![
                (Priority::None.label(), current == Priority::None),
                (Priority::Low.label(), current == Priority::Low),
                (Priority::Medium.label(), current == Priority::Medium),
                (Priority::High.label(), current == Priority::High),
            ],
            move |index| {
                let priority = [Priority::None, Priority::Low, Priority::Medium, Priority::High][index];
                store.update_task(id, &TaskPatch { priority: Some(priority), ..Default::default() });
            },
        )
        .into_element()
    })
}

/// A labelled row of small toggle buttons (a segmented control).
fn segmented(
    label: &'static str,
    options: Vec<(&'static str, bool)>,
    on_select: impl Fn(usize) + Copy + 'static,
) -> impl IntoElement {
    view()
        .flex_col()
        .gap_1()
        .child(section_heading(label))
        .child(
            view()
                .flex_row()
                .gap_1()
                .children(options.iter().enumerate().map(|(index, (name, selected))| {
                    let selected = *selected;
                    button()
                        .on_press(move |_: &()| on_select(index))
                        .accessibility_label(Some((*name).into()))
                        .h(px(26.0))
                        .px_2()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .border_1()
                        .border_color(move || {
                            if selected { tokens::accent() } else { tokens::border_default() }
                        })
                        .bg(move || {
                            if selected { tokens::accent_soft() } else { theme().background.opacity(0.0) }
                        })
                        .text_color(move || {
                            if selected { tokens::accent() } else { tokens::fg_body() }
                        })
                        .hover(|s| s.bg(tokens::control_bg()))
                        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                        .child(text(*name).text_size(px(12.0)))
                })),
        )
}

/// Scheduled and Due editors: independent date and optional time fields.
/// The drafts follow external writes (Carryover, another view) unless
/// the user has diverged from the last committed value.
fn scheduling_section(store: Store, id: i64, aggregate: Detail) -> impl IntoElement {
    view()
        .flex_col()
        .gap_3()
        .child(date_time_editor(
            store,
            id,
            aggregate,
            "排期",
            "排期日 (YYYY-MM-DD)",
            |task| task.scheduled_date,
            |task| task.scheduled_time,
            |date| TaskPatch { scheduled_date: Some(date), ..Default::default() },
            |time| TaskPatch { scheduled_time: Some(time), ..Default::default() },
        ))
        .child(date_time_editor(
            store,
            id,
            aggregate,
            "截止",
            "截止日 (YYYY-MM-DD)",
            |task| task.due_date,
            |task| task.due_time,
            |date| TaskPatch { due_date: Some(date), ..Default::default() },
            |time| TaskPatch { due_time: Some(time), ..Default::default() },
        ))
}

#[allow(clippy::too_many_arguments)]
fn date_time_editor(
    store: Store,
    id: i64,
    aggregate: Detail,
    label: &'static str,
    date_placeholder: &'static str,
    date_of: fn(&crate::domain::Task) -> Option<LocalDate>,
    time_of: fn(&crate::domain::Task) -> Option<LocalTime>,
    date_patch: fn(Option<LocalDate>) -> TaskPatch,
    time_patch: fn(Option<LocalTime>) -> TaskPatch,
) -> impl IntoElement {
    // Drafts keep partial typing ("2026-11-") editable; a valid value
    // commits, an invalid one waits for more input. The clear button
    // rewrites both drafts alongside the store.
    let initial = aggregate.get();
    let date_draft = signal(initial.as_ref().map(|a| date_of(&a.task)).unwrap_or_default().map(|d| d.to_string()).unwrap_or_default());
    let time_draft = signal(initial.as_ref().map(|a| time_of(&a.task)).unwrap_or_default().map(|t| t.to_string()).unwrap_or_default());
    // External writes refresh the drafts while they still agree with the
    // previously committed value; a draft the user diverged from (a
    // partial edit) is left alone.
    let date_seen = signal(initial.as_ref().map(|a| date_of(&a.task)).unwrap_or_default());
    let time_seen = signal(initial.as_ref().map(|a| time_of(&a.task)).unwrap_or_default());
    effect(move || {
        let committed = store.task(id).map(|a| date_of(&a.task)).unwrap_or_default();
        if committed == date_seen.get() {
            return;
        }
        let draft = date_draft.get_untracked();
        let matches_seen = LocalDate::parse(draft.trim()) == date_seen.get()
            || (draft.trim().is_empty() && date_seen.get().is_none());
        date_seen.set(committed);
        if matches_seen {
            date_draft.set(committed.map(|d| d.to_string()).unwrap_or_default());
        }
    });
    effect(move || {
        let committed = store.task(id).map(|a| time_of(&a.task)).unwrap_or_default();
        if committed == time_seen.get() {
            return;
        }
        let draft = time_draft.get_untracked();
        let matches_seen = LocalTime::parse(draft.trim()) == time_seen.get()
            || (draft.trim().is_empty() && time_seen.get().is_none());
        time_seen.set(committed);
        if matches_seen {
            time_draft.set(committed.map(|t| t.to_string()).unwrap_or_default());
        }
    });
    let date_label = format!("{label}日");
    let time_label = format!("{label}时间");
    view()
        .flex_col()
        .gap_1()
        .child(section_heading(label))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    input()
                        .value(move || date_draft.get())
                        .on_change(move |text: &str| {
                            date_draft.set(text.to_owned());
                            if let Some(date) = LocalDate::parse(text) {
                                store.update_task(id, &date_patch(Some(date)));
                            } else if text.trim().is_empty() {
                                store.update_task(id, &date_patch(None));
                            }
                        })
                        .placeholder(date_placeholder)
                        .placeholder_color(|| color(theme().muted_foreground))
                        .text_size(px(13.0))
                        .h(px(30.0))
                        .px_2()
                        .rounded(tokens::radius_row())
                        .bg(tokens::control_bg())
                        .border_1()
                        .border_color(move || theme().background.opacity(0.0))
                        .focus(|s| s.border_color(tokens::accent()))
                        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                        .label(date_label),
                )
                .child(
                    input()
                        .value(move || time_draft.get())
                        .on_change(move |text: &str| {
                            time_draft.set(text.to_owned());
                            if let Some(time) = LocalTime::parse(text) {
                                store.update_task(id, &time_patch(Some(time)));
                            } else if text.trim().is_empty() {
                                store.update_task(id, &time_patch(None));
                            }
                        })
                        .placeholder("时分 (HH:MM)")
                        .placeholder_color(|| color(theme().muted_foreground))
                        .text_size(px(13.0))
                        .h(px(30.0))
                        .w(px(76.0))
                        .px_2()
                        .rounded(tokens::radius_row())
                        .bg(tokens::control_bg())
                        .border_1()
                        .border_color(move || theme().background.opacity(0.0))
                        .focus(|s| s.border_color(tokens::accent()))
                        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                        .label(time_label),
                )
                .child(when(
                    move || !date_draft.get().trim().is_empty() || !time_draft.get().trim().is_empty(),
                    move || {
                        panel_icon_button("✕", "清除日期", move || {
                            date_draft.set(String::new());
                            time_draft.set(String::new());
                            store.update_task(id, &date_patch(None));
                            store.update_task(id, &time_patch(None));
                        })
                    },
                )),
        )
}

/// The Project selector: assigning a Project moves the Task out of the
/// Inbox (issue #4); `无项目` returns it there.
fn project_section(store: Store, id: i64, aggregate: Detail) -> impl IntoElement {
    let options = move || {
        store.bump.get();
        let mut options = vec![("无项目".to_owned(), None)];
        options.extend(store.projects().into_iter().map(|project| (project.name, Some(project.id))));
        options
    };
    view()
        .flex_col()
        .gap_1()
        .child(section_heading("项目"))
        .child(each(
            options,
            |(_, project): &(String, Option<i64>)| *project,
            move |option, _index| {
                let current = aggregate.get().and_then(|a| a.task.project_id);
                let name = option.with(|(name, _)| name.clone());
                let project = option.with(|(_, project)| *project);
                let selected = current == project;
                option_chip(name, selected, move || {
                    store.update_task(id, &TaskPatch { project_id: Some(project), ..Default::default() });
                })
                .into_element()
            },
        ))
}

/// The Milestone selector: the Milestones of the Task's current
/// Project; `无里程碑` clears the reference.
fn milestone_section(store: Store, id: i64, aggregate: Detail) -> impl IntoElement {
    let options = move || {
        store.bump.get();
        let mut options = vec![("无里程碑".to_owned(), None)];
        let project_id = aggregate.get().and_then(|a| a.task.project_id);
        if let Some(project_id) = project_id {
            options.extend(
                store
                    .milestones(project_id)
                    .into_iter()
                    .map(|milestone| (milestone.title, Some(milestone.id))),
            );
        }
        options
    };
    view()
        .flex_col()
        .gap_1()
        .child(section_heading("里程碑"))
        .child(when(
            move || aggregate.get().and_then(|a| a.task.project_id).is_none(),
            move || {
                text("先关联项目，再选择里程碑")
                    .text_size(px(11.0))
                    .text_color(tokens::fg_muted())
            },
        ))
        .child(when(
            move || aggregate.get().and_then(|a| a.task.project_id).is_some(),
            move || {
                each(
                    options,
                    |(_, milestone): &(String, Option<i64>)| *milestone,
                    move |option, _index| {
                        let current = aggregate.get().and_then(|a| a.task.milestone_id);
                        let title = option.with(|(title, _)| title.clone());
                        let milestone = option.with(|(_, milestone)| *milestone);
                        let selected = current == milestone;
                        option_chip(title, selected, move || {
                            store.update_task(
                                id,
                                &TaskPatch { milestone_id: Some(milestone), ..Default::default() },
                            );
                        })
                        .into_element()
                    },
                )
            },
        ))
}

/// One option of the Project/Milestone selectors: a small toggle chip.
fn option_chip(
    name: String,
    selected: bool,
    on_press: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    button()
        .on_press(move |_: &()| on_press())
        .accessibility_label(Some(format!("选择 {name}").into()))
        .h(px(26.0))
        .px_2()
        .items_center()
        .justify_center()
        .rounded_full()
        .border_1()
        .border_color(move || if selected { tokens::accent() } else { tokens::border_default() })
        .bg(move || {
            if selected { tokens::accent_soft() } else { theme().background.opacity(0.0) }
        })
        .text_color(move || if selected { tokens::accent() } else { tokens::fg_body() })
        .hover(|s| s.bg(tokens::control_bg()))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .child(text(name).text_size(px(12.0)))
}

fn subtasks_section(store: Store, id: i64, aggregate: Detail) -> impl IntoElement {
    let new_subtask = signal(String::new());
    view()
        .flex_col()
        .gap_1()
        .child(section_heading("子任务"))
        .child(dynamic(move || {
            let subtasks = aggregate.get().map(|a| a.subtasks).unwrap_or_default();
            view().flex_col().gap_1().children(
                subtasks
                    .iter()
                    .map(|subtask| subtask_row(store, subtask.clone()).into_element())
                    .collect::<Vec<_>>(),
            )
        }))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    input()
                        .value(move || new_subtask.get())
                        .on_change(move |text: &str| new_subtask.set(text.to_owned()))
                        .on_submit(move |text: &str| {
                            store.add_subtask(id, text);
                            new_subtask.set(String::new());
                        })
                        .placeholder("添加子任务，回车保存")
                        .placeholder_color(|| color(theme().muted_foreground))
                        .text_size(px(13.0))
                        .h(px(30.0))
                        .px_2()
                        .rounded(tokens::radius_row())
                        .bg(tokens::control_bg())
                        .border_1()
                        .border_color(move || theme().background.opacity(0.0))
                        .focus(|s| s.border_color(tokens::accent()))
                        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                        .label("新子任务"),
                )
                .child(app_button(ButtonKind::Soft, "添加", move || {
                    store.add_subtask(id, &new_subtask.get());
                    new_subtask.set(String::new());
                })),
        )
}

fn subtask_row(store: Store, subtask: crate::domain::Subtask) -> impl IntoElement {
    let subtask_id = subtask.id;
    let done_title = subtask.title.clone();
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(
            button()
                .on_press(move || store.set_subtask_done(subtask_id, !subtask.done))
                .accessibility_label(Some(
                    if subtask.done {
                        format!("标记未完成 {done_title}")
                    } else {
                        format!("完成 {done_title}")
                    }
                    .into(),
                ))
                .size(px(18.0))
                .items_center()
                .justify_center()
                .rounded(px(5.0))
                .border_1()
                .border_color(move || {
                    if subtask.done { tokens::positive() } else { tokens::border_default() }
                })
                .bg(move || {
                    if subtask.done { tokens::positive() } else { theme().background.opacity(0.0) }
                })
                .text_color(theme().primary_foreground)
                .child(text(move || if subtask.done { "✓" } else { "" }).text_size(px(11.0))),
        )
        .child(
            text(subtask.title.clone())
                .flex_1()
                .min_w(px(0.0))
                .text_size(px(13.0))
                .text_color(tokens::fg_body())
                .style_when(subtask.done, |s| s.line_through()),
        )
        .child(panel_icon_button("✕", "删除子任务", move || {
            store.remove_subtask(subtask_id);
        }))
}

fn tags_section(store: Store, id: i64, aggregate: Detail) -> impl IntoElement {
    let new_tag = signal(String::new());
    view()
        .flex_col()
        .gap_1()
        .child(section_heading("标签"))
        .child(dynamic(move || {
            let tags = aggregate.get().map(|a| a.tags).unwrap_or_default();
            view()
                .flex_row()
                .flex_wrap()
                .gap_1()
                .children(
                    tags.iter()
                        .map(|tag| {
                            let tag = tag.clone();
                            view()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .px_1p5()
                                .py(px(2.0))
                                .rounded(px(6.0))
                                .bg(tokens::info_soft())
                                .child(
                                    text(format!("#{}", tag.name))
                                        .text_size(px(11.0))
                                        .text_color(tokens::info()),
                                )
                                .child(
                                    button()
                                        .on_press(move || store.remove_tag(id, tag.id))
                                        .accessibility_label(Some(format!("移除标签 {}", tag.name).into()))
                                        .items_center()
                                        .justify_center()
                                        .text_color(tokens::fg_muted())
                                        .child(text("✕").text_size(px(9.0))),
                                )
                                .into_element()
                        })
                        .collect::<Vec<_>>(),
                )
        }))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    input()
                        .value(move || new_tag.get())
                        .on_change(move |text: &str| new_tag.set(text.to_owned()))
                        .on_submit(move |text: &str| {
                            store.add_tag(id, text);
                            new_tag.set(String::new());
                        })
                        .placeholder("添加标签，回车保存")
                        .placeholder_color(|| color(theme().muted_foreground))
                        .text_size(px(13.0))
                        .h(px(30.0))
                        .px_2()
                        .rounded(tokens::radius_row())
                        .bg(tokens::control_bg())
                        .border_1()
                        .border_color(move || theme().background.opacity(0.0))
                        .focus(|s| s.border_color(tokens::accent()))
                        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                        .label("新标签"),
                )
                .child(app_button(ButtonKind::Soft, "添加", move || {
                    store.add_tag(id, &new_tag.get());
                    new_tag.set(String::new());
                })),
        )
}

fn memo_field(store: Store, id: i64, aggregate: Detail) -> impl IntoElement {
    input()
        .value(move || aggregate.get().map(|a| a.task.memo).unwrap_or_default())
        .on_change(move |text: &str| {
            store.update_task(id, &TaskPatch { memo: Some(text.to_owned()), ..Default::default() });
        })
        .placeholder("备注")
        .placeholder_color(|| color(theme().muted_foreground))
        .multiline(true)
        .text_size(px(13.0))
        .min_h(px(64.0))
        .p_2()
        .rounded(tokens::radius_row())
        .bg(tokens::control_bg())
        .border_1()
        .border_color(move || theme().background.opacity(0.0))
        .focus(|s| s.border_color(tokens::accent()))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .label("备注")
}

fn trash_row(store: Store, id: i64) -> impl IntoElement {
    view().pt_2().child(app_button(ButtonKind::Danger, "移入回收站", move || {
        store.trash_task(id);
    }))
}
