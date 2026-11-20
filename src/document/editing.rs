//! Editing mode (docs/ui-design.md §4.4, `editing = true`): the sticky
//! format toolbar, Markdown shortcuts, the `/` block-insertion menu, and
//! continuous auto-save. The tab owns the single editor lock for this
//! Document; finishing (or closing the tab) records the finish-to-read
//! transition as a history snapshot and frees the lock.

use std::sync::Arc;
use std::time::{Duration, Instant};

use nsh_base::rich_editor::{
    commands::{BoxCommand, chain, command, toggle_mark, tiptap},
    extensions::{
        history::{redo, undo},
        suggestion::{replace_suggestion, suggestion_state},
    },
    model::Fragment,
    state::EditorState,
    tables::commands::insert_table,
    view::{EditorSession, NodeViews, rich_editor},
};
use nsh_core::prelude::*;
use serde_json::{Value, json};

use crate::domain::{WorkspaceError, should_snapshot};
use crate::shell::Store;
use crate::ui::controls::{app_button, app_button_with, empty_state, panel_icon_button, ButtonKind};
use crate::ui::theme as tokens;

use super::{SLASH_KEY, build_session, doc_markdown};

/// The auto-save indicator states. `Saved` shows only after a commit
/// acknowledgement from SQLite; `Failed` keeps the draft and offers a
/// retry (another writer committed first).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SaveState {
    Dirty,
    Saved,
    Failed,
}

