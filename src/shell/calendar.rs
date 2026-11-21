//! The Calendar view (日历, docs/ui-design.md §4.2) and the right
//! panel's Mini Calendar (迷你日历): tasks laid onto their Scheduled
//! Date, due-date-only tasks under a 截止 flag on their Due Date,
//! Project Milestones as ◇ markers, and a per-day quick add.
//! `docs/kanon.html` `calendar(mini)` is the visual reference; the
//! native `3 天 / 周 / 月` switch replaces the prototype's `seg`.
//!
//! Motion inventory (docs/ui-design.md §6): range switch and day press
//! — instant-token color wash; event hover — instant-token wash with a
//! fast-token press scale; the quick-add reveal — fast-token
//! transition. `tokens::ms` collapses every duration under reduced
//! motion.

use std::collections::BTreeMap;

use nsh_base::{button, theme};
use nsh_core::prelude::*;

use crate::domain::{LocalDate, Milestone, Project, TaskAggregate, TaskStatus};
use crate::ui::controls::{ButtonKind, app_button, panel_icon_button, submit_field};
use crate::ui::theme as tokens;

use super::store::{PREF_CALENDAR_RANGE, Store};
use super::tabs::Route;

pub(super) const PREF_WEEK_START: &str = "calendar_week_start";

/// The calendar's range: `3 天 / 周 / 月`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Range {
    ThreeDays,
    Week,
    Month,
}

impl Range {
    fn all() -> [Self; 3] {
        [Self::ThreeDays, Self::Week, Self::Month]
    }

    fn label(self) -> &'static str {
        match self {
            Self::ThreeDays => "3 天",
            Self::Week => "周",
            Self::Month => "月",
        }
    }

    /// The columns the range lays its days into.
    fn columns(self) -> u16 {
        match self {
            Self::ThreeDays => 3,
            Self::Week | Self::Month => 7,
        }
    }
}

/// The days a range shows: the first day, how many, and the column
/// count (3 天 keeps its three days side by side).
type Window = (LocalDate, usize, u16);

/// The plan behind every day cell: scheduled Tasks by date, due-date-
/// only Tasks by Due Date (the 截止 flag rows), and Project Milestones
/// by target date.
#[derive(Debug, Clone, Default, PartialEq)]
struct Plan {
    scheduled: BTreeMap<LocalDate, Vec<TaskAggregate>>,
    due_only: BTreeMap<LocalDate, Vec<TaskAggregate>>,
    milestones: BTreeMap<LocalDate, Vec<(Project, Milestone)>>,
}

