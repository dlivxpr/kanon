//! The Kanon archive codec for editor-only constructs: Linked Task Cards
//! and inline `@` mentions survive the committed markdown body as text
//! markers (domain-owned formats, see [`crate::domain`]). Parsing swaps
//! marker paragraphs for interactive card nodes and marker text runs for
//! mention nodes; serializing swaps them back before the markdown writer
//! runs, so the standard blocks keep their exact markdown round-trip.

use nsh_base::rich_editor::model::{Attrs, Fragment, Node, NodeSpec, Schema};
use serde_json::json;

use crate::domain::{parse_mention_marker, parse_task_card_marker, mention_marker, task_card_marker};

/// The Linked Task Card block node (`group: "block"`, an atom carrying
/// the referenced Task's id).
pub(crate) const CARD_NODE: &str = "taskCard";

/// The inline `@` mention node (the stock tiptap mention).
pub(crate) const MENTION_NODE: &str = "mention";

/// The block node spec of a Linked Task Card.
pub(crate) fn card_spec() -> NodeSpec {
    let mut spec = NodeSpec::default();
    spec.group = Some("block".into());
    spec.atom = true;
    spec.attrs = vec![("taskId".into(), Default::default())];
    spec
}

/// Swaps archive markers in the parsed `doc` for editor nodes: a
/// top-level paragraph holding exactly one card marker becomes a Linked
/// Task Card; `@m{…}(…)` text runs become mention nodes (inline, anywhere
/// text flows).
pub(crate) fn materialize_markers(schema: &Schema, doc: &Node) -> Node {
    let children = doc
        .content()
        .iter()
        .map(|child| materialize_node(schema, child, true))
        .collect::<Vec<_>>();
    doc.copy(Fragment::from(children))
}

fn materialize_node(schema: &Schema, node: &Node, top_level: bool) -> Node {
    if top_level
        && node.ty().name() == "paragraph"
        && let Some(task_id) = parse_task_card_marker(&node.text_content())
        && let Some(ty) = schema.node_type(CARD_NODE)
    {
        let attrs: Attrs =
            [("taskId".to_owned(), json!(task_id))].into_iter().collect();
        if let Ok(card) = ty.create(Some(attrs), Fragment::empty(), vec![]) {
            return card;
        }
    }
    let children = node
        .content()
        .iter()
        .flat_map(|child| {
            if child.text().is_some() {
                split_mentions(schema, child)
            } else {
                vec![materialize_node(schema, child, false)]
            }
        })
        .collect::<Vec<_>>();
    node.copy(Fragment::from(children))
}

/// Splits one text node into text and mention nodes at every valid
/// mention marker; text that almost looks like a marker stays text.
fn split_mentions(schema: &Schema, text: &Node) -> Vec<Node> {
    let Some(content) = text.text() else { return vec![text.clone()] };
    let marks = text.marks().to_vec();
    let mut out = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("@m{") {
        match parse_mention_marker(&rest[start..]) {
            Some((parsed, byte_len)) => {
                if start > 0 {
                    out.push(schema.text(&rest[..start], marks.clone()));
                }
                let id = format!("{}:{}", parsed.kind, parsed.id_part);
                let attrs: Attrs = [
                    ("id".to_owned(), json!(id)),
                    ("label".to_owned(), json!(parsed.label)),
                ]
                .into_iter()
                .collect();
                match schema.node_type(MENTION_NODE).and_then(|ty| ty.create(Some(attrs), Fragment::empty(), vec![]).ok()) {
                    Some(mention) => out.push(mention),
                    None => out.push(schema.text(&rest[start..start + byte_len], marks.clone())),
                }
                rest = &rest[start + byte_len..];
            }
            None => {
                out.push(schema.text(&rest[..start + 1], marks.clone()));
                rest = &rest[start + 1..];
            }
        }
    }
    if !rest.is_empty() {
        out.push(schema.text(rest, marks));
    }
    out
}

/// Swaps editor-only nodes in `doc` back to their archive markers, the
/// inverse of [`materialize_markers`]; the result feeds the markdown
/// serializer.
pub(crate) fn dematerialize_markers(schema: &Schema, doc: &Node) -> Node {
    let children = doc
        .content()
        .iter()
        .map(|child| dematerialize_node(schema, child))
        .collect::<Vec<_>>();
    doc.copy(Fragment::from(children))
}

fn dematerialize_node(schema: &Schema, node: &Node) -> Node {
    if node.text().is_some() {
        return node.clone();
    }
    if node.ty().name() == CARD_NODE {
        let task_id = node.attrs().get("taskId").and_then(|v| v.as_i64()).unwrap_or(0);
        let marker = task_card_marker(task_id);
        let paragraph = schema.node_type("paragraph").expect("the paragraph type");
        return paragraph
            .create(None, Fragment::from(schema.text(&marker, vec![])), vec![])
            .expect("a marker paragraph builds");
    }
    let children = node
        .content()
        .iter()
        .map(|child| {
            if child.ty().name() == MENTION_NODE {
                let attrs = child.attrs();
                let id = attrs.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let label = attrs.get("label").and_then(|v| v.as_str()).unwrap_or("");
                let (kind, id_part) = id.split_once(':').unwrap_or(("", id));
                let marker = mention_marker(kind, id_part, label);
                schema.text(&marker, child.marks().to_vec())
            } else {
                dematerialize_node(schema, child)
            }
        })
        .collect::<Vec<_>>();
    node.copy(Fragment::from(children))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nsh_base::rich_editor::view::NodeViews;
    use nsh_core::prelude::*;

    use super::*;
    use crate::document::{build_session, doc_markdown};

    fn node_names(node: &Node) -> Vec<String> {
        let mut out = vec![node.ty().name().to_owned()];
        for child in node.content().iter() {
            out.extend(node_names(child));
        }
        out
    }

    #[test]
    fn card_and_mention_markers_round_trip_through_sessions() {
        let body = "前言\n\n@task(3)\n\n- [ ] 普通项\n\n前文@m{document:7}(手册)后文@m{date:2026-11-20}(2026-11-20)";
        let session = build_session(body, NodeViews::new().with(CARD_NODE, Arc::new(ProbeCard)));
        let doc = session.state().doc().clone();
        let all = node_names(&doc);
        assert!(all.contains(&"taskCard".to_owned()), "the card marker becomes a card node: {all:?}");
        let mentions = all.iter().filter(|name| *name == "mention").count();
        assert_eq!(mentions, 2, "both mention markers become mention nodes: {all:?}");
        assert!(all.contains(&"taskItem".to_owned()), "the plain checklist item stays one");

        let serialized = doc_markdown(&session);
        assert_eq!(serialized, body, "the kanon serialization restores the archive body byte for byte");
    }

    #[test]
    fn plain_bodies_round_trip_unchanged() {
        // Tables keep their loose (padded) pipe form and the writer has
        // no trailing newline — both pre-existing markdown behaviors the
        // app already lived with; block constructs round-trip exactly.
        let body = "# 标题\n\n- [ ] 清单项\n- [x] 已完成\n\n```\n# not a heading\n```\n\n> 引用";
        let session = build_session(body, NodeViews::new());
        assert_eq!(doc_markdown(&session), body);
    }

    struct ProbeCard;

    impl nsh_base::rich_editor::view::NodeView for ProbeCard {
        fn create(&self, _cx: &nsh_base::rich_editor::view::NodeViewCx, node: &Node) -> AnyElement {
            let task_id = node.attrs().get("taskId").and_then(|v| v.as_i64()).unwrap_or(0);
            view()
                .child(text(format!("card {task_id}")))
                .into_element()
        }
    }
}