/// The editing chrome of one Document tab (its holder owns the lock).
#[component]
pub fn editing_view(store: Store, tab_id: u64, document_id: i64) -> impl IntoElement {
    let Some(document) = store.document(document_id) else {
        return empty_state("文档不存在", "它可能已被删除；关闭此标签页即可").into_element();
    };
    // Leaving the view — 完成, close, or navigate away — ends the edit
    // session: the finish-to-read transition snapshots, then frees the
    // single-editor lock.
    cleanup(move || store.release_editor(tab_id, true));

    let session = build_session(&document.body, NodeViews::new());
    let draft_title = RwSignal::new(document.title.clone());
    let draft_body = RwSignal::new(document.body.clone());
    let base_version = RwSignal::new(document.version);
    let save_state = RwSignal::new(SaveState::Saved);
    let composing = RwSignal::new(false);
    let history_open = RwSignal::new(false);
    // Debounce bookkeeping: the newest scheduled save wins.
    let save_epoch = RwSignal::new(0_u64);
    // History policy bookkeeping (milliseconds since this view started).
    let started = Instant::now();
    let now_ms = move || started.elapsed().as_millis() as u64;
    let last_edit_at = RwSignal::new(now_ms());
    let last_snapshot_at = RwSignal::new(None::<u64>);
    let maybe_snapshot = move || {
        if should_snapshot(
            last_edit_at.get_untracked(),
            last_snapshot_at.get_untracked(),
            now_ms(),
            10_000,
            60_000,
        ) {
            store.take_snapshot(document_id);
            last_snapshot_at.set(Some(now_ms()));
        }
    };
    let _interval = set_interval(Duration::from_secs(5), move || maybe_snapshot());

    let try_save = move || {
        let result = store.save_document_body(
            document_id,
            &draft_title.get_untracked(),
            &draft_body.get_untracked(),
            base_version.get_untracked(),
        );
        match result {
            Ok(committed) => {
                base_version.set(committed.version);
                save_state.set(SaveState::Saved);
                maybe_snapshot();
            }
            Err(WorkspaceError::StaleDocument) => save_state.set(SaveState::Failed),
            Err(err) => {
                log::warn!("auto-save failed: {err}");
                save_state.set(SaveState::Failed);
            }
        }
    };
    let schedule_save = {
        let try_save = try_save.clone();
        move || {
            save_state.set(SaveState::Dirty);
            save_epoch.update(|epoch| *epoch += 1);
            let epoch = save_epoch.get_untracked();
            let try_save = try_save.clone();
            set_timeout(Duration::from_millis(600), move || {
                if save_epoch.get_untracked() != epoch {
                    return;
                }
                try_save();
            });
        }
    };
    let mark_edited = {
        let schedule_save = schedule_save.clone();
        move || {
            last_edit_at.set(now_ms());
            schedule_save();
        }
    };
    let on_title = {
        let mark_edited = mark_edited.clone();
        move |text: &str| {
            draft_title.set(text.to_owned());
            mark_edited();
        }
    };

    // 完成: flush the pending draft, then hand the lock back. A failed
    // flush keeps the editor open so the draft survives.
    let finish = move || {
        if save_state.get_untracked() == SaveState::Dirty {
            try_save();
        }
        if save_state.get_untracked() == SaveState::Failed {
            return;
        }
        store.release_editor(tab_id, true);
    };
    // Retry speaks for the retained draft: take the freshest base and
    // commit it (an explicit user decision to overwrite the concurrent
    // change).
    let retry = {
        let try_save = try_save.clone();
        move || {
            if let Some(document) = store.document(document_id) {
                base_version.set(document.version);
            }
            try_save();
        }
    };

    // The editor reports every document change: re-serialize the draft,
    // note the edit for the history policy, and debounce the save. While
    // a Chinese IME composition is running the draft still updates, but
    // the `/` menu stays shut (see slash_panel).
    let change_session = session.clone();
    let on_change = move |_: &_| {
        draft_body.set(doc_markdown(&change_session));
        mark_edited();
    };
    let on_composition = move |composing_now: &bool| {
        composing.set(*composing_now);
    };
    // Re-read suggestion state on every change/selection event.
    let editor_events = RwSignal::new(0_u64);
    let on_activity = move |_: &_| {
        editor_events.update(|events| *events += 1);
    };

    view()
        .flex_1()
        .min_h(px(0.0))
        .flex_col()
        .child(editing_header(
            store,
            document_id,
            draft_title,
            save_state,
            history_open,
            finish,
            retry,
            on_title,
        ))
        .child(toolbar(session.clone()))
        .child(slash_panel(session.clone(), editor_events, composing))
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
                                .py_4()
                                .child(
                                    rich_editor()
                                        .key(format!("editing-{document_id}"))
                                        .editor(session.clone())
                                        .auto_focus(true)
                                        .accessibility_label("文档正文")
                                        .placeholder("输入正文，/ 插入块")
                                        .on_change(on_change)
                                        .on_selection_change(on_activity)
                                        .on_composition_change(on_composition)
                                        .text_color(tokens::fg_body()),
                                ),
                        ),
                )
                .child(super::history::history_panel(store, document_id, false, history_open)),
        )
        .into_element()
}

#[allow(clippy::too_many_arguments)]
fn editing_header(
    store: Store,
    document_id: i64,
    draft_title: RwSignal<String>,
    save_state: RwSignal<SaveState>,
    history_open: RwSignal<bool>,
    finish: impl Fn() + Copy + 'static,
    retry: impl Fn() + Copy + 'static,
    on_title: impl Fn(&str) + Copy + 'static,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .px_4()
        .py_2()
        .border_b_1()
        .border_color(tokens::border_subtle())
        .child(
            nsh_base::input()
                .value(move || draft_title.get())
                .on_change(on_title)
                .placeholder("文档标题")
                .placeholder_color(move || color(nsh_base::theme().muted_foreground))
                .h(px(34.0))
                .px_3()
                .text_size(px(15.0))
                .font_weight(FontWeight::SEMIBOLD)
                .rounded(move || nsh_base::theme().radius)
                .bg(tokens::control_bg())
                .flex_1()
                .min_w(px(0.0)),
        )
        .child(save_indicator(save_state, retry))
        .child(super::history::history_button(history_open))
        .child(app_button_with(ButtonKind::Positive, move || "完成".into(), finish).flex_none())
}

