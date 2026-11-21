//! The window and workspace navigation icon family.

use nsh_base::{ButtonElement, button, theme, tooltip};
use nsh_core::{
    PointerEvents,
    element::{ImageSrc, Svg},
    prelude::*,
};

use super::theme as tokens;

#[derive(Clone, Copy)]
pub enum Icon {
    Sidebar,
    Back,
    Forward,
    Local,
    Plus,
    Inbox,
    Panel,
    Pin,
    Close,
    Search,
    Start,
    Sun,
    Clock,
    Tasks,
    Calendar,
    Note,
    Star,
    Tag,
    Books,
    Book,
    Document,
    Project,
    Trash,
    Settings,
    Brand,
    Down,
    Outline,
    Links,
    Undo,
    Redo,
    Heading,
    Bold,
    Italic,
    Underline,
    Strike,
    Code,
    CodeBlock,
    Bullets,
    Ordered,
    Quote,
    Table,
    Divider,
    ClearFormat,
    More,
}

impl Icon {
    fn markup(self) -> &'static str {
        macro_rules! svg {
            ($shapes:literal) => { concat!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">"#, $shapes, "</svg>") };
        }
        match self {
            Self::Sidebar => {
                svg!(r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/>"#)
            }
            Self::Back => svg!(r#"<path d="m15 18-6-6 6-6"/>"#),
            Self::Forward => svg!(r#"<path d="m9 18 6-6-6-6"/>"#),
            Self::Down => svg!(r#"<path d="m6 9 6 6 6-6"/>"#),
            Self::Outline => svg!(r#"<path d="M5 6h14M5 12h10M5 18h6"/>"#),
            Self::Links => svg!(
                r#"<path d="M10 13a5 5 0 0 0 7.5.5l2.5-2.5a5 5 0 0 0-7-7L11.5 5.5M14 11a5 5 0 0 0-7.5-.5L4 13a5 5 0 0 0 7 7l1.5-1.5"/>"#
            ),
            Self::Brand => {
                svg!(r#"<path d="M5 4v16m3-7L19 4m-9 8 9 8M9 4v5" stroke-width="3.2"/>"#)
            }
            Self::Local => svg!(
                r#"<rect x="3" y="5" width="18" height="14" rx="3"/><path d="M3 13h18m-14 3h.01m4 0h.01"/>"#
            ),
            Self::Plus => svg!(r#"<path d="M12 5v14M5 12h14"/>"#),
            Self::Inbox => svg!(
                r#"<path d="M5 4h14l3 10v5a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1v-5Zm-3 10h6l2 3h4l2-3h6"/>"#
            ),
            Self::Panel => {
                svg!(r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M15 4v16"/>"#)
            }
            Self::Pin => svg!(r#"<path d="M15 4l5 5-3 1-2 4v3l-8-8h3l4-2ZM11 13l-7 7"/>"#),
            Self::Close => svg!(r#"<path d="m6 6 12 12M18 6 6 18"/>"#),
            Self::Search => svg!(r#"<circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 4 4"/>"#),
            Self::Start => svg!(
                r#"<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>"#
            ),
            Self::Sun => svg!(
                r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5"/>"#
            ),
            Self::Clock => svg!(r#"<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>"#),
            Self::Tasks => svg!(
                r#"<rect x="3" y="3" width="18" height="18" rx="3"/><path d="m7 12 3 3 7-7"/>"#
            ),
            Self::Calendar => svg!(
                r#"<rect x="3" y="5" width="18" height="16" rx="2"/><path d="M16 3v4M8 3v4M3 11h18"/>"#
            ),
            Self::Note => svg!(r#"<path d="M4 5h16M4 10h16M4 15h10M4 20h7"/>"#),
            Self::Star => svg!(
                r#"<path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-3-5.6 3 1.1-6.2L3 9.6l6.2-.9Z"/>"#
            ),
            Self::Tag => {
                svg!(r#"<path d="M3 4h8l10 10-7 7L3 10Z"/><circle cx="7.5" cy="8" r=".8"/>"#)
            }
            Self::Books => svg!(r#"<path d="M4 5h4v15H4zm4 0h5v15H8zm6 2 4-1 3 13-4 1Z"/>"#),
            Self::Book => svg!(
                r#"<path d="M6 3h13v18H6a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2Zm-2 15a2 2 0 0 1 2-2h13M8 7h7"/>"#
            ),
            Self::Document => svg!(r#"<path d="M5 3h9l5 5v13H5Zm9 0v6h5M8 13h8m-8 4h6"/>"#),
            Self::Project => svg!(
                r#"<path d="M3 6a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/>"#
            ),
            Self::Trash => svg!(r#"<path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13m-8 4v5m4-5v5"/>"#),
            Self::Settings => svg!(r#"<path d="M4 7h16M4 12h16M4 17h16M8 5v4m8 1v4m-6 1v4"/>"#),
            Self::Undo => svg!(r#"<path d="M9 6 4 11l5 5M4 11h10a5 5 0 0 1 5 5v2"/>"#),
            Self::Redo => svg!(r#"<path d="m15 6 5 5-5 5m5-5H10a5 5 0 0 0-5 5v2"/>"#),
            Self::Heading => svg!(r#"<path d="M5 5v14M19 5v14M5 12h14"/>"#),
            Self::Bold => {
                svg!(r#"<path d="M6 4h7a4 4 0 0 1 0 8H6V4Zm0 8h8a4 4 0 0 1 0 8H6v-8Z"/>"#)
            }
            Self::Italic => svg!(r#"<path d="M11 4h8M5 20h8m2-16L9 20"/>"#),
            Self::Underline => svg!(r#"<path d="M6 4v8a6 6 0 0 0 12 0V4M4 21h16"/>"#),
            Self::Strike => svg!(
                r#"<path d="M17 6a7 7 0 0 0-5-2c-3 0-5 1.5-5 4m10 8c0 2.5-2 4-5 4a8 8 0 0 1-6-2M3 12h18"/>"#
            ),
            Self::Code => svg!(r#"<path d="m8 7-5 5 5 5m8-10 5 5-5 5m-3-13-2 16"/>"#),
            Self::CodeBlock => svg!(
                r#"<rect x="2.5" y="4" width="19" height="16" rx="2"/><path d="m9 9-3 3 3 3m6-6 3 3-3 3"/>"#
            ),
            Self::Bullets => {
                svg!(r#"<path d="M9 6h11M9 12h11M9 18h11M4 6h.01M4 12h.01M4 18h.01"/>"#)
            }
            Self::Ordered => svg!(
                r#"<path d="M9 6h11M9 12h11M9 18h11M3.5 5l1-1v5m-1 4c0-1.5 3-1.5 3 0 0 1-3 2-3 4h3"/>"#
            ),
            Self::Quote => svg!(
                r#"<path d="M10 6H5a2 2 0 0 0-2 2v5h5c0 2-1 4-3 5m16-12h-5a2 2 0 0 0-2 2v5h5c0 2-1 4-3 5"/>"#
            ),
            Self::Table => svg!(
                r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 10h18M3 15h18M10 4v16"/>"#
            ),
            Self::Divider => svg!(r#"<path d="M3 12h18"/>"#),
            Self::ClearFormat => svg!(r#"<path d="M4 5h15M12 5 8 16m-4 4h5m6-5 5 5m0-5-5 5"/>"#),
            Self::More => svg!(r#"<path d="M5 12h.01M12 12h.01M19 12h.01"/>"#),
        }
    }
}

pub fn icon(glyph: impl Fn() -> Icon + 'static, size: Points) -> Svg {
    svg(move || ImageSrc::svg_markup(glyph().markup()))
        .size(size)
        .flex_shrink(0.0)
        .pointer_events(PointerEvents::None)
}

pub fn hint(label: impl IntoReactive<SharedString>) -> nsh_base::TooltipElement {
    tooltip()
        .placement(Placement::Bottom)
        .px_2()
        .py_1()
        .rounded(tokens::radius_row())
        .border_1()
        .border_color(tokens::border_default)
        .bg(move || theme().popover)
        .child(
            text(label)
                .text_size(px(12.0))
                .text_color(move || theme().popover_foreground),
        )
}

pub fn chrome_button(
    glyph: impl Fn() -> Icon + 'static,
    label: impl Fn() -> SharedString + Copy + 'static,
    compact: bool,
) -> ButtonElement {
    button()
        .accessibility_label(move || Some(label()))
        .size(px(if compact { 22.0 } else { 32.0 }))
        .flex_shrink(0.0)
        .items_center()
        .justify_center()
        .rounded(tokens::radius_row())
        .text_color(tokens::shell_muted)
        .hover(|s| s.bg(tokens::shell_surface).text_color(tokens::shell_fg))
        .active(|s| {
            s.bg(tokens::shell_surface_active)
                .scale(if reduce_motion() { 1.0 } else { 0.95 })
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
        .disabled_style(|s| s.opacity(0.4))
        .transition_colors()
        .transition_transform()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(icon(glyph, px(if compact { 13.0 } else { 16.0 })))
        .child(hint(label))
}

pub fn search_dialog(
    items: impl Fn() -> Vec<nsh_base::CommandItem> + 'static,
    on_query: impl Fn(&SharedString) + 'static,
    on_select: impl Fn(&SharedString) + 'static,
    on_close: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let enter = super::motion::dialog_enter();
    nsh_base::dialog()
        .label("全局搜索")
        .on_cancel(on_close)
        .animation(ModalAnimation::None)
        .flex_1()
        .p_4()
        .items_center()
        .justify_center()
        .bg(move || tokens::shell_fg().opacity(0.24))
        .child(
            nsh_base::command()
                .bind_motion(&enter)
                .items(items)
                .on_query_change(on_query)
                .on_select(on_select)
                .on_cancel(on_close)
                .filterable(false)
                .auto_focus(true)
                .accessibility_label("搜索结果")
                .search_accessibility_label("搜索工作区")
                .placeholder("搜索工作区，或输入 > 查找命令")
                .w_full()
                .max_w(px(520.0))
                .p_3()
                .gap_2()
                .bg(move || theme().popover)
                .rounded(tokens::radius_card())
                .border_1()
                .border_color(tokens::border_default)
                .shadow_lg()
                .search_class("h-9 px-3 rounded-md bg-secondary text-foreground")
                .list_class("max-h-72")
                .item_class("rounded-md px-3 py-2 text-sm text-foreground focus:bg-secondary active:bg-secondary")
                .group_class("px-3 pt-2 text-xs text-muted-foreground")
                .empty(text("未找到匹配结果").p_3().text_sm().text_color(tokens::fg_muted)),
        )
}
