//! The window top chrome: sidebar toggle, per-tab back/forward, the
//! workbench tab strip, and the right-panel toggle
//! (docs/ui-design.md §3.1).

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::ui::controls::{icon_button, task_circle};
use crate::ui::theme as tokens;

use super::store::{use_store, Store};
use super::tabs::Tab;

/// The 56 px chrome row on the dark shell background.
pub fn top_bar() -> impl IntoElement {
    let store = use_store();
    view()
        .h(px(56.0))
        .flex_row()
        .items_center()
        .gap_2()
        .px_2()
        .bg(tokens::shell_bg())
        .border_b_1()
        .border_color(tokens::shell_border())
        .child(icon_button("☰", "切换侧栏", move || store.toggle_sidebar()))
        .child(when(
            move || store.strip.get().tabs[store.strip.get().active].can_back(),
            move || icon_button("←", "后退", move || store.back()),
        ))
        .child(when(
            move || store.strip.get().tabs[store.strip.get().active].can_forward(),
            move || icon_button("→", "前进", move || store.forward()),
        ))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_1()
                .px_2()
                .child(view().size(px(7.0)).rounded_full().bg(tokens::positive()))
                .child(text("本地").text_size(px(11.0)).text_color(tokens::shell_muted())),
        )
        .child(tab_strip(store).flex_1().min_w(px(0.0)))
        .child(inbox_badge(store))
        .child(icon_button("▤", "切换上下文面板", move || store.toggle_panel()))
}

/// The top-bar Inbox badge (docs/ui-design.md §4.6): it counts strict
/// Inbox tasks only and jumps to the Tasks view, whose 收件箱 group is
/// the Inbox.
fn inbox_badge(store: Store) -> impl IntoElement {
    let count = move || {
        store.bump.get();
        store.inbox_count()
    };
    nsh_base::button()
        .on_press(move || store.open_or_focus(super::tabs::Route::Tasks))
        .accessibility_label(move || Some(format!("收件箱 {} 项任务", count()).into()))
        .test_id("inbox-badge")
        .h(px(28.0))
        .px_2()
        .items_center()
        .gap_1()
        .rounded(move || theme().radius)
        .bg(move || theme().background.opacity(0.0))
        .text_color(move || if count() > 0 { tokens::shell_fg() } else { tokens::shell_muted() })
        .hover(|s| s.bg(tokens::shell_surface_active()).text_color(tokens::shell_fg()))
        .active(|s| s.opacity(0.7))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(text("收件箱").text_size(px(12.0)))
        .child(
            view()
                .min_w(px(18.0))
                .h(px(18.0))
                .px_1()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(move || if count() > 0 { tokens::accent() } else { tokens::shell_surface_active() })
                .child(text(move || count().to_string()).text_size(px(11.0)).text_color(move || {
                    if count() > 0 { theme().primary_foreground } else { tokens::shell_muted() }
                })),
        )
}

/// The workbench tab strip: one chip per tab plus the new-tab button.
fn tab_strip(store: Store) -> View {
    let tabs = computed(move || store.strip.get().tabs.clone());
    let active_id = computed(move || {
        let strip = store.strip.get();
        strip.tabs.get(strip.active).map(|tab| tab.id).unwrap_or(0)
    });
    view()
        .flex_row()
        .items_center()
        .gap_1()
        .min_w(px(0.0))
        .child(
            scroll()
                .horizontal()
                .scrollbar(false)
                .flex_1()
                .min_w(px(0.0))
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .px_1()
                        .child(each(tabs, |tab: &Tab| tab.id, move |tab, _index| {
                            tab_chip(store, tab, active_id)
                        })),
                ),
        )
        .child(icon_button("＋", "新建标签页", move || store.open_new_tab()))
}

fn tab_chip(store: Store, tab: ReadSignal<Tab>, active_id: Memo<u64>) -> impl IntoElement {
    let id = move || tab.with(|tab| tab.id);
    let is_active = move || id() == active_id.get();
    let pinned = move || tab.with(|tab| tab.pinned);
    let title = move || store.route_title(&tab.with(|tab| tab.current.clone()));
    let glyph = move || tab.with(|tab| tab.current.glyph());
    view()
        .flex_row()
        .items_center()
        .gap_1()
        .pl_2()
        .pr_1()
        .h(px(30.0))
        .rounded(tokens::radius_row())
        .bg(move || {
            if is_active() {
                tokens::shell_surface_active()
            } else {
                theme().background.opacity(0.0)
            }
        })
        .hover(|s| s.bg(tokens::shell_surface_active()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(
            // The pin toggle (small, before the title so the chip's main
            // press is the wide title button).
            nsh_base::button()
                .on_press(move || store.toggle_pin(id()))
                .accessibility_label("固定标签页")
                .size(px(16.0))
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(move || theme().background.opacity(0.0))
                .text_color(move || {
                    if pinned() {
                        tokens::accent()
                    } else {
                        theme().background.opacity(0.0)
                    }
                })
                .hover(|s| {
                    s.bg(tokens::shell_surface()).text_color(move || {
                        if pinned() { tokens::accent() } else { tokens::shell_muted() }
                    })
                })
                .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                .child(text("📌").text_size(px(11.0))),
        )
        .child(
            nsh_base::button()
                .on_press(move || store.activate_tab_by_id(id()))
                .accessibility_label(move || Some(format!("标签页 {}", title()).into()))
                .h(px(28.0))
                .px_1()
                .items_center()
                .gap_1()
                .rounded(tokens::radius_row())
                .child(text(glyph).text_size(px(13.0)).text_color(move || {
                    if is_active() { tokens::shell_fg() } else { tokens::shell_muted() }
                }))
                .child(text(title).text_size(px(13.0)).text_color(move || {
                    if is_active() { tokens::shell_fg() } else { tokens::shell_muted() }
                })),
        )
        .child(
            nsh_base::button()
                .on_press(move || store.close_tab(id()))
                .accessibility_label("关闭标签页")
                .size(px(16.0))
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(move || theme().background.opacity(0.0))
                .text_color(tokens::shell_muted())
                .hover(|s| s.bg(tokens::shell_surface()).text_color(tokens::shell_fg()))
                .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                .child(text("✕").text_size(px(10.0))),
        )
}

/// A compact row for the overview lists: circle, title, meta on the
/// right. Reused by the right context panel.
pub fn mini_task_row(store: Store, aggregate: crate::domain::TaskAggregate) -> impl IntoElement {
    let id = aggregate.task.id;
    let row_title = aggregate.task.title.clone();
    let circle_title = aggregate.task.title.clone();
    let overdue = aggregate.task.is_overdue_at(store.now());
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .px_2()
        .py_1p5()
        .rounded(tokens::radius_row())
        .hover(|s| s.bg(tokens::control_bg()))
        .child(task_circle(
            move || {
                store
                    .task(id)
                    .is_some_and(|aggregate| aggregate.task.status == crate::domain::TaskStatus::Done)
            },
            format!("完成 {circle_title}"),
            move || store.toggle_circle(id),
        ))
        .child(
            nsh_base::button()
                .on_press(move || store.open_task(id))
                .accessibility_label(Some(format!("打开任务 {row_title}").into()))
                .flex_1()
                .min_w(px(0.0))
                .items_center()
                .child(
                    view()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .child(text(row_title.clone()).text_size(px(13.0)).text_color(tokens::fg_primary()).number_of_lines(1))
                        .child(when(move || overdue, crate::ui::controls::overdue_badge)),
                ),
        )
}
