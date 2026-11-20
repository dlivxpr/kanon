//! Editing mode (docs/ui-design.md §4.4, `editing = true`): the sticky
//! format toolbar, Markdown shortcuts, the `/` block-insertion menu, and
//! continuous auto-save. The tab owns the single editor lock for this
//! Document; finishing (or closing the tab) records the finish-to-read
//! transition as a history snapshot and frees the lock.

use std::sync::Arc;
use std::time::{Duration, Instant};

use nsh_base::rich_editor::{
    commands::{BoxCommand, chain, command, tiptap, toggle_mark},
    extensions::{
        history::{close_history, redo, undo},
        suggestion::{replace_suggestion, suggestion_state},
    },
    model::Fragment,
    state::EditorState,
    tables::commands::insert_table,
    view::{EditorSession, NodeViews, rich_editor},
};
use nsh_core::prelude::*;
use serde_json::{Value, json};

use crate::domain::{LocalDate, WorkspaceError, should_snapshot};
use crate::shell::Store;
use crate::ui::controls::{
    ButtonKind, app_button, app_button_with, empty_state, panel_icon_button,
};
use crate::ui::theme as tokens;

use super::task_card::TaskCardView;
use super::{CARD_NODE, MENTION_KEY, SLASH_KEY, build_session, doc_markdown};

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
pub fn editing_view(store: Store, tab_id: u64, document_id: i64) -> impl IntoElement {
    let Some(document) = store.document(document_id) else {
        return empty_state("文档不存在", "它可能已被删除；关闭此标签页即可").into_element();
    };
    // Leaving the view — 完成 or the tab navigating away — ends the edit
    // session: the finish-to-read transition snapshots, then frees the
    // single-editor lock. Switching to another tab unmounts too, but the
    // lock belongs to the tab and must survive (route-aware release).
    cleanup(move || store.release_editor_if_route_left(tab_id, true));

    let session = build_session(
        &document.body,
        NodeViews::new()
            .with(CARD_NODE, Arc::new(TaskCardView { store }))
            .with(super::kanon::MENTION_NODE, Arc::new(super::mention::KanonMentionView { store })),
    );
    let draft_title = signal(document.title.clone());
    let draft_body = signal(document.body.clone());
    let base_version = signal(document.version);
    let save_state = signal(SaveState::Saved);
    let composing = signal(false);
    let history_open = signal(false);
    // The promote chooser (转为任务) and the task card picker.
    let promote_open = signal(false);
    let card_picker_open = signal(false);
    let card_filter = signal(String::new());
    // Debounce bookkeeping: the newest scheduled save wins.
    let save_epoch = signal(0_u64);
    // History policy bookkeeping (milliseconds since this view started).
    let started = Instant::now();
    let now_ms = move || started.elapsed().as_millis() as u64;
    let last_edit_at = signal(now_ms());
    let last_snapshot_at = signal(None::<u64>);
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
    let _interval = set_interval(Duration::from_secs(5), maybe_snapshot);

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
        move || {
            save_state.set(SaveState::Dirty);
            save_epoch.update(|epoch| *epoch += 1);
            let epoch = save_epoch.get_untracked();
            set_timeout(Duration::from_millis(600), move || {
                if save_epoch.get_untracked() != epoch {
                    return;
                }
                try_save();
            });
        }
    };
    let mark_edited = {
        move || {
            last_edit_at.set(now_ms());
            schedule_save();
        }
    };
    let on_title = {
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
    let editor_events = signal(0_u64);
    let on_activity = move |_: &_| {
        editor_events.update(|events| *events += 1);
    };

    view()
        .flex_1()
        .min_h(px(0.0))
        .flex_col()
        .child(editing_header(
            draft_title,
            save_state,
            history_open,
            finish,
            retry,
            on_title,
        ))
        .child(toolbar(
            session.clone(),
            editor_events,
            composing,
            promote_open,
            card_picker_open,
        ))
        .child(promote_panel(
            store,
            session.clone(),
            document_id,
            base_version,
            save_state,
            editor_events,
            composing,
            promote_open,
        ))
        .child(task_card_picker(
            store,
            session.clone(),
            card_picker_open,
            card_filter,
        ))
        .child(mention_panel(
            store,
            session.clone(),
            editor_events,
            composing,
        ))
        .child(slash_panel(store, session.clone(), editor_events, composing))
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
                .child(super::history::history_panel(
                    store,
                    document_id,
                    false,
                    history_open,
                )),
        )
        .into_element()
}

