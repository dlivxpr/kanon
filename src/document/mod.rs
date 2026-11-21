//! The Document screen (docs/ui-design.md §4.4, issue #6): distraction-free
//! reading, single-owner editing with continuous auto-save, local history,
//! and the single-editor lock across workbench tabs.
//!
//! A tab is either reading a Document or editing it; the app-wide
//! [`Store::editor_lock`] decides which chrome renders, so requesting Edit
//! from a second tab focuses the tab that already holds the lock.

mod editing;
mod export;
mod history;
mod insert;
pub(crate) mod kanon;
mod mention;
mod reading;
mod task_card;
pub(crate) mod templates;

use nsh_base::rich_editor::{
    extensions::{
        extension::{Extension, ExtensionManager},
        kit::{starter_kit_extensions, table_kit},
        suggestion::{SuggestionOptions, mention_node, suggestion},
    },
    formats::{DocFormat, MarkdownFormat, PlainTextFormat},
    model::Node,
    state::{EditorState, EditorStateConfig},
    view::{EditorSession, NodeViews},
};
use nsh_core::prelude::*;

use crate::shell::Store;

pub(crate) const SLASH_KEY: &str = "slash";
/// The `@` inline-mention suggestion's plugin key.
pub(crate) const MENTION_KEY: &str = "mention";
pub(crate) use kanon::CARD_NODE;

fn wide_paper(store: Store) -> bool {
    store
        .preference(crate::shell::store::PREF_DOCUMENT_WIDTH)
        .as_deref()
        == Some("wide")
}

/// One Document tab: reading chrome or editing chrome, by lock ownership.
pub fn document_view(store: Store, tab_id: u64, document_id: i64) -> impl IntoElement {
    let is_editor = move || {
        store
            .editor_lock
            .get()
            .iter()
            .any(|lock| lock.tab_id == tab_id && lock.document_id == document_id)
    };
    let edit = move || {
        if !is_editor()
            && store.document(document_id).is_some()
            && let crate::shell::store::EditOutcome::FocusOther(other) =
                store.request_edit(tab_id, document_id)
        {
            store.activate_tab_by_id(other);
        }
    };
    use nsh_core::{element::KeyOptions, keymap::When};
    on_key_with(
        "e",
        KeyOptions {
            when: When::NotEditing,
            ..Default::default()
        },
        move |_| edit(),
    );
    on_key_with(
        "mod+e",
        KeyOptions {
            when: When::Always,
            ..Default::default()
        },
        move |_| edit(),
    );
    dynamic(move || {
        // Only the lock selects the chrome; building the content reads
        // workspace signals (through store.document's bump) and must not
        // re-run this region.
        let editing = is_editor();
        untracked(move || {
            if editing {
                editing::editing_view(store, tab_id, document_id).into_element()
            } else {
                reading::reading_view(store, tab_id, document_id).into_element()
            }
        })
    })
}

/// Kanon's editor kernel: the starter kit (headings, lists, local
/// Checklist Items, marks, history, Markdown shortcuts) plus tables, the
/// `/` block-insertion suggestion, the Linked Task Card node and the `@`
/// inline-mention suggestion.
fn build_extensions(finish_requested: Option<Signal<u64>>) -> ExtensionManager {
    let mut extensions = vec![
        Extension::new("starterKit").add_extensions(|_| {
            starter_kit_extensions()
                .into_iter()
                .map(|(name, ext)| {
                    if name == "blockquote" {
                        // Markdown represents an empty quote as a block with no children.
                        let mut spec = ext.node_spec().expect("blockquote node spec");
                        spec.content = Some("block*".into());
                        ext.with_node_spec(move |_| spec.clone())
                    } else {
                        ext
                    }
                })
                .collect()
        }),
        table_kit(),
        Extension::node(kanon::CARD_NODE, |_| kanon::card_spec()),
        Extension::node(kanon::MENTION_NODE, |_| {
            mention_node(kanon::MENTION_NODE, "@").1
        }),
        Extension::new("kanonSlash")
            .with_priority(1000)
            .add_plugins(|_| {
                let mut slash = SuggestionOptions::default();
                slash.key = SLASH_KEY.into();
                slash.trigger.char = "/".into();
                slash.trigger.allowed_prefixes = None;
                slash.capture_keys = ["ArrowUp", "ArrowDown", "Enter"].map(str::to_owned).into();
                slash.allow = Some(std::sync::Arc::new(|state, range, _| {
                    state
                        .doc()
                        .resolve(range.from)
                        .is_ok_and(|pos| pos.parent().ty().name() != "codeBlock")
                }));
                vec![suggestion(slash)]
            }),
        Extension::new("kanonMention")
            .with_priority(1000)
            .add_plugins(|_| {
                let mut mention = SuggestionOptions::default();
                mention.key = MENTION_KEY.into();
                mention.trigger.char = "@".into();
                mention.trigger.allowed_prefixes = None;
                vec![suggestion(mention)]
            }),
    ];
    if let Some(requested) = finish_requested {
        // NSH's focused RichEditor takes modified characters before window shortcuts.
        // Handle Mod+E through its own keymap and let the view finish the save.
        extensions.push(
            Extension::new("kanonFinishEditing")
                .with_priority(1000)
                .add_keyboard_shortcuts(move |_| {
                    vec![(
                        "Mod-e".into(),
                        nsh_base::rich_editor::commands::command(move |_, dispatch| {
                            if dispatch.is_some() {
                                requested.update(|sequence| *sequence += 1);
                            }
                            true
                        }),
                    )]
                }),
        );
    }
    ExtensionManager::with_core(extensions).expect("the kanon editor extensions build")
}

