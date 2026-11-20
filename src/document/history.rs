//! The local history panel (docs/ui-design.md §4.4): automatic snapshots
//! taken while editing, user-named versions saved on demand, and restore.
//! A restore archives the current state first, then brings back only the
//! Document's title/body (references re-anchor; no Task is ever touched).

use nsh_core::prelude::*;

use crate::shell::Store;
use crate::ui::controls::{app_button, panel_icon_button, ButtonKind};
use crate::ui::theme as tokens;

/// The header toggle of the docked history panel.
pub fn history_button(open: RwSignal<bool>) -> impl IntoElement {
    panel_icon_button("🕘", "本地历史版本", move || open.update(|value| *value = !*value))
}

/// The panel itself, docked at the document column's right edge.
pub fn history_panel(
    store: Store,
    document_id: i64,
    can_restore: bool,
    open: RwSignal<bool>,
) -> impl IntoElement {
    let revisions = computed(move || {
        store.bump.get();
        store.document_revisions(document_id)
    });
    let name = RwSignal::new(String::new());
    when(
        move || open.get(),
        move || {
            view()
                .w(px(300.0))
                .flex_none()
                .flex_col()
                .rounded(tokens::radius_card())
                .bg(tokens::panel_subtle())
                .border_1()
                .border_color(tokens::border_subtle())
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .px_3()
                        .py_2()
                        .border_b_1()
                        .border_color(tokens::border_subtle())
                        .child(
                            text("本地历史版本")
                                .text_size(px(13.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(tokens::fg_primary())
                                .flex_1(),
                        )
                        .child(panel_icon_button("✕", "关闭历史", move || open.set(false))),
                )
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .p_3()
                        .border_b_1()
                        .border_color(tokens::border_subtle())
                        .child(
                            nsh_base::input()
                                .placeholder("命名当前版本，如「评审版」")
                                .value(move || name.get())
                                .on_change(move |text: &str| name.set(text.to_owned()))
                                .placeholder_color(move || color(nsh_base::theme().muted_foreground))
                                .h(px(30.0))
                                .px_3()
                                .text_size(px(12.0))
                                .rounded(move || nsh_base::theme().radius)
                                .bg(tokens::control_bg())
                                .flex_1()
                                .min_w(px(0.0)),
                        )
                        .child(
                            app_button(ButtonKind::Positive, "保存版本", move || {
                                let trimmed = name.get_untracked();
                                if store.save_named_version(document_id, &trimmed) {
                                    name.set(String::new());
                                }
                            })
                            .flex_none(),
                        ),
                )
                .child(
                    scroll().flex_1().min_h(px(0.0)).child(
                        view()
                            .flex_col()
                            .p_2()
                            .gap_1()
                            .child(each(revisions, |revision: &crate::domain::DocumentRevision| revision.id, move |revision, _index| {
                                revision_row(store, can_restore, revision)
                            }))
                            .child(when(
                                move || revisions.with(|list| list.is_empty()),
                                || text("暂无历史版本").text_size(px(12.0)).text_color(tokens::fg_muted()).px_2().py_3()),
                    ),
                )
        },
    )
}

fn revision_row(
    store: Store,
    can_restore: bool,
    revision: ReadSignal<crate::domain::DocumentRevision>,
) -> impl IntoElement {
    let label = move || {
        revision.with(|revision| {
            revision
                .name
                .clone()
                .unwrap_or_else(|| "自动快照".to_owned())
        })
    };
    let stamped = move || revision.with(|revision| revision.created_at.clone());
    let title = move || revision.with(|revision| revision.title.clone());
    let revision_id = move || revision.with(|revision| revision.id);
    view()
        .flex_col()
        .gap_1()
        .px_2()
        .py_2()
        .rounded(tokens::radius_row())
        .hover(|s| s.bg(tokens::control_bg()))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(label)
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(tokens::fg_primary())
                        .flex_1()
                        .min_w(px(0.0)),
                )
                .child(text(stamped).text_size(px(11.0)).text_color(tokens::fg_muted())),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(title)
                        .text_size(px(11.0))
                        .text_color(tokens::fg_muted())
                        .flex_1()
                        .min_w(px(0.0)),
                )
                .child(when(
                    move || can_restore,
                    move || {
                        app_button(ButtonKind::Ghost, "恢复", move || {
                            store.restore_revision(revision_id());
                            // Restoring swaps the committed body: reading
                            // views resync through the store's bump.
                        })
                    },
                ))
                .child(when(
                    move || !can_restore,
                    || text("退出编辑后可恢复").text_size(px(11.0)).text_color(tokens::fg_muted()),
                )),
        )
}
