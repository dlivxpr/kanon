//! The insertion catalog, shared by the toolbar and the caret suggestions.

use std::sync::Arc;

use nsh_base::rich_editor::{
    commands::tiptap,
    extensions::suggestion::{replace_suggestion, set_suggestion_open, suggestion_state},
    model::Fragment,
    view::{EditorSession, SessionEventKind},
};
use nsh_core::prelude::*;
use serde_json::{Value, json};

use crate::shell::Store;
use crate::ui::{chrome::Icon, editor as ui};

use super::SLASH_KEY;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Block {
    Heading(u8),
    Bullets,
    Ordered,
    Checklist,
    Quote,
    Code,
    Table,
    Divider,
    LinkedTask,
}

const GROUPS: [(&str, &[Block]); 3] = [
    (
        "文本结构",
        &[
            Block::Heading(1),
            Block::Heading(2),
            Block::Heading(3),
            Block::Heading(4),
            Block::Quote,
            Block::Divider,
        ],
    ),
    ("列表", &[Block::Bullets, Block::Ordered, Block::Checklist]),
    ("内容卡片", &[Block::Table, Block::Code, Block::LinkedTask]),
];

impl Block {
    fn presentation(self) -> (&'static str, Icon, &'static str) {
        match self {
            Self::Heading(1) => ("一级标题", Icon::Heading, "h1 heading"),
            Self::Heading(2) => ("二级标题", Icon::Heading, "h2 heading"),
            Self::Heading(3) => ("三级标题", Icon::Heading, "h3 heading"),
            Self::Heading(_) => ("四级标题", Icon::Heading, "h4 heading"),
            Self::Bullets => ("无序列表", Icon::Bullets, "ul bullet list"),
            Self::Ordered => ("有序列表", Icon::Ordered, "ol ordered list"),
            Self::Checklist => ("清单", Icon::Tasks, "todo checklist"),
            Self::Quote => ("引用", Icon::Quote, "quote"),
            Self::Code => ("代码块", Icon::CodeBlock, "code"),
            Self::Table => ("表格", Icon::Table, "table"),
            Self::Divider => ("分割线", Icon::Divider, "hr divider"),
            Self::LinkedTask => ("关联任务卡片", Icon::Links, "task link"),
        }
    }

    fn matches(self, query: &str) -> bool {
        let (label, _, aliases) = self.presentation();
        label.contains(query)
            || aliases
                .split_whitespace()
                .any(|alias| alias.starts_with(query))
    }

    fn content(self) -> Value {
        let p = || json!({"type": "paragraph"});
        let list =
            |kind, item| json!({"type": kind, "content": [{"type": item, "content": [p()]}]});
        match self {
            Self::Heading(level) => json!({"type": "heading", "attrs": {"level": level}}),
            Self::Bullets => list("bulletList", "listItem"),
            Self::Ordered => list("orderedList", "listItem"),
            Self::Checklist => list("taskList", "taskItem"),
            Self::Quote => json!({"type": "blockquote", "content": [p()]}),
            Self::Code => json!({"type": "codeBlock"}),
            Self::Table => {
                let cell = |kind| json!({"type": kind, "content": [p()]});
                let row = |kind| json!({"type": "tableRow", "content": [cell(kind), cell(kind)]});
                json!({"type": "table", "content": [row("tableHeader"), row("tableCell"), row("tableCell")]})
            }
            Self::Divider => json!({"type": "horizontalRule"}),
            Self::LinkedTask => unreachable!("linked tasks open the task picker"),
        }
    }
}

fn apply_block(session: &EditorSession, block: Block, slash: bool) {
    let state = session.state();
    let Ok(node) = state.schema().node_from_json(&block.content()) else {
        return;
    };
    let content = Fragment::from(node);
    if slash {
        session.run(&replace_suggestion(SLASH_KEY, content));
    } else {
        session.run(&tiptap::insert_content(content, Default::default()));
    }
    session.focus();
}

pub(super) fn toolbar_panel(
    session: EditorSession,
    composing: Signal<bool>,
    open_task_picker: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let open = signal(false);
    let query = signal(String::new());
    let selected = signal(0_usize);
    let focus = element_ref();
    let matches = computed(move || {
        let query = query.with(|q| q.trim().to_ascii_lowercase());
        GROUPS
            .into_iter()
            .flat_map(|(_, blocks)| blocks)
            .copied()
            .filter(|block| block.matches(&query))
            .collect::<Vec<_>>()
    });
    let apply = stored(move |block| {
        open.set(false);
        if block == Block::LinkedTask {
            open_task_picker();
        } else {
            // Popover returns focus to its trigger on closing. Focus the editor after that commit.
            let session = session.clone();
            spawn(async move { apply_block(&session, block, false) });
        }
    });
    let options = nsh_core::element::KeyOptions {
        when: nsh_core::keymap::When::Always,
        for_focus: true,
        ..Default::default()
    };
    effect(move || {
        if !open.get() {
            return;
        }
        for (key, forward) in [("up", false), ("down", true)] {
            on_key_with(key, options, move |_| {
                step(selected, matches.with(Vec::len), forward)
            });
        }
    });
    ui::insert_popover(focus, open, composing)
        .on_open_change(move |event: &nsh_base::PopoverEvent| {
            let nsh_base::PopoverEvent::OpenChange(next, _) = event else {
                return;
            };
            if *next {
                query.set(String::new());
                selected.set(0);
            }
            open.set(*next);
        })
        .child(ui::insert_search(focus, query, selected, move || {
            if let Some(block) = matches.with(|rows| rows.get(selected.get_untracked()).copied()) {
                apply.with(|run| run(block));
            }
        }))
        .child(dynamic(move || {
            let mut index = 0;
            let groups = matches.with(|visible| {
                GROUPS.map(|(label, blocks)| {
                    let rows = blocks
                        .iter()
                        .copied()
                        .filter(|block| visible.contains(block))
                        .map(|block| {
                            let (label, glyph, _) = block.presentation();
                            let row = index;
                            index += 1;
                            ui::insert_row(
                                label,
                                glyph,
                                true,
                                move || selected.get() == row,
                                move || apply.with(|run| run(block)),
                            )
                        })
                        .collect::<Vec<_>>();
                    (!rows.is_empty()).then(|| ui::insert_group(label, rows))
                })
            });
            if index == 0 {
                ui::insert_empty().into_element()
            } else {
                ui::insert_catalog(groups.into_iter().flatten()).into_element()
            }
        }))
}

