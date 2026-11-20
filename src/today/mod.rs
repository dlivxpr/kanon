//! The Today view (docs/ui-design.md §4.2): date header with
//! prev/today/next controls, the horizontal date strip, the greeting
//! card with stats and Carryover, quick task creation for the selected
//! date, and the task list with overdue badges and circle toggles.

use nsh_base::{button, theme};
use nsh_core::prelude::*;

use crate::domain::{LocalDate, TaskAggregate, TaskStatus};
use crate::ui::controls::{
    app_button, chip, empty_state, input_field, list_row, overdue_badge, task_circle, ButtonKind,
};
use crate::ui::theme as tokens;

use super::shell::store::Store;

/// The Today view for the active tab (its selected date lives on the
/// tab, so every tab plans its own day).
pub fn today_view(store: Store) -> impl IntoElement {
    let day = computed(move || {
        let strip = store.strip.get();
        strip.tabs[strip.active].selected_date
    });
    view()
        .flex_1()
        .min_h(px(0.0))
        .flex_col()
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
                        .child(date_header(store, day))
                        .child(date_strip(store, day))
                        .child(greeting_card(store, day))
                        .child(quick_add(store, day))
                        .child(task_list(store, day)),
                ),
        )
}

fn date_header(store: Store, day: Memo<LocalDate>) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .child(
            view()
                .flex_col()
                .gap_1()
                .child(
                    text(move || day.get().long_label())
                        .text_size(px(12.0))
                        .text_color(tokens::fg_muted()),
                )
                .child(
                    text(move || {
                        if day.get() == store.now().date {
                            "今天".to_owned()
                        } else {
                            day.get().to_string()
                        }
                    })
                    .text_size(px(28.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(tokens::fg_primary()),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(nav_pill(store, day, "‹ 前一天", -1))
                .child(
                    app_button(ButtonKind::Soft, "今天", move || {
                        store.set_selected_date(store.now().date);
                    })
                    .test_id("goto-today"),
                )
                .child(nav_pill(store, day, "后一天 ›", 1)),
        )
}

fn nav_pill(store: Store, day: Memo<LocalDate>, label: &'static str, offset: i64) -> impl IntoElement {
    app_button(ButtonKind::Ghost, label, move || {
        store.set_selected_date(day.get().add_days(offset));
    })
}

/// The horizontal strip of 16 days, from two days before the selected
/// one; today and the selected day are highlighted.
fn date_strip(store: Store, day: Memo<LocalDate>) -> impl IntoElement {
    view()
        .test_id("date-strip")
        .flex_row()
        .gap_1()
        .children((-2..14).map(|offset| date_cell(store, day, offset)))
}

fn date_cell(store: Store, day: Memo<LocalDate>, offset: i64) -> impl IntoElement {
    let cell_date = move || day.get().add_days(offset);
    let is_selected = move || cell_date() == day.get();
    let is_today = move || cell_date() == store.now().date;
    let label = move || {
        if is_today() {
            "今天".to_owned()
        } else {
            cell_date().weekday_label().to_owned()
        }
    };
    button()
        .on_press(move || store.set_selected_date(cell_date()))
        .accessibility_label(move || Some(format!("选择 {}", cell_date()).into()))
        .flex_1()
        .min_w(px(0.0))
        .flex_col()
        .items_center()
        .gap(px(2.0))
        .py_2()
        .rounded(tokens::radius_card())
        .border_1()
        .border_color(move || {
            if is_today() { tokens::accent() } else { theme().background.opacity(0.0) }
        })
        .bg(move || {
            if is_selected() { tokens::accent_soft() } else { theme().background.opacity(0.0) }
        })
        .hover(|s| s.bg(tokens::control_bg()))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(
            text(move || cell_date().day.to_string())
                .text_size(px(15.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || {
                    if is_selected() { tokens::accent() } else { tokens::fg_primary() }
                }),
        )
        .child(
            text(label)
                .text_size(px(11.0))
                .text_color(move || {
                    if is_today() || is_selected() { tokens::accent() } else { tokens::fg_muted() }
                }),
        )
}

/// The greeting card: counts for the selected day plus the Carryover
/// entry when past days still hold active tasks.
fn greeting_card(store: Store, day: Memo<LocalDate>) -> impl IntoElement {
    let stats = computed(move || {
        store.bump.get();
        let list = store.tasks_for_date(day.get());
        let active = list.iter().filter(|agg| agg.task.status.is_active()).count();
        let high = list
            .iter()
            .filter(|agg| agg.task.status.is_active() && agg.task.priority == crate::domain::Priority::High)
            .count();
        let doing = list.iter().filter(|agg| agg.task.status == TaskStatus::Doing).count();
        (list.len(), active, high, doing)
    });
    let carryover_count = computed(move || {
        store.bump.get();
        store.carryover_count()
    });
    let greeting = move || {
        if day.get() != store.now().date {
            return "当日安排".to_owned();
        }
        match store.now().time.hour {
            0..=5 => "夜深了",
            6..=10 => "早上好",
            11..=13 => "中午好",
            14..=17 => "下午好",
            _ => "晚上好",
        }
        .to_owned()
    };
    view()
        .rounded(tokens::radius_card())
        .bg(tokens::panel_subtle())
        .p_4()
        .flex_row()
        .items_center()
        .justify_between()
        .child(
            view()
                .flex_col()
                .gap_1()
                .child(
                    text(greeting)
                        .text_size(px(20.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(tokens::fg_primary()),
                )
                .child(
                    text(move || {
                        let (total, active, _, _) = stats.get();
                        if total == 0 {
                            "这一天暂时没有排期。".to_owned()
                        } else {
                            format!("还有 {active} 项任务待处理。")
                        }
                    })
                    .text_size(px(13.0))
                    .text_color(tokens::fg_body()),
                )
                .child(
                    view()
                        .flex_row()
                        .gap_3()
                        .pt_1()
                        .child(stat("项任务", move || stats.get().0))
                        .child(stat("项高优先级", move || stats.get().2))
                        .child(stat("项进行中", move || stats.get().3)),
                ),
        )
        .child(when(
            move || carryover_count.get() > 0,
            move || {
                crate::ui::controls::app_button_with(
                    crate::ui::controls::ButtonKind::Primary,
                    move || format!("结转 {} 项任务", carryover_count.get()).into(),
                    move || {
                        store.carryover();
                    },
                )
                .test_id("carryover")
            },
        ))
}

fn stat(label: &'static str, value: impl Fn() -> usize + 'static) -> impl IntoElement {
    view()
        .flex_row()
        .items_baseline()
        .gap_1()
        .child(text(move || value().to_string()).text_size(px(16.0)).font_weight(FontWeight::BOLD).text_color(tokens::accent()))
        .child(text(label).text_size(px(12.0)).text_color(tokens::fg_muted()))
}

/// The quick-create row: a task title enters the selected date.
fn quick_add(store: Store, day: Memo<LocalDate>) -> impl IntoElement {
    let draft = signal(String::new());
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(input_field(
            "添加任务，回车保存到所选日期",
            move || draft.get(),
            move |text: &str| draft.set(text.to_owned()),
        ))
        .child(
            app_button(ButtonKind::Positive, "添加任务", move || {
                if let Some(id) = store.create_task_on(day.get(), &draft.get()) {
                    draft.set(String::new());
                    store.open_task(id);
                }
            })
            .test_id("quick-add"),
        )
}

/// The task list of the selected date: non-cancelled tasks with an
/// overdue badge where due, and a quick circle toggle per row.
fn task_list(store: Store, day: Memo<LocalDate>) -> impl IntoElement {
    let list = computed(move || {
        store.bump.get();
        store.tasks_for_date(day.get())
    });
    view()
        .flex_col()
        .gap_1()
        .child(when(
            move || list.get().is_empty(),
            move || {
                empty_state(
                    "这一天是空的",
                    "在上方输入一条任务，或在右侧收件箱速记；也可以从顶部新建标签页。",
                )
            },
        ))
        .child(dynamic(move || {
            view().flex_col().gap(px(2.0)).children(
                list.get()
                    .into_iter()
                    .map(|aggregate| task_row(store, aggregate).into_element())
                    .collect::<Vec<_>>(),
            )
        }))
}

fn task_row(store: Store, aggregate: TaskAggregate) -> impl IntoElement {
    let id = aggregate.task.id;
    let row_title = aggregate.task.title.clone();
    let circle_title = aggregate.task.title.clone();
    let overdue = aggregate.task.is_overdue_at(store.now());
    let project = aggregate.project.clone();
    let tags = aggregate.tags.clone();
    let (done_subs, total_subs) = crate::domain::Task::subtask_progress(&aggregate.subtasks);
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
                    store
                        .task(id)
                        .is_some_and(|aggregate| aggregate.task.status == TaskStatus::Done)
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
