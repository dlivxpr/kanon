//! The Tasks view (待办, docs/ui-design.md §4.2): every open Task in the
//! strict mutual-exclusion grouping `逾期 > 今天 > 即将到来 > 收件箱 >
//! 未排期（项目内或仅有截止）`, plus `已完成` and `已取消`, with
//! status / priority / Project filtering and in-group sorting.

use nsh_base::{button, theme};
use nsh_core::prelude::*;

use crate::domain::{Priority, ProjectFilter, TaskAggregate, TaskFilter, TaskGroups, TaskSort, TaskStatus};
use crate::ui::controls::{empty_state, section_heading};
use crate::ui::theme as tokens;

use super::shell::store::Store;
use super::shell::task_row::task_row;

/// The Tasks view for the active tab.
pub fn tasks_view(store: Store) -> impl IntoElement {
    let filter = signal(TaskFilter::default());
    let sort = signal(TaskSort::default());
    // One computed for the whole view: the unfiltered total drives the
    // empty states, the filtered + sorted groups drive the sections.
    let data = computed(move || {
        store.bump.get();
        let all = store.task_groups();
        let total = all.total();
        (all.filtered(&filter.get()).sorted(sort.get()), total)
    });
    view()
        .flex_1()
        .min_h(px(0.0))
        .child(
            scroll()
                .flex_1()
                .child(
                    view()
                        .flex_col()
                        .gap_4()
                        .p_5()
                        .max_w(px(860.0))
                        .w_full()
                        .items_stretch()
                        .self_center()
                        .child(header(data))
                        .child(filter_bar(store, filter, sort))
                        .child(when(
                            move || data.get().1 == 0,
                            || {
                                empty_state(
                                    "暂无任务",
                                    "在「今天」视图添加任务，或用右侧任务速览的速记框收进收件箱。",
                                )
                            },
                        ))
                        .child(when(
                            move || data.get().1 > 0 && groups_total(&data.get().0) == 0,
                            || {
                                empty_state(
                                    "没有符合筛选的任务",
                                    "调整上方的状态、优先级或项目筛选条件。",
                                )
                            },
                        ))
                        .child(group_section(store, "逾期", move || data.get().0.overdue))
                        .child(group_section(store, "今天", move || data.get().0.today))
                        .child(group_section(store, "即将到来", move || data.get().0.upcoming))
                        .child(group_section(store, "收件箱", move || data.get().0.inbox))
                        .child(group_section(store, "未排期", move || data.get().0.unscheduled))
                        .child(group_section(store, "已完成", move || data.get().0.done))
                        .child(group_section(store, "已取消", move || data.get().0.cancelled)),
                ),
        )
}

fn groups_total(groups: &TaskGroups) -> usize {
    groups.total()
}

fn header(data: Memo<(TaskGroups, usize)>) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(text("待办").text_size(px(28.0)).font_weight(FontWeight::BOLD).text_color(tokens::fg_primary()))
        .child(
            text(move || {
                let (groups, _) = data.get();
                let active = groups.total_active();
                match active {
                    0 => "没有未完成的任务".to_owned(),
                    count => format!("{count} 项未完成任务"),
                }
            })
            .text_size(px(13.0))
            .text_color(tokens::fg_muted()),
        )
}

