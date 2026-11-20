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
            .child(project_row(aggregate))
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
fn scheduling_section(store: Store, id: i64, aggregate: Detail) -> impl IntoElement {
    let initial = aggregate.get();
    view()
        .flex_col()
        .gap_3()
        .child(date_time_editor(
            store,
            id,
            "排期",
            "排期日 (YYYY-MM-DD)",
            initial.as_ref().and_then(|a| a.task.scheduled_date),
            initial.as_ref().and_then(|a| a.task.scheduled_time),
            |date| TaskPatch { scheduled_date: Some(date), ..Default::default() },
            |time| TaskPatch { scheduled_time: Some(time), ..Default::default() },
        ))
        .child(date_time_editor(
            store,
            id,
            "截止",
            "截止日 (YYYY-MM-DD)",
            initial.as_ref().and_then(|a| a.task.due_date),
            initial.as_ref().and_then(|a| a.task.due_time),
            |date| TaskPatch { due_date: Some(date), ..Default::default() },
            |time| TaskPatch { due_time: Some(time), ..Default::default() },
        ))
}

#[allow(clippy::too_many_arguments)]
fn date_time_editor(
    store: Store,
    id: i64,
    label: &'static str,
    date_placeholder: &'static str,
    date: Option<LocalDate>,
    time: Option<LocalTime>,
    date_patch: fn(Option<LocalDate>) -> TaskPatch,
    time_patch: fn(Option<LocalTime>) -> TaskPatch,
) -> impl IntoElement {
    // Drafts keep partial typing ("2026-11-") editable; a valid value
    // commits, an invalid one waits for more input. The clear button
    // rewrites both drafts alongside the store.
    let date_draft = signal(date.map(|d| d.to_string()).unwrap_or_default());
    let time_draft = signal(time.map(|t| t.to_string()).unwrap_or_default());
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

fn project_row(aggregate: Detail) -> impl IntoElement {
    view()
        .flex_col()
        .gap_1()
        .child(section_heading("项目"))
        .child(dynamic(move || {
            let project = aggregate.get().and_then(|a| a.project);
            match project {
                Some(project) => view()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .child(view().size(px(8.0)).rounded_full().bg(theme().primary))
                    .child(text(project.name).text_size(px(13.0)).text_color(tokens::fg_body()))
                    .into_element(),
                None => view().into_element(),
            }
        }))
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
