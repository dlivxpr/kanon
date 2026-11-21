//! The Quick Notes surface (小记, docs/kanon.html): the sticky composer,
//! the search-filtered stream in the persisted 时间流/网格 layouts, pin,
//! tags, the toggleable checklist lines, and the note-menu actions —
//! 转为知识库文档 (the note is archived), 提取为任务, 移入回收站.
//!
//! The right context panel reuses [`quick_capture_box`] (its 速记小记
//! box) and [`recent_notes`] (its 最近小记 rows).

use nsh_base::{button, menu, menu_item, menu_separator, theme};
use nsh_core::EllipsizeMode;
use nsh_core::prelude::*;

use crate::domain::{
    NoteLine, NoteSegment, NotesLayout, QuickNoteRow, parse_note_content, parse_tag_names,
};
use crate::ui::chrome::{Icon, icon};
use crate::ui::controls::{
    ButtonKind, app_button, chip, empty_state, input_field, panel_icon_button, task_circle,
};
use crate::ui::theme as tokens;

use super::store::Store;
use super::tabs::Route;

/// The Quick Notes tab: composer on the left, the stream on the right.
pub(super) fn quick_notes_view(store: Store, focused: Option<i64>) -> impl IntoElement {
    let converting = signal(None::<i64>);
    let archived = signal(false);
    let compact = move || {
        let width = window().width.0;
        width < 860.0 || (960.0..1050.0).contains(&width) && store.panel_visible()
    };
    view()
        .flex_1()
        .min_h(px(0.0))
        .child(
            scroll().flex_1().child(
                view()
                    .flex_col()
                    .gap_5()
                    .p_5()
                    .max_w(px(1000.0))
                    .w_full()
                    .self_center()
                    .child(stream_header(store, archived, focused.is_some()))
                    .child(
                        view()
                            .flex_row()
                            .items_start()
                            .gap_5()
                            .min_w(px(0.0))
                            .style_when(compact, |s| s.flex_col().items_stretch())
                            .child(composer(store, compact))
                            .child(match focused {
                                Some(id) => focused_note(store, id, converting).into_element(),
                                None => note_stream(store, converting, archived).into_element(),
                            }),
                    ),
            ),
        )
        .child(convert_dialog(store, converting))
}

/// The segmented 时间流/网格 switch; the choice persists in the
/// workspace preferences.
fn stream_header(store: Store, archived: Signal<bool>, focused: bool) -> impl IntoElement {
    let header = view()
        .flex_row()
        .flex_wrap()
        .gap_2()
        .items_center()
        .justify_between()
        .child(
            text("小记")
                .role(AccessibilityRole::Header)
                .test_id("quick-notes-heading")
                .text_size(px(24.0))
                .font_weight(FontWeight::BOLD)
                .text_color(tokens::fg_primary()),
        );
    if focused {
        return header.child(app_button(
            ButtonKind::Ghost,
            "返回全部小记",
            move || store.open_or_focus(Route::QuickNotes),
        ));
    }
    header
        .child(
            app_button(ButtonKind::Ghost, "已归档", move || {
                archived.update(|value| *value = !*value)
            })
            .selected(move || archived.get())
            .selected_style(|s| s.bg(tokens::accent_soft).text_color(tokens::accent_ink)),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_1()
                .p_1()
                .rounded(tokens::radius_row())
                .bg(tokens::control_bg())
                .children(
                    NotesLayout::ALL
                        .into_iter()
                        .map(|layout| layout_button(store, layout).into_element())
                        .collect::<Vec<_>>(),
                ),
        )
}

