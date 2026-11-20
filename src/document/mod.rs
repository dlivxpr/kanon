//! The Document screen (docs/ui-design.md §4.4, issue #6): distraction-free
//! reading, single-owner editing with continuous auto-save, local history,
//! and the single-editor lock across workbench tabs.
//!
//! A tab is either reading a Document or editing it; the app-wide
//! [`Store::editor_lock`] decides which chrome renders, so requesting Edit
//! from a second tab focuses the tab that already holds the lock.

mod editing;
mod history;
mod reading;

use nsh_base::rich_editor::{
    extensions::{
        extension::{Extension, ExtensionManager},
        kit::{starter_kit, table_kit},
        suggestion::{SuggestionOptions, suggestion},
    },
    formats::{DocFormat, markdown::MarkdownFormat},
    model::{Fragment, Node},
    state::{EditorState, EditorStateConfig},
    view::{EditorSession, NodeViews},
};
use nsh_core::prelude::*;

use crate::shell::Store;

pub(crate) const SLASH_KEY: &str = "slash";

/// One Document tab: reading chrome or editing chrome, by lock ownership.
pub fn document_view(store: Store, tab_id: u64, document_id: i64) -> impl IntoElement {
    let is_editor = move || {
        store
            .editor_lock
            .get()
            .is_some_and(|lock| lock.tab_id == tab_id && lock.document_id == document_id)
    };
    dynamic(move || {
        if is_editor() {
            editing::editing_view(store, tab_id, document_id).into_element()
        } else {
            reading::reading_view(store, tab_id, document_id).into_element()
        }
    })
}

/// Kanon's editor kernel: the starter kit (headings, lists, local
/// Checklist Items, marks, history, Markdown shortcuts) plus tables and
/// the `/` block-insertion suggestion.
fn build_extensions() -> ExtensionManager {
    let mut slash = SuggestionOptions::default();
    slash.key = SLASH_KEY.into();
    slash.trigger.char = "/".into();
    slash.trigger.allowed_prefixes = None;
    ExtensionManager::with_core(vec![
        starter_kit(),
        table_kit(),
        Extension::new("kanonSlash").with_priority(1000).add_plugins(|_| vec![suggestion(slash)]),
    ])
    .expect("the kanon editor extensions build")
}

/// A session over the committed markdown body. `node_views` lets the
/// caller take over the rendering of chosen node types (the reading tab
/// renders Checklist Item checkboxes itself).
pub(crate) fn build_session(body: &str, node_views: NodeViews) -> EditorSession {
    let manager = build_extensions();
    let schema = manager.schema().clone();
    let format = MarkdownFormat::default();
    let doc = format
        .parse(&schema, body)
        .unwrap_or_else(|_| schema.top_node_type().create(None, Fragment::empty(), &[]).expect("empty doc"));
    let state =
        EditorState::create(EditorStateConfig::new(schema).doc(doc).plugins(manager.plugins()))
            .expect("the editor state builds");
    EditorSession::new(state, node_views)
}

/// The markdown a session's document serializes to (the auto-save payload).
pub(crate) fn doc_markdown(session: &EditorSession) -> String {
    MarkdownFormat::default().serialize_fragment(&session.state().doc().content())
}

/// Where the `ordinal`-th heading starts in `doc` (ProseMirror position),
/// counting only top-level blocks: headings never nest inside one another
/// at the top level, so a flat walk is exact.
pub(crate) fn heading_position(doc: &Node, ordinal: usize) -> Option<usize> {
    let mut seen = 0_usize;
    let mut pos = 0_usize;
    for child in doc.children() {
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
    let Some(pos) = heading_position(&state.doc(), ordinal) else { return };
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
        let doc = session.state().doc();
        let sizes = doc.children().map(|child| child.node_size()).collect::<Vec<_>>();
        assert_eq!(sizes.len(), 4);
        assert_eq!(heading_position(&doc, 0), Some(0));
        assert_eq!(heading_position(&doc, 1), Some(sizes[0] + sizes[1]));
        assert_eq!(heading_position(&doc, 2), Some(sizes[0] + sizes[1] + sizes[2]));
        assert_eq!(heading_position(&doc, 3), None);
    }

    #[test]
    fn session_round_trips_markdown_bodies() {
        let body = "# 标题\n\n- [ ] 清单项\n- [x] 已完成\n\n```\n# not a heading\n```\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";
        let session = build_session(body, NodeViews::new());
        let markdown = doc_markdown(&session);
        assert!(markdown.contains("# 标题"), "headings serialize");
        assert!(markdown.contains("- [ ] 清单项") && markdown.contains("- [x] 已完成"), "checklist states serialize");
        assert!(markdown.contains("# not a heading"), "code fences keep their text");
        fn has_table(node: &Node) -> bool {
            node.ty().name() == "table" || node.children().any(|child| has_table(&child))
        }
        assert!(has_table(&session.state().doc()), "tables parse into table nodes");
    }
}
