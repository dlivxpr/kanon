//! Quick capture forms shared by the start page and window toolbar.

use nsh_core::prelude::*;

use crate::document::templates::{TEMPLATES, Template};
use crate::ui::{
    chrome::{Icon, icon},
    controls::{ButtonKind, app_button, input_field},
    theme as tokens,
};

use super::{Store, tabs::Route};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Create {
    Menu,
    Task,
    Book,
    Document(Option<Template>),
    Templates,
}

pub(super) fn create_dialog(
    store: Store,
    active: Signal<Option<Create>>,
    screen: Create,
) -> impl IntoElement {
    let close = move || active.set(None);
    let title = match screen {
        Create::Menu => "快速新建",
        Create::Task => "新建任务",
        Create::Book => "新建知识库",
        Create::Document(_) => "新建文档",
        Create::Templates => "模板中心",
    };
    let content = match screen {
        Create::Menu => view()
            .gap_1()
            .child(choice(Icon::Tasks, "新建任务", move || {
                active.set(Some(Create::Task))
            }))
            .child(choice(Icon::Project, "新建项目", move || {
                close();
                store.open_project_editor(None);
            }))
            .child(choice(Icon::Books, "新建知识库", move || {
                active.set(Some(Create::Book))
            }))
            .child(choice(Icon::Document, "新建文档", move || {
                active.set(Some(Create::Document(None)))
            }))
            .child(choice(Icon::Note, "新建小记", move || {
                close();
                store.open_or_focus(Route::QuickNotes);
            }))
            .into_element(),
        Create::Templates => view()
            .gap_1()
            .children(TEMPLATES.into_iter().map(|template| {
                choice(
                    Icon::Document,
                    format!("{} · {}", template.title, template.sections.join(" / ")),
                    move || active.set(Some(Create::Document(Some(template)))),
                )
            }))
            .into_element(),
        _ => creation_form(store, active, screen).into_element(),
    };
    let enter = crate::ui::motion::dialog_enter();
    nsh_base::dialog()
        .label(title)
        .on_cancel(close)
        .animation(ModalAnimation::None)
        .flex_1()
        .p_4()
        .items_center()
        .justify_center()
        .bg(tokens::overlay)
        .child(
            view()
                .bind_motion(&enter)
                .w_full()
                .max_w(px(460.0))
                .p_5()
                .gap_4()
                .rounded(tokens::radius_panel())
                .bg(tokens::panel_raised)
                .border_1()
                .border_color(tokens::border_default)
                .shadow_lg()
                .child(
                    text(title)
                        .role(AccessibilityRole::Header)
                        .text_xl()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens::fg_primary),
                )
                .child(content)
                .child(view().flex_row().justify_end().child(app_button(
                    ButtonKind::Soft,
                    "关闭",
                    close,
                ))),
        )
}

fn choice(
    glyph: Icon,
    label: impl Into<SharedString>,
    choose: impl Fn() + 'static,
) -> impl IntoElement {
    let label = label.into();
    nsh_base::button()
        .on_press(choose)
        .accessibility_label(label.clone())
        .flex_row()
        .items_center()
        .justify_start()
        .gap_2()
        .h(px(36.0))
        .px_2()
        .rounded(tokens::radius_row())
        .text_color(tokens::fg_primary)
        .hover(|s| s.bg(tokens::control_hover))
        .active(|s| s.bg(tokens::control_bg))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
        .child(icon(move || glyph, px(16.0)).text_color(tokens::info))
        .child(text(label).text_sm().number_of_lines(1))
}

fn creation_form(store: Store, active: Signal<Option<Create>>, screen: Create) -> impl IntoElement {
    let books = store.knowledge_bases();
    let book = signal(books.first().map(|b| b.id));
    let draft = signal(match screen {
        Create::Document(Some(template)) => template.title.to_owned(),
        _ => String::new(),
    });
    let error = signal(false);
    let submit = move || {
        let name = draft.get_untracked();
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let created = match screen {
            Create::Task => store.create_inbox_task(name).map(|id| store.open_task(id)),
            Create::Book => store
                .create_knowledge_base(name)
                .map(|id| store.open_book(id)),
            Create::Document(template) => book.get_untracked().and_then(|book| {
                let body = template.map_or_else(String::new, Template::markdown);
                let id = store.create_document_with_body_in(book, None, name, &body)?;
                store.open_document(id);
                let tab_id = store
                    .strip
                    .with_untracked(|strip| strip.tabs[strip.active].id);
                store.request_edit(tab_id, id);
                Some(())
            }),
            _ => None,
        };
        if created.is_some() {
            active.set(None);
        } else {
            error.set(true);
        }
    };
    view()
        .gap_3()
        .child(text("名称").text_sm().text_color(tokens::fg_muted))
        .child(
            view().h(px(32.0)).child(
                input_field(
                    "输入名称…",
                    move || draft.get(),
                    move |value| draft.set(value.to_owned()),
                )
                .label("名称")
                .auto_focus(true)
                .on_submit(move |_: &str| submit()),
            ),
        )
        .child(matches!(screen, Create::Document(_)).then(|| {
            view()
                .gap_2()
                .child(text("归属知识库").text_sm().text_color(tokens::fg_muted))
                .child(books.is_empty().then(|| {
                    app_button(ButtonKind::Soft, "先创建知识库…", move || {
                        active.set(Some(Create::Book))
                    })
                }))
                .child(scroll().max_h(px(140.0)).child(view().gap_1().children(
                    books.into_iter().map(move |item| {
                        let id = item.id;
                        app_button(ButtonKind::Soft, item.name, move || book.set(Some(id)))
                            .selected(move || book.get() == Some(id))
                            .selected_style(|s| {
                                s.bg(tokens::accent_soft).border_color(tokens::accent_ink)
                            })
                    }),
                )))
        }))
        .child(when(error, || {
            text("无法创建，请检查工作区后重试。")
                .text_sm()
                .text_color(tokens::danger)
        }))
        .child(
            app_button(ButtonKind::Primary, "创建", submit)
                .self_end()
                .disabled(move || {
                    draft.with(|name| name.trim().is_empty())
                        || matches!(screen, Create::Document(_)) && book.get().is_none()
                }),
        )
}