fn layout_button(store: Store, layout: NotesLayout) -> nsh_base::ButtonElement {
    let active = move || store.quick_notes_layout() == layout;
    button()
        .on_press(move || store.set_quick_notes_layout(layout))
        .accessibility_label(Some(layout.label().into()))
        .selected(active)
        .h(px(26.0))
        .px_3()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .bg(move || {
            if active() {
                tokens::panel_bg()
            } else {
                theme().background.opacity(0.0)
            }
        })
        .text_color(move || {
            if active() {
                tokens::fg_primary()
            } else {
                tokens::fg_muted()
            }
        })
        .style_when(active, |s| s.shadow_sm())
        .hover(move |s| {
            if active() {
                s
            } else {
                s.bg(tokens::control_hover())
            }
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .child(text(layout.label()).text_size(px(12.0)))
}

/// The sticky composer: the fragment, its tags, and 发布.
fn composer(store: Store, compact: impl Fn() -> bool + 'static) -> impl IntoElement {
    let draft = signal(String::new());
    let tags = signal(String::new());
    // The prototype binds Ctrl/⌘+Enter to the composer form only
    // (`el.closest('#noteForm')`); the key binding below is window-wide
    // (NSH treats Enter as an editing key, so it asks for When::Always),
    // so the shortcut is gated on the composer's focus-within.
    let composer_ref = element_ref();
    let composer_focused = signal(false);
    composer_ref.on_focus_in(move || composer_focused.set(true));
    composer_ref.on_focus_out(move || composer_focused.set(false));
    let publish = move || {
        let names = parse_tag_names(&tags.get_untracked());
        if store
            .publish_quick_note(&draft.get_untracked(), &names)
            .is_some()
        {
            draft.set(String::new());
            tags.set(String::new());
        }
    };
    // NSH treats Enter as an editing key, including modified Enter.
    on_key_with(
        "mod+enter",
        nsh_core::element::KeyOptions {
            when: nsh_core::keymap::When::Always,
            ..Default::default()
        },
        move |_| {
            if composer_focused.get_untracked() {
                publish();
            }
        },
    );
    view()
        .element_ref(&composer_ref)
        .w(px(320.0))
        .flex_shrink_0()
        .self_start()
        .sticky()
        .style_when(compact, |s| s.w_full().self_stretch().relative())
        .flex_col()
        .gap_3()
        .p_4()
        .rounded(tokens::radius_card())
        .bg(move || theme().card)
        .border_1()
        .border_color(tokens::border_subtle())
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(icon(|| Icon::Note, px(15.0)))
                .child(
                    text("写小记")
                        .text_size(px(14.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens::fg_primary()),
                ),
        )
        .child(
            nsh_base::input()
                .multiline(true)
                .label("小记正文")
                .value(move || draft.get())
                .on_change(move |text: &str| draft.set(text.to_owned()))
                .placeholder("记下一闪而过的想法…")
                .placeholder_color(|| color(theme().muted_foreground))
                .caret_color(|| color(theme().primary))
                .min_h(px(120.0))
                .p_3()
                .text_size(px(13.0))
                .rounded(tokens::radius_row())
                .bg(tokens::control_bg())
                .border_1()
                .border_color(move || theme().background.opacity(0.0))
                .focus(|s| s.border_color(tokens::accent()))
                .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                .transition_colors()
                .duration(tokens::ms(tokens::MOTION_INSTANT_MS)),
        )
        .child(
            input_field(
                "# 添加标签，逗号分隔",
                move || tags.get(),
                move |text: &str| tags.set(text.to_owned()),
            )
            .label("小记标签"),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    text("Ctrl / ⌘ ↵")
                        .text_size(px(11.0))
                        .text_color(tokens::fg_muted()),
                )
                .child(
                    app_button(ButtonKind::Primary, "发布", publish)
                        .disabled(move || draft.with(|body| body.trim().is_empty()))
                        .test_id("publish-note"),
                ),
        )
}

/// Render the exact note without relying on its preview being unique.
fn focused_note(store: Store, id: i64, converting: Signal<Option<i64>>) -> impl IntoElement {
    view()
        .flex_1()
        .min_w(px(0.0))
        .child(dynamic(move || match store.live_quick_note(id) {
            Some(note) => note_card(store, note, converting).into_element(),
            None => empty_state("小记不可用", "这条小记可能已移入回收站或删除。").into_element(),
        }))
}

/// The search-filtered stream. Pinned notes head the list (the domain
/// order); the query matches content and Tag names, case-insensitive.
fn note_stream(
    store: Store,
    converting: Signal<Option<i64>>,
    archived: Signal<bool>,
) -> impl IntoElement {
    let query = signal(String::new());
    let notes = computed(move || {
        store.bump.get();
        let needle = query.get().trim().to_lowercase();
        let entries = if archived.get() {
            store.archived_quick_notes()
        } else {
            store.quick_notes()
        };
        entries
            .into_iter()
            .filter(|note| {
                needle.is_empty()
                    || note.content.to_lowercase().contains(&needle)
                    || note
                        .tags
                        .iter()
                        .any(|tag| tag.name.to_lowercase().contains(&needle))
            })
            .collect::<Vec<_>>()
    });
    view()
        .flex_1()
        .min_w(px(0.0))
        .flex_col()
        .gap_3()
        .child(input_field(
            "搜索小记或标签…",
            move || query.get(),
            move |text: &str| query.set(text.to_owned()),
        ))
        .child(when(
            move || notes.with(|list| list.is_empty()),
            move || {
                if query.get().trim().is_empty() && archived.get() {
                    empty_state("暂无归档小记", "转为知识库文档的小记会保留在这里。").into_element()
                } else if query.get().trim().is_empty() {
                    empty_state(
                        "还没有小记",
                        "在左侧写下第一条小记；右栏的收件箱面板也可以速记。",
                    )
                    .into_element()
                } else {
                    empty_state("没有匹配的小记", "换个关键词，或清空搜索框。").into_element()
                }
            },
        ))
        .child(dynamic(move || {
            let cards = notes
                .get()
                .into_iter()
                .map(|note| note_card(store, note, converting).into_element())
                .collect::<Vec<_>>();
            match store.quick_notes_layout() {
                NotesLayout::Grid => view()
                    .flex_row()
                    .flex_wrap()
                    .items_start()
                    .gap_3()
                    .children(cards)
                    .into_element(),
                NotesLayout::Stream => view().flex_col().gap_3().children(cards).into_element(),
            }
        }))
}

/// One note card: date and pin in the header, the content's display
/// lines, the Tag chips, and the actions menu.
fn note_card(
    store: Store,
    note: QuickNoteRow,
    converting: Signal<Option<i64>>,
) -> impl IntoElement {
    let id = note.id;
    let pinned = note.pinned;
    let date = format!(
        "{}/{}",
        note.created_at.date.month, note.created_at.date.day
    );
    let grid = store.quick_notes_layout() == NotesLayout::Grid;
    let mut card = view()
        .flex_col()
        .gap_2()
        .p_4()
        .rounded(tokens::radius_card())
        .border_1()
        .border_color(tokens::border_subtle())
        .bg(move || theme().card)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(when(
                    move || pinned,
                    || {
                        view()
                            .flex_row()
                            .items_center()
                            .gap_1()
                            .child(icon(|| Icon::Pin, px(12.0)))
                            .child(
                                text("已置顶")
                                    .text_size(px(11.0))
                                    .text_color(tokens::accent()),
                            )
                    },
                ))
                .child(
                    text(date)
                        .text_size(px(11.0))
                        .text_color(tokens::fg_muted()),
                )
                .child(view().flex_1())
                .child((!note.archived).then(|| note_menu(store, id, pinned, converting))),
        )
        .child(note_lines(store, &note));
    if grid {
        card = card.w(px(220.0)).flex_shrink_0();
    }
    if !note.tags.is_empty() {
        card = card.child(
            view().flex_row().flex_wrap().gap_1().children(
                note.tags
                    .into_iter()
                    .map(|tag| chip(format!("#{}", tag.name))),
            ),
        );
    }
    card
}

