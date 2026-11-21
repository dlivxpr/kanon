//! Reading mode (docs/ui-design.md §4.4, `editing = false`): the
//! distraction-free layout. The committed body renders in a centered
//! narrow column with an outline, a word count, and Checklist Items that
//! toggle in place while no edit session is active anywhere; while one
//! is, they lock read-only with a jump-to-editor prompt.

use std::sync::Arc;

use nsh_base::{
    rich_editor::{
        model::Node,
        view::{EditorSession, NodeView, NodeViewCx, NodeViews},
    },
    theme,
};
use nsh_core::prelude::*;
use serde_json::Value;

use crate::shell::Store;
use crate::shell::store::{EditOutcome, PREF_OUTLINE, PREF_WORD_COUNT};
use crate::ui::chrome::{Icon, chrome_button};
use crate::ui::controls::{
    ButtonKind, app_button, app_button_with, empty_state, panel_icon_button,
};
use crate::ui::editor as editor_ui;
use crate::ui::theme as tokens;

use super::export::{document_overflow_menu, reading_dialog};
use super::task_card::TaskCardView;
use super::{CARD_NODE, build_session, doc_markdown, reveal_heading};

/// The reading chrome of one Document tab.
pub fn reading_view(store: Store, tab_id: u64, document_id: i64) -> impl IntoElement {
    if store.document(document_id).is_none() {
        return empty_state("文档不存在", "它可能已被删除；关闭此标签页即可").into_element();
    }
    // The committed body the displayed session was built from. The sync
    // effect rebuilds the editor only when the committed body differs
    // from this: a checklist toggle this tab just made updates it first,
    // so the view stays put; another tab's commit does not.
    let shown_body = signal(
        store
            .document(document_id)
            .map(|doc| doc.body)
            .unwrap_or_default(),
    );
    let epoch = signal(0_u64);
    let history_open = signal(false);
    // Failed document actions report under the reading header. Cancel is silent.
    let export_notice = signal(None::<String>);
    let dialog = signal(None);
    // The export task binds to the reading view's owner, not the menu
    // popup's, so the dialog outlives the popup (export.rs).
    let export_owner = Owner::current();
    // The session the dynamic below last built, for outline jumps and
    // checklist toggles.
    let current: Signal<Option<EditorSession>> = signal(None);
    effect(move || {
        store.bump.get();
        let committed = store
            .document(document_id)
            .map(|doc| doc.body)
            .unwrap_or_default();
        if committed != shown_body.get_untracked() {
            shown_body.set(committed);
            epoch.update(|value| *value += 1);
        }
    });
    effect(move || {
        let Some(request) = store.outline_request.get() else {
            return;
        };
        if request.tab_id != tab_id {
            return;
        }
        if let Some(session) = current.get_untracked() {
            reveal_heading(&session, request.ordinal);
        }
    });

    view()
        .flex_1()
        .min_h(px(0.0))
        .flex_col()
        .child(lock_banner(store, tab_id, document_id))
        .child(export_notice_banner(export_notice))
        .child(reading_header(
            store,
            tab_id,
            document_id,
            history_open,
            export_notice,
            dialog,
            export_owner,
        ))
        .child(
            view()
                .flex_1()
                .min_h(px(0.0))
                .flex_row()
                .gap_3()
                .px_4()
                .child(
                    view()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex_row()
                        .justify_center()
                        .child(
                            view()
                                .test_id("document-paper")
                                .w_full()
                                .max_w(px(860.0))
                                .style_when(
                                    move || super::wide_paper(store),
                                    |s| s.max_w(px(1120.0)),
                                )
                                .flex_1()
                                .min_h(px(0.0))
                                .py_6()
                                .flex_col()
                                .gap_1()
                                .child(
                                    // The paper's title (docs/kanon.html
                                    // `.paper > h1`).
                                    text(move || {
                                        store.bump.get();
                                        store
                                            .document(document_id)
                                            .map(|doc| doc.title)
                                            .unwrap_or_default()
                                    })
                                    .text_size(tokens::text_px(editor_ui::PAPER_TITLE_SIZE))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(tokens::fg_primary()),
                                )
                                .child(
                                    // Its meta line: owning book and the
                                    // last-update date (`.doc-meta`).
                                    text(move || {
                                        store.bump.get();
                                        let book = store
                                            .document(document_id)
                                            .and_then(|doc| store.knowledge_base(doc.book_id))
                                            .map(|book| book.name)
                                            .unwrap_or_else(|| "知识库".to_owned());
                                        let updated = store
                                            .document(document_id)
                                            .map(|doc| {
                                                doc.updated_at
                                                    .split(' ')
                                                    .next()
                                                    .unwrap_or("")
                                                    .to_owned()
                                            })
                                            .unwrap_or_default();
                                        format!("{} · {} 更新", book, updated)
                                    })
                                    .text_size(tokens::text_px(11.0))
                                    .text_color(tokens::fg_muted())
                                    .mb_3(),
                                )
                                .child(dynamic(move || {
                                    epoch.get();
                                    // `shown_body` is the committed body
                                    // this session was built from (the
                                    // sync effect keeps it current), so
                                    // the editor rebuilds only when the
                                    // body actually changes.
                                    let body = shown_body.get();
                                    let session = build_session(
                                        &body,
                                        NodeViews::new()
                                            .with(
                                                "taskItem",
                                                Arc::new(ReadingTaskItem { store, document_id }),
                                            )
                                            .with(CARD_NODE, Arc::new(TaskCardView { store }))
                                            .with(
                                                super::kanon::MENTION_NODE,
                                                Arc::new(super::mention::KanonMentionView {
                                                    store,
                                                }),
                                            ),
                                    );
                                    current.set(Some(session.clone()));
                                    let change_session = session.clone();
                                    editor_ui::paper_body()
                                        .key(format!("reading-{document_id}-{}", epoch.get()))
                                        .editor(session)
                                        .editable(false)
                                        .accessibility_label("文档正文")
                                        .placeholder("正文为空")
                                        .show_only_when_editable(false)
                                        .on_change(move |_: &_| {
                                            // A reading Checklist toggle: persist
                                            // the body in place, keeping the view.
                                            let body = doc_markdown(&change_session);
                                            shown_body.set(body.clone());
                                            store.toggle_reading_checklist(document_id, &body);
                                        })
                                        .into_element()
                                }))
                                .child(when(
                                    move || store.preference_enabled(PREF_WORD_COUNT),
                                    move || {
                                        text(move || {
                                            store.bump.get();
                                            store
                                                .document(document_id)
                                                .map(|doc| format!("{} 字", doc.word_count))
                                                .unwrap_or_default()
                                        })
                                        .text_size(tokens::text_px(11.0))
                                        .text_color(tokens::fg_muted())
                                        .pt_2()
                                    },
                                )),
                        ),
                )
                .child(outline_rail(store, tab_id, current))
                .child(super::history::history_panel(
                    store,
                    document_id,
                    true,
                    history_open,
                )),
        )
        .child(reading_dialog(
            store,
            document_id,
            export_notice,
            dialog,
            export_owner,
        ))
        .into_element()
}

