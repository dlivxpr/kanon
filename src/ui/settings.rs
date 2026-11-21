//! Styled preference controls for the desktop settings surface.
use nsh_core::prelude::*;

use super::chrome::{Icon, icon};
use super::theme as tokens;

pub(crate) fn setting_select(
    label: &'static str,
    items: &'static [(&'static str, &'static str)],
    current: impl Fn() -> SharedString + Copy + 'static,
    change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    nsh_base::select()
        .items(
            items
                .iter()
                .map(|&(value, title)| nsh_base::SelectItem::new(value, title))
                .collect::<Vec<_>>(),
        )
        .accessibility_label(label)
        .value(move || Some(current()))
        .on_change(move |next: &Option<SharedString>| {
            if let Some(value) = next {
                change(value.as_str());
            }
        })
        .offset(px(4.0))
        .popup_style(|s| {
            s.p_1()
                .rounded(tokens::radius_card())
                .border_1()
                .border_color(tokens::border_default)
                .bg(tokens::panel_raised)
                .shadow_md()
                .transition_all()
                .duration(tokens::ms(tokens::MOTION_FAST_MS))
                .ease_out()
        })
        .popup_starting_style(|s| s.opacity(if reduce_motion() { 1.0 } else { 0.0 }))
        .popup_ending_style(|s| s.opacity(if reduce_motion() { 1.0 } else { 0.0 }))
        .item_class("rounded-md focus:bg-secondary selected:bg-secondary active:bg-accent-soft")
        .render_item(|entry: &nsh_base::SelectEntry| {
            text(entry.title.clone())
                .text_size(tokens::text_px(13.0))
                .text_color(tokens::fg_primary)
                .px_3()
                .py_2()
        })
        .trigger(
            view()
                .h(tokens::control_h())
                .px_3()
                .gap_2()
                .flex_row()
                .items_center()
                .justify_between()
                .rounded(tokens::radius_row())
                .border_1()
                .border_color(tokens::border_default)
                .bg(tokens::panel_raised)
                .text_color(tokens::fg_primary)
                .hover(|s| s.bg(tokens::control_hover))
                .active(|s| s.bg(tokens::control_bg))
                .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
                .transition_colors()
                .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
                .child(
                    text(move || {
                        let value = current();
                        items
                            .iter()
                            .find(|&&(key, _)| value.as_str() == key)
                            .map_or("", |&(_, title)| title)
                    })
                    .text_size(tokens::text_px(13.0)),
                )
                .child(icon(|| Icon::Down, px(14.0))),
        )
}

pub(crate) fn setting_switch(
    label: SharedString,
    checked: impl Fn() -> bool + 'static,
    disabled: impl Fn() -> bool + 'static,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    nsh_base::switch()
        .accessibility_label(label)
        .checked(checked)
        .disabled(disabled)
        .disabled_style(|s| s.opacity(0.55))
        .on_change(move |checked: &bool| on_change(*checked))
        .h(tokens::control_h())
        .min_w(px(38.0))
        .justify_center()
        .rounded_full()
        .active(|s| s.opacity(0.78))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
        .track_style(|s| {
            s.w(px(38.0))
                .h_5()
                .p_0p5()
                .justify_center()
                .rounded_full()
                .bg(tokens::control_bg)
                .border_1()
                .border_color(tokens::border_default)
                .checked(|s| s.bg(tokens::accent_ink).border_color(tokens::accent_ink))
                .transition_colors()
                .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        })
        .thumb_style(|s| {
            s.size(px(14.0))
                .rounded_full()
                .bg(tokens::fg_muted)
                .checked(|s| s.translate_x(px(18.0)).bg(tokens::panel_raised))
                .transition_transform()
                .duration(tokens::ms(tokens::MOTION_FAST_MS))
                .ease_out()
        })
}