/// The card menu: pin, the two conversions, and Trash.
fn note_menu(
    store: Store,
    id: i64,
    pinned: bool,
    converting: Signal<Option<i64>>,
) -> nsh_base::MenuElement {
    menu()
        .offset(px(4.0))
        .popup_style(|s| {
            s.p_1()
                .rounded(tokens::radius_card())
                .bg(theme().popover)
                .shadow_md()
                .w(px(180.0))
        })
        .trigger(
            panel_icon_button("⋯", "小记操作", || {})
                .accessibility_label(Some(format!("小记操作 {id}").into())),
        )
        .child(
            menu_item()
                .on_select(move || store.set_quick_note_pinned(id, !pinned))
                .px_3()
                .py_2()
                .rounded(tokens::radius_row())
                .hover(|s| s.bg(tokens::control_hover()))
                .child(
                    text(if pinned { "取消置顶" } else { "置顶" })
                        .text_size(px(13.0))
                        .text_color(tokens::fg_primary()),
                ),
        )
        .child(
            menu_item()
                .on_select(move || converting.set(Some(id)))
                .px_3()
                .py_2()
                .rounded(tokens::radius_row())
                .hover(|s| s.bg(tokens::control_hover()))
                .child(
                    text("转为知识库文档")
                        .text_size(px(13.0))
                        .text_color(tokens::fg_primary()),
                ),
        )
        .child(
            menu_item()
                .on_select(move || {
                    if let Some(task_id) = store.extract_quick_note_task(id) {
                        store.open_task(task_id);
                    }
                })
                .px_3()
                .py_2()
                .rounded(tokens::radius_row())
                .hover(|s| s.bg(tokens::control_hover()))
                .child(
                    text("提取为任务")
                        .text_size(px(13.0))
                        .text_color(tokens::fg_primary()),
                ),
        )
        .child(
            menu_separator()
                .h(px(1.0))
                .my_1()
                .bg(tokens::border_subtle()),
        )
        .child(
            menu_item()
                .on_select(move || store.trash_quick_note(id))
                .px_3()
                .py_2()
                .rounded(tokens::radius_row())
                .hover(|s| s.bg(tokens::control_hover()))
                .child(
                    text("移入回收站")
                        .text_size(px(13.0))
                        .text_color(tokens::danger()),
                ),
        )
}