/// The full Calendar view for the active tab. The range, the anchor day
/// and the quick-add selection live on the keyed view, so every tab
/// plans its own stretch of days.
pub(super) fn calendar_view(store: Store) -> impl IntoElement {
    let range = signal(match store.preference(PREF_CALENDAR_RANGE).as_deref() {
        Some("three") => Range::ThreeDays,
        Some("week") => Range::Week,
        _ => Range::Month,
    });
    let anchor = signal(store.now().date);
    let quick_add_day = signal(None::<LocalDate>);
    let plan = plan(store);
    let window = computed(move || {
        let anchor = anchor.get();
        let first_weekday = first_weekday(store);
        match range.get() {
            Range::Month => month_window(anchor, first_weekday),
            Range::Week => (
                anchor.add_days(-weekday_offset(anchor, first_weekday)),
                7,
                Range::Week.columns(),
            ),
            Range::ThreeDays => (anchor, 3, Range::ThreeDays.columns()),
        }
    });
    view()
        .test_id("calendar")
        .flex_1()
        .min_h(px(0.0))
        .flex_col()
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .px_5()
                .pt_5()
                .pb_2()
                .max_w(px(860.0))
                .w_full()
                .self_center()
                .child(
                    text("日历")
                        .text_size(px(28.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(tokens::fg_primary()),
                )
                .child(range_switch(range)),
        )
        .child(
            view()
                .px_5()
                .pb_2()
                .max_w(px(860.0))
                .w_full()
                .self_center()
                .child(period_bar(range, anchor, window)),
        )
        .child(
            scroll().test_id("calendar-scroll").flex_1().child(
                view()
                    .flex_col()
                    .gap_1()
                    .px_5()
                    .pb_5()
                    .max_w(px(860.0))
                    .w_full()
                    .items_stretch()
                    .self_center()
                    .child(weekday_row(window))
                    .child(grid(store, plan, range, anchor, window, quick_add_day)),
            ),
        )
}

/// The native `3 天 / 周 / 月` switch: the Upcoming view's mode-switch
/// pattern, standing in for the prototype's `seg` control.
fn range_switch(range: Signal<Range>) -> impl IntoElement {
    view()
        .test_id("calendar-range")
        .flex_row()
        .gap_1()
        .children(Range::all().into_iter().map(|option| {
            let selected = move || range.get() == option;
            button()
                .on_press(move |_: &()| range.set(option))
                .accessibility_label(Some(format!("范围 {}", option.label()).into()))
                .h(px(28.0))
                .px_3()
                .items_center()
                .justify_center()
                .rounded_full()
                .border_1()
                .border_color(move || {
                    if selected() {
                        tokens::accent()
                    } else {
                        tokens::border_default()
                    }
                })
                .bg(move || {
                    if selected() {
                        tokens::accent_soft()
                    } else {
                        theme().background.opacity(0.0)
                    }
                })
                .text_color(move || {
                    if selected() {
                        tokens::accent()
                    } else {
                        tokens::fg_body()
                    }
                })
                .hover(|s| s.bg(tokens::control_bg()))
                .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                .child(text(option.label()).text_size(px(12.0)))
                .into_element()
        }))
}

/// The period label (`2026年11月` / `11月16日 — 11月22日`) with the
/// prev/next controls: one month step in 月, one whole window in the
/// other ranges.
fn period_bar(
    range: Signal<Range>,
    anchor: Signal<LocalDate>,
    window: Memo<Window>,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .child(
            text(move || {
                if range.get() == Range::Month {
                    let anchor = anchor.get();
                    format!("{}年{}月", anchor.year, anchor.month)
                } else {
                    let (start, days, _) = window.get();
                    format!(
                        "{} — {}",
                        short_label(start),
                        short_label(start.add_days(days as i64 - 1))
                    )
                }
            })
            .text_size(px(15.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(tokens::fg_primary()),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_1()
                .child(panel_icon_button("‹", "上一期", move || {
                    anchor.set(step(range.get(), anchor.get(), -1));
                }))
                .child(panel_icon_button("›", "下一期", move || {
                    anchor.set(step(range.get(), anchor.get(), 1));
                })),
        )
}

/// One step of the period controls.
fn step(range: Range, anchor: LocalDate, direction: i64) -> LocalDate {
    match range {
        Range::Month => add_months(LocalDate::new(anchor.year, anchor.month, 1), direction),
        Range::Week => anchor.add_days(7 * direction),
        Range::ThreeDays => anchor.add_days(3 * direction),
    }
}

/// The centered column headers follow the range, including its column count.
fn weekday_row(window: Memo<Window>) -> impl IntoElement {
    view().test_id("calendar-weekdays").child(dynamic(move || {
        let (start, _, columns) = window.get();
        view()
            .grid()
            .grid_cols(columns)
            .gap(px(3.0))
            .pb_1()
            .children((0..i64::from(columns)).map(|offset| {
                text(start.add_days(offset).weekday_label())
                    .text_xs()
                    .text_center()
                    .text_color(tokens::fg_muted)
            }))
    }))
}

/// The day grid: `days` cells from the window's first day laid into its
/// columns. One region, rebuilt when the window, the plan or the
/// quick-add selection changes.
fn grid(
    store: Store,
    plan: Memo<Plan>,
    range: Signal<Range>,
    anchor: Signal<LocalDate>,
    window: Memo<Window>,
    quick_add_day: Signal<Option<LocalDate>>,
) -> impl IntoElement {
    view().test_id("calendar-grid").child(dynamic(move || {
        let range = range.get();
        let anchor = anchor.get();
        let (start, days, columns) = window.get();
        let plan = plan.get();
        view()
            .grid()
            .grid_cols(columns)
            .gap(px(3.0))
            .children((0..days as i64).map(|offset| {
                let date = start.add_days(offset);
                let in_month = range != Range::Month
                    || (date.year == anchor.year && date.month == anchor.month);
                day_cell(store, &plan, date, in_month, quick_add_day).into_element()
            }))
            .into_element()
    }))
}

/// One day cell: the date number (today accented, other-month muted),
/// scheduled task events, due-date-only events under the 截止 flag,
/// ◇ Milestones, and the quick-add row of the selected day.
fn day_cell(
    store: Store,
    plan: &Plan,
    date: LocalDate,
    in_month: bool,
    quick_add_day: Signal<Option<LocalDate>>,
) -> impl IntoElement {
    let is_today = date == store.now().date;
    let is_selected = move || quick_add_day.get() == Some(date);
    let scheduled = plan.scheduled.get(&date).cloned().unwrap_or_default();
    let due_only = plan.due_only.get(&date).cloned().unwrap_or_default();
    let milestones = plan.milestones.get(&date).cloned().unwrap_or_default();
    view()
        .test_id(&format!("calendar-day-{date}"))
        .flex_col()
        .gap_1()
        .p_1p5()
        .min_h(px(76.0))
        .rounded(tokens::radius_row())
        .border_1()
        .border_color(tokens::border_subtle())
        .bg(move || {
            if is_selected() {
                tokens::accent_soft()
            } else {
                theme().card
            }
        })
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(date_number(date, in_month, is_today, quick_add_day))
        .children(
            scheduled
                .into_iter()
                .map(|aggregate| task_event(store, aggregate, false).into_element()),
        )
        .children(
            due_only
                .into_iter()
                .map(|aggregate| task_event(store, aggregate, true).into_element()),
        )
        .children(
            milestones.into_iter().map(|(project, milestone)| {
                milestone_event(store, project, milestone).into_element()
            }),
        )
        .child(when(is_selected, move || {
            quick_add_row(store, date, quick_add_day)
        }))
}

/// The date number: today gets the accent disc, other-month days read
/// muted. The press toggles the day's quick-add row.
fn date_number(
    date: LocalDate,
    in_month: bool,
    is_today: bool,
    quick_add_day: Signal<Option<LocalDate>>,
) -> impl IntoElement {
    button()
        .on_press(move || {
            quick_add_day.update(|current| {
                *current = if *current == Some(date) {
                    None
                } else {
                    Some(date)
                };
            });
        })
        .accessibility_label(Some(format!("{date} 添加任务").into()))
        .size(px(22.0))
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(move || {
            if is_today {
                tokens::accent()
            } else {
                theme().background.opacity(0.0)
            }
        })
        .text_color(move || {
            if is_today {
                theme().primary_foreground
            } else if in_month {
                tokens::fg_primary()
            } else {
                tokens::fg_muted()
            }
        })
        .hover(move |s| {
            if is_today {
                s
            } else {
                s.bg(tokens::control_bg())
            }
        })
        .active(move |s| s.opacity(0.8))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(text(date.day.to_string()).text_size(px(12.0)))
}

/// A task event: its title with the scheduled time-of-day trailing; a
/// done row reads struck-through and muted. `due_flag` rows are the
/// due-date-only tasks, marked 截止 in the warning token. The press
/// opens the Task Detail in the right context panel.
fn task_event(store: Store, aggregate: TaskAggregate, due_flag: bool) -> impl IntoElement {
    let id = aggregate.task.id;
    let title = aggregate.task.title.clone();
    let done = aggregate.task.status == TaskStatus::Done;
    let time = if due_flag {
        aggregate.task.due_time
    } else {
        aggregate.task.scheduled_time
    };
    button()
        .on_press(move || store.open_task(id))
        .accessibility_label(Some(format!("打开任务 {title}").into()))
        .flex_row()
        .items_center()
        .gap_1()
        .px_1()
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(move || {
            if due_flag {
                tokens::warning_soft()
            } else {
                tokens::control_bg()
            }
        })
        .hover(move |s| {
            s.bg(if due_flag {
                tokens::warning_soft()
            } else {
                tokens::control_hover()
            })
        })
        .active(move |s| s.opacity(0.8).scale(0.98))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .transition_transform()
        .duration(tokens::ms(tokens::MOTION_FAST_MS))
        .child(when(
            move || due_flag,
            move || {
                text("截止 ")
                    .text_size(px(10.0))
                    .text_color(tokens::warning())
                    .into_element()
            },
        ))
        .child(
            text(title)
                .text_size(px(11.0))
                .flex_1()
                .min_w(px(0.0))
                .number_of_lines(1)
                .text_color(move || {
                    if done {
                        tokens::fg_muted()
                    } else {
                        tokens::fg_body()
                    }
                })
                .style_when(move || done, |s| s.line_through()),
        )
        .child(when(
            move || time.is_some(),
            move || {
                text(time.map(|time| store.format_time(time)).unwrap_or_default())
                    .text_size(px(10.0))
                    .text_color(tokens::fg_muted())
                    .into_element()
            },
        ))
}

/// A Project Milestone on its target date: the ◇ marker in the info
/// token; the press opens the Project.
fn milestone_event(store: Store, project: Project, milestone: Milestone) -> impl IntoElement {
    let project_id = project.id;
    let title = milestone.title.clone();
    button()
        .on_press(move || store.open_project(project_id))
        .accessibility_label(Some(format!("里程碑 {} · {}", title, project.name).into()))
        .flex_row()
        .items_center()
        .gap_1()
        .px_1()
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(tokens::info_soft())
        .hover(|s| s.bg(tokens::control_hover()))
        .active(|s| s.opacity(0.8).scale(0.98))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .transition_transform()
        .duration(tokens::ms(tokens::MOTION_FAST_MS))
        .child(text("◇").text_size(px(10.0)).text_color(tokens::info()))
        .child(
            text(title)
                .text_size(px(11.0))
                .flex_1()
                .min_w(px(0.0))
                .number_of_lines(1)
                .text_color(tokens::fg_body()),
        )
}

/// The selected day's quick add: Enter (or the ＋ button) creates a task
/// scheduled for that day and opens its detail, like the Today view's
/// quick add.
fn quick_add_row(
    store: Store,
    date: LocalDate,
    quick_add_day: Signal<Option<LocalDate>>,
) -> impl IntoElement {
    let draft = signal(String::new());
    let submit = move |text: &str| {
        if let Some(id) = store.create_task_on(date, text) {
            quick_add_day.set(None);
            store.open_task(id);
        }
    };
    view()
        .test_id(&format!("calendar-add-{date}"))
        .flex_row()
        .items_center()
        .gap_1()
        .pt_1()
        .child(submit_field(
            "添加任务，回车保存",
            move || draft.get(),
            move |text: &str| draft.set(text.to_owned()),
            submit,
        ))
        .child(when(
            move || !draft.get().trim().is_empty(),
            move || {
                app_button(ButtonKind::Positive, "添加", move || {
                    draft.with_untracked(|title| submit(title));
                })
                .test_id("calendar-add-submit")
            },
        ))
        .child(panel_icon_button("✕", "取消添加", move || {
            quick_add_day.set(None);
        }))
}

/// The right panel's Mini Calendar (docs/ui-design.md §4.6 迷你日历):
/// the month around its own anchor, dots on days holding tasks or
/// Milestones, today highlighted. A day press jumps the workbench to
/// 今天 with that date selected.
pub(super) fn mini_calendar(store: Store) -> impl IntoElement {
    let anchor = signal(store.now().date);
    let plan = plan(store);
    let month = computed(move || month_window(anchor.get(), first_weekday(store)));
    view()
        .test_id("mini-calendar")
        .flex_col()
        .gap_2()
        .p_3()
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    text(move || {
                        let anchor = anchor.get();
                        format!("{}年{}月", anchor.year, anchor.month)
                    })
                    .text_size(px(13.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(tokens::fg_primary()),
                )
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .child(panel_icon_button("‹", "上一月", move || {
                            let anchor_now = anchor.get();
                            anchor.set(add_months(
                                LocalDate::new(anchor_now.year, anchor_now.month, 1),
                                -1,
                            ));
                        }))
                        .child(panel_icon_button("›", "下一月", move || {
                            let anchor_now = anchor.get();
                            anchor.set(add_months(
                                LocalDate::new(anchor_now.year, anchor_now.month, 1),
                                1,
                            ));
                        })),
                ),
        )
        .child(weekday_row(month))
        .child(dynamic(move || {
            let (start, days, _) = month.get();
            let plan = plan.get();
            let month_of = (anchor.get().year, anchor.get().month);
            view()
                .grid()
                .grid_cols(7)
                .gap(px(2.0))
                .children((0..days as i64).map(|offset| {
                    mini_day(store, &plan, start.add_days(offset), month_of).into_element()
                }))
                .into_element()
        }))
}

/// One Mini Calendar day: the number (today accented), the dot under a
/// day holding tasks or Milestones, and the jump to 今天.
fn mini_day(store: Store, plan: &Plan, date: LocalDate, month_of: (i32, u8)) -> impl IntoElement {
    let is_today = date == store.now().date;
    let in_month = (date.year, date.month) == month_of;
    let count = plan.scheduled.get(&date).map_or(0, Vec::len)
        + plan.due_only.get(&date).map_or(0, Vec::len);
    let has_marks = count > 0 || plan.milestones.contains_key(&date);
    button()
        .on_press(move || {
            store.open_or_focus(Route::Today);
            store.set_selected_date(date);
        })
        .accessibility_label(Some(format!("{date} · {count} 项任务").into()))
        .test_id(&format!("mini-day-{date}"))
        .h(px(30.0))
        .flex_col()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .bg(move || {
            if is_today {
                tokens::accent_soft()
            } else {
                theme().background.opacity(0.0)
            }
        })
        .hover(|s| s.bg(tokens::control_bg()))
        .active(|s| s.opacity(0.8))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(
            text(date.day.to_string())
                .text_size(px(12.0))
                .font_weight(if is_today {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .text_color(move || {
                    if is_today {
                        tokens::accent_ink()
                    } else if in_month {
                        tokens::fg_primary()
                    } else {
                        tokens::fg_muted()
                    }
                }),
        )
        .child(view().size(px(4.0)).rounded_full().bg(move || {
            if has_marks {
                tokens::accent()
            } else {
                theme().background.opacity(0.0)
            }
        }))
}

/// The reactive plan: every live Task placed on its Scheduled Date
/// (done included; cancelled and trashed excluded) or, when it has no
/// Scheduled Date, on its Due Date under the 截止 flag; every Project
/// Milestone on its target date. One pass over `live_tasks`, grouped
/// and time-ordered per day.
fn plan(store: Store) -> Memo<Plan> {
    computed(move || {
        store.bump.get();
        let mut plan = Plan::default();
        for aggregate in store.live_tasks() {
            let task = &aggregate.task;
            if task.status == TaskStatus::Cancelled {
                continue;
            }
            match (task.scheduled_date, task.due_date) {
                (Some(date), _) => plan.scheduled.entry(date).or_default().push(aggregate),
                (None, Some(due)) => plan.due_only.entry(due).or_default().push(aggregate),
                _ => {}
            }
        }
        for rows in plan.scheduled.values_mut() {
            rows.sort_by_key(|a| (a.task.scheduled_time.is_none(), a.task.scheduled_time));
        }
        for rows in plan.due_only.values_mut() {
            rows.sort_by_key(|a| (a.task.due_time.is_none(), a.task.due_time));
        }
        for project in store.projects() {
            for milestone in store.milestones(project.id) {
                if let Some(date) = milestone.target_date {
                    plan.milestones
                        .entry(date)
                        .or_default()
                        .push((project.clone(), milestone));
                }
            }
        }
        plan
    })
}

fn first_weekday(store: Store) -> u8 {
    if store.preference(PREF_WEEK_START).as_deref() == Some("sunday") {
        6
    } else {
        0
    }
}

fn weekday_offset(date: LocalDate, first_weekday: u8) -> i64 {
    (i64::from(date.weekday()) - i64::from(first_weekday)).rem_euclid(7)
}

/// The month grid starts on the configured first weekday and spans whole weeks.
fn month_window(anchor: LocalDate, first_weekday: u8) -> Window {
    let first = LocalDate::new(anchor.year, anchor.month, 1);
    let offset = weekday_offset(first, first_weekday);
    let days = days_in_month(anchor.year, anchor.month) as i64;
    let count = ((offset + days + 6) / 7) * 7;
    (first.add_days(-offset), count as usize, 7)
}

/// `11月16日`.
fn short_label(date: LocalDate) -> String {
    format!("{}月{}日", date.month, date.day)
}

/// The date one month away, clamped to the target month's day count.
fn add_months(date: LocalDate, delta: i64) -> LocalDate {
    let total = date.year as i64 * 12 + (date.month as i64 - 1) + delta;
    let year = total.div_euclid(12) as i32;
    let month = total.rem_euclid(12) as u8 + 1;
    let day = date.day.min(days_in_month(year, month));
    LocalDate::new(year, month, day)
}

/// The number of days in a month, from the civil-day distance between
/// its first day and the next month's.
fn days_in_month(year: i32, month: u8) -> u8 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    (LocalDate::new(next_year, next_month, 1).days_from_civil()
        - LocalDate::new(year, month, 1).days_from_civil()) as u8
}
