//! The Upcoming view (即将到来, docs/ui-design.md §4.2): future
//! Scheduled-Date Tasks in `列表` mode (grouped by day) and `看板` mode
//! (one column per day), with Due indicators inline.

use std::collections::BTreeMap;

use nsh_base::{button, theme};
use nsh_core::prelude::*;

use crate::domain::{LocalDate, TaskAggregate};
use crate::ui::controls::empty_state;
use crate::ui::theme as tokens;

use super::shell::store::Store;
use super::shell::task_row::{board_card, task_row};

/// How the Upcoming view lays its days out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    List,
    Board,
}

impl Mode {
    fn label(self) -> &'static str {
        match self {
            Self::List => "列表",
            Self::Board => "看板",
        }
    }
}

/// The Upcoming view for the active tab.
pub fn upcoming_view(store: Store) -> impl IntoElement {
    let mode = signal(Mode::List);
    let days = computed(move || {
        store.bump.get();
        group_by_day(store.upcoming_tasks())
    });
    view()
        .flex_1()
        .min_h(px(0.0))
        .flex_col()
        .child(header(mode))
        .child(when(
            move || days.get().is_empty(),
            || {
                empty_state(
                    "没有即将到来的任务",
                    "在任务详情或「今天」视图把任务排期到明天之后。",
                )
            },
        ))
        .child(when(
            move || !days.get().is_empty() && mode.get() == Mode::List,
            move || list_mode(store, days).into_element(),
        ))
        .child(when(
            move || !days.get().is_empty() && mode.get() == Mode::Board,
            move || board_mode(store, days).into_element(),
        ))
}

fn header(mode: Signal<Mode>) -> impl IntoElement {
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
            text("即将到来")
                .text_size(px(28.0))
                .font_weight(FontWeight::BOLD)
                .text_color(tokens::fg_primary()),
        )
        .child(dynamic(move || {
            let current = mode.get();
            // docs/kanon.html `.seg`: a control track, the selected
            // option standing on the raised surface.
            view()
                .flex_row()
                .gap(px(2.0))
                .p(px(3.0))
                .rounded(tokens::radius_row())
                .bg(tokens::control_bg())
                .children([Mode::List, Mode::Board].into_iter().map(|option| {
                    let selected = current == option;
                    button()
                        .on_press(move |_: &()| mode.set(option))
                        .accessibility_label(Some(format!("视图 {}", option.label()).into()))
                        .h(px(28.0))
                        .px_3()
                        .items_center()
                        .justify_center()
                        .rounded(move || theme().radius)
                        .bg(move || {
                            if selected {
                                theme().card
                            } else {
                                theme().background.opacity(0.0)
                            }
                        })
                        .text_color(move || {
                            if selected {
                                tokens::fg_primary()
                            } else {
                                tokens::fg_muted()
                            }
                        })
                        .hover(|s| s.bg(tokens::control_hover()))
                        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                        .transition_colors()
                        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
                        .child(text(option.label()).text_size(px(12.0)))
                        .into_element()
                }))
                .into_element()
        }))
}

/// The upcoming days grouped by their Scheduled Date, in date order.
fn group_by_day(tasks: Vec<TaskAggregate>) -> Vec<(LocalDate, Vec<TaskAggregate>)> {
    let mut days: BTreeMap<LocalDate, Vec<TaskAggregate>> = BTreeMap::new();
    for aggregate in tasks {
        if let Some(date) = aggregate.task.scheduled_date {
            days.entry(date).or_default().push(aggregate);
        }
    }
    days.into_iter().collect()
}

/// `列表` mode: one section per day, task rows beneath its heading.
fn list_mode(store: Store, days: Memo<Vec<(LocalDate, Vec<TaskAggregate>)>>) -> impl IntoElement {
    scroll().test_id("upcoming-list").flex_1().child(
        view()
            .flex_col()
            .gap_4()
            .px_5()
            .pb_5()
            .max_w(px(860.0))
            .w_full()
            .items_stretch()
            .self_center()
            .child(dynamic(move || {
                view()
                    .flex_col()
                    .gap_4()
                    .children(
                        days.get()
                            .into_iter()
                            .map(|(date, items)| day_section(store, date, items).into_element())
                            .collect::<Vec<_>>(),
                    )
                    .into_element()
            })),
    )
}

fn day_section(store: Store, date: LocalDate, items: Vec<TaskAggregate>) -> impl IntoElement {
    let count = items.len();
    view()
        .flex_col()
        .gap_1()
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    // docs/kanon.html `groupsList` names each day with a
                    // page-level `<h2>`.
                    text(date.long_label())
                        .text_size(px(18.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens::fg_primary()),
                )
                .child(
                    text(format!("{count} 项"))
                        .text_size(px(12.0))
                        .text_color(tokens::fg_muted()),
                ),
        )
        .child(
            view().flex_col().gap(px(2.0)).children(
                items
                    .into_iter()
                    .map(|aggregate| task_row(store, aggregate.task.id).into_element()),
            ),
        )
}

/// `看板` mode: one column per day, task cards inside, scrolled
/// horizontally.
fn board_mode(store: Store, days: Memo<Vec<(LocalDate, Vec<TaskAggregate>)>>) -> impl IntoElement {
    scroll()
        .test_id("upcoming-board")
        .horizontal()
        .flex_1()
        .child(
            view()
                .flex_row()
                .gap_3()
                .p_5()
                .items_stretch()
                .child(dynamic(move || {
                    view()
                        .flex_row()
                        .gap_3()
                        .items_stretch()
                        .children(
                            days.get()
                                .into_iter()
                                .map(|(date, items)| {
                                    board_column(store, date, items).into_element()
                                })
                                .collect::<Vec<_>>(),
                        )
                        .into_element()
                })),
        )
}

fn board_column(store: Store, date: LocalDate, items: Vec<TaskAggregate>) -> impl IntoElement {
    let count = items.len();
    view()
        // docs/kanon.html `.lane`: equal-width columns with a floor.
        .flex_1()
        .min_w(px(152.0))
        .min_h(px(245.0))
        .flex_col()
        .gap_1()
        .rounded(tokens::radius_card())
        .bg(tokens::panel_subtle())
        .px(px(10.0))
        .py(px(10.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .px_1()
                .pb_1()
                .child(
                    text(date.long_label())
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens::fg_primary()),
                )
                .child(
                    text(format!("{count}"))
                        .text_size(px(12.0))
                        .text_color(tokens::fg_muted()),
                ),
        )
        .child(
            view().flex_col().gap_2().children(
                items
                    .into_iter()
                    .map(|aggregate| board_card(store, aggregate).into_element())
                    .collect::<Vec<_>>(),
            ),
        )
}
