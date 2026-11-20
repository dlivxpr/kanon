//! Kanon's reusable styled control layer above `nsh-base`
//! (docs/ui-design.md §5.3). Every control draws from the semantic
//! tokens in `ui/theme.rs`, ships keyboard focus rings, and animates its
//! hover/press states with the motion tokens (honoring reduced motion).

use nsh_base::{button, theme};
use nsh_core::prelude::*;

use super::theme as tokens;

/// The visual variants of the app's button (docs/ui-design.md §5.3).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// The coral accent: the one primary action of a surface.
    Primary,
    /// Yuque green: positive actions (save, update).
    Positive,
    /// A quiet control on a panel background.
    Soft,
    /// No chrome until hovered; toolbars and tab chrome.
    Ghost,
    /// Destructive actions.
    Danger,
}

/// The app's button. One wrapper, every screen composes from it.
pub fn app_button(
    kind: ButtonKind,
    label: impl Into<SharedString> + Clone + 'static,
    on_press: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    button_base(kind, on_press)
        .accessibility_label(Some(label.clone().into()))
        .child(text(label.clone().into()).text_size(px(13.0)).text_color(move || fg_of(kind)))
}

/// The same button with a label that follows a signal (counts, states).
pub fn app_button_with(
    kind: ButtonKind,
    label: impl Fn() -> SharedString + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    button_base(kind, on_press)
        .accessibility_label(move || Some(label()))
        .child(text(label).text_size(px(13.0)).text_color(move || fg_of(kind)))
}

fn fg_of(kind: ButtonKind) -> nsh_core::element::AdaptiveColor {
    match kind {
        ButtonKind::Primary | ButtonKind::Positive | ButtonKind::Danger => {
            theme().primary_foreground
        }
        ButtonKind::Soft | ButtonKind::Ghost => tokens::fg_primary(),
    }
}

/// The shared chrome of the app's button (no label yet).
fn button_base(kind: ButtonKind, on_press: impl Fn() + 'static) -> nsh_base::ButtonElement {
    let (bg, hover_bg) = match kind {
        ButtonKind::Primary => (tokens::accent(), tokens::accent()),
        ButtonKind::Positive => (tokens::positive(), tokens::positive()),
        ButtonKind::Soft => (tokens::control_bg(), tokens::control_hover()),
        ButtonKind::Ghost => (theme().background.opacity(0.0), tokens::control_hover()),
        ButtonKind::Danger => (tokens::danger(), tokens::danger()),
    };
    button()
        .on_press(on_press)
        .h(px(32.0))
        .px_4()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(bg)
        .border_1()
        .border_color(theme().background.opacity(0.0))
        .hover(|s| s.bg(hover_bg))
        .active(|s| s.opacity(0.85))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .disabled_style(|s| s.opacity(0.5))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
}

/// A 28 px square icon button for toolbars and tab chrome.
pub fn icon_button(
    glyph: impl IntoReactive<SharedString>,
    label: &'static str,
    on_press: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    button()
        .on_press(on_press)
        .accessibility_label(Some(label.into()))
        .size(px(28.0))
        .items_center()
        .justify_center()
        .rounded(move || theme().radius)
        .bg(move || theme().background.opacity(0.0))
        .text_color(tokens::shell_muted())
        .hover(|s| s.bg(tokens::shell_surface_active()).text_color(tokens::shell_fg()))
        .active(|s| s.opacity(0.7))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(text(glyph).text_size(px(14.0)))
}

/// A quiet icon button that lives on a panel (not the dark shell).
pub fn panel_icon_button(
    glyph: impl IntoReactive<SharedString>,
    label: &'static str,
    on_press: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    button()
        .on_press(on_press)
        .accessibility_label(Some(label.into()))
        .size(px(26.0))
        .items_center()
        .justify_center()
        .rounded(move || theme().radius)
        .bg(move || theme().background.opacity(0.0))
        .text_color(tokens::fg_muted())
        .hover(|s| s.bg(tokens::control_hover()).text_color(tokens::fg_primary()))
        .active(|s| s.opacity(0.7))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(text(glyph).text_size(px(13.0)))
}

