//! The themed window chrome and the icon-only work tab strip.

use nsh_core::element::desktop::title_bar;
use nsh_core::prelude::*;
use nsh_core::{TouchEvent, TouchPhase};

use crate::ui::chrome::{Icon, chrome_button, icon};
use crate::ui::controls::task_circle;
use crate::ui::theme as tokens;

use super::ContextChoice;
use super::create::Create;
use super::store::{Store, use_store};
use super::tabs::Tab;

/// A single app-drawn row; NSH still owns the system caption buttons.
pub(super) fn top_bar(
    search_open: Signal<bool>,
    create_open: Signal<Option<Create>>,
) -> impl IntoElement {
    let store = use_store();
    view()
        .test_id("window-topbar")
        .h(|| px(56.0).max(title_bar().top))
        .flex_shrink(0.0)
        .window_drag(true)
        .flex_row()
        .items_center()
        .gap_1()
        .pl(|| title_bar().left + px(8.0))
        .pr(|| title_bar().right + px(8.0))
        .bg(tokens::shell_bg)
        .child(
            chrome_button(|| Icon::Sidebar, || "切换侧栏".into(), false)
                .on_press(move || store.toggle_sidebar()),
        )
        .child(
            chrome_button(|| Icon::Back, || "后退".into(), false)
                .disabled(move || {
                    store
                        .strip
                        .with(|strip| !strip.tabs[strip.active].can_back())
                })
                .on_press(move || store.back()),
        )
        .child(
            chrome_button(|| Icon::Forward, || "前进".into(), false)
                .disabled(move || {
                    store
                        .strip
                        .with(|strip| !strip.tabs[strip.active].can_forward())
                })
                .on_press(move || store.forward()),
        )
        .child(
            view()
                .size(px(28.0))
                .flex_shrink(0.0)
                .items_center()
                .justify_center()
                .label("本地工作区")
                .tooltip(crate::ui::chrome::hint("本地工作区"))
                .text_color(tokens::shell_muted)
                .child(icon(|| Icon::Local, px(15.0))),
        )
        .child(tab_strip(store).flex_shrink(1.0).min_w(px(38.0)))
        .child(view().min_w(px(28.0)).h_full().flex_1().window_drag(true))
        .child(
            chrome_button(|| Icon::Search, || "全局搜索".into(), false)
                .on_press(move || search_open.set(true)),
        )
        .child(
            chrome_button(|| Icon::Plus, || "快速新建".into(), false)
                .on_press(move || create_open.set(Some(Create::Menu))),
        )
        .child(inbox_badge(store))
        .child(panel_modes(store))
        .child(
            chrome_button(|| Icon::Panel, || "切换上下文面板".into(), false)
                .selected(move || store.panel_visible())
                .selected_style(|s| s.bg(tokens::shell_surface_active))
                .on_press(move || store.toggle_panel()),
        )
}

fn inbox_badge(store: Store) -> impl IntoElement {
    let count = computed(move || {
        store.bump.get();
        store.inbox_count()
    });
    chrome_button(
        || Icon::Inbox,
        move || format!("收件箱 {} 项任务", count.get()).into(),
        false,
    )
    .test_id("inbox-badge")
    .selected(move || {
        store.panel_visible()
            && store.selected_task().is_none()
            && (store.panel_choice() == ContextChoice::Inbox
                || (store.panel_choice() == ContextChoice::Auto
                    && super::context::active_document(store).is_none()))
    })
    .selected_style(|s| s.bg(tokens::shell_surface_active))
    .on_press(move || {
        store.choose_panel(ContextChoice::Inbox);
    })
    .child(when(
        move || count.get() > 0,
        || {
            view()
                .absolute()
                .top_1()
                .right_1()
                .size(px(6.0))
                .rounded_full()
                .bg(tokens::accent_ink)
        },
    ))
}