fn save_indicator(save_state: RwSignal<SaveState>, retry: impl Fn() + Copy + 'static) -> impl IntoElement {
    let label = move || {
        match save_state.get() {
            SaveState::Dirty => "保存中…".to_owned(),
            SaveState::Saved => "已自动保存".to_owned(),
            SaveState::Failed => "保存失败 · 草稿已保留".to_owned(),
        }
    };
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(text(label).text_size(px(12.0)).text_color(move || {
            if save_state.get() == SaveState::Failed {
                tokens::danger()
            } else {
                tokens::fg_muted()
            }
        }))
        .child(when(
            move || save_state.get() == SaveState::Failed,
            move || app_button(ButtonKind::Soft, "重试", retry).flex_none(),
        ))
}

// ---------------------------------------------------------------------
// The format toolbar: undo/redo, block kinds, marks, lists, table, and
// clear formatting — one row of the app's quiet buttons driving session
// commands (Markdown shortcuts and `/` cover the keyboard path).
// ---------------------------------------------------------------------

fn toolbar(session: EditorSession) -> impl IntoElement {
    let state = session.state();
    let schema = state.schema();
    let heading = schema.node_type("heading").unwrap();
    let paragraph = schema.node_type("paragraph").unwrap();
    let bullet_list = schema.node_type("bulletList").unwrap();
    let ordered_list = schema.node_type("orderedList").unwrap();
    let list_item = schema.node_type("listItem").unwrap();
    let task_list = schema.node_type("taskList").unwrap();
    let task_item = schema.node_type("taskItem").unwrap();
    let blockquote = schema.node_type("blockquote").unwrap();
    let code_block = schema.node_type("codeBlock").unwrap();
    let level = |n: u64| [("level".to_owned(), json!(n))].into_iter().collect::<nsh_base::rich_editor::model::Attrs>();
    let no_attrs = nsh_base::rich_editor::model::Attrs::new();
    let language_none =
        [("language".to_owned(), Value::Null)].into_iter().collect::<nsh_base::rich_editor::model::Attrs>();
    let bold = schema.mark_type("bold").unwrap();
    let italic = schema.mark_type("italic").unwrap();
    let underline = schema.mark_type("underline").unwrap();
    let strike = schema.mark_type("strike").unwrap();
    let code = schema.mark_type("code").unwrap();
    drop(state);

    let tools: Vec<(&'static str, BoxCommand)> = vec![
        ("撤销", command(undo)),
        ("重做", command(redo)),
        ("正文", tiptap::set_node(paragraph.clone(), no_attrs.clone())),
        ("一级标题", tiptap::toggle_node(heading.clone(), paragraph.clone(), level(1))),
        ("二级标题", tiptap::toggle_node(heading.clone(), paragraph.clone(), level(2))),
        ("三级标题", tiptap::toggle_node(heading.clone(), paragraph.clone(), level(3))),
        ("加粗", toggle_mark(bold, None, true)),
        ("斜体", toggle_mark(italic, None, true)),
        ("下划线", toggle_mark(underline, None, true)),
        ("删除线", toggle_mark(strike, None, true)),
        ("行内代码", toggle_mark(code, None, true)),
        ("无序列表", tiptap::toggle_list(bullet_list, list_item.clone(), false, no_attrs.clone())),
        ("有序列表", tiptap::toggle_list(ordered_list, list_item, false, no_attrs.clone())),
        ("清单", tiptap::toggle_list(task_list, task_item, false, no_attrs)),
        ("引用", tiptap::toggle_wrap(blockquote, no_attrs)),
        ("代码块", tiptap::toggle_node(code_block, paragraph, language_none)),
        ("表格", insert_table(3, 2, true)),
        ("清除格式", chain(vec![tiptap::unset_all_marks(), tiptap::clear_nodes()])),
    ];

    view()
        .flex_row()
        .items_center()
        .gap_1()
        .px_4()
        .py_2()
        .flex_wrap()
        .border_b_1()
        .border_color(tokens::border_subtle())
        .children(tools.into_iter().map(|(label, cmd)| {
            let run = session.clone();
            panel_icon_button(label, label, move || {
                run.run(&cmd);
                run.focus();
            })
        }))
}