/// The app's text field: 32 px pill on a control background.
pub fn input_field(
    placeholder: &'static str,
    value: impl Fn() -> String + 'static,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    nsh_base::input()
        .value(value)
        .on_change(on_change)
        .placeholder(placeholder)
        .placeholder_color(|| color(theme().muted_foreground))
        .caret_color(|| color(theme().primary))
        .h(px(32.0))
        .px_3()
        .text_size(px(13.0))
        .rounded(move || theme().radius)
        .bg(tokens::control_bg())
        .border_1()
        .border_color(move || theme().background.opacity(0.0))
        .focus(|s| s.border_color(tokens::accent()))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .flex_1()
        .min_w(px(0.0))
}

/// The same field plus an Enter-to-commit handler (quick capture boxes).
pub fn submit_field(
    placeholder: &'static str,
    value: impl Fn() -> String + 'static,
    on_change: impl Fn(&str) + 'static,
    on_submit: impl Fn(&str) + 'static,
) -> impl IntoElement {
    nsh_base::input()
        .value(value)
        .on_change(on_change)
        .on_submit(on_submit)
        .placeholder(placeholder)
        .placeholder_color(|| color(theme().muted_foreground))
        .caret_color(|| color(theme().primary))
        .h(px(32.0))
        .px_3()
        .text_size(px(13.0))
        .rounded(move || theme().radius)
        .bg(tokens::control_bg())
        .border_1()
        .border_color(move || theme().background.opacity(0.0))
        .focus(|s| s.border_color(tokens::accent()))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .flex_1()
        .min_w(px(0.0))
}

/// A rounded workbench panel (main canvas, right context panel).
pub fn panel() -> View {
    view().flex_1().rounded(tokens::radius_panel()).bg(tokens::panel_bg()).min_w(px(0.0)).min_h(px(0.0))
}

/// A small label chip (a Task's Tag, `设计`).
pub fn chip(label: impl IntoReactive<SharedString>) -> impl IntoElement {
    view()
        .px_2()
        .py(px(1.0))
        .rounded(px(6.0))
        .bg(tokens::info_soft())
        .child(text(label).text_size(px(11.0)).text_color(tokens::info()))
}

/// A red overdue badge (`逾期`).
pub fn overdue_badge() -> impl IntoElement {
    view()
        .px_1p5()
        .py(px(1.0))
        .rounded(px(6.0))
        .bg(tokens::accent_soft())
        .child(text("逾期").text_size(px(11.0)).text_color(tokens::accent()))
}

/// The task circle: a quick checkbox that toggles `done ↔ todo`.
///
/// `checked` is the binding; `label` names the control for keyboard and
/// screen-reader users (and the headless tests).
pub fn task_circle(
    checked: impl Fn() -> bool + Copy + 'static,
    label: impl Into<SharedString>,
    on_toggle: impl Fn() + 'static,
) -> nsh_base::CheckboxElement {
    nsh_base::checkbox()
        .checked(checked)
        .on_change(move |_: &bool| on_toggle())
        .accessibility_label(Some(label.into()))
        .size(px(20.0))
        .items_center()
        .justify_center()
        .rounded_full()
        .border_2()
        .border_color(move || if checked() { tokens::positive() } else { tokens::border_default() })
        .bg(move || {
            if checked() { tokens::positive() } else { theme().background.opacity(0.0) }
        })
        .indicator_style(|s| s.size(px(16.0)).items_center().justify_center())
        .indicator(text(move || if checked() { "✓" } else { "" }).text_size(px(12.0)).text_color(theme().primary_foreground))
        .active(|s| s.scale(0.9))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_FAST_MS))
}

/// An unstyled-by-design row in a grouped task list.
pub fn list_row() -> View {
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .px_3()
        .py_2()
        .min_h(px(50.0))
        .rounded(tokens::radius_row())
        .hover(|s| s.bg(tokens::control_bg()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
}

/// A section heading inside a panel (`今天`, `收件箱`).
pub fn section_heading(label: impl IntoReactive<SharedString>) -> impl IntoElement {
    text(label)
        .text_size(px(13.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(tokens::fg_muted())
}

/// The centered empty state of an unused surface or unreleased view.
pub fn empty_state(title: impl IntoReactive<SharedString>, hint: impl IntoReactive<SharedString>) -> impl IntoElement {
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_2()
        .child(text(title).text_size(px(16.0)).font_weight(FontWeight::SEMIBOLD).text_color(tokens::fg_muted()))
        .child(text(hint).text_size(px(13.0)).text_color(tokens::fg_muted()))
}
