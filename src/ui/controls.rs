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
        .child(
            text(label.clone().into())
                .text_size(tokens::text_px(13.0))
                .text_color(move || fg_of(kind)),
        )
}

/// The same button with a label that follows a signal (counts, states).
pub fn app_button_with(
    kind: ButtonKind,
    label: impl Fn() -> SharedString + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    button_base(kind, on_press)
        .accessibility_label(move || Some(label()))
        .child(
            text(label)
                .text_size(tokens::text_px(13.0))
                .text_color(move || fg_of(kind)),
        )
}

fn fg_of(kind: ButtonKind) -> nsh_core::element::AdaptiveColor {
    match kind {
        ButtonKind::Positive => tokens::positive_foreground(),
        ButtonKind::Primary | ButtonKind::Danger => theme().primary_foreground,
        ButtonKind::Soft | ButtonKind::Ghost => tokens::fg_primary(),
    }
}

fn bg_of(kind: ButtonKind) -> nsh_core::element::AdaptiveColor {
    match kind {
        ButtonKind::Primary => tokens::accent(),
        ButtonKind::Positive => tokens::positive(),
        ButtonKind::Soft => tokens::control_bg(),
        ButtonKind::Ghost => theme().background.opacity(0.0),
        ButtonKind::Danger => tokens::danger(),
    }
}

/// The hovered background of the app's button, by variant.
fn hover_bg_of(kind: ButtonKind) -> nsh_core::element::AdaptiveColor {
    match kind {
        ButtonKind::Primary => tokens::accent(),
        ButtonKind::Positive => tokens::positive(),
        ButtonKind::Soft => tokens::control_hover(),
        ButtonKind::Ghost => tokens::control_hover(),
        ButtonKind::Danger => tokens::danger(),
    }
}

