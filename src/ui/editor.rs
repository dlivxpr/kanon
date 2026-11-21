//! Styled pieces of the document formatting bar and writing paper.
use nsh_base::rich_editor::view::{
    PartStyles, RichEditorClasses, RichEditorElement, RichEditorStyles, rich_editor,
};
use nsh_core::prelude::*;
use nsh_core::{LineHeight, PropertyValue, StyleKey, element::Style};

use super::{
    chrome::{Icon, hint, icon},
    theme as tokens,
};

pub(crate) fn tool_face(positive: bool) -> View {
    let face = view()
        .h(px(30.0))
        .min_w(px(30.0))
        .px_1()
        .flex_none()
        .flex_row()
        .gap_1()
        .items_center()
        .justify_center()
        .rounded(tokens::radius_row())
        .text_color(tokens::fg_body)
        .hover(|s| s.bg(tokens::control_hover).text_color(tokens::fg_primary))
        .active(|s| s.bg(tokens::control_bg))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
        .disabled_style(|s| s.opacity(0.4))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS));
    if positive {
        face.w(px(30.0))
            .rounded_full()
            .bg(tokens::positive)
            .text_color(tokens::positive_foreground)
            .hover(|s| s.opacity(0.88))
            .active(|s| s.opacity(0.75))
    } else {
        face
    }
}

pub(crate) fn format_button(
    label: &'static str,
    glyph: Icon,
    selected: impl Fn() -> bool + 'static,
    disabled: impl Fn() -> bool + 'static,
    press: impl Fn() + 'static,
) -> impl IntoElement {
    nsh_base::button()
        .h(px(30.0))
        .w(px(30.0))
        .flex_none()
        .accessibility_label(label)
        .disabled(disabled)
        .selected(selected)
        .items_center()
        .justify_center()
        .rounded(tokens::radius_row())
        .text_color(tokens::fg_body)
        .selected_style(|s| s.bg(tokens::control_bg).text_color(tokens::fg_primary))
        .hover(|s| s.bg(tokens::control_hover).text_color(tokens::fg_primary))
        .active(|s| s.opacity(0.74))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
        .disabled_style(|s| s.opacity(0.4))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .on_press(press)
        .child(icon(move || glyph, px(16.0)))
        .child(hint(label))
}

pub(crate) fn format_menu(
    trigger: View,
    items: impl IntoIterator<Item = AnyElement>,
) -> impl IntoElement {
    nsh_base::menu()
        .flex_none()
        .offset(px(4.0))
        .popup_style(|s| {
            s.w(px(196.0))
                .p_1()
                .rounded(tokens::radius_card())
                .bg(tokens::panel_raised)
                .border_1()
                .border_color(tokens::border_default)
                .shadow_md()
                .transition_all()
                .duration(tokens::ms(tokens::MOTION_FAST_MS))
                .ease_out()
        })
        .popup_starting_style(|s| s.opacity(if reduce_motion() { 1.0 } else { 0.0 }))
        .popup_ending_style(|s| s.opacity(if reduce_motion() { 1.0 } else { 0.0 }))
        .trigger(trigger)
        .children(items)
}

pub(crate) fn format_item(
    label: &'static str,
    glyph: Icon,
    disabled: impl Fn() -> bool + 'static,
    select: impl Fn() + 'static,
) -> AnyElement {
    nsh_base::menu_item()
        .accessibility_label(label)
        .disabled(disabled)
        .on_select(select)
        .min_h(px(32.0))
        .px_2()
        .gap_2()
        .flex_row()
        .items_center()
        .text_color(tokens::fg_body)
        .rounded(tokens::radius_row())
        .hover(|s| s.bg(tokens::control_hover))
        .focus(|s| s.bg(tokens::control_hover))
        .active(|s| s.bg(tokens::control_bg))
        .disabled_style(|s| s.opacity(0.4))
        .child(icon(move || glyph, px(15.0)))
        .child(text(label).text_size(tokens::text_px(13.0)))
        .into_element()
}