/// A session over the committed markdown body. `node_views` lets the
/// caller take over the rendering of chosen node types (the reading tab
/// renders Checklist Item checkboxes itself; both tabs render Linked
/// Task Cards).
pub(crate) fn build_session(body: &str, node_views: NodeViews) -> EditorSession {
    build_session_with_shortcut(body, node_views, None)
}

fn build_session_with_shortcut(
    body: &str,
    node_views: NodeViews,
    finish_requested: Option<Signal<u64>>,
) -> EditorSession {
    let manager = build_extensions(finish_requested);
    let schema = manager.schema().clone();
    let format = MarkdownFormat::default();
    let doc = format.parse(&schema, body).unwrap_or_else(|err| {
        log::warn!("Cannot parse document Markdown; showing source as text: {err}");
        PlainTextFormat::default()
            .parse(&schema, body)
            .expect("the editor schema supports paragraphs of plain text")
    });
    let doc = kanon::materialize_markers(&schema, &doc);
    let state = EditorState::create(
        EditorStateConfig::new(schema)
            .doc(doc)
            .plugins(manager.plugins()),
    )
    .expect("the editor state builds");
    EditorSession::new(state, node_views)
}

/// The markdown a session's document serializes to (the auto-save
/// payload): editor-only nodes become their archive markers first, so
/// the committed body round-trips through [`build_session`].
pub(crate) fn doc_markdown(session: &EditorSession) -> String {
    let state = session.state();
    let doc = kanon::dematerialize_markers(state.schema(), state.doc());
    MarkdownFormat::default().serialize_fragment(doc.content())
}

/// Where the `ordinal`-th heading starts in `doc` (ProseMirror position),
/// counting only top-level blocks: headings never nest inside one another
/// at the top level, so a flat walk is exact.
pub(crate) fn heading_position(doc: &Node, ordinal: usize) -> Option<usize> {
    let mut seen = 0_usize;
    let mut pos = 0_usize;
    for child in doc.content().iter() {
        if child.ty().name() == "heading" {
            if seen == ordinal {
                return Some(pos);
            }
            seen += 1;
        }
        pos += child.node_size();
    }
    None
}

/// Reveals the `ordinal`-th heading (document order) by selecting inside
/// it and focusing: the view scrolls a focused selection into sight.
pub(crate) fn reveal_heading(session: &EditorSession, ordinal: usize) {
    use nsh_base::rich_editor::commands::tiptap;
    let state = session.state();
    let Some(pos) = heading_position(state.doc(), ordinal) else {
        return;
    };
    let at = (pos + 2).min(state.doc().node_size().saturating_sub(2));
    session.run(&tiptap::set_text_selection(at, at));
    session.focus();
}

#[cfg(test)]
mod tests {
    use super::*;
    use nsh_base::rich_editor::view::NodeViews;

    #[test]
    fn heading_positions_follow_document_order() {
        let session = build_session("# 一\n\n段落\n\n## 二\n\n### 三\n", NodeViews::new());
        let state = session.state();
        let doc = state.doc();
        let sizes = doc
            .content()
            .iter()
            .map(|child| child.node_size())
            .collect::<Vec<_>>();
        assert_eq!(sizes.len(), 4);
        assert_eq!(heading_position(doc, 0), Some(0));
        assert_eq!(heading_position(doc, 1), Some(sizes[0] + sizes[1]));
        assert_eq!(
            heading_position(doc, 2),
            Some(sizes[0] + sizes[1] + sizes[2])
        );
        assert_eq!(heading_position(doc, 3), None);
    }

    #[test]
    fn session_round_trips_markdown_bodies() {
        let body = "# 标题\n\n- [ ] 清单项\n- [x] 已完成\n\n```\n# not a heading\n```\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";
        let session = build_session(body, NodeViews::new());
        let markdown = doc_markdown(&session);
        assert!(markdown.contains("# 标题"), "headings serialize");
        assert!(
            markdown.contains("- [ ] 清单项") && markdown.contains("- [x] 已完成"),
            "checklist states serialize"
        );
        assert!(
            markdown.contains("# not a heading"),
            "code fences keep their text"
        );
        fn has_table(node: &Node) -> bool {
            node.ty().name() == "table" || node.content().iter().any(has_table)
        }
        assert!(
            has_table(session.state().doc()),
            "tables parse into table nodes"
        );
    }

    #[test]
    fn empty_quote_reopens_with_an_editable_paragraph_and_keeps_following_content() {
        let session = build_session(">\n\n# 后续标题\n", NodeViews::new());
        let state = session.state();
        let quote = state.doc().child(0);
        assert_eq!(quote.ty().name(), "blockquote");
        assert_eq!(quote.child(0).ty().name(), "paragraph");
        let reopened = build_session(&doc_markdown(&session), NodeViews::new());
        assert_eq!(reopened.state().doc().child(1).text_content(), "后续标题");
    }
}