fn editing_header(
    draft_title: Signal<String>,
    save_state: Signal<SaveState>,
    history_open: Signal<bool>,
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

fn save_indicator(
    save_state: Signal<SaveState>,
    retry: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let label = move || match save_state.get() {
        SaveState::Dirty => "保存中…".to_owned(),
        SaveState::Saved => "已自动保存".to_owned(),
        SaveState::Failed => "保存失败 · 草稿已保留".to_owned(),
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
// The format toolbar: undo/redo, block kinds, marks, lists, table, the
// Checklist Item → Task promotion and the Linked Task Card picker — one
// row of the app's quiet buttons driving session commands (Markdown
// shortcuts and `/` cover the keyboard path).
// ---------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn toolbar(
    session: EditorSession,
    editor_events: Signal<u64>,
    composing: Signal<bool>,
    promote_open: Signal<bool>,
    card_picker_open: Signal<bool>,
) -> impl IntoElement {
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
    let level = |n: u64| {
        [("level".to_owned(), json!(n))]
            .into_iter()
            .collect::<nsh_base::rich_editor::model::Attrs>()
    };
    let no_attrs = nsh_base::rich_editor::model::Attrs::new();
    let language_none = [("language".to_owned(), Value::Null)]
        .into_iter()
        .collect::<nsh_base::rich_editor::model::Attrs>();
    let bold = schema.mark_type("bold").unwrap();
    let italic = schema.mark_type("italic").unwrap();
    let underline = schema.mark_type("underline").unwrap();
    let strike = schema.mark_type("strike").unwrap();
    let code = schema.mark_type("code").unwrap();
    drop(state);

    let tools: Vec<(&'static str, BoxCommand)> = vec![
        ("撤销", command(undo)),
        ("重做", command(redo)),
        (
            "正文",
            tiptap::set_node(paragraph.clone(), no_attrs.clone()),
        ),
        (
            "一级标题",
            tiptap::toggle_node(heading.clone(), paragraph.clone(), level(1)),
        ),
        (
            "二级标题",
            tiptap::toggle_node(heading.clone(), paragraph.clone(), level(2)),
        ),
        (
            "三级标题",
            tiptap::toggle_node(heading.clone(), paragraph.clone(), level(3)),
        ),
        ("加粗", toggle_mark(bold, None, true)),
        ("斜体", toggle_mark(italic, None, true)),
        ("下划线", toggle_mark(underline, None, true)),
        ("删除线", toggle_mark(strike, None, true)),
        ("行内代码", toggle_mark(code, None, true)),
        (
            "无序列表",
            tiptap::toggle_list(bullet_list, list_item.clone(), false, no_attrs.clone()),
        ),
        (
            "有序列表",
            tiptap::toggle_list(ordered_list, list_item, false, no_attrs.clone()),
        ),
        (
            "清单",
            tiptap::toggle_list(task_list, task_item, false, no_attrs.clone()),
        ),
        ("引用", tiptap::toggle_wrap(blockquote, no_attrs.clone())),
        (
            "代码块",
            tiptap::toggle_node(code_block, paragraph, language_none),
        ),
        ("表格", insert_table(3, 2, true)),
        (
            "清除格式",
            chain(vec![tiptap::unset_all_marks(), tiptap::clear_nodes()]),
        ),
    ];

    let promote_session = session.clone();
    let in_checklist = move || {
        editor_events.get();
        !composing.get() && selection_task_item(&promote_session).is_some()
    };

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
        .child(when(
            in_checklist,
            move || {
                // 转为任务 (ADR-0002): promote the selected Checklist
                // Item into a real Task; the chooser below picks the
                // Inbox or a Project.
                app_button_with(
                    ButtonKind::Soft,
                    move || "转为任务".into(),
                    move || {
                        promote_open.update(|open| *open = !*open);
                        card_picker_open.set(false);
                    },
                )
                .flex_none()
            },
        ))
        .child(
            // 关联任务卡片 (GLOSSARY): pick an existing Task to embed as
            // a Linked Task Card block.
            app_button_with(
                ButtonKind::Soft,
                move || "关联任务卡片".into(),
                move || {
                    card_picker_open.update(|open| *open = !*open);
                    promote_open.set(false);
                },
            )
            .flex_none(),
        )
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
            Self::Heading(_) => "标题",
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
    store: Store,
    session: EditorSession,
    editor_events: Signal<u64>,
    composing: Signal<bool>,
) -> impl IntoElement {
    let items_session = session.clone();
    let items = move || {
        editor_events.get();
        if composing.get() {
            return Vec::new();
        }
        let state: EditorState = items_session.state();
        let Some(suggestion) = suggestion_state(&state, SLASH_KEY) else {
            return Vec::new();
        };
        if !suggestion.active {
            return Vec::new();
        }
        let query = suggestion.query.clone().unwrap_or_default();
        SLASH_ITEMS
            .iter()
            .copied()
            .filter(|item| query.is_empty() || item.label().contains(query.as_str()))
            .collect::<Vec<_>>()
    };
    // The `/` embed path (docs/ui-design.md §4.4): matching Tasks embed
    // as interactive Linked Task Cards.
    let tasks_store = store;
    let tasks_session = session.clone();
    let task_matches = move || {
        editor_events.get();
        if composing.get() {
            return Vec::new();
        }
        let state: EditorState = tasks_session.state();
        let Some(suggestion) = suggestion_state(&state, SLASH_KEY) else { return Vec::new() };
        if !suggestion.active {
            return Vec::new();
        }
        let query = suggestion.query.clone().unwrap_or_default();
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        tasks_store
            .live_tasks()
            .into_iter()
            .filter(|aggregate| aggregate.task.title.to_lowercase().contains(&needle))
            .take(4)
            .collect::<Vec<_>>()
    };
    let items_list = items.clone();
    let session_list = session.clone();
    let tasks_list = task_matches.clone();
    let session_cards = session.clone();
    when(
        move || !items().is_empty() || !task_matches().is_empty(),
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
                .child({
                    let list = items_list.clone();
                    let pick_source = session_list.clone();
                    each(
                        list,
                        |item: &SlashItem| item.label(),
                        move |item, _index| {
                            let pick = pick_source.clone();
                            app_button(ButtonKind::Soft, item.get().label(), move || {
                                let state = pick.state();
                                let Ok(node) = state.schema().node_from_json(&item.get().json())
                                else {
                                    return;
                                };
                                pick.run(&replace_suggestion(SLASH_KEY, Fragment::from(node)));
                                pick.focus();
                            })
                            .flex_none()
                        },
                    )
                })
                .child({
                    let list = tasks_list.clone();
                    let cards = session_cards.clone();
                    each(
                        list,
                        |aggregate: &crate::domain::TaskAggregate| aggregate.task.id,
                        move |aggregate, _index| {
                            let pick = cards.clone();
                            let id = aggregate.with(|a| a.task.id);
                            let title = aggregate.with(|a| a.task.title.clone());
                            app_button(
                                ButtonKind::Soft,
                                format!("关联任务卡片 · {title}"),
                                move || {
                                    insert_card_at_suggestion(&pick, SLASH_KEY, id);
                                },
                            )
                            .flex_none()
                            .into_element()
                        },
                    )
                })
        },
    )
}

// ---------------------------------------------------------------------
// Checklist Item → Task promotion (ADR-0002). The toolbar's 转为任务
// opens the chooser; picking Inbox or a Project swaps the selected
// Checklist Item for a Linked Task Card in the editor, closes the undo
// boundary, and commits Task creation + card body + reference index in
// one SQLite transaction (see `Store::promote_checklist_item`). One
// editor Undo afterwards reverts only the Document and the index: the
// Task and its later edits survive.
// ---------------------------------------------------------------------

/// The innermost `taskItem` (Checklist Item) holding the selection, if
/// any — the promotion target.
fn selection_task_item(session: &EditorSession) -> Option<()> {
    let state = session.state();
    let resolved = state.doc().resolve(state.selection().from()).ok()?;
    (1..=resolved.depth)
        .any(|depth| resolved.node(depth).ty().name() == "taskItem")
        .then_some(())
}

/// The chooser below the toolbar: 收件箱 or one Project per button.
#[allow(clippy::too_many_arguments)]
fn promote_panel(
    store: Store,
    session: EditorSession,
    document_id: i64,
    base_version: Signal<i64>,
    save_state: Signal<SaveState>,
    editor_events: Signal<u64>,
    composing: Signal<bool>,
    promote_open: Signal<bool>,
) -> impl IntoElement {
    let available_session = session.clone();
    let available = move || {
        editor_events.get();
        !composing.get() && selection_task_item(&available_session).is_some()
    };
    when(
        move || promote_open.get() && available(),
        move || {
            let panel_session = session.clone();
            let inbox_session = session.clone();
            view()
                .mx_4()
                .my_1()
                .flex_row()
                .items_center()
                .gap_1()
                .flex_wrap()
                .rounded(tokens::radius_card())
                .bg(tokens::panel_subtle())
                .border_1()
                .border_color(tokens::border_subtle())
                .p_2()
                .child(text("新任务存入").text_size(px(12.0)).text_color(tokens::fg_muted()))
                .child(app_button(ButtonKind::Positive, "收件箱", move || {
                    promote_open.set(false);
                    promote_selected_item(
                        store,
                        &inbox_session,
                        document_id,
                        base_version,
                        save_state,
                        None,
                    );
                }))
                .children(store.projects().into_iter().map(|project| {
                    let pick_session = panel_session.clone();
                    let label = format!("项目 · {}", project.name);
                    app_button(ButtonKind::Soft, label, move || {
                        promote_open.set(false);
                        promote_selected_item(
                            store,
                            &pick_session,
                            document_id,
                            base_version,
                            save_state,
                            Some(project.id),
                        );
                    })
                    .into_element()
                }))
        },
    )
}

/// Swaps the selected Checklist Item for a placeholder Linked Task Card,
/// serializes, and asks the store to promote: one SQLite transaction
/// creates the Task, commits the card body (placeholder substituted with
/// the real id) and indexes the reference. On success the in-session
/// card attr is fixed without adding an undo step; on failure the swap
/// is undone so the draft stays a Checklist Item.
#[allow(clippy::too_many_arguments)]
fn promote_selected_item(
    store: Store,
    session: &EditorSession,
    document_id: i64,
    base_version: Signal<i64>,
    save_state: Signal<SaveState>,
    project_id: Option<i64>,
) {
    let state = session.state();
    let Ok(resolved) = state.doc().resolve(state.selection().from()) else { return };
    let mut item_depth = None;
    for depth in (1..=resolved.depth).rev() {
        if resolved.node(depth).ty().name() == "taskItem" {
            item_depth = Some(depth);
            break;
        }
    }
    let Some(depth) = item_depth else { return };
    let title = resolved.node(depth).text_content().trim().to_owned();
    if title.is_empty() {
        return;
    }
    let (start, end) = (resolved.start(depth), resolved.end(depth));
    let insert_at = if depth >= 1
        && resolved.node(depth - 1).ty().name() == "taskList"
        && resolved.node(depth - 1).child_count() == 1
    {
        // A single-item list becomes the card whole.
        resolved.start(depth - 1)
    } else {
        // Inside a longer list the transform fits the card by splitting.
        start
    };
    let schema = state.schema().clone();
    let Ok(card) =
        schema.node_from_json(&json!({"type": CARD_NODE, "attrs": {"taskId": 0}}))
    else {
        return;
    };
    let mut tr = state.tr();
    // The undo boundary: one editor Undo reverts exactly this swap —
    // nothing before it (the typing) and nothing after it (the attr fix).
    close_history(&mut tr);
    let swapped = if insert_at == start {
        tr.replace_with(start, end, Fragment::from(card))
    } else {
        tr.replace_with(insert_at, resolved.end(depth - 1), Fragment::from(card))
    };
    if swapped.is_err() || session.dispatch(tr).is_err() {
        return;
    }

    let editor_body = doc_markdown(session);
    match store.promote_checklist_item(
        document_id,
        base_version.get_untracked(),
        &editor_body,
        &title,
        project_id,
    ) {
        Ok(promotion) => {
            base_version.set(promotion.document.version);
            save_state.set(SaveState::Saved);
            // Bind the card to the real Task without an undo step.
            let state = session.state();
            let mut fix = state.tr();
            fix.set_add_to_history(false);
            if let Some(pos) = card_node_position(&state, insert_at) {
                let _ = fix.set_node_attribute(pos, "taskId", json!(promotion.task_id));
            }
            let _ = session.dispatch(fix);
            session.focus();
        }
        Err(err) => {
            log::warn!("promote checklist item failed: {err}");
            save_state.set(SaveState::Failed);
            // The swap stays uncommitted: revert it so the auto-save
            // keeps the Checklist Item draft.
            let _ = session.run(&command(undo));
        }
    }
}

/// Where the Linked Task Card inserted at `at` sits in `state`'s
/// document. A document may already hold other cards — with an existing
/// card earlier in the body, the first card overall is the WRONG node;
/// the freshly promoted placeholder is the first card reaching the
/// insertion point.
fn card_node_position(state: &EditorState, at: usize) -> Option<usize> {
    let doc = state.doc();
    let mut pos = 0_usize;
    for child in doc.content().iter() {
        let size = child.node_size();
        if child.ty().name() == CARD_NODE && at < pos + size {
            return Some(pos);
        }
        pos += size;
    }
    None
}

/// The toolbar's Linked Task Card picker: filter field plus every live
/// Task, each embedding a card block at the caret.
fn task_card_picker(
    store: Store,
    session: EditorSession,
    open: Signal<bool>,
    filter: Signal<String>,
) -> impl IntoElement {
    let matches = move || {
        store.bump.get();
        let needle = filter.get().trim().to_lowercase();
        let mut found = store
            .live_tasks()
            .into_iter()
            .filter(|aggregate| {
                needle.is_empty() || aggregate.task.title.to_lowercase().contains(&needle)
            })
            .take(8)
            .collect::<Vec<_>>();
        found.sort_by_key(|aggregate| aggregate.task.title.clone());
        found
    };
    when(
        move || open.get(),
        move || {
            view()
                .mx_4()
                .my_1()
                .flex_col()
                .gap_1()
                .rounded(tokens::radius_card())
                .bg(tokens::panel_subtle())
                .border_1()
                .border_color(tokens::border_subtle())
                .p_2()
                .child(
                    nsh_base::input()
                        .value(move || filter.get())
                        .on_change(move |text: &str| filter.set(text.to_owned()))
                        .placeholder("搜索任务，点选插入关联任务卡片")
                        .placeholder_color(move || color(nsh_base::theme().muted_foreground))
                        .h(px(30.0))
                        .px_2()
                        .text_size(px(13.0))
                        .rounded(tokens::radius_row())
                        .bg(tokens::control_bg())
                        .label("搜索任务"),
                )
                .child({
                    let cards = session.clone();
                    each(
                        matches,
                        |aggregate: &crate::domain::TaskAggregate| aggregate.task.id,
                        move |aggregate, _index| {
                            let pick_session = cards.clone();
                            let id = aggregate.with(|a| a.task.id);
                            let title = aggregate.with(|a| a.task.title.clone());
                            app_button(
                                ButtonKind::Soft,
                                format!("插入卡片 · {title}"),
                                move || {
                                    open.set(false);
                                    insert_card_block(&pick_session, id);
                                },
                            )
                            .into_element()
                        },
                    )
                })
        },
    )
}

/// Inserts a Linked Task Card block right after the top-level block the
/// caret sits in.
fn insert_card_block(session: &EditorSession, task_id: i64) {
    let state = session.state();
    let schema = state.schema().clone();
    let Ok(card) = schema.node_from_json(&json!({"type": CARD_NODE, "attrs": {"taskId": task_id}}))
    else {
        return;
    };
    let doc = state.doc();
    let at = state.selection().from();
    let mut insert_at = doc.content().size();
    let mut pos = 0_usize;
    for child in doc.content().iter() {
        let next = pos + child.node_size();
        if at < next {
            insert_at = next;
            break;
        }
        pos = next;
    }
    let mut tr = state.tr();
    if tr.insert(insert_at, Fragment::from(card)).is_ok() {
        let _ = session.dispatch(tr);
        session.focus();
    }
}

/// Replaces the active `/` (or `@`) suggestion with a Linked Task Card
/// of an existing Task; falls back to deleting the trigger text and
/// inserting the card after the enclosing top-level block.
fn insert_card_at_suggestion(session: &EditorSession, key: &str, task_id: i64) {
    let state = session.state();
    let Some(suggestion) = suggestion_state(&state, key).filter(|s| s.active) else { return };
    let schema = state.schema().clone();
    let Ok(card) = schema.node_from_json(&json!({"type": CARD_NODE, "attrs": {"taskId": task_id}}))
    else {
        return;
    };
    let range = suggestion.range;
    let mut tr = state.tr();
    if tr.replace_with(range.from, range.to, Fragment::from(card.clone())).is_err() {
        // Delete the trigger text, then put the card after the
        // enclosing top-level block (positions computed on the original
        // document, shifted by the deletion when after it).
        let _ = tr.replace_with(range.from, range.to, Fragment::empty());
        let mut insert_at = state.doc().content().size();
        let mut pos = 0_usize;
        for child in state.doc().content().iter() {
            let next = pos + child.node_size();
            if range.from < next {
                insert_at = next;
                break;
            }
            pos = next;
        }
        if insert_at >= range.to {
            insert_at -= range.to - range.from;
        }
        let _ = tr.insert(insert_at, Fragment::from(card));
    }
    if session.dispatch(tr).is_ok() {
        session.focus();
    }
}

// ---------------------------------------------------------------------
// The `@` inline-mention menu (docs/ui-design.md §4.4): opens on the `@`
// trigger, lists dates (a typed `YYYY-MM-DD`), Documents, Projects and
// Tasks matching the query, and replaces the trigger with a mention
// chip that flows inside the paragraph. The next auto-save indexes it
// into `document_references` (the right panel's backlinks).
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum MentionItem {
    Date(String),
    Document(i64, String),
    Project(i64, String),
    Task(i64, String),
}

impl MentionItem {
    /// The one decomposition every accessor derives from:
    /// (kind, label prefix, id part, display label).
    fn parts(&self) -> (&'static str, &'static str, String, String) {
        match self {
            Self::Date(text) => ("date", "日期", text.clone(), text.clone()),
            Self::Document(id, title) => ("document", "文档", id.to_string(), title.clone()),
            Self::Project(id, name) => ("project", "项目", id.to_string(), name.clone()),
            Self::Task(id, title) => ("task", "任务", id.to_string(), title.clone()),
        }
    }

    fn label(&self) -> String {
        let (_, prefix, _, text) = self.parts();
        format!("{prefix} · {text}")
    }

    /// The mention node's `id` attr (`kind:id_part`).
    fn id(&self) -> String {
        let (kind, _, id_part, _) = self.parts();
        format!("{kind}:{id_part}")
    }

    fn label_attr(&self) -> String {
        self.parts().3
    }
}

fn mention_items(store: Store, session: &EditorSession, editor_events: Signal<u64>, composing: Signal<bool>) -> Vec<MentionItem> {
    editor_events.get();
    if composing.get() {
        return Vec::new();
    }
    let state = session.state();
    let Some(suggestion) = suggestion_state(&state, MENTION_KEY) else { return Vec::new() };
    if !suggestion.active {
        return Vec::new();
    }
    let query = suggestion.query.clone().unwrap_or_default();
    let needle = query.trim().to_lowercase();
    let mut items = Vec::new();
    if let Some(date) = LocalDate::parse(needle.as_str()) {
        items.push(MentionItem::Date(date.to_string()));
    }
    store.bump.get();
    for (id, title) in store.document_titles_matching(&query, 4) {
        items.push(MentionItem::Document(id, title));
    }
    for project in store.projects() {
        if items.len() >= 9 {
            break;
        }
        if needle.is_empty() || project.name.to_lowercase().contains(&needle) {
            items.push(MentionItem::Project(project.id, project.name));
        }
    }
    for aggregate in store.live_tasks() {
        if items.len() >= 13 {
            break;
        }
        if needle.is_empty() || aggregate.task.title.to_lowercase().contains(&needle) {
            items.push(MentionItem::Task(aggregate.task.id, aggregate.task.title));
        }
    }
    items
}

fn mention_panel(
    store: Store,
    session: EditorSession,
    editor_events: Signal<u64>,
    composing: Signal<bool>,
) -> impl IntoElement {
    let items_session = session.clone();
    let items = move || mention_items(store, &items_session, editor_events, composing);
    let items_list = items.clone();
    let pick_session = session.clone();
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
                .child({
                    let list = items_list.clone();
                    let pick = pick_session.clone();
                    each(
                        list,
                        |item: &MentionItem| item.label(),
                        move |item, _index| {
                            let pick = pick.clone();
                            app_button(ButtonKind::Soft, item.with(|item| item.label()), move || {
                                // Picking a Task embeds an interactive
                                // Linked Task Card (ADR-0002), like the
                                // `/` path — a plain inline mention would
                                // lose the live card and its Trash
                                // degradation.
                                if let MentionItem::Task(task_id, _) = item.with(|item| item.clone()) {
                                    insert_card_at_suggestion(&pick, MENTION_KEY, task_id);
                                    return;
                                }
                                let state = pick.state();
                                let Ok(node) = state.schema().node_from_json(&json!({
                                    "type": super::kanon::MENTION_NODE,
                                    "attrs": {"id": item.with(|item| item.id()), "label": item.with(|item| item.label_attr())},
                                })) else {
                                    return;
                                };
                                pick.run(&replace_suggestion(MENTION_KEY, Fragment::from(node)));
                                pick.focus();
                            })
                            .into_element()
                        },
                    )
                })
        },
    )
}