#[derive(Clone, PartialEq, Eq)]
enum Choice {
    Block(Block),
    Task(i64, String),
}

fn step(selected: Signal<usize>, len: usize, forward: bool) {
    if len > 0 {
        let index = selected.get_untracked().min(len - 1);
        selected.set(if forward {
            (index + 1) % len
        } else {
            (index + len - 1) % len
        });
    }
}

pub(super) fn caret_menu(
    store: Store,
    session: EditorSession,
    focused: Signal<bool>,
    composing: Signal<bool>,
    open_task_picker: impl Fn() + Copy + 'static,
    insert_task: impl Fn(&EditorSession, i64) + Copy + 'static,
) -> impl IntoElement {
    let revision = signal(0_u64);
    // Escape and navigation change only plugin state, not the document or selection.
    let listener = session.on(
        SessionEventKind::Transaction,
        Arc::new(move |_| revision.update(|r| *r += 1)),
    );
    let subscribed = session.clone();
    cleanup(move || subscribed.off(listener));

    let read = session.clone();
    let suggestion = computed(move || {
        revision.get();
        suggestion_state(&read.state(), SLASH_KEY).filter(|s| s.active)
    });
    let query = computed(move || {
        suggestion.with(|s| {
            s.as_ref()
                .map(|s| s.query.as_deref().unwrap_or("").trim().to_ascii_lowercase())
        })
    });
    let matches = computed(move || {
        let mut rows = Vec::new();
        query.with(|query| {
            let Some(query) = query else { return };
            rows.extend(
                GROUPS
                    .into_iter()
                    .flat_map(|(_, blocks)| blocks)
                    .copied()
                    .filter(|block| block.matches(query))
                    .map(Choice::Block),
            );
            if !query.is_empty() {
                rows.extend(
                    store
                        .live_tasks()
                        .into_iter()
                        .filter(|row| row.task.title.to_lowercase().contains(query))
                        .take(4)
                        .map(|row| Choice::Task(row.task.id, row.task.title)),
                );
            }
        });
        rows
    });
    let selected = signal(0_usize);
    let show = computed(move || focused.get() && !composing.get() && !matches.with(Vec::is_empty));
    let choose = {
        let session = session.clone();
        move |choice| match choice {
            Choice::Block(Block::LinkedTask) => {
                // The picker must replace the trigger text at this caret, not append to it.
                open_task_picker();
            }
            Choice::Block(block) => apply_block(&session, block, true),
            Choice::Task(id, _) => insert_task(&session, id),
        }
    };
    let choose = stored(choose);
    let last_key = stored(0_u64);
    let set_open = session.clone();
    effect(move || {
        query.get();
        selected.set(0);
    });
    effect(move || {
        let showing = show.get();
        let event = suggestion.with(|s| s.as_ref().map(|s| (s.key_count, s.last_key.clone())));
        untracked(|| {
            set_open.run(&set_suggestion_open(SLASH_KEY, showing));
            let Some((count, key)) = event else { return };
            if count == last_key.replace(count) || !showing {
                return;
            }
            match key.as_deref() {
                Some("ArrowUp") => step(selected, matches.with(Vec::len), false),
                Some("ArrowDown") => step(selected, matches.with(Vec::len), true),
                Some("Enter") => {
                    if let Some(row) =
                        matches.with(|rows| rows.get(selected.get_untracked()).cloned())
                    {
                        choose.with(|run| run(row));
                    }
                }
                _ => {}
            }
        });
    });
    when(show, move || {
        ui::slash_popup(dynamic(move || {
            let rows = matches.with(|rows| {
                rows.iter()
                    .cloned()
                    .enumerate()
                    .map(|(index, row)| {
                        let (label, glyph) = match &row {
                            Choice::Block(block) => {
                                let (label, glyph, _) = block.presentation();
                                (SharedString::from(label), glyph)
                            }
                            Choice::Task(_, title) => {
                                (SharedString::from(title.clone()), Icon::Links)
                            }
                        };
                        ui::insert_row(
                            label,
                            glyph,
                            false,
                            move || selected.get() == index,
                            move || choose.with(|run| run(row.clone())),
                        )
                    })
                    .collect::<Vec<_>>()
            });
            ui::insert_grid(rows)
        }))
    })
}