fn panel_modes(store: Store) -> impl IntoElement {
    let document = move || super::context::active_document(store).is_some();
    let narrow = || window().width.0 <= 700.0;
    let mode_button = move |mode, glyph, label: &'static str| {
        chrome_button(move || glyph, move || label.into(), false)
            .selected(move || {
                store.panel_choice() == mode
                    && store.panel_visible()
                    && store.selected_task().is_none()
            })
            .selected_style(|s| s.bg(tokens::accent_soft).text_color(tokens::accent_ink))
            .on_press(move || store.choose_panel(mode))
    };
    let menu_item = move |mode: ContextChoice, label: &'static str| {
        nsh_base::menu_item()
            .on_select(move || store.choose_panel(mode))
            .px_3()
            .py_2()
            .rounded(tokens::radius_row())
            .hover(|s| s.bg(tokens::control_hover))
            .child(text(label).text_sm().text_color(tokens::fg_primary))
    };
    view()
        .flex_row()
        .items_center()
        .gap_1()
        .child(when(
            move || !narrow() && document(),
            move || {
                view()
                    .flex_row()
                    .gap_1()
                    .child(mode_button(ContextChoice::Outline, Icon::Outline, "大纲"))
                    .child(mode_button(ContextChoice::Links, Icon::Links, "关联与反链"))
            },
        ))
        .child(when(
            move || !narrow() && !document(),
            move || mode_button(ContextChoice::Calendar, Icon::Calendar, "迷你日历"),
        ))
        .child(when(narrow, move || {
            nsh_base::menu()
                .popup_style(|s| {
                    s.p_1()
                        .rounded(tokens::radius_card())
                        .border_1()
                        .border_color(tokens::border_default)
                        .bg(tokens::panel_raised)
                        .shadow_md()
                })
                .trigger(
                    view()
                        .size(px(32.0))
                        .items_center()
                        .justify_center()
                        .rounded(tokens::radius_row())
                        .text_color(tokens::shell_muted)
                        .hover(|s| s.bg(tokens::shell_surface))
                        .label("选择面板模式")
                        .child(icon(|| Icon::Outline, px(16.0))),
                )
                .child(when(document, move || {
                    menu_item(ContextChoice::Outline, "大纲")
                }))
                .child(when(document, move || {
                    menu_item(ContextChoice::Links, "关联与反链")
                }))
                .child(when(
                    move || !document(),
                    move || menu_item(ContextChoice::Calendar, "迷你日历"),
                ))
        }))
}

/// The workbench tab strip: one chip per tab plus the new-tab button.
fn tab_strip(store: Store) -> View {
    let tabs = computed(move || store.strip.get().tabs.clone());
    let active_id = computed(move || {
        let strip = store.strip.get();
        strip.tabs.get(strip.active).map(|tab| tab.id).unwrap_or(0)
    });
    let drag = TabDrag {
        chips: signal(Vec::new()),
        rects: signal(Vec::new()),
    };
    view()
        .flex_row()
        .items_center()
        .gap_1()
        .h(px(38.0))
        .border_l_1()
        .border_color(tokens::shell_border)
        .pl_1()
        .min_w(px(0.0))
        .child(
            scroll()
                .horizontal()
                .scrollbar(false)
                .window_drag(false)
                .h_full()
                .test_id("tab-strip")
                .w(move || px(8.0 + 92.0 * tabs.with(Vec::len) as f32))
                .flex_shrink(1.0)
                .min_w(px(0.0))
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .h_full()
                        .gap_1()
                        .px_1()
                        .child(each(
                            tabs,
                            |tab: &Tab| tab.id,
                            move |tab, _index| tab_chip(store, tab, active_id, drag.clone()),
                        )),
                ),
        )
        .child(
            chrome_button(|| Icon::Plus, || "新建标签页".into(), false)
                .on_press(move || store.open_new_tab()),
        )
}

/// The tab reorder's shared state (docs/ui-design.md §3.1 拖拽重排):
/// one element ref per mounted chip, for the release hit-test measured
/// when a drag starts.
#[derive(Clone)]
struct TabDrag {
    chips: Signal<Vec<(u64, ElementRef)>>,
    rects: Signal<Vec<(u64, Bounds<Points>)>>,
}

impl TabDrag {
    /// Remeasures every mounted chip; unmounted chips (closed since the
    /// last drag) drop out of the hit-test, and the pruned registry is
    /// kept so it stays the size of the strip.
    fn measure_chips(&self) {
        let chips = self.chips;
        let rects = self.rects;
        rects.set(Vec::new());
        spawn(async move {
            let mut measured = Vec::with_capacity(chips.with(Vec::len));
            let mut alive = Vec::with_capacity(chips.with(Vec::len));
            for (id, reference) in chips.get() {
                if let Some(measurement) = reference.measure().await {
                    alive.push((id, reference));
                    measured.push((
                        id,
                        Bounds {
                            origin: measurement.page,
                            size: measurement.frame.size,
                        },
                    ));
                }
            }
            chips.set(alive);
            rects.set(measured);
        });
    }
}

/// The chip under `at` that isn't the dragged one: the release
/// hit-test, against the frames measured at the drag's start.
fn chip_at(rects: &[(u64, Bounds<Points>)], dragged: u64, at: Point<Points>) -> Option<u64> {
    rects
        .iter()
        .find(|(id, bounds)| *id != dragged && bounds.contains(&at))
        .map(|(id, _)| *id)
}

