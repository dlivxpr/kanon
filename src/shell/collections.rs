//! The 收藏 (Favorites) and 标签 (Tags) surfaces (docs/ui-design.md
//! §2.2; docs/kanon.html `desk()`'s `favorites` and `tags` views).
//!
//! 收藏 shows Knowledge Bases, Documents and Projects. 标签 renders
//! the tag cloud with its linked Tasks, Documents and Quick Notes.
//! Workspace mutations update both surfaces without a reload.

use nsh_base::button;
use nsh_core::prelude::*;

use crate::domain::{FavoriteEntry, FavoriteKind, TagUsage};
use crate::ui::chrome::{Icon, icon};
use crate::ui::controls::{
    ButtonKind, app_button, chip, empty_state, input_field, section_heading,
};
use crate::ui::theme as tokens;

use super::store::Store;
use super::tabs::Route;

/// The 收藏 view for the active tab.
pub(super) fn favorites_view(store: Store) -> impl IntoElement {
    let filter = signal(None::<FavoriteKind>);
    let favorites = computed(move || store.favorites());
    view().flex_1().min_h(px(0.0)).child(
        scroll().flex_1().child(
            view()
                .flex_col()
                .gap_4()
                .p_5()
                .max_w(px(860.0))
                .w_full()
                .self_center()
                .child(
                    text("收藏")
                        .role(AccessibilityRole::Header)
                        .test_id("favorites-heading")
                        .text_size(px(28.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(tokens::fg_primary()),
                )
                .child(
                    view().flex_row().flex_wrap().gap_2().children(
                        [
                            (None, "全部"),
                            (Some(FavoriteKind::Book), "知识库"),
                            (Some(FavoriteKind::Document), "文档"),
                            (Some(FavoriteKind::Project), "项目"),
                        ]
                        .map(|(kind, title)| {
                            app_button(ButtonKind::Soft, title, move || filter.set(kind))
                                .accessibility_label(format!("筛选收藏：{title}"))
                                .selected(move || filter.get() == kind)
                                .selected_style(|s| {
                                    s.bg(tokens::accent_soft).border_color(tokens::accent_ink)
                                })
                        }),
                    ),
                )
                .child(dynamic(move || {
                    let filter = filter.get();
                    let rows = favorites.with(|entries| {
                        entries
                            .iter()
                            .filter(|entry| filter.is_none_or(|kind| kind == entry.kind))
                            .map(|entry| favorite_row(store, entry.clone()).into_element())
                            .collect::<Vec<_>>()
                    });
                    if !rows.is_empty() {
                        view().flex_col().children(rows).into_element()
                    } else if filter.is_some() {
                        empty_state("此类型暂无收藏", "选择「全部」查看其他类型的收藏。")
                            .into_element()
                    } else {
                        empty_state("暂无收藏", "在知识库、文档或项目中点亮星标后会显示在这里。")
                            .into_element()
                    }
                })),
        ),
    )
}

/// One bucket row (docs/kanon.html `.listline`): icon and title
/// navigate to the item, a trailing label names the module, and the
/// star unfavorites it (a favorited row that gets unstarred leaves the
/// bucket on the next bind, like the prototype's re-render).
fn favorite_row(store: Store, entry: FavoriteEntry) -> impl IntoElement {
    let kind = entry.kind;
    let id = entry.id;
    let title = entry.title;
    let accessibility = match &entry.book_name {
        Some(book) => format!("收藏 {title}（{book}）"),
        None => format!("收藏 {title}"),
    };
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .min_h(px(40.0))
        .py(px(9.0))
        .border_b_1()
        .border_color(tokens::border_subtle())
        .child(
            view().flex_1().min_w(px(0.0)).child(
                button()
                    .on_press(move || match kind {
                        FavoriteKind::Book => store.open_book(id),
                        FavoriteKind::Document => store.open_document(id),
                        FavoriteKind::Project => store.open_project(id),
                    })
                    .accessibility_label(Some(accessibility.into()))
                    .flex_row()
                    .justify_start()
                    .min_w(px(0.0))
                    .rounded(tokens::radius_row())
                    .hover(|s| s.bg(tokens::control_hover))
                    .active(|s| s.bg(tokens::control_bg))
                    .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
                    .items_center()
                    .gap_2()
                    .child(icon(
                        move || match kind {
                            FavoriteKind::Book => Icon::Book,
                            FavoriteKind::Document => Icon::Document,
                            FavoriteKind::Project => Icon::Project,
                        },
                        px(16.0),
                    ))
                    .child(
                        text(title.clone())
                            .text_size(px(14.0))
                            .text_color(tokens::fg_primary())
                            .number_of_lines(1),
                    ),
            ),
        )
        .child(
            text(kind.label())
                .text_size(px(11.0))
                .text_color(tokens::fg_muted()),
        )
        .child(
            button()
                .on_press(move || match kind {
                    FavoriteKind::Book => store.set_kb_favorite(id, false),
                    FavoriteKind::Document => store.set_document_favorite(id, false),
                    FavoriteKind::Project => store.set_project_favorite(id, false),
                })
                .accessibility_label(Some(format!("取消收藏 {title}").into()))
                .size(px(26.0))
                .items_center()
                .justify_center()
                .rounded(tokens::radius_row())
                .text_color(tokens::warning())
                .hover(|s| s.bg(tokens::control_bg()))
                .active(|s| s.opacity(0.75))
                .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
                .child(text("★").text_size(px(14.0))),
        )
}

/// The global tag cloud and the live contents of the active tag route.
pub(super) fn tags_view(store: Store, selected: Option<i64>) -> impl IntoElement {
    let selected_name = computed(move || selected.and_then(|id| store.tag_name(id)));
    let renaming = signal(false);
    let usages = move || store.tag_usages();
    view()
        .flex_1()
        .min_h(px(0.0))
        .child(
            scroll().flex_1().child(
                view()
                    .flex_col()
                    .gap_4()
                    .p_5()
                    .max_w(px(860.0))
                    .w_full()
                    .self_center()
                    .child(
                        view()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(
                                text(move || {
                                    selected_name.with(|name| {
                                        name.as_ref().map_or_else(
                                            || "标签".to_owned(),
                                            |name| format!("#{name}"),
                                        )
                                    })
                                })
                                .role(AccessibilityRole::Header)
                                .test_id("tags-heading")
                                .text_size(px(28.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(tokens::fg_primary()),
                            )
                            .child(when(
                                move || selected_name.with(Option::is_some),
                                move || {
                                    app_button(ButtonKind::Soft, "重命名标签", move || {
                                        renaming.set(true)
                                    })
                                },
                            )),
                    )
                    .child(when(
                        move || usages().is_empty(),
                        || empty_state("暂无标签", "在任务、文档或小记上添加标签后会显示在这里。"),
                    ))
                    .child(dynamic(move || {
                        view()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .flex_wrap()
                            .children(
                                usages()
                                    .into_iter()
                                    .map(|usage| tag_chip(store, usage, selected).into_element())
                                    .collect::<Vec<_>>(),
                            )
                    }))
                    .child(selected.map(|id| {
                        when(
                            move || selected_name.with(Option::is_some),
                            move || related_section(store, id),
                        )
                    })),
            ),
        )
        .child(when(renaming, move || {
            selected
                .zip(selected_name.get())
                .map(|(id, name)| rename_tag_dialog(store, id, name, renaming))
        }))
}

fn rename_tag_dialog(
    store: Store,
    id: i64,
    name: String,
    renaming: Signal<bool>,
) -> impl IntoElement {
    let draft = signal(name);
    let failed = signal(false);
    let close = move || renaming.set(false);
    let save = move |proposed: &str| {
        let proposed = proposed.trim();
        if proposed.is_empty() {
            return;
        }
        if let Some(survivor) = store.rename_tag(id, proposed) {
            close();
            if survivor != id {
                store.open_or_focus(Route::Tag(survivor));
            }
        } else {
            failed.set(true);
        }
    };
    nsh_base::dialog()
        .label("重命名标签")
        .on_cancel(close)
        .flex_1()
        .p_4()
        .items_center()
        .justify_center()
        .bg(tokens::overlay)
        .child(
            view()
                .w_full()
                .max_w(px(420.0))
                .p_5()
                .gap_3()
                .rounded(tokens::radius_panel())
                .bg(tokens::panel_raised)
                .border_1()
                .border_color(tokens::border_default)
                .shadow_lg()
                .child(section_heading("重命名标签"))
                .child(
                    text("若新名称已存在，会合并标签并保留所有关联内容。")
                        .text_sm()
                        .text_color(tokens::fg_muted),
                )
                .child(
                    input_field(
                        "新名称",
                        move || draft.get(),
                        move |value| {
                            draft.set(value.to_owned());
                            failed.set(false);
                        },
                    )
                    .label("新名称")
                    .auto_focus(true)
                    .on_submit(save),
                )
                .child(when(failed, || {
                    text("标签未能保存，请重试。")
                        .text_sm()
                        .text_color(tokens::danger)
                }))
                .child(
                    view()
                        .flex_row()
                        .justify_end()
                        .gap_2()
                        .child(app_button(ButtonKind::Ghost, "取消", close))
                        .child(
                            app_button(ButtonKind::Primary, "保存标签", move || {
                                draft.with_untracked(|name| save(name))
                            })
                            .disabled(move || draft.with(|name| name.trim().is_empty())),
                        ),
                ),
        )
}

/// One cloud chip (`#设计`): selecting it shows its 相关内容;
/// selecting it again clears the pick (the prototype navigates back to
/// the bare `tags` route).
fn tag_chip(store: Store, usage: TagUsage, selected: Option<i64>) -> impl IntoElement {
    let id = usage.tag.id;
    button()
        .selected(selected == Some(id))
        .on_press(move || {
            store.open_or_focus(if selected == Some(id) {
                Route::Tags
            } else {
                Route::Tag(id)
            })
        })
        .accessibility_label(Some(format!("查看标签 {}", usage.tag.name).into()))
        .child(chip(format!("#{}", usage.tag.name)))
}

/// The picked tag's 相关内容 (docs/kanon.html tags bucket): its live
/// Tasks, Documents and Quick Notes as one list, each row navigating to
/// its module (a Task opens the right panel's Task Detail, like every
/// task title).
fn related_section(store: Store, id: i64) -> impl IntoElement {
    let content = computed(move || {
        (
            store.tasks_for_tag(id),
            store.documents_for_tag(id),
            store.quick_notes_for_tag(id),
        )
    });
    view()
        .flex_col()
        .gap_2()
        .child(section_heading("相关内容"))
        .child(when(
            move || {
                content.with(|(tasks, docs, notes)| {
                    tasks.is_empty() && docs.is_empty() && notes.is_empty()
                })
            },
            || empty_state("暂无内容", "该标签下没有未移入回收站的内容。"),
        ))
        .child(dynamic(move || {
            let (tasks, docs, notes) = content.get();
            view().flex_col().children(
                tasks
                    .into_iter()
                    .map(|aggregate| {
                        let id = aggregate.task.id;
                        let title = aggregate.task.title;
                        related_row(Icon::Tasks, title, "任务", move || store.open_task(id))
                            .into_element()
                    })
                    .chain(docs.into_iter().map(|document| {
                        let id = document.id;
                        let title = document.title;
                        related_row(Icon::Document, title, "文档", move || {
                            store.open_document(id)
                        })
                        .into_element()
                    }))
                    .chain(notes.into_iter().map(|note| {
                        let id = note.id;
                        let title = note_title(&note.content);
                        related_row(Icon::Note, title, "小记", move || {
                            store.open_or_focus(Route::QuickNote(id));
                        })
                        .into_element()
                    }))
                    .collect::<Vec<_>>(),
            )
        }))
}

/// One 相关内容 row, same shape as the favorites bucket row.
fn related_row(
    glyph: Icon,
    title: String,
    kind_label: &'static str,
    on_open: impl Fn() + 'static,
) -> impl IntoElement {
    let label = title.clone();
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .min_h(px(40.0))
        .py(px(9.0))
        .border_b_1()
        .border_color(tokens::border_subtle())
        .child(
            view().flex_1().min_w(px(0.0)).child(
                button()
                    .on_press(on_open)
                    .accessibility_label(Some(format!("打开 {label}").into()))
                    .flex_row()
                    .justify_start()
                    .min_w(px(0.0))
                    .rounded(tokens::radius_row())
                    .hover(|s| s.bg(tokens::control_hover))
                    .active(|s| s.bg(tokens::control_bg))
                    .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
                    .items_center()
                    .gap_2()
                    .child(icon(move || glyph, px(16.0)))
                    .child(
                        text(label.clone())
                            .text_size(px(14.0))
                            .text_color(tokens::fg_primary())
                            .number_of_lines(1),
                    ),
            ),
        )
        .child(
            text(kind_label)
                .text_size(px(11.0))
                .text_color(tokens::fg_muted()),
        )
}

/// A Quick Note's bucket title: its first line, trimmed to one row
/// (docs/kanon.html slices note rows the same way).
fn note_title(content: &str) -> String {
    let first = content.lines().next().unwrap_or("").trim();
    let mut title: String = first.chars().take(48).collect();
    if first.chars().count() > 48 {
        title.push('…');
    }
    title
}
