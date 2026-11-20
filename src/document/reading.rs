//! Reading mode (docs/ui-design.md §4.4, `editing = false`): the
//! distraction-free layout. The committed body renders in a centered
//! narrow column with an outline, a word count, and Checklist Items that
//! toggle in place while no edit session is active anywhere; while one
//! is, they lock read-only with a jump-to-editor prompt.

use std::sync::Arc;

use nsh_base::{
    rich_editor::{
        model::Node,
        view::{rich_editor, EditorSession, NodeView, NodeViewCx, NodeViews},
    },
    theme,
};
use nsh_core::prelude::*;
use serde_json::Value;

use crate::shell::store::EditOutcome;
use crate::shell::Store;
use crate::ui::controls::{
    app_button, app_button_with, empty_state, panel_icon_button, ButtonKind,
};
use crate::ui::theme as tokens;

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
        .child(lock_banner(store, tab_id))
        .child(reading_header(store, tab_id, document_id, history_open))
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
                                .w(px(780.0))
                                .max_w(px(780.0))
                                .flex_1()
                                .min_h(px(0.0))
                                .py_6()
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
                                            .with("taskItem", Arc::new(ReadingTaskItem { store }))
                                            .with(CARD_NODE, Arc::new(TaskCardView { store })),
                                    );
                                    current.set(Some(session.clone()));
                                    let change_session = session.clone();
                                    rich_editor()
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
                                        .text_color(tokens::fg_body())
                                        .into_element()
                                })),
                        ),
                )
                .child(super::history::history_panel(
                    store,
                    document_id,
                    true,
                    history_open,
                )),
        )
        .child(word_count_bar(store, document_id))
        .into_element()
}

/// The slim notice while another tab edits: reading tabs stay synced but
/// their Checklist Items lock read-only, with the jump to the editor.
fn lock_banner(store: Store, tab_id: u64) -> impl IntoElement {
    let locked_elsewhere = move || {
        store
            .editor_lock
            .get()
            .is_some_and(|lock| lock.tab_id != tab_id)
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
                    .text_size(px(12.0))
                    .text_color(tokens::info()),
            )
            .child(
                app_button(ButtonKind::Ghost, "跳到编辑标签", move || {
                    if let Some(lock) = store.editor_lock.get_untracked() {
                        store.activate_tab_by_id(lock.tab_id);
                    }
                })
                .flex_none(),
            )
    })
}

fn reading_header(
    store: Store,
    tab_id: u64,
    document_id: i64,
    history_open: Signal<bool>,
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
                .text_size(px(12.0))
                .text_color(tokens::fg_muted()),
        )
        .child(
            text(title)
                .text_size(px(14.0))
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

fn word_count_bar(store: Store, document_id: i64) -> impl IntoElement {
    let counter = move || {
        store.bump.get();
        store
            .document(document_id)
            .map(|doc| format!("{} 字", doc.word_count))
            .unwrap_or_default()
    };
    view()
        .flex_row()
        .items_center()
        .px_4()
        .py_2()
        .border_t_1()
        .border_color(tokens::border_subtle())
        .child(
            text(counter)
                .text_size(px(11.0))
                .text_color(tokens::fg_muted()),
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
}

impl NodeView for ReadingTaskItem {
    fn create(&self, cx: &NodeViewCx, _node: &Node) -> AnyElement {
        let store = self.store;
        let session = cx.session.clone();
        let pos = cx.get_pos();
        let node = cx.node();
        let checked = move || {
            node.with(|node| node.attrs().get("checked").and_then(Value::as_bool) == Some(true))
        };
        let locked = move || store.editing_locked();
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
                        if store.editing_locked() {
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
                            .text_size(px(10.0))
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