pub(crate) fn insert_popover(
    field: ElementRef,
    open: Signal<bool>,
    composing: Signal<bool>,
) -> nsh_base::PopoverElement {
    nsh_base::popover()
        .open(move || open.get())
        .flex_none()
        .align(Align::Start)
        .offset(px(5.0))
        .on_open(move || field.focus())
        .popup_style(|s| {
            insert_surface(s)
                .transition_all()
                .duration(tokens::ms(tokens::MOTION_FAST_MS))
                .ease_out()
        })
        .popup_starting_style(|s| s.opacity(if reduce_motion() { 1.0 } else { 0.0 }))
        .popup_ending_style(|s| s.opacity(if reduce_motion() { 1.0 } else { 0.0 }))
        .trigger(
            tool_face(true)
                .label("插入卡片")
                .disabled(composing)
                .child(icon(|| Icon::Plus, px(17.0)))
                .child(hint("插入卡片")),
        )
}

fn insert_surface<S: Styled>(surface: S) -> S {
    surface
        .w(px(296.0))
        .p_2()
        .rounded(tokens::radius_card())
        .border_1()
        .border_color(tokens::border_default)
        .bg(tokens::panel_raised)
        .shadow_md()
}

pub(crate) fn insert_search(
    field: ElementRef,
    query: Signal<String>,
    selected: Signal<usize>,
    accept: impl Fn() + 'static,
) -> impl IntoElement {
    super::controls::input_field(
        "搜索插入内容",
        move || query.get(),
        move |value| {
            selected.set(0);
            query.set(value.to_owned());
        },
    )
    .element_ref(&field)
    .label("搜索插入内容")
    .flex_none()
    .min_h(tokens::control_h())
    .w_full()
    .mb_2()
    .on_submit(move |_: &str| accept())
}

pub(crate) fn insert_catalog(groups: impl IntoIterator<Item = View>) -> impl IntoElement {
    view().gap_2().children(groups)
}

pub(crate) fn insert_group(label: &str, rows: Vec<AnyElement>) -> View {
    view()
        .gap_1()
        .child(
            text(label.to_owned())
                .px_2()
                .text_size(tokens::text_px(12.0))
                .text_color(tokens::fg_muted),
        )
        .child(insert_grid(rows))
}

pub(crate) fn insert_grid(mut rows: Vec<AnyElement>) -> View {
    let lines = std::iter::from_fn(move || {
        if rows.is_empty() {
            None
        } else {
            let count = rows.len().min(2);
            let line = view().flex_row().gap_1().children(rows.drain(..count));
            Some(line)
        }
    });
    view().gap_0p5().children(lines)
}

pub(crate) fn insert_row(
    label: impl Into<SharedString>,
    glyph: Icon,
    focusable: bool,
    selected: impl Fn() -> bool + Copy + 'static,
    accept: impl Fn() + 'static,
) -> AnyElement {
    let label = label.into();
    view()
        .role(AccessibilityRole::Button)
        .label(label.clone())
        // The slash picker uses arrows from the editor; mouse clicks must
        // not blur it and unmount the row before release.
        .focusable(focusable)
        .w(px(136.0))
        .h(px(32.0))
        .px_2()
        .flex_row()
        .gap_2()
        .items_center()
        .rounded(tokens::radius_row())
        .accessibility_state(move || AccessibilityState {
            selected: selected(),
            ..Default::default()
        })
        .style_when(selected, |s| {
            s.bg(tokens::control_bg).text_color(tokens::fg_primary)
        })
        .hover(|s| s.bg(tokens::control_hover))
        .active(|s| s.opacity(0.74))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .on_press(accept)
        .child(icon(move || glyph, px(15.0)).flex_none())
        .child(
            text(label)
                .number_of_lines(1)
                .text_size(tokens::text_px(13.0)),
        )
        .into_element()
}

pub(crate) fn insert_empty() -> impl IntoElement {
    text("没有匹配的插入项")
        .p_2()
        .text_size(tokens::text_px(13.0))
        .text_color(tokens::fg_muted)
}

pub(crate) fn slash_popup(rows: impl IntoElement) -> impl IntoElement {
    insert_surface(view())
        .test_id("document-slash-menu")
        .child(rows)
}

pub(crate) fn divider() -> View {
    view()
        .w(px(1.0))
        .h_4()
        .mx_1()
        .flex_none()
        .bg(tokens::border_subtle)
}

