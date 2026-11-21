//! Reading actions: create a sibling from a template, export Markdown,
//! or move the document's catalog subtree to Trash after confirmation.

use std::time::Duration;

use nsh_base::{menu, menu_item, menu_separator, theme};
use nsh_core::file_dialog::{FileFilter, SaveDialog};
use nsh_core::prelude::*;

use crate::shell::Store;
use crate::ui::controls::{ButtonKind, app_button, dialog_card, panel_icon_button};
use crate::ui::theme as tokens;

use super::templates::{TEMPLATES, Template};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ReadingDialog {
    Template,
    Trash,
}

/// Bind the export task to the reading view, not the disappearing popup.
pub(super) fn document_overflow_menu(
    store: Store,
    document_id: i64,
    notice: Signal<Option<String>>,
    dialog: Signal<Option<ReadingDialog>>,
    owner: Owner,
) -> impl IntoElement {
    menu()
        .offset(px(4.0))
        .popup_style(|s| {
            s.p_1()
                .rounded(tokens::radius_card())
                .bg(theme().popover)
                .shadow_md()
                .w(px(180.0))
        })
        .trigger(panel_icon_button("⋯", "更多", || {}))
        .child(action_item("从模板新建…", false, move || {
            dialog.set(Some(ReadingDialog::Template));
        }))
        .child(action_item("导出 Markdown", false, move || {
            owner.run(|| export_markdown(store, document_id, notice));
        }))
        .child(menu_separator().h(px(1.0)).my_1().bg(tokens::border_subtle))
        .child(action_item("移入回收站", true, move || {
            dialog.set(Some(ReadingDialog::Trash));
        }))
}

fn action_item(
    label: &'static str,
    danger: bool,
    on_select: impl Fn() + 'static,
) -> impl IntoElement {
    menu_item()
        .on_select(on_select)
        .px_3()
        .py_2()
        .rounded(tokens::radius_row())
        .hover(|s| s.bg(tokens::control_hover))
        .child(text(label).text_sm().text_color(move || {
            if danger {
                tokens::danger()
            } else {
                tokens::fg_primary()
            }
        }))
}

pub(super) fn reading_dialog(
    store: Store,
    document_id: i64,
    notice: Signal<Option<String>>,
    dialog: Signal<Option<ReadingDialog>>,
    owner: Owner,
) -> impl IntoElement {
    let close = move || dialog.set(None);
    dynamic(move || {
        let content = match dialog.get()? {
            ReadingDialog::Template => dialog_card(
                "从模板新建",
                "在当前文档所在目录创建新文档。",
                AccessibilityRole::Dialog,
                close,
                view()
                    .gap_2()
                    .children(TEMPLATES.into_iter().map(|template| {
                        app_button(ButtonKind::Soft, template.title, move || {
                            close();
                            owner
                                .run(|| create_from_template(store, document_id, template, notice));
                        })
                        .justify_start()
                    }))
                    .child(view().flex_row().justify_end().child(app_button(
                        ButtonKind::Ghost,
                        "取消",
                        close,
                    ))),
            )
            .into_element(),
            ReadingDialog::Trash => {
                let doc = store.document(document_id)?;
                dialog_card(
                    format!("将「{}」移入回收站？", doc.title),
                    "其下的目录分组和文档也会移入回收站，可以一并恢复。",
                    AccessibilityRole::AlertDialog,
                    close,
                    view()
                        .flex_row()
                        .justify_end()
                        .gap_2()
                        .child(app_button(ButtonKind::Ghost, "取消", close))
                        .child(app_button(
                            ButtonKind::Danger,
                            "确认移入回收站",
                            move || {
                                close();
                                owner.run(|| {
                                    trash_document_subtree(store, doc.book_id, document_id, notice)
                                });
                            },
                        )),
                )
                .into_element()
            }
        };
        Some(content)
    })
}