/// The slim notice while another tab edits this Document: reading tabs
/// stay synced but their Checklist Items lock read-only, with the jump
/// to the editor. Edit sessions for OTHER Documents do not lock this
/// tab (ownership is per document_id, issue #6).
fn lock_banner(store: Store, tab_id: u64, document_id: i64) -> impl IntoElement {
    let locked_elsewhere = move || {
        store
            .editor_lock
            .get()
            .iter()
            .any(|lock| lock.tab_id != tab_id && lock.document_id == document_id)
    };
    when(locked_elsewhere, move || {
        view()
            .flex_row()
            .items_center()
            .gap_3()
            .px_4()
            .py_2()
            .bg(tokens::info_soft())
            .child(
                text("此文档正在其他标签页编辑 · 清单项已锁定")
                    .text_size(tokens::text_px(12.0))
                    .text_color(tokens::info()),
            )
            .child(
                app_button(ButtonKind::Ghost, "跳到编辑标签", move || {
                    let editor = store
                        .editor_lock
                        .get_untracked()
                        .into_iter()
                        .find(|lock| lock.document_id == document_id);
                    if let Some(lock) = editor {
                        store.activate_tab_by_id(lock.tab_id);
                    }
                })
                .flex_none(),
            )
    })
}

/// The transient 导出 Markdown outcome: a save dialog that could not
/// show, or a write that failed, explains itself here for a few seconds
/// (export.rs sets and clears it); Cancel never reaches it.
fn export_notice_banner(notice: Signal<Option<String>>) -> impl IntoElement {
    when(
        move || notice.get().is_some(),
        move || {
            view()
                .flex_row()
                .items_center()
                .gap_3()
                .px_4()
                .py_2()
                .bg(tokens::warning_soft())
                .child(
                    text(move || notice.get().unwrap_or_default())
                        .text_size(tokens::text_px(12.0))
                        .text_color(tokens::danger()),
                )
        },
    )
}

