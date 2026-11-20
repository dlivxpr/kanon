//! The inline `@` mention node view (docs/ui-design.md §4.4, issue #7):
//! the chip that flows inside a paragraph. Live targets render their
//! current name; a target in Trash (or permanently deleted) renders the
//! invalid placeholder — it recovers automatically on restore because
//! the bindings re-run on every workspace change.

use nsh_base::{
    rich_editor::{
        model::Node,
        view::{NodeView, NodeViewCx},
    },
    theme,
};
use nsh_core::prelude::*;

use crate::shell::Store;
use crate::ui::theme as tokens;

/// Renders `mention` nodes in both reading and editing tabs; a mention
/// never writes the Document, so it stays readable while another tab
/// holds the single-editor lock.
pub(crate) struct KanonMentionView {
    pub store: Store,
}

impl KanonMentionView {
    /// Whether the target currently resolves to a live, non-trashed
    /// entity. Dates always do; the other kinds read the single source
    /// of truth (a Document survives only while its catalog row is not
    /// trashed, matching the backlinks query's degraded state).
    fn live(store: Store, kind: &str, id: i64) -> bool {
        match kind {
            "document" => store.document(id).is_some_and(|_| {
                store
                    .catalog_node_of_document(id)
                    .is_some_and(|node| !store.catalog_node_is_trashed(node.id))
            }),
            "project" => store.projects().iter().any(|project| project.id == id),
            "task" => store.task(id).is_some_and(|aggregate| aggregate.task.trashed_at.is_none()),
            _ => true,
        }
    }
}

impl NodeView for KanonMentionView {
    fn create(&self, _cx: &NodeViewCx, node: &Node) -> AnyElement {
        let store = self.store;
        let (kind, _id_part) = node
            .attrs()
            .get("id")
            .and_then(|value| value.as_str())
            .and_then(|id| id.split_once(':'))
            .map(|(kind, id_part)| (kind.to_owned(), id_part.to_owned()))
            .unwrap_or_default();
        let target_id = _id_part.parse::<i64>().unwrap_or(0);
        let label = node
            .attrs()
            .get("label")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_owned();

        let live_kind = kind.clone();
        let live = move || Self::live(store, &live_kind, target_id);
        let border_kind = kind.clone();
        let border_live = move || Self::live(store, &border_kind, target_id);
        let bg_kind = kind.clone();
        let bg_live = move || Self::live(store, &bg_kind, target_id);
        let fg_kind = kind.clone();
        let fg_live = move || Self::live(store, &fg_kind, target_id);
        let chip_label = move || {
            if live() {
                label.clone()
            } else {
                format!("{label}（已失效）")
            }
        };

        view()
            .items_center()
            .gap_1()
            .px_1p5()
            .rounded(px(6.0))
            .border_1()
            .style_when(!border_live(), |s| s.border_dashed())
            .border_color(move || if border_live() { tokens::info() } else { tokens::fg_muted() })
            .bg(move || {
                if bg_live() { tokens::info_soft() } else { theme().background.opacity(0.0) }
            })
            .child(
                text(chip_label)
                    .text_size(px(13.0))
                    .text_color(move || if fg_live() { tokens::info() } else { tokens::fg_muted() }),
            )
            .into_element()
    }
}