fn create_from_template(
    store: Store,
    document_id: i64,
    template: Template,
    notice: Signal<Option<String>>,
) {
    let Some(node) = store.catalog_node_of_document(document_id) else {
        return;
    };
    let Some(id) = store.create_document_with_body_in(
        node.book_id,
        node.parent_id,
        template.title,
        &template.markdown(),
    ) else {
        show_notice(notice, "无法创建文档。请检查当前目录是否可用。".to_owned());
        return;
    };
    store.open_document(id);
    let active_tab = store
        .strip
        .with_untracked(|strip| strip.tabs[strip.active].id);
    store.request_edit(active_tab, id);
}

fn trash_document_subtree(
    store: Store,
    book_id: i64,
    document_id: i64,
    notice: Signal<Option<String>>,
) {
    let outline = store.catalog_outline(book_id);
    let Some(offset) = outline
        .iter()
        .position(|entry| entry.node.document_id == Some(document_id))
    else {
        return;
    };
    let target = &outline[offset];
    let editing = std::iter::once(target)
        .chain(
            outline[offset + 1..]
                .iter()
                .take_while(|entry| entry.depth > target.depth),
        )
        .filter_map(|entry| entry.node.document_id)
        .any(|id| store.editing_locked(id));
    if editing {
        show_notice(notice, "请先完成此文档或其子文档的编辑。".to_owned());
        return;
    }
    store.trash_catalog_node(target.node.id);
}

/// Asks where to save the Document's committed body as Markdown, then
/// writes it there. The dialog already confirmed an overwrite when it
/// returns a path; `Ok(None)` is the user's Cancel and stays silent;
/// only failures reach `notice`.
pub(crate) fn export_markdown(store: Store, document_id: i64, notice: Signal<Option<String>>) {
    notice.set(None);
    let Some(document) = store.document(document_id) else {
        return;
    };
    let title = document.title;
    let body = document.body;
    let suggested = file_name(&title);
    spawn(async move {
        let picked = pick_save_path(
            SaveDialog::new()
                .title(format!("导出「{title}」"))
                .button_label("导出")
                .file_name(suggested)
                .filter(FileFilter::new("Markdown", ["md", "markdown"])),
        )
        .await;
        match picked {
            Ok(Some(path)) => {
                let mut markdown = format!("# {title}\n\n{body}");
                if !markdown.ends_with('\n') {
                    markdown.push('\n');
                }
                if let Err(error) = std::fs::write(&path, markdown) {
                    log::error!("导出 Markdown 写入 {} 失败: {error}", path.display());
                    show_notice(notice, format!("导出失败：无法写入 {}", path.display()));
                }
            }
            Ok(None) => {}
            Err(error) => {
                log::error!("导出 Markdown 对话框无法显示: {error}");
                show_notice(notice, format!("导出失败：{error}"));
            }
        }
    });
}

/// Shows `message` under the reading header for a few seconds. The
/// timer shares the export task's owner: a reading tab that goes away
/// cancels it, and the signal is never written after its owner is gone.
fn show_notice(notice: Signal<Option<String>>, message: String) {
    notice.set(Some(message));
    spawn(async move {
        timer(Duration::from_secs(6)).await;
        notice.set(None);
    });
}

/// The dialog's suggested name: `<title>.md` with the characters no
/// Windows or macOS file name allows replaced (the dialog saves to a
/// real file system), a trailing dot or space trimmed, and a fall back
/// for an empty title.
fn file_name(title: &str) -> String {
    let sanitized = title
        .chars()
        .map(|ch| match ch {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            ch => ch,
        })
        .collect::<String>();
    let trimmed = sanitized.trim_end_matches(['.', ' ']);
    if trimmed.is_empty() {
        "未命名.md".to_owned()
    } else {
        format!("{trimmed}.md")
    }
}

#[cfg(test)]
mod tests {
    use super::file_name;

    #[test]
    fn suggested_names_are_safe_markdown_files() {
        assert_eq!(file_name("季度复盘"), "季度复盘.md");
        assert_eq!(file_name("a/b\\c:d*e?f\"g<h>i|j"), "a_b_c_d_e_f_g_h_i_j.md");
        assert_eq!(file_name("报告 v2: 终稿?"), "报告 v2_ 终稿_.md");
        assert_eq!(file_name("name."), "name.md");
        assert_eq!(file_name("  "), "未命名.md");
        assert_eq!(file_name(""), "未命名.md");
    }
}
