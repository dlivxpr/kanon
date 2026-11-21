//! The Today view (docs/ui-design.md §4.2): date header with
//! prev/today/next controls, the horizontal date strip, the greeting
//! card with stats and Carryover, quick task creation for the selected
//! date, and the task list with overdue badges and circle toggles.

use nsh_base::{button, theme};
use nsh_core::prelude::*;

use crate::domain::{LocalDate, TaskStatus};
use crate::ui::controls::{ButtonKind, app_button, empty_state, panel_icon_button, submit_field};
use crate::ui::theme as tokens;

use super::shell::store::{PREF_CARRYOVER, Store};
use super::shell::task_row::task_row;

/// The Today view for the active tab (its selected date lives on the
/// tab, so every tab plans its own day).
pub fn today_view(store: Store) -> impl IntoElement {
    let day = computed(move || {
        let strip = store.strip.get();
        strip.tabs[strip.active].selected_date
    });
    view().flex_1().min_h(px(0.0)).flex_col().child(
        scroll().flex_1().child(
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
                .child(carryover_strip(store, day))
                .child(
                    text("当日任务")
                        .role(AccessibilityRole::Header)
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens::fg_primary),
                )
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
                        let date = day.get();
                        if date == store.now().date {
                            "今天".to_owned()
                        } else {
                            // docs/kanon.html `dt(d)`: month-long + day.
                            format!("{}月{}日", date.month, date.day)
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
                .child(date_nav(store, day, "‹", "前一天", -1))
                .child(
                    app_button(ButtonKind::Soft, "今天", move || {
                        store.set_selected_date(store.now().date);
                    })
                    .test_id("goto-today"),
                )
                .child(date_nav(store, day, "›", "后一天", 1)),
        )
}

/// The previous/next day steppers of the date header
/// (docs/kanon.html's icon-only `ib('day', …)` buttons).
fn date_nav(
    store: Store,
    day: Memo<LocalDate>,
    glyph: &'static str,
    label: &'static str,
    offset: i64,
) -> impl IntoElement {
    button()
        .on_press(move || store.set_selected_date(day.get().add_days(offset)))
        .accessibility_label(Some(label.into()))
        .size(px(30.0))
        .items_center()
        .justify_center()
        .rounded(move || theme().radius)
        .bg(move || theme().background.opacity(0.0))
        .text_color(tokens::fg_body())
        .hover(|s| s.bg(tokens::control_bg()))
        .active(|s| s.opacity(0.7))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(text(glyph).text_size(px(16.0)))
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
        .bg(move || {
            // docs/kanon.html `.date-cell.chosen`: the selected day is a
            // filled accent cell, today is only its label text.
            if is_selected() {
                tokens::accent()
            } else {
                theme().background.opacity(0.0)
            }
        })
        .hover(move |s| {
            if is_selected() {
                s
            } else {
                s.bg(tokens::control_bg())
            }
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(
            text(move || cell_date().day.to_string())
                .text_size(px(18.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || {
                    if is_selected() {
                        theme().primary_foreground
                    } else {
                        tokens::fg_primary()
                    }
                }),
        )
        .child(text(label).text_size(px(10.0)).text_color(move || {
            if is_selected() {
                theme().primary_foreground
            } else {
                tokens::fg_muted()
            }
        }))
}

/// The greeting card: counts for the selected day plus the Carryover
/// entry when past days still hold active tasks.
fn greeting_card(store: Store, day: Memo<LocalDate>) -> impl IntoElement {
    let stats = computed(move || {
        store.bump.get();
        let list = store.tasks_for_date(day.get());
        let active = list
            .iter()
            .filter(|agg| agg.task.status.is_active())
            .count();
        let high = list
            .iter()
            .filter(|agg| {
                agg.task.status.is_active() && agg.task.priority == crate::domain::Priority::High
            })
            .count();
        let doing = list
            .iter()
            .filter(|agg| agg.task.status == TaskStatus::Doing)
            .count();
        (list.len(), active, high, doing)
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
        .bg(tokens::daybreak_bg)
        .relative()
        .overflow_hidden()
        .p_5()
        .min_h(px(135.0))
        .flex_row()
        .items_center()
        .justify_between()
        .child(
            svg(nsh_core::element::ImageSrc::svg_markup(include_str!(
                "../../assets/daybreak.svg"
            )))
            .absolute()
            .right_0()
            .bottom_0()
            .w(percentage(0.40))
            .h_full()
            .text_color(tokens::daybreak_art)
            .pointer_events(nsh_core::PointerEvents::None),
        )
        .child(
            view()
                .flex_col()
                .gap_1()
                .child(
                    text(greeting)
                        .text_size(px(24.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(tokens::daybreak_fg),
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
                    .text_size(px(12.0))
                    .text_color(tokens::daybreak_fg),
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
}

/// The Carryover strip (docs/kanon.html `.carry`): only while today is
/// the selected day; moves every still-active task scheduled before
/// today onto today, clearing its time-of-day (the Due Date/Time stay).
fn carryover_strip(store: Store, day: Memo<LocalDate>) -> impl IntoElement {
    let count = computed(move || store.carryover_count());
    let expanded = signal(false);
    when(
        move || {
            day.get() == store.now().date
                && count.get() > 0
                && store.preference_enabled(PREF_CARRYOVER)
        },
        move || {
            view()
                .gap_2()
                .child(
                    view()
                        .test_id("carryover-strip")
                        .rounded(tokens::radius_card())
                        .bg(tokens::accent_soft)
                        .px(px(14.0))
                        .py_2()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            text(move || format!("昨日结转 · {} 项", count.get()))
                                .text_size(px(13.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(tokens::accent_ink),
                        )
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .child(
                                    app_button(ButtonKind::Soft, "移至今天", move || {
                                        store.carryover();
                                        expanded.set(false);
                                    })
                                    .test_id("carryover"),
                                )
                                .child(
                                    panel_icon_button(
                                        move || if expanded.get() { "▴" } else { "▾" },
                                        "展开结转任务",
                                        move || expanded.set(!expanded.get_untracked()),
                                    )
                                    .accessibility_label(
                                        move || {
                                            Some(
                                                if expanded.get() {
                                                    "收起结转任务"
                                                } else {
                                                    "展开结转任务"
                                                }
                                                .into(),
                                            )
                                        },
                                    ),
                                ),
                        ),
                )
                .child(when(expanded, move || {
                    view().test_id("carryover-preview").gap_1().child(each(
                        move || store.carryover_task_ids(),
                        |id| *id,
                        move |id, _| task_row(store, id.get_untracked()).into_element(),
                    ))
                }))
                .into_element()
        },
    )
}

fn stat(label: &'static str, value: impl Fn() -> usize + 'static) -> impl IntoElement {
    view()
        .flex_row()
        .items_baseline()
        .gap_1()
        .child(
            text(move || value().to_string())
                .text_size(px(16.0))
                .font_weight(FontWeight::BOLD)
                .text_color(tokens::fg_primary()),
        )
        .child(
            text(label)
                .text_size(px(12.0))
                .text_color(tokens::fg_muted()),
        )
}

/// The quick-create row: a task title enters the selected date.
fn quick_add(store: Store, day: Memo<LocalDate>) -> impl IntoElement {
    let draft = signal(String::new());
    let submit = move |title: &str| {
        if let Some(id) = store.create_task_on(day.get(), title) {
            draft.set(String::new());
            store.open_task(id);
        }
    };
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(submit_field(
            "添加任务，回车保存到所选日期",
            move || draft.get(),
            move |text: &str| draft.set(text.to_owned()),
            submit,
        ))
        .child(
            app_button(ButtonKind::Positive, "添加任务", move || {
                submit(&draft.get_untracked())
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
        .test_id("today-task-list")
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
        .child(each(
            list,
            |aggregate: &crate::domain::TaskAggregate| aggregate.task.id,
            move |aggregate, _index| task_row(store, aggregate.with(|a| a.task.id)).into_element(),
        ))
}