// ---------------------------------------------------------------------
// The `/` block-insertion menu: opens on the slash trigger, filters by
// the typed query, and replaces the trigger with the picked block. It
// stays shut while a Chinese IME composition is running, so candidate
// text never becomes a menu.
// ---------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlashItem {
    Heading(u64),
    BulletList,
    OrderedList,
    TaskList,
    Blockquote,
    CodeBlock,
    Table,
    HorizontalRule,
}

impl SlashItem {
    fn label(self) -> &'static str {
        match self {
            Self::Heading(1) => "一级标题",
            Self::Heading(2) => "二级标题",
            Self::Heading(3) => "三级标题",
            Self::BulletList => "无序列表",
            Self::OrderedList => "有序列表",
            Self::TaskList => "清单",
            Self::Blockquote => "引用",
            Self::CodeBlock => "代码块",
            Self::Table => "表格",
            Self::HorizontalRule => "分割线",
        }
    }

    fn json(self) -> Value {
        let paragraph = || json!({"type": "paragraph"});
        match self {
            Self::Heading(level) => json!({"type": "heading", "attrs": {"level": level}}),
            Self::BulletList => {
                json!({"type": "bulletList", "content": [{"type": "listItem", "content": [paragraph()]}]})
            }
            Self::OrderedList => {
                json!({"type": "orderedList", "content": [{"type": "listItem", "content": [paragraph()]}]})
            }
            Self::TaskList => json!({
                "type": "taskList",
                "content": [{"type": "taskItem", "attrs": {"checked": false}, "content": [paragraph()]}],
            }),
            Self::Blockquote => json!({"type": "blockquote", "content": [paragraph()]}),
            Self::CodeBlock => json!({"type": "codeBlock", "attrs": {"language": null}}),
            Self::Table => {
                let cell = |header: bool| {
                    json!({
                        "type": if header { "tableHeader" } else { "tableCell" },
                        "content": [paragraph()],
                    })
                };
                json!({
                    "type": "table",
                    "content": [{
                        "type": "tableRow",
                        "content": [cell(true), cell(true)],
                    }],
                })
            }
            Self::HorizontalRule => json!({"type": "horizontalRule"}),
        }
    }
}

const SLASH_ITEMS: &[SlashItem] = &[
    SlashItem::Heading(1),
    SlashItem::Heading(2),
    SlashItem::Heading(3),
    SlashItem::BulletList,
    SlashItem::OrderedList,
    SlashItem::TaskList,
    SlashItem::Blockquote,
    SlashItem::CodeBlock,
    SlashItem::Table,
    SlashItem::HorizontalRule,
];

fn slash_panel(
    session: EditorSession,
    editor_events: RwSignal<u64>,
    composing: RwSignal<bool>,
) -> impl IntoElement {
    let items = move || {
        editor_events.get();
        if composing.get() {
            return Vec::new();
        }
        let state: EditorState = session.state();
        let Some(suggestion) = suggestion_state(&state, SLASH_KEY) else { return Vec::new() };
        if !suggestion.active {
            return Vec::new();
        }
        let query = suggestion.query.unwrap_or_default();
        SLASH_ITEMS
            .iter()
            .copied()
            .filter(|item| query.is_empty() || item.label().contains(query.as_str()))
            .collect::<Vec<_>>()
    };
    when(
        move || !items().is_empty(),
        move || {
            view()
                .mx_4()
                .my_1()
                .flex_row()
                .gap_1()
                .flex_wrap()
                .rounded(tokens::radius_card())
                .bg(tokens::panel_subtle())
                .border_1()
                .border_color(tokens::border_subtle())
                .p_2()
                .child(each(items, |item: &SlashItem| item.label(), move |item, _index| {
                    let pick = session.clone();
                    app_button(ButtonKind::Soft, item.get().label(), move || {
                        let state = pick.state();
                        let Ok(node) = state.schema().node_from_json(&item.get().json()) else { return };
                        pick.run(&replace_suggestion(SLASH_KEY, Fragment::from(node)));
                        pick.focus();
                    })
                    .flex_none()
                }))
        },
    )
}