fn tab_chip(
    store: Store,
    tab: ReadSignal<Tab>,
    active_id: Memo<u64>,
    drag: TabDrag,
) -> impl IntoElement {
    let id = move || tab.with(|tab| tab.id);
    let is_active = move || id() == active_id.get();
    let pinned = move || tab.with(|tab| tab.pinned);
    let title = move || store.route_title(&tab.with(|tab| tab.current.clone()));
    // The drag: a horizontal pan follows the finger with the chip (the
    // strip is icon-only, so an x-only pan value keeps the drag
    // axis-specific natively and leaves vertical drags to presses); the
    // touch-down point plus the pan's translation gives the release
    // point, hit-tested against the chips' measured frames; a release
    // that lands on no chip, or a cancelled pan, springs back without
    // touching the order. A plain press still activates (the pan claims
    // the gesture only past the drag slop).
    let chip_id = tab.with(|tab| tab.id);
    let chip_ref = element_ref();
    drag.chips.update(|chips| chips.push((chip_id, chip_ref)));
    let drag_x = animated_value(0.0);
    let down = signal(None::<Point<Points>>);
    view()
        .test_id(&format!("tab-chip-{chip_id}"))
        .element_ref(&chip_ref)
        .bind_translate_x(&drag_x)
        .pan_values(Some(&drag_x), None)
        .on_touch(move |touch: &TouchEvent| {
            if matches!(touch.phase, TouchPhase::Start)
                && let Some(point) = touch.points.first()
            {
                down.set(Some(point.page));
            }
        })
        .on_pan_start({
            let drag = drag.clone();
            move |_: &PanEvent| drag.measure_chips()
        })
        .on_pan_end(move |event: &PanEvent| {
            let released = down
                .get()
                .map(|origin| origin + event.translation)
                .filter(|_| !event.cancelled);
            down.set(None);
            if let Some(target) = released.and_then(|at| {
                drag.rects
                    .with_untracked(|rects| chip_at(rects, chip_id, at))
            }) {
                store.strip.update(|strip| {
                    strip.reorder(chip_id, target);
                });
                drag_x.set(0.0);
            } else if reduce_motion() {
                drag_x.set(0.0);
            } else {
                drag_x.spring(0.0, Spring::default());
            }
        })
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .px_1()
        .h(px(32.0))
        .flex_shrink(0.0)
        .window_drag(false)
        .rounded(tokens::radius_row())
        .style_when(is_active, |s| s.bg(tokens::shell_surface_active))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(
            chrome_button(|| Icon::Pin, || "固定标签页".into(), true)
                .selected(pinned)
                .selected_style(|s| s.text_color(tokens::accent_ink))
                .on_press(move || store.toggle_pin(id())),
        )
        .child(
            chrome_button(
                move || tab.with(|tab| tab.current.icon()),
                move || format!("标签页 {}", title()).into(),
                false,
            )
            .h(px(28.0))
            .selected(is_active)
            .selected_style(|s| s.text_color(tokens::accent_ink))
            .on_press(move || store.activate_tab_by_id(id())),
        )
        .child(
            chrome_button(|| Icon::Close, || "关闭标签页".into(), true)
                .disabled(pinned)
                .on_press(move || store.close_tab(id())),
        )
}

/// A compact row for the overview lists: circle, title, meta on the
/// right. Reused by the right context panel.
pub fn mini_task_row(
    store: Store,
    row: ReadSignal<crate::domain::TaskAggregate>,
) -> impl IntoElement {
    let id = row.with(|row| row.task.id);
    let overdue = move || row.with(|row| row.task.is_overdue_at(store.now()));
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .px_2()
        .py_1p5()
        .rounded(tokens::radius_row())
        .hover(|s| s.bg(tokens::control_bg()))
        .child(task_circle(
            move || row.with(|row| row.task.status == crate::domain::TaskStatus::Done),
            move || Some(format!("完成 {}", row.with(|row| row.task.title.clone())).into()),
            move || store.toggle_circle(id),
        ))
        .child(
            nsh_base::button()
                .on_press(move || store.open_task(id))
                .accessibility_label(move || {
                    Some(format!("打开任务 {}", row.with(|row| row.task.title.clone())).into())
                })
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
                        .child(
                            text(move || row.with(|row| row.task.title.clone()))
                                .text_size(px(13.0))
                                .text_color(tokens::fg_primary())
                                .number_of_lines(1),
                        )
                        .child(when(overdue, crate::ui::controls::overdue_badge)),
                ),
        )
}
