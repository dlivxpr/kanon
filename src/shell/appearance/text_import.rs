//! Import a local file as a new document in an explicitly chosen knowledge base.

use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use nsh_base::{menu, menu_item};
use nsh_core::file_dialog::{FileFilter, OpenDialog};
use nsh_core::prelude::*;

use crate::domain::KnowledgeBase;
use crate::shell::Store;
use crate::ui::{
    controls::{ButtonKind, app_button},
    theme as tokens,
};

const MAX_BYTES: u64 = 4 * 1024 * 1024;
const EXTENSIONS: [&str; 3] = ["md", "markdown", "txt"];

pub(super) fn import_control(
    store: Store,
    busy: Signal<bool>,
    notice: Signal<Option<(bool, String)>>,
) -> impl IntoElement {
    let books = computed(move || store.knowledge_bases());
    let owner = Owner::current();
    view()
        .gap_2()
        .child(
            menu()
                .offset(px(4.0))
                .popup_style(|s| {
                    s.p_1()
                        .rounded(tokens::radius_card())
                        .bg(tokens::panel_raised)
                        .shadow_md()
                })
                .trigger(
                    app_button(ButtonKind::Soft, "导入到知识库…", || {})
                        .disabled(move || busy.get() || books.with(Vec::is_empty))
                        .self_start(),
                )
                .child(each(
                    move || books.get(),
                    |book: &KnowledgeBase| book.id,
                    move |book, _| {
                        menu_item()
                            .on_select(move || {
                                let id = book.with(|book| book.id);
                                owner.run(|| pick_document(store, id, busy, notice));
                            })
                            .p_2()
                            .rounded(tokens::radius_row())
                            .hover(|s| s.bg(tokens::control_hover))
                            .child(
                                text(move || book.with(|book| book.name.clone()))
                                    .text_sm()
                                    .text_color(tokens::fg_primary),
                            )
                    },
                )),
        )
        .child(when(
            move || books.with(Vec::is_empty),
            || {
                text("请先创建知识库，再导入文档。")
                    .text_xs()
                    .text_color(tokens::fg_muted)
            },
        ))
}

fn pick_document(
    store: Store,
    book_id: i64,
    busy: Signal<bool>,
    notice: Signal<Option<(bool, String)>>,
) {
    busy.set(true);
    notice.set(None);
    spawn(async move {
        let result = pick_paths(
            OpenDialog::files()
                .title("导入文本文档")
                .button_label("选择文件")
                .filter(FileFilter::new("Markdown 或文本", EXTENSIONS)),
        )
        .await;
        busy.set(false);
        let path = match result {
            Ok(Some(paths)) => paths.into_iter().next(),
            Ok(None) => None,
            Err(error) => {
                log::warn!("document import picker failed: {error}");
                notice.set(Some((false, "无法打开文件对话框。请重试。".to_owned())));
                None
            }
        };
        let Some(path) = path else { return };
        if !path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| {
                EXTENSIONS
                    .iter()
                    .any(|expected| expected.eq_ignore_ascii_case(ext))
            })
        {
            notice.set(Some((
                false,
                "请选择 Markdown 或 UTF-8 文本文件。".to_owned(),
            )));
            return;
        }
        let body = match read_text(&path) {
            Ok(body) => body,
            Err(message) => {
                notice.set(Some((false, message.to_owned())));
                return;
            }
        };
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("导入的文档");
        let body = portable_markdown(body.strip_prefix('\u{feff}').unwrap_or(&body));
        match store.create_document_with_body_in(book_id, None, title, &body) {
            Some(id) => store.open_document(id),
            None => notice.set(Some((
                false,
                "无法导入。请确认知识库仍存在、工作区可写，然后重试。".to_owned(),
            ))),
        }
    });
}

fn read_text(path: &Path) -> Result<String, &'static str> {
    let metadata = std::fs::metadata(path).map_err(|_| "无法读取文件。请选择可访问的本地文件。")?;
    if !metadata.is_file() {
        return Err("请选择普通文本文件。");
    }
    if metadata.len() > MAX_BYTES {
        return Err("文件超过 4 MiB。请拆成多个文档后再导入。");
    }
    let file = File::open(path).map_err(|_| "无法读取文件。请选择可访问的本地文件。")?;
    let mut text = String::new();
    file.take(MAX_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|_| "无法读取 UTF-8 文本。请另存为 UTF-8 后重试。")?;
    if text.len() as u64 > MAX_BYTES {
        return Err("文件超过 4 MiB。请拆成多个文档后再导入。");
    }
    Ok(text)
}

/// Workspace-local IDs from an external file cannot establish relationships.
fn portable_markdown(body: &str) -> Cow<'_, str> {
    let mut markers = body
        .match_indices('@')
        .filter(|(at, _)| {
            let rest = &body[*at..];
            rest.starts_with("@task(") || rest.starts_with("@m{")
        })
        .peekable();
    if markers.peek().is_none() {
        return Cow::Borrowed(body);
    }
    let mut output = String::with_capacity(body.len());
    let mut start = 0;
    for (at, _) in markers {
        output.push_str(&body[start..at]);
        output.push('＠');
        start = at + 1;
    }
    output.push_str(&body[start..]);
    Cow::Owned(output)
}