/// The shared chrome of the app's button (no label yet).
fn button_base(kind: ButtonKind, on_press: impl Fn() + 'static) -> nsh_base::ButtonElement {
    button()
        .on_press(on_press)
        .h(tokens::control_h())
        .px_4()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(move || bg_of(kind))
        .border_1()
        .border_color(theme().background.opacity(0.0))
        .hover(move |s| s.bg(hover_bg_of(kind)))
        // Press feedback per docs/ui-design.md §6.2: scale to 0.97 on
        // the fast token (reduced motion: instant, wash only).
        .active(|s| {
            s.opacity(0.85)
                .scale(if reduce_motion() { 1.0 } else { 0.97 })
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .disabled_style(|s| s.opacity(0.5))
        .transition_colors()
        .transition_transform()
        .duration(tokens::ms(tokens::MOTION_FAST_MS))
}

/// A themed modal card; NSH owns focus and dismissal.
pub(crate) fn dialog_card(
    title: impl Into<SharedString>,
    description: &'static str,
    role: AccessibilityRole,
    on_cancel: impl Fn() + 'static,
    content: impl IntoElement,
) -> impl IntoElement {
    let title = title.into();
    nsh_base::dialog()
        .role(role)
        .label(title.clone())
        .accessibility_description(description)
        .animation(if reduce_motion() {
            ModalAnimation::None
        } else {
            ModalAnimation::Fade
        })
        .on_cancel(on_cancel)
        .flex_1()
        .p_4()
        .items_center()
        .justify_center()
        .bg(tokens::overlay)
        .child(
            view()
                .w_full()
                .max_w(px(420.0))
                .gap_3()
                .p_5()
                .rounded(tokens::radius_panel())
                .border_1()
                .border_color(tokens::border_default)
                .bg(tokens::panel_raised)
                .shadow_lg()
                .child(
                    text(title)
                        .role(AccessibilityRole::Header)
                        .text_size(tokens::text_px(20.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens::fg_primary),
                )
                .child(
                    text(description)
                        .text_size(tokens::text_px(14.0))
                        .text_color(tokens::fg_muted),
                )
                .child(content),
        )
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
        .text_color(tokens::fg_muted)
        .hover(|s| {
            s.bg(tokens::control_hover())
                .text_color(tokens::fg_primary())
        })
        .active(|s| {
            s.opacity(0.7)
                .scale(if reduce_motion() { 1.0 } else { 0.95 })
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .transition_transform()
        .duration(tokens::ms(tokens::MOTION_FAST_MS))
        .child(text(glyph).text_size(tokens::text_px(13.0)))
}

/// The app's text field: 32 px pill on a control background.
pub fn input_field(
    placeholder: &'static str,
    value: impl Fn() -> String + 'static,
    on_change: impl Fn(&str) + 'static,
) -> nsh_base::Input {
    nsh_base::input()
        .value(value)
        .on_change(on_change)
        .placeholder(placeholder)
        .placeholder_color(|| color(theme().muted_foreground))
        .caret_color(|| color(theme().primary))
        .h(tokens::control_h())
        .px_3()
        .text_size(tokens::text_px(13.0))
        .rounded(move || theme().radius)
        .bg(tokens::control_bg)
        .border_1()
        .border_color(move || theme().background.opacity(0.0))
        .focus(|s| s.border_color(tokens::accent()))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .flex_1()
        .min_w(px(0.0))
}

/// The same field plus an Enter-to-commit handler (quick capture boxes
/// and the Knowledge Base creation rows, where Enter commits).
pub fn submit_field(
    placeholder: impl Into<String> + Clone + 'static,
    value: impl Fn() -> String + 'static,
    on_change: impl Fn(&str) + 'static,
    on_submit: impl Fn(&str) + 'static,
) -> impl IntoElement {
    let label: String = placeholder.into();
    let placeholder_text = label.clone();
    nsh_base::input()
        .value(value)
        .on_change(on_change)
        .on_submit(on_submit)
        .placeholder(move || placeholder_text.clone())
        .label(move || label.clone())
        .placeholder_color(|| color(theme().muted_foreground))
        .caret_color(|| color(theme().primary))
        .h(tokens::control_h())
        .px_3()
        .text_size(tokens::text_px(13.0))
        .rounded(move || theme().radius)
        .bg(tokens::control_bg)
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
    view()
        .flex_1()
        .rounded(tokens::radius_panel())
        .border_1()
        .border_color(tokens::border_subtle)
        .overflow_hidden()
        .bg(tokens::panel_bg())
        .min_w(px(0.0))
        .min_h(px(0.0))
}

/// A small label chip (a Task's Tag, `设计`).
pub fn chip(label: impl IntoReactive<SharedString>) -> impl IntoElement {
    view()
        .px_2()
        .py(px(1.0))
        .rounded(px(6.0))
        .bg(tokens::info_soft())
        .child(
            text(label)
                .text_size(tokens::text_px(11.0))
                .text_color(tokens::info()),
        )
}

/// A red overdue badge (`逾期`).
pub fn overdue_badge() -> impl IntoElement {
    view()
        .px_1p5()
        .py(px(1.0))
        .rounded(px(6.0))
        .bg(tokens::accent_soft())
        .child(
            text("逾期")
                .text_size(tokens::text_px(11.0))
                .text_color(tokens::accent()),
        )
}

/// The task circle: a quick checkbox that toggles `done ↔ todo`.
///
/// `checked` is the binding; `label` names the control for keyboard and
/// screen-reader users (and the headless tests).
pub fn task_circle<M>(
    checked: impl Fn() -> bool + Copy + 'static,
    label: impl nsh_core::element::IntoProp<Option<SharedString>, M>,
    on_toggle: impl Fn() + 'static,
) -> nsh_base::CheckboxElement {
    nsh_base::checkbox()
        .checked(checked)
        .on_change(move |_: &bool| on_toggle())
        .accessibility_label(label)
        .size(px(20.0))
        .items_center()
        .justify_center()
        .rounded_full()
        .border_2()
        .border_color(move || {
            if checked() {
                tokens::positive()
            } else {
                tokens::border_default()
            }
        })
        .bg(move || {
            if checked() {
                tokens::positive()
            } else {
                theme().background.opacity(0.0)
            }
        })
        .indicator_style(|s| s.size(px(16.0)).items_center().justify_center())
        .indicator(
            text(move || if checked() { "✓" } else { "" })
                .text_size(tokens::text_px(12.0))
                .text_color(tokens::positive_foreground),
        )
        .active(|s| {
            s.opacity(0.8)
                .scale(if reduce_motion() { 1.0 } else { 0.9 })
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .transition_transform()
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
        .min_h(tokens::row_min_h())
        .rounded(tokens::radius_row())
        .hover(|s| s.bg(tokens::control_bg()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
}

/// A section heading inside a panel (`今天`, `收件箱`).
pub fn section_heading(label: impl IntoReactive<SharedString>) -> impl IntoElement {
    text(label)
        .text_size(tokens::text_px(13.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(tokens::fg_muted())
}

/// The centered empty state of an unused surface or unreleased view.
pub fn empty_state(
    title: impl IntoReactive<SharedString>,
    hint: impl IntoReactive<SharedString>,
) -> impl IntoElement {
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_2()
        .child(
            text(title)
                .text_size(tokens::text_px(16.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens::fg_muted()),
        )
        .child(
            text(hint)
                .text_size(tokens::text_px(13.0))
                .text_color(tokens::fg_muted()),
        )
}