fn reading_header(
    store: Store,
    tab_id: u64,
    document_id: i64,
    history_open: Signal<bool>,
    export_notice: Signal<Option<String>>,
    dialog: Signal<Option<super::export::ReadingDialog>>,
    export_owner: Owner,
) -> impl IntoElement {
    let crumb = move || {
        store.bump.get();
        let name = store
            .document(document_id)
            .and_then(|doc| store.knowledge_base(doc.book_id))
            .map(|book| book.name)
            .unwrap_or_else(|| "知识库".to_owned());
        format!("📕 {} /", name)
    };
    let title = move || {
        store.bump.get();
        store
            .document(document_id)
            .map(|doc| doc.title)
            .unwrap_or_default()
    };
    let favorite = move || store.document_is_favorite(document_id);
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .px_4()
        .py_2()
        .border_b_1()
        .border_color(tokens::border_subtle())
        .child(
            text(crumb)
                .text_size(tokens::text_px(12.0))
                .text_color(tokens::fg_muted()),
        )
        .child(
            text(title)
                .text_size(tokens::text_px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens::fg_primary())
                .flex_1()
                .min_w(px(0.0)),
        )
        .child(panel_icon_button(
            "▣",
            "在新标签页打开",
            move || {
                store.open_document_in_new_tab(document_id);
            },
        ))
        .child(super::history::history_button(history_open))
        .child(
            chrome_button(
                || Icon::Star,
                move || {
                    if favorite() {
                        "取消收藏文档"
                    } else {
                        "收藏文档"
                    }
                    .into()
                },
                true,
            )
            .selected(favorite)
            .selected_style(|s| s.text_color(tokens::warning))
            .on_press(move || store.set_document_favorite(document_id, !favorite())),
        )
        .child(
            // Templates, Markdown export and Trash share this overflow menu.
            document_overflow_menu(store, document_id, export_notice, dialog, export_owner),
        )
        .child(
            app_button_with(
                ButtonKind::Primary,
                move || "编辑".into(),
                move || request_edit(store, tab_id, document_id),
            )
            .flex_none(),
        )
}

/// The read/edit switch: grant this tab the single editor lock, or focus
/// the tab that already holds it.
fn request_edit(store: Store, tab_id: u64, document_id: i64) {
    match store.request_edit(tab_id, document_id) {
        EditOutcome::Granted => {}
        EditOutcome::FocusOther(other) => store.activate_tab_by_id(other),
    }
}