pub(crate) const PAPER_TITLE_SIZE: f32 = 32.0;

/// Reading and writing use the same typography. NSH supplies the document
/// structure but leaves the visible treatment of its nodes and marks to us.
pub(crate) fn paper_body() -> RichEditorElement {
    let mut classes = RichEditorClasses::default();
    classes.marks = Some(
        [
            ("bold", "font-bold"),
            ("italic", "italic"),
            ("underline", "underline"),
            ("strike", "line-through"),
            ("code", "font-mono bg-muted"),
            ("link", "text-accent-ink underline"),
        ]
        .map(|(name, class)| (name.into(), class.into()))
        .into(),
    );
    classes.nodes = Some(
        [
            (
                "heading",
                "font-semibold leading-[1.35] mt-3 mb-1 text-(--fg-primary)",
            ),
            ("blockquote", "my-2 pl-3 border-l-2 border-border"),
            ("codeBlock", "font-mono my-2 p-3 rounded-md bg-muted"),
            ("horizontalRule", "h-px my-4 bg-border"),
            ("bulletList", "pl-6"),
            ("orderedList", "pl-6"),
        ]
        .map(|(name, class)| (name.into(), class.into()))
        .into(),
    );
    classes.task_checkbox = Some(
        "size-4 mt-1.5 mr-2 flex-none rounded-sm border border-(--fg-muted) checked:border-4 checked:border-(--positive) focus-within:outline-2 focus-within:outline-(--accent-ink)".into(),
    );
    rich_editor()
        .classes(classes)
        .styles(paper_body_styles)
        .text_size(tokens::text_px(15.0))
        .line_height(LineHeight::Multiple(1.78))
        .text_color(tokens::fg_body)
}

fn paper_body_styles() -> RichEditorStyles {
    let scale = tokens::text_scale();
    let nodes = [
        ("heading[level=1]", 26.0),
        ("heading[level=2]", 22.0),
        ("heading[level=3]", 18.0),
        ("heading[level=4]", 16.0),
    ];
    let mut styles = RichEditorStyles::default();
    styles.nodes = Some(PartStyles(
        nodes
            .into_iter()
            .map(|(name, size)| {
                // RichEditor's part merger uses declarations, not typed refinements.
                let style = Style::default().with(
                    StyleKey::FontSize,
                    PropertyValue::FontSize(Some(px(size * scale)), None),
                );
                (name.into(), style)
            })
            .collect(),
    ));
    styles
}

pub(crate) fn paper_title(
    value: impl Fn() -> String + 'static,
    on_change: impl Fn(&str) + 'static,
) -> nsh_base::Input {
    nsh_base::input()
        .value(value)
        .on_change(on_change)
        .placeholder("请输入标题")
        .label("文档标题")
        .placeholder_color(|| color(tokens::fg_muted()))
        .caret_color(|| color(tokens::positive()))
        .h(px(48.0))
        .flex_none()
        .min_w(px(0.0))
        .text_size(tokens::text_px(PAPER_TITLE_SIZE))
        .font_weight(FontWeight::BOLD)
        .text_color(tokens::fg_primary)
        .focus_visible(|s| s.outline(px(1.0)).outline_color(tokens::border_default))
}

pub(crate) fn template_card(
    title: &'static str,
    sections: &'static [&'static str],
    apply: impl Fn() + 'static,
) -> impl IntoElement {
    nsh_base::button()
        .accessibility_label(format!("套用模板 {title}"))
        .on_press(apply)
        .flex_1()
        .min_w(px(148.0))
        .min_h(px(76.0))
        .p_3()
        .gap_2()
        .items_start()
        .rounded(tokens::radius_card())
        .border_1()
        .border_color(tokens::border_default)
        .bg(tokens::panel_raised)
        .text_color(tokens::fg_body)
        .hover(|s| s.bg(tokens::control_hover))
        .active(|s| s.bg(tokens::control_bg))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(icon(|| Icon::Document, px(16.0)).text_color(tokens::positive))
                .child(
                    text(title)
                        .text_size(tokens::text_px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens::fg_primary),
                ),
        )
        .child(
            text(sections.join(" · "))
                .text_size(tokens::text_px(12.0))
                .text_color(tokens::fg_muted),
        )
}