/// The content of one note: headings, checklist items, and inline
/// `**bold**` / `` `code` `` runs.
fn note_lines(store: Store, note: &QuickNoteRow) -> impl IntoElement {
    let id = note.id;
    let archived = note.archived;
    view().flex_col().gap_1().min_w(px(0.0)).children(
        parse_note_content(&note.content)
            .into_iter()
            .enumerate()
            .map(|(index, line)| note_line(store, id, index, line, archived).into_element())
            .collect::<Vec<_>>(),
    )
}

fn note_line(
    store: Store,
    note_id: i64,
    index: usize,
    line: NoteLine,
    archived: bool,
) -> impl IntoElement {
    match line {
        NoteLine::Check { done, label } if archived => {
            text(format!("{} {label}", if done { "☑" } else { "☐" }))
                .text_size(px(13.0))
                .text_color(tokens::fg_body())
                .into_element()
        }
        NoteLine::Check { done, label } => view()
            .flex_row()
            .items_center()
            .gap_2()
            .child(task_circle(
                move || done,
                label.clone(),
                move || {
                    store.toggle_quick_note_check(note_id, index);
                },
            ))
            .child(
                text(label)
                    .text_size(px(13.0))
                    .text_color(tokens::fg_body()),
            )
            .into_element(),
        NoteLine::Heading(segments) => segment_row(segments, true).into_element(),
        NoteLine::Text(segments) => segment_row(segments, false).into_element(),
    }
}

fn segment_row(segments: Vec<NoteSegment>, heading: bool) -> View {
    view().flex_row().flex_wrap().min_w(px(0.0)).children(
        segments
            .into_iter()
            .map(|segment| {
                let mut part = text(segment.text)
                    .text_size(px(if heading { 14.0 } else { 13.0 }))
                    .text_color(tokens::fg_body());
                if heading || segment.bold {
                    part = part
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens::fg_primary());
                }
                if segment.code {
                    part = part.bg(tokens::control_bg()).rounded(px(4.0)).px_1();
                }
                part.into_element()
            })
            .collect::<Vec<_>>(),
    )
}