/// The floating outline rail of the reading canvas (docs/kanon.html
/// `.outline-wrap`, docs/ui-design.md §4.4.3): while the right context
/// panel is closed, a slim handle on the paper's right edge expands the
/// heading outline; a click jumps to the heading through the same
/// single-editor outline channel the context panel uses.
fn outline_rail(
    store: Store,
    tab_id: u64,
    current: Signal<Option<EditorSession>>,
) -> impl IntoElement {
    let open = signal(false);
    let headings = move || {
        // The session the reading tab last built: tracked, so the list
        // refreshes whenever the committed body rebuilds it.
        let Some(session) = current.get() else {
            return Vec::new();
        };
        let state = session.state();
        let doc = state.doc();
        let mut items = Vec::new();
        let mut ordinal = 0_usize;
        for child in doc.content().iter() {
            if child.ty().name() == "heading" {
                let level = child
                    .attrs()
                    .get("level")
                    .and_then(Value::as_u64)
                    .unwrap_or(1);
                let text = child.text_content();
                if !text.trim().is_empty() {
                    items.push((ordinal, level, text));
                }
                ordinal += 1;
            }
        }
        items
    };
    let headings_empty = headings;
    when(
        move || !store.panel_visible() && store.preference_enabled(PREF_OUTLINE),
        move || {
            view()
                .w(px(24.0))
                .flex_none()
                .relative()
                .flex_col()
                .items_center()
                .gap_2()
                .pt_6()
                .child(panel_icon_button("☰", "文档大纲", move || {
                    open.update(|value| *value = !*value)
                }))
                .child(when(
                    move || open.get(),
                    move || {
                        view()
                            .absolute()
                            .top(px(34.0))
                            .right(px(28.0))
                            .w(px(220.0))
                            .flex_col()
                            .gap_1()
                            .rounded(tokens::radius_card())
                            .border_1()
                            .border_color(tokens::border_subtle())
                            .bg(move || theme().card)
                            .shadow_md()
                            .p_2()
                            .child(each(
                                headings,
                                |item: &(usize, u64, String)| item.0,
                                move |item, _index| {
                                    let jump = store;
                                    nsh_base::button()
                                        .on_press(move || {
                                            open.set(false);
                                            jump.request_outline_jump(
                                                tab_id,
                                                item.with(|item| item.0),
                                            );
                                        })
                                        .accessibility_label(move || {
                                            Some(item.with(|item| item.2.clone()).into())
                                        })
                                        .w_full()
                                        .items_center()
                                        .px_2()
                                        .py(px(3.0))
                                        .rounded(tokens::radius_row())
                                        .hover(|s| s.bg(tokens::control_hover()))
                                        .child(
                                            text(move || item.with(|item| item.2.clone()))
                                                .text_size(tokens::text_px(12.0))
                                                .text_color(tokens::fg_body())
                                                .number_of_lines(1)
                                                .pl(move || {
                                                    px(item.with(|item| item.1) as f32 * 8.0)
                                                }),
                                        )
                                        .into_element()
                                },
                            ))
                            .child(when(
                                move || headings_empty().is_empty(),
                                || {
                                    text("添加标题后会显示大纲。")
                                        .text_size(tokens::text_px(12.0))
                                        .text_color(tokens::fg_muted())
                                        .px_2()
                                        .py_1()
                                },
                            ))
                    },
                ))
        },
    )
}

// ---------------------------------------------------------------------
// The reading tab's own Checklist Item node view: the stock checkbox is
// disabled in read-only editors, so reading renders its own — enabled
// while no edit session is active, disabled (with the banner's jump
// prompt) while one is.
// ---------------------------------------------------------------------

struct ReadingTaskItem {
    store: Store,
    document_id: i64,
}

impl NodeView for ReadingTaskItem {
    fn create(&self, cx: &NodeViewCx, _node: &Node) -> AnyElement {
        let store = self.store;
        let document_id = self.document_id;
        let session = cx.session.clone();
        let pos = cx.get_pos();
        let node = cx.node();
        let checked = move || {
            node.with(|node| node.attrs().get("checked").and_then(Value::as_bool) == Some(true))
        };
        // Only an edit session of THIS Document locks the Checklist
        // Items (single-editor ownership is per document_id, issue #6).
        let locked = move || store.editing_locked(document_id);
        view()
            .flex_row()
            .items_start()
            .gap_2()
            .child(
                nsh_base::checkbox()
                    .accessibility_label(Some("清单项".into()))
                    .checked(checked)
                    .disabled(locked)
                    .on_change(move |next: &bool| {
                        if store.editing_locked(document_id) {
                            return;
                        }
                        let Some(pos) = pos() else { return };
                        let mut change = session.state().tr();
                        let _ = change.set_node_attribute(pos, "checked", Value::Bool(*next));
                        let _ = session.dispatch(change);
                    })
                    .size(px(16.0))
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.0))
                    .border_2()
                    .mt_1()
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
                    .indicator_style(|s| s.size(px(12.0)).items_center().justify_center())
                    .indicator(
                        text(move || if checked() { "✓" } else { "" })
                            .text_size(tokens::text_px(10.0))
                            .text_color(move || theme().primary_foreground),
                    )
                    .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent())),
            )
            .child(cx.take_content().expect("a task item's content"))
            .into_element()
    }

    fn editable_content_slot(&self) -> bool {
        true
    }
}