/// The status / priority / Project filter rows and the sort row. Every
/// chip commits to the `filter` / `sort` signals; the sections re-group
/// from the domain.
fn filter_bar(
    store: Store,
    filter: Signal<TaskFilter>,
    sort: Signal<TaskSort>,
) -> impl IntoElement {
    view()
        .flex_col()
        .gap_2()
        .rounded(tokens::radius_card())
        .bg(tokens::panel_subtle())
        .p_3()
        .child(dynamic(move || {
            let current = filter.get();
            chip_row(
                "状态",
                &["全部", "待办", "进行中", "已完成", "已取消"],
                &[
                    current.status.is_none(),
                    current.status == Some(TaskStatus::Todo),
                    current.status == Some(TaskStatus::Doing),
                    current.status == Some(TaskStatus::Done),
                    current.status == Some(TaskStatus::Cancelled),
                ],
                "状态筛选",
                move |index| {
                    let status = [None, Some(TaskStatus::Todo), Some(TaskStatus::Doing), Some(TaskStatus::Done), Some(TaskStatus::Cancelled)][index as usize];
                    filter.update(|f| f.status = status);
                },
            )
            .into_element()
        }))
        .child(dynamic(move || {
            let current = filter.get();
            chip_row(
                "优先级",
                &["全部", "无", "低", "中", "高"],
                &[
                    current.priority.is_none(),
                    current.priority == Some(Priority::None),
                    current.priority == Some(Priority::Low),
                    current.priority == Some(Priority::Medium),
                    current.priority == Some(Priority::High),
                ],
                "优先级筛选",
                move |index| {
                    let priority = [None, Some(Priority::None), Some(Priority::Low), Some(Priority::Medium), Some(Priority::High)][index as usize];
                    filter.update(|f| f.priority = priority);
                },
            )
            .into_element()
        }))
        .child(dynamic(move || {
            let current = filter.get();
            // `-1` marks 全部, `-2` marks 无项目, otherwise a Project id;
            // each chip owns its scope so the row needs no shared borrow.
            let mut options: Vec<(String, i64)> =
                vec![("全部".to_owned(), -1), ("无项目".to_owned(), -2)];
            options.extend(store.projects().into_iter().map(|project| (project.name, project.id)));
            let names = options.iter().map(|(name, _)| name.clone()).collect::<Vec<_>>();
            let selected = options
                .iter()
                .map(|(_, id)| match *id {
                    -1 => current.project == ProjectFilter::All,
                    -2 => current.project == ProjectFilter::Unassigned,
                    id => current.project == ProjectFilter::Project(id),
                })
                .collect::<Vec<_>>();
            chip_row_owned(
                "项目",
                names,
                selected,
                "项目筛选",
                options.iter().map(|(_, id)| *id).collect(),
                move |scope| match scope {
                    -1 => filter.update(|f| f.project = ProjectFilter::All),
                    -2 => filter.update(|f| f.project = ProjectFilter::Unassigned),
                    id => filter.update(|f| f.project = ProjectFilter::Project(id)),
                },
            )
            .into_element()
        }))
        .child(dynamic(move || {
            let current = sort.get();
            chip_row(
                "排序",
                &["手动", "创建时间", "截止日", "优先级"],
                &[
                    current == TaskSort::Manual,
                    current == TaskSort::CreatedAt,
                    current == TaskSort::DueDate,
                    current == TaskSort::Priority,
                ],
                "排序方式",
                move |index| {
                    sort.set([TaskSort::Manual, TaskSort::CreatedAt, TaskSort::DueDate, TaskSort::Priority][index as usize]);
                },
            )
            .into_element()
        }))
}

/// A labelled row of small toggle chips (the Tasks view's filters); the
/// option at `index` reports `scopes[index]` to `on_select`.
fn chip_row(
    label: &'static str,
    names: &[&str],
    selected: &[bool],
    action: &'static str,
    on_select: impl Fn(i64) + Copy + 'static,
) -> impl IntoElement {
    chip_row_owned(
        label,
        names.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>(),
        selected.to_vec(),
        action,
        (0..names.len() as i64).collect(),
        on_select,
    )
}

#[allow(clippy::too_many_arguments)]
fn chip_row_owned(
    label: &'static str,
    names: Vec<String>,
    selected: Vec<bool>,
    action: &'static str,
    scopes: Vec<i64>,
    on_select: impl Fn(i64) + Copy + 'static,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(view().w(px(56.0)).child(section_heading(label)))
        .child(
            view()
                .flex_row()
                .gap_1()
                .flex_wrap()
                .children(names.iter().enumerate().map(|(index, name)| {
                    let selected = selected[index];
                    let scope = scopes[index];
                    let name = name.clone();
                    button()
                        .on_press(move |_: &()| on_select(scope))
                        .accessibility_label(Some(format!("{action} {name}").into()))
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
                        .child(text(name).text_size(px(12.0)))
                        .into_element()
                })),
        )
}

/// One group of the mutual-exclusion grouping: heading, count, rows.
fn group_section(
    store: Store,
    title: &'static str,
    items: impl Fn() -> Vec<TaskAggregate> + Copy + 'static,
) -> impl IntoElement {
    view().flex_col().gap_1().child(when(
        move || !items().is_empty(),
        move || {
            view()
                .test_id(&format!("tasks-group-{title}"))
                .flex_col()
                .gap_1()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(section_heading(title))
                        .child(
                            text(move || format!("{}", items().len()))
                                .text_size(px(11.0))
                                .text_color(tokens::fg_muted()),
                        ),
                )
                .child(dynamic(move || {
                    view().flex_col().gap(px(2.0)).children(
                        items()
                            .into_iter()
                            .map(|aggregate| task_row(store, aggregate).into_element())
                            .collect::<Vec<_>>(),
                    )
                }))
        },
    ))
}
