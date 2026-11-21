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
    formats::{DocFormat, MarkdownFormat},
    model::Fragment,
    state::EditorState,
    tables::commands::insert_table,
    view::{EditorSession, NodeViews},
};
use nsh_core::prelude::*;
use serde_json::{Value, json};

use crate::domain::{LocalDate, WorkspaceError, should_snapshot};
use crate::shell::Store;
use crate::ui::controls::{ButtonKind, app_button, app_button_with, empty_state};
use crate::ui::theme as tokens;
use crate::ui::{
    chrome::{Icon, hint, icon},
    editor as editor_ui,
};

use super::task_card::TaskCardView;
use super::templates::TEMPLATES;
use super::{CARD_NODE, MENTION_KEY, SLASH_KEY, build_session_with_shortcut, doc_markdown};

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
pub(super) fn editing_view(store: Store, tab_id: u64, document_id: i64) -> impl IntoElement {
    let Some(document) = store.document(document_id) else {
        return empty_state("文档不存在", "它可能已被删除；关闭此标签页即可").into_element();
    };
    // Leaving the view — 完成 or the tab navigating away — ends the edit
    // session: the finish-to-read transition snapshots, then frees the
    // single-editor lock. Switching to another tab unmounts too, but the
    // lock belongs to the tab and must survive (route-aware release).
    cleanup(move || store.release_editor_if_route_left(tab_id, true));

    let finish_requested = signal(0_u64);
    let session = build_session_with_shortcut(
        &document.body,
        NodeViews::new()
            .with(CARD_NODE, Arc::new(TaskCardView { store }))
            .with(
                super::kanon::MENTION_NODE,
                Arc::new(super::mention::KanonMentionView { store }),
            ),
        Some(finish_requested),
    );
    let outline_session = session.clone();
    effect(move || {
        if let Some(request) = store.outline_request.get()
            && request.tab_id == tab_id
        {
            super::reveal_heading(&outline_session, request.ordinal);
        }
    });
    let draft_title = signal(document.title.clone());
    let draft_body = signal(document.body.clone());
    let base_version = signal(document.version);
    let save_state = signal(SaveState::Saved);
    let composing = signal(false);
    let body_focused = signal(false);
    let history_open = signal(false);
    // The promote chooser (转为任务) and the task card picker.
    let promote_open = signal(false);
    let card_picker_open = signal(false);
    let card_from_slash = signal(false);
    let card_filter = signal(String::new());
    let open_card_picker = move |from_slash| {
        card_from_slash.set(from_slash);
        card_filter.set(String::new());
        promote_open.set(false);
        card_picker_open.set(true);
    };
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
    let flush = move || {
        if save_state.get_untracked() == SaveState::Dirty {
            try_save();
        }
        save_state.get_untracked() == SaveState::Saved
    };
    crate::desktop::check_before_quit(move || !composing.get_untracked() && flush());
    nsh_core::element::desktop::on_app_quit(move || {
        flush();
        std::future::ready(())
    });
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
        if !flush() {
            return;
        }
        store.release_editor(tab_id, true);
    };
    let finish_if_available = move || {
        if !composing.get_untracked()
            && !history_open.get_untracked()
            && !promote_open.get_untracked()
            && !card_picker_open.get_untracked()
        {
            finish();
        }
    };
    effect(move || {
        if finish_requested.get() > 0 {
            untracked(finish_if_available);
        }
    });
    use nsh_core::{element::KeyOptions, keymap::When};
    on_key_with(
        "mod+e",
        KeyOptions {
            when: When::Always,
            ..Default::default()
        },
        move |_| finish_if_available(),
    );
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
    let editor_events = signal(0_u64);
    let on_change = move |_: &_| {
        draft_body.set(doc_markdown(&change_session));
        mark_edited();
        editor_events.update(|events| *events += 1);
    };
    let on_composition = move |composing_now: &bool| {
        composing.set(*composing_now);
    };
    // Re-read suggestion state on every change/selection event.
    let on_activity = move |_: &_| {
        editor_events.update(|events| *events += 1);
    };
    let blank = computed(move || draft_body.with(|body| body.trim().is_empty()));
    let book_id = document.book_id;
    let book_name = computed(move || {
        store
            .knowledge_base(book_id)
            .map_or_else(|| "知识库".to_owned(), |book| book.name)
    });

    view()
        .flex_1()
        .min_h(px(0.0))
        .flex_col()
        .child(editing_header(
            book_name,
            save_state,
            history_open,
            finish,
            retry,
        ))
        .child(toolbar(
            session.clone(),
            editor_events,
            composing,
            promote_open,
            card_picker_open,
            move || open_card_picker(false),
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
            card_from_slash,
            card_filter,
        ))
        .child(mention_panel(
            store,
            session.clone(),
            editor_events,
            composing,
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
                                .pt_8()
                                .pb_4()
                                .px_10()
                                .style_when(|| window().width.0 < 960.0, |s| s.px_4().pt_4())
                                .gap_4()
                                .child(editor_ui::paper_title(move || draft_title.get(), on_title))
                                .child(
                                    editor_ui::paper_body()
                                        .key(format!("editing-{document_id}"))
                                        .editor(session.clone())
                                        .auto_focus(true)
                                        .flex_1()
                                        .min_h(px(0.0))
                                        .style_when(blank, |s| s.flex_none().min_h(px(72.0)))
                                        .accessibility_label("文档正文")
                                        .placeholder("开始写作，或输入 / 插入内容")
                                        .on_focus(move || body_focused.set(true))
                                        .on_blur(move || body_focused.set(false))
                                        .bubble_menu_placement(Placement::Bottom)
                                        .bubble_menu(super::insert::caret_menu(
                                            store,
                                            session.clone(),
                                            body_focused,
                                            composing,
                                            move || open_card_picker(true),
                                            |session, id| {
                                                insert_card_at_suggestion(session, SLASH_KEY, id)
                                            },
                                        ))
                                        .on_change(on_change)
                                        .on_selection_change(on_activity)
                                        .on_composition_change(on_composition),
                                )
                                .child(blank_templates(session.clone(), blank)),
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

fn blank_templates(session: EditorSession, blank: Memo<bool>) -> impl IntoElement {
    when(blank, move || {
        view()
            .gap_2()
            .pt_4()
            .border_t_1()
            .border_color(tokens::border_subtle)
            .child(
                text("从模板开始")
                    .text_size(tokens::text_px(13.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(tokens::fg_primary),
            )
            .child(
                text("选择一个模板，直接插入常用章节。")
                    .text_size(tokens::text_px(12.0))
                    .text_color(tokens::fg_muted),
            )
            .child(
                view()
                    .flex_row()
                    .flex_wrap()
                    .gap_2()
                    .children(TEMPLATES.map(|template| {
                        let session = session.clone();
                        editor_ui::template_card(template.title, template.sections, move || {
                            if !doc_markdown(&session).trim().is_empty() {
                                return;
                            }
                            let state = session.state();
                            let Ok(content) = MarkdownFormat::default()
                                .parse(state.schema(), &template.markdown())
                            else {
                                return;
                            };
                            let mut transaction = state.tr();
                            if transaction
                                .replace_with(
                                    0,
                                    state.doc().content().size(),
                                    content.content().clone(),
                                )
                                .is_ok()
                                && session.dispatch(transaction).is_ok()
                            {
                                session.focus();
                            }
                        })
                    })),
            )
    })
}

fn editing_header(
    book_name: Memo<String>,
    save_state: Signal<SaveState>,
    history_open: Signal<bool>,
    finish: impl Fn() + Copy + 'static,
    retry: impl Fn() + Copy + 'static,
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
            view()
                .flex_1()
                .min_w(px(0.0))
                .flex_row()
                .items_center()
                .gap_2()
                .text_color(tokens::fg_muted)
                .child(icon(|| Icon::Book, px(16.0)))
                .child(
                    text(move || book_name.get())
                        .text_size(tokens::text_px(13.0))
                        .number_of_lines(1)
                        .flex_1(),
                ),
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
        .child(
            text(label)
                .text_size(tokens::text_px(12.0))
                .text_color(move || {
                    if save_state.get() == SaveState::Failed {
                        tokens::danger()
                    } else {
                        tokens::fg_muted()
                    }
                }),
        )
        .child(when(
            move || save_state.get() == SaveState::Failed,
            move || app_button(ButtonKind::Soft, "重试", retry).flex_none(),
        ))
}

/// One action description drives both the toolbar and its overflow menu.
#[derive(Clone)]
struct FormatAction {
    label: &'static str,
    icon: Icon,
    command: BoxCommand,
    active: Option<&'static str>,
}

impl FormatAction {
    fn new(label: &'static str, icon: Icon, command: BoxCommand) -> Self {
        Self {
            label,
            icon,
            command,
            active: None,
        }
    }

    fn toggle(mut self, name: &'static str) -> Self {
        self.active = Some(name);
        self
    }

    fn button(
        &self,
        session: &EditorSession,
        changed: Memo<u64>,
        composing: Signal<bool>,
    ) -> AnyElement {
        let check = session.clone();
        let active = self.active;
        let can = self.command.clone();
        let enable = session.clone();
        let apply = session.clone();
        let command = self.command.clone();
        editor_ui::format_button(
            self.label,
            self.icon,
            move || {
                changed.get();
                active.is_some_and(|name| check.is_active(Some(name), None))
            },
            move || {
                changed.get();
                composing.get() || !enable.can(&can)
            },
            move || {
                apply.run(&command);
                apply.focus();
            },
        )
        .into_element()
    }

    fn item(
        &self,
        session: &EditorSession,
        changed: Memo<u64>,
        composing: Signal<bool>,
    ) -> AnyElement {
        let enable = session.clone();
        let can = self.command.clone();
        let apply = session.clone();
        let command = self.command.clone();
        let owner = Owner::current();
        editor_ui::format_item(
            self.label,
            self.icon,
            move || {
                changed.get();
                composing.get() || !enable.can(&can)
            },
            move || {
                apply.run(&command);
                let apply = apply.clone();
                // NSH returns focus to the menu trigger on close. The editor
                // takes it back on the next turn, after the menu has closed.
                owner.run(|| set_timeout(Duration::ZERO, move || apply.focus()));
            },
        )
    }
}

const HEADINGS: [(u64, &str); 4] = [
    (1, "一级标题"),
    (2, "二级标题"),
    (3, "三级标题"),
    (4, "四级标题"),
];

fn heading_level(level: u64) -> nsh_base::rich_editor::model::Attrs {
    [("level".to_owned(), json!(level))].into_iter().collect()
}

fn toolbar(
    session: EditorSession,
    editor_events: Signal<u64>,
    composing: Signal<bool>,
    promote_open: Signal<bool>,
    card_picker_open: Signal<bool>,
    open_card_picker: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let state = session.state();
    let schema = state.schema();
    let node = |name: &str| schema.node_type(name).expect("registered editor node");
    let attrs = nsh_base::rich_editor::model::Attrs::new;
    let changed = computed(move || editor_events.get());
    let paragraph = FormatAction::new(
        "正文",
        Icon::Note,
        tiptap::set_node(node("paragraph"), attrs()),
    );
    let headings = HEADINGS.map(|(level, label)| {
        FormatAction::new(
            label,
            Icon::Heading,
            tiptap::set_node(node("heading"), heading_level(level)),
        )
    });
    let [bold, italic, underline, strike, code] = [
        ("加粗", Icon::Bold, "bold"),
        ("斜体", Icon::Italic, "italic"),
        ("下划线", Icon::Underline, "underline"),
        ("删除线", Icon::Strike, "strike"),
        ("行内代码", Icon::Code, "code"),
    ]
    .map(|(label, icon, name)| {
        FormatAction::new(
            label,
            icon,
            toggle_mark(
                schema.mark_type(name).expect("registered editor mark"),
                None,
                true,
            ),
        )
        .toggle(name)
    });
    let [bullets, ordered, checklist] = [
        ("无序列表", Icon::Bullets, "bulletList", "listItem"),
        ("有序列表", Icon::Ordered, "orderedList", "listItem"),
        ("清单", Icon::Tasks, "taskList", "taskItem"),
    ]
    .map(|(label, icon, list, item)| {
        FormatAction::new(
            label,
            icon,
            tiptap::toggle_list(node(list), node(item), false, attrs()),
        )
        .toggle(list)
    });
    let quote = FormatAction::new(
        "引用",
        Icon::Quote,
        tiptap::toggle_wrap(node("blockquote"), attrs()),
    )
    .toggle("blockquote");
    let code_block = FormatAction::new(
        "代码块",
        Icon::CodeBlock,
        tiptap::toggle_node(
            node("codeBlock"),
            node("paragraph"),
            [("language".to_owned(), Value::Null)].into_iter().collect(),
        ),
    )
    .toggle("codeBlock");
    let table = FormatAction::new("表格", Icon::Table, insert_table(3, 2, true));
    let clear = FormatAction::new(
        "清除格式",
        Icon::ClearFormat,
        chain(vec![tiptap::unset_all_marks(), tiptap::clear_nodes()]),
    );
    drop(state);

    let heading_session = session.clone();
    let current_heading = move || {
        changed.get();
        HEADINGS
            .into_iter()
            .find(|&(level, _)| {
                heading_session.is_active(Some("heading"), Some(heading_level(level)))
            })
            .map_or("正文", |(_, label)| label)
    };
    let promote_session = session.clone();
    let in_checklist = move || {
        changed.get();
        !composing.get() && selection_task_item(&promote_session).is_some()
    };
    let compact = signal(true);
    let menu_item = |action: &FormatAction| action.item(&session, changed, composing);
    let direct = |action: &FormatAction| action.button(&session, changed, composing);
    let marks = [underline, strike, code];
    let blocks = [bullets, ordered, checklist, quote, code_block, table, clear];
    let extra_session = session.clone();

    view()
        .test_id("document-format-toolbar")
        // Reserve room for the contextual checklist action as well as the fixed icons.
        .on_layout(move |frame: &Bounds<Points>| compact.set(frame.size.width.0 < 704.0))
        .bind_motion(&crate::ui::motion::toolbar_enter())
        .h(px(44.0))
        .flex_none()
        .flex_row()
        .items_center()
        .gap_0p5()
        .px_4()
        .bg(tokens::panel_bg)
        .border_b_1()
        .border_color(tokens::border_subtle)
        .child(super::insert::toolbar_panel(
            session.clone(),
            composing,
            open_card_picker,
        ))
        .child(editor_ui::divider())
        .child(direct(&FormatAction::new(
            "撤销",
            Icon::Undo,
            command(undo),
        )))
        .child(direct(&FormatAction::new(
            "重做",
            Icon::Redo,
            command(redo),
        )))
        .child(editor_ui::divider())
        .child(editor_ui::format_menu(
            editor_ui::tool_face(false)
                .w(px(84.0))
                .px_2()
                .justify_between()
                .label("段落样式")
                .child(
                    text(current_heading)
                        .text_size(tokens::text_px(13.0))
                        .number_of_lines(1),
                )
                .child(icon(|| Icon::Down, px(13.0))),
            std::iter::once(menu_item(&paragraph)).chain(headings.iter().map(menu_item)),
        ))
        .child(editor_ui::divider())
        .child(direct(&bold))
        .child(direct(&italic))
        .child(dynamic(move || {
            if compact.get() {
                editor_ui::format_menu(
                    editor_ui::tool_face(false)
                        .label("更多格式")
                        .child(icon(|| Icon::More, px(16.0)))
                        .child(hint("更多格式")),
                    marks
                        .iter()
                        .chain(&blocks)
                        .map(|action| action.item(&extra_session, changed, composing)),
                )
                .into_element()
            } else {
                view()
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .gap_0p5()
                    .children(
                        marks
                            .iter()
                            .map(|action| action.button(&extra_session, changed, composing)),
                    )
                    .child(editor_ui::divider())
                    .children(
                        blocks
                            .iter()
                            .map(|action| action.button(&extra_session, changed, composing)),
                    )
                    .into_element()
            }
        }))
        .child(when(in_checklist, move || {
            app_button(ButtonKind::Soft, "转为任务", move || {
                promote_open.update(|open| *open = !*open);
                card_picker_open.set(false);
            })
            .flex_none()
        }))
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
            let inbox_session = session.clone();
            let projects_session = session.clone();
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
                .child(
                    text("新任务存入")
                        .text_size(tokens::text_px(12.0))
                        .text_color(tokens::fg_muted()),
                )
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
                // A `when` branch builds once per flip, so the project
                // list below is a dynamic region: it follows workspace
                // changes while the chooser stays open.
                .child(dynamic(move || {
                    view()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .flex_wrap()
                        .children(store.projects().into_iter().map(|project| {
                            let pick_session = projects_session.clone();
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
    let Ok(resolved) = state.doc().resolve(state.selection().from()) else {
        return;
    };
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
    let Ok(card) = schema.node_from_json(&json!({"type": CARD_NODE, "attrs": {"taskId": 0}}))
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
    from_slash: Signal<bool>,
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
            let field = element_ref();
            let return_focus = session.clone();
            on_key_with(
                "escape",
                nsh_core::element::KeyOptions {
                    when: nsh_core::keymap::When::Always,
                    ..Default::default()
                },
                move |_| {
                    if from_slash.get_untracked() {
                        return_focus.run(
                            &nsh_base::rich_editor::extensions::suggestion::exit_suggestion(
                                SLASH_KEY,
                            ),
                        );
                    }
                    open.set(false);
                    return_focus.focus();
                },
            );
            spawn(async move { field.focus() });
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
                        .element_ref(&field)
                        .value(move || filter.get())
                        .on_change(move |text: &str| filter.set(text.to_owned()))
                        .placeholder("搜索任务，点选插入关联任务卡片")
                        .placeholder_color(move || color(nsh_base::theme().muted_foreground))
                        .h(px(30.0))
                        .px_2()
                        .text_size(tokens::text_px(13.0))
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
                                    if from_slash.get_untracked() {
                                        insert_card_at_suggestion(&pick_session, SLASH_KEY, id);
                                    } else {
                                        insert_card_block(&pick_session, id);
                                    }
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
    let Some(suggestion) = suggestion_state(&state, key).filter(|s| s.active) else {
        return;
    };
    let schema = state.schema().clone();
    let Ok(card) = schema.node_from_json(&json!({"type": CARD_NODE, "attrs": {"taskId": task_id}}))
    else {
        return;
    };
    let range = suggestion.range;
    let mut tr = state.tr();
    if tr
        .replace_with(range.from, range.to, Fragment::from(card.clone()))
        .is_err()
    {
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

fn mention_items(
    store: Store,
    session: &EditorSession,
    editor_events: Signal<u64>,
    composing: Signal<bool>,
) -> Vec<MentionItem> {
    editor_events.get();
    if composing.get() {
        return Vec::new();
    }
    let state = session.state();
    let Some(suggestion) = suggestion_state(&state, MENTION_KEY) else {
        return Vec::new();
    };
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
                        // The key is the target's identity, not its label:
                        // two Tasks can share a title, and a duplicated
                        // label would collide in the row list.
                        |item: &MentionItem| item.id(),
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
