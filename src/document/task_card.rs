//! The Linked Task Card node view (GLOSSARY, ADR-0002): the Document's
//! block rendering of a referenced live Task — its circle toggles the
//! Task itself, its title opens the Task Detail, and every mutation
//! flows from the single source of truth (`Store` over `Workspace`), so
//! Today, Task Detail, every tab of this Document and the restart all
//! agree. A Task in Trash (or permanently deleted) renders the
//! non-interactive removed-task placeholder instead; restoring the Task
//! brings the live card back.

use nsh_base::{
    button,
    rich_editor::{
        model::Node,
        view::{NodeView, NodeViewCx},
    },
};
use nsh_core::prelude::*;
use serde_json::Value as JsonValue;

use crate::domain::TaskStatus;
use crate::shell::Store;
use crate::ui::controls::{chip, task_circle};
use crate::ui::theme as tokens;

/// Renders `taskCard` nodes in both reading and editing tabs; the card
/// never writes the Document (its toggle is a Task write), so it stays
/// interactive even while another tab holds the single-editor lock.
pub(crate) struct TaskCardView {
    pub store: Store,
}

impl NodeView for TaskCardView {
    fn create(&self, _cx: &NodeViewCx, node: &Node) -> AnyElement {
        let store = self.store;
        let task_id = node.attrs().get("taskId").and_then(JsonValue::as_i64).unwrap_or(0);
        // Live lookups against the single source of truth: any workspace
        // mutation (bump) re-runs these bindings.
        let any_task = move || {
            store.bump.get();
            store.task(task_id)
        };
        let live = move || {
            any_task().filter(|aggregate| aggregate.task.trashed_at.is_none())
        };
        let placeholder_title = move || {
            any_task().map(|aggregate| aggregate.task.title.clone()).unwrap_or_default()
        };
        let label = move || live().map(|aggregate| aggregate.task.title.clone()).unwrap_or_default();
        let done = move || live().is_some_and(|aggregate| aggregate.task.status == TaskStatus::Done);

        view()
            .flex_row()
            .items_center()
            .gap_2()
            .my_1()
            .px_3()
            .py_2()
            .rounded(tokens::radius_card())
            .border_1()
            .border_color(move || {
                if live().is_some() { tokens::border_subtle() } else { tokens::border_default() }
            })
            .bg(move || {
                if live().is_some() { tokens::panel_subtle() } else { tokens::control_bg().opacity(0.4) }
            })
            .child(dynamic(move || match live() {
                Some(_) => {
                    let open = store;
                    let open_id = task_id;
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .flex_1()
                        .min_w(px(0.0))
                        .child(task_circle(
                            done,
                            format!("完成 {}", label()),
                            move || store.toggle_circle(task_id),
                        ))
                        .child(
                            button()
                                .on_press(move || open.open_task(open_id))
                                .accessibility_label(Some(nsh_core::element::SharedString::from(format!(
                                    "打开任务 {}",
                                    label()
                                ))))
                                .items_center()
                                .child(
                                    text(label)
                                        .text_size(px(14.0))
                                        .text_color(move || {
                                            if done() {
                                                tokens::fg_muted()
                                            } else {
                                                tokens::fg_primary()
                                            }
                                        }),
                                ),
                        )
                        .child(when(
                            move || {
                                live().is_some_and(|aggregate| aggregate.task.status == TaskStatus::Doing)
                            },
                            || chip("进行中"),
                        ))
                        .into_element()
                }
                None => view()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        text("任务已移除")
                            .text_size(px(12.0))
                            .text_color(tokens::fg_muted()),
                    )
                    .child(
                        text(placeholder_title())
                            .text_size(px(13.0))
                            .text_color(tokens::fg_muted())
                            .number_of_lines(1),
                    )
                    .into_element(),
            }))
            .into_element()
    }
}