/// The book picker behind 转为知识库文档: one row per Knowledge Base,
/// Escape or the backdrop cancels.
fn convert_dialog(store: Store, converting: Signal<Option<i64>>) -> impl IntoElement {
    when(
        move || converting.get().is_some(),
        move || {
            let note_id = converting.get().unwrap_or_default();
            let books = store.knowledge_bases();
            nsh_base::dialog()
                .label("转为知识库文档")
                .on_cancel(move || converting.set(None))
                .flex_1()
                .p_4()
                .items_center()
                .justify_center()
                .bg(move || tokens::shell_fg().opacity(0.24))
                .child(
                    view()
                        .w(px(340.0))
                        .flex_col()
                        .gap_2()
                        .p_4()
                        .rounded(tokens::radius_card())
                        .bg(move || theme().popover)
                        .border_1()
                        .border_color(tokens::border_default)
                        .shadow_lg()
                        .child(
                            text("选择目标知识库")
                                .text_size(px(14.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(tokens::fg_primary()),
                        )
                        .child(if books.is_empty() {
                            empty_state("还没有知识库", "先在「知识库」标签页创建一本。")
                                .into_element()
                        } else {
                            view()
                                .flex_col()
                                .gap_1()
                                .children(
                                    books
                                        .into_iter()
                                        .map(|book| {
                                            button()
                                                .on_press(move || {
                                                    converting.set(None);
                                                    if let Some(document) = store
                                                        .convert_quick_note_to_document(
                                                            note_id, book.id,
                                                        )
                                                    {
                                                        store.open_document(document);
                                                    }
                                                })
                                                .accessibility_label(Some(book.name.clone().into()))
                                                .flex_row()
                                                .items_center()
                                                .gap_2()
                                                .px_3()
                                                .py_2()
                                                .w_full()
                                                .min_w(px(0.0))
                                                .rounded(tokens::radius_row())
                                                .justify_start()
                                                .bg(move || theme().background.opacity(0.0))
                                                .hover(|s| s.bg(tokens::control_hover()))
                                                .focus_visible(|s| {
                                                    s.outline(px(2.0))
                                                        .outline_color(tokens::accent())
                                                })
                                                .child(icon(|| Icon::Book, px(15.0)))
                                                .child(
                                                    text(book.name)
                                                        .text_size(px(13.0))
                                                        .text_color(tokens::fg_primary()),
                                                )
                                                .into_element()
                                        })
                                        .collect::<Vec<_>>(),
                                )
                                .into_element()
                        })
                        .child(app_button(ButtonKind::Ghost, "取消", move || {
                            converting.set(None);
                        })),
                )
        },
    )
}

/// The right context panel's 速记小记 box: publishes a fragment as a
/// tagless note and keeps itself ready for the next one.
pub(super) fn quick_capture_box(store: Store) -> impl IntoElement {
    let draft = signal(String::new());
    view()
        .flex_col()
        .gap_2()
        .child(
            nsh_base::input()
                .multiline(true)
                .label("速记小记内容")
                .value(move || draft.get())
                .on_change(move |text: &str| draft.set(text.to_owned()))
                .placeholder("记下想法或摘录…")
                .placeholder_color(|| color(theme().muted_foreground))
                .caret_color(|| color(theme().primary))
                .min_h(px(64.0))
                .p_3()
                .text_size(px(13.0))
                .rounded(tokens::radius_row())
                .bg(tokens::control_bg())
                .border_1()
                .border_color(move || theme().background.opacity(0.0))
                .focus(|s| s.border_color(tokens::accent()))
                .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent())),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    text("保存到小记")
                        .text_size(px(11.0))
                        .text_color(tokens::fg_muted()),
                )
                .child(
                    app_button(ButtonKind::Primary, "保存小记", move || {
                        if store.publish_quick_note(&draft.get(), &[]).is_some() {
                            draft.set(String::new());
                        }
                    })
                    .disabled(move || draft.with(|body| body.trim().is_empty()))
                    .test_id("quick-capture-note"),
                ),
        )
}

pub(super) fn note_preview(content: &str, limit: usize) -> String {
    let first = content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    let mut chars = first.chars();
    let mut preview: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        preview.push('…');
    }
    preview
}

/// The right context panel's 最近小记 rows: the latest fragments, each
/// opening that exact note in the Quick Notes surface.
pub(super) fn recent_notes(store: Store, limit: u32) -> impl IntoElement {
    let notes = computed(move || store.recent_quick_notes(limit));
    view()
        .flex_col()
        .gap_2()
        .child(when(
            move || notes.with(Vec::is_empty),
            || empty_state("还没有小记", "在上面速记一条想法。"),
        ))
        .child(dynamic(move || {
            view().flex_col().gap_2().children(
                notes
                    .get()
                    .into_iter()
                    .map(|note| {
                        let id = note.id;
                        let preview = note_preview(&note.content, 80);
                        let meta = format!(
                            "{}/{} {}",
                            note.created_at.date.month,
                            note.created_at.date.day,
                            note.tags
                                .iter()
                                .map(|tag| format!("#{}", tag.name))
                                .collect::<Vec<_>>()
                                .join(" ")
                        );
                        button()
                            .on_press(move || store.open_or_focus(Route::QuickNote(id)))
                            .accessibility_label(Some(preview.clone().into()))
                            .flex_col()
                            .items_start()
                            .gap_1()
                            .p_3()
                            .w_full()
                            .min_w(px(0.0))
                            .rounded(tokens::radius_row())
                            .border_1()
                            .border_color(tokens::border_subtle())
                            .bg(move || theme().card)
                            .hover(|s| s.bg(tokens::control_hover()))
                            .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                            .child(
                                text(preview)
                                    .text_size(px(12.0))
                                    .text_color(tokens::fg_body())
                                    .text_overflow(EllipsizeMode::Tail),
                            )
                            .child(
                                text(meta)
                                    .text_size(px(11.0))
                                    .text_color(tokens::fg_muted()),
                            )
                            .into_element()
                    })
                    .collect::<Vec<_>>(),
            )
        }))
}
