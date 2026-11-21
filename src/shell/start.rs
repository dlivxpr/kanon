//! The Start workbench: shortcuts, today's digest and recent documents.

use nsh_core::prelude::*;

use crate::domain::RecentScope;
use crate::ui::controls::{ButtonKind, app_button, app_button_with};

use super::{Store, create::Create, tabs::Route};
use crate::ui::{
    chrome::{Icon, icon},
    theme as tokens,
};

pub(super) fn start_view(store: Store, create: Signal<Option<Create>>) -> impl IntoElement {
    let digest = computed(move || {
        let groups = store.task_groups();
        let date = store.now().date;
        // Overdue tasks are grouped separately even when scheduled for today.
        let today = groups.today.len()
            + groups
                .overdue
                .iter()
                .filter(|task| task.task.scheduled_date == Some(date))
                .count();
        (today, store.projects().len(), groups.overdue.len())
    });
    scroll().flex_1().child(
        view()
            .p_8()
            .style_when(|| window().width.0 < 680.0, |s| s.p_4())
            .gap_4()
            .child(
                text("开始")
                    .role(AccessibilityRole::Header)
                    .test_id("start-heading")
                    .text_3xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(tokens::fg_primary)
                    .mb_2(),
            )
            .child(
                view()
                    .test_id("start-launchers")
                    .flex_row()
                    .flex_wrap()
                    .gap(px(10.0))
                    .children([
                        launcher(Icon::Document, "新建文档", move || {
                            create.set(Some(Create::Document(None)))
                        }),
                        launcher(Icon::Books, "新建知识库", move || {
                            create.set(Some(Create::Book))
                        }),
                        launcher(Icon::Project, "新建项目", move || {
                            store.open_project_editor(None)
                        }),
                        launcher(Icon::Start, "模板中心", move || {
                            create.set(Some(Create::Templates))
                        }),
                    ]),
            )
            .child(
                nsh_base::button()
                    .on_press(move || store.open_or_focus(Route::Today))
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .justify_start()
                    .gap_2()
                    .min_h(px(50.0))
                    .p_3()
                    .rounded(tokens::radius_card())
                    .bg(tokens::accent_soft)
                    .text_color(tokens::accent_ink)
                    .hover(|s| s.bg(tokens::control_hover))
                    .active(|s| s.opacity(0.8))
                    .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
                    .child(icon(|| Icon::Sun, px(22.0)))
                    .child(text("今日速览").text_sm().font_weight(FontWeight::SEMIBOLD))
                    .child(
                        text(move || {
                            let (today, projects, overdue) = digest.get();
                            format!("{today} 项待办 · {projects} 个项目 · {overdue} 项逾期")
                        })
                        .text_sm()
                        .flex_1(),
                    )
                    .child(icon(|| Icon::Forward, px(16.0))),
            )
            .child(recent_section(store)),
    )
}

fn launcher(
    glyph: Icon,
    label: &'static str,
    on_press: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    nsh_base::button()
        .on_press(on_press)
        .accessibility_label(label)
        .min_w(px(112.0))
        .flex_1()
        .h(px(88.0))
        .p_3()
        .gap_3()
        .items_start()
        .justify_center()
        .rounded(tokens::radius_card())
        .border_1()
        .border_color(tokens::border_subtle)
        .bg(tokens::panel_subtle)
        .text_color(tokens::fg_primary)
        .hover(|s| s.bg(tokens::control_hover))
        .active(|s| s.bg(tokens::control_bg))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
        .child(icon(move || glyph, px(17.0)).text_color(tokens::info))
        .child(text(label).text_sm().font_weight(FontWeight::SEMIBOLD))
}

fn recent_section(store: Store) -> impl IntoElement {
    let scope = signal(RecentScope::Edited);
    let book = signal(None::<i64>);
    let recent = computed(move || store.recent_documents(book.get(), scope.get(), 16));
    view()
        .gap_2()
        .pt_2()
        .child(
            text("最近文档")
                .role(AccessibilityRole::Header)
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens::fg_primary),
        )
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .gap_2()
                .items_center()
                .justify_between()
                .child(
                    view()
                        .flex_row()
                        .gap_1()
                        .p_1()
                        .rounded(tokens::radius_row())
                        .bg(tokens::control_bg)
                        .children(
                            [
                                (RecentScope::Edited, "最近编辑"),
                                (RecentScope::Viewed, "最近浏览"),
                                (RecentScope::Favorites, "已收藏"),
                            ]
                            .map(|(choice, label)| {
                                app_button(ButtonKind::Ghost, label, move || scope.set(choice))
                                    .selected(move || scope.get() == choice)
                                    .selected_style(|s| s.bg(tokens::panel_raised))
                            }),
                        ),
                )
                .child(
                    nsh_base::menu()
                        .popup_style(|s| {
                            s.p_1()
                                .rounded(tokens::radius_card())
                                .bg(tokens::panel_raised)
                                .border_1()
                                .border_color(tokens::border_default)
                                .shadow_md()
                        })
                        .trigger(
                            app_button_with(
                                ButtonKind::Soft,
                                move || {
                                    book.get()
                                        .and_then(|id| store.knowledge_base(id))
                                        .map_or_else(|| "全部知识库".into(), |b| b.name.into())
                                },
                                || {},
                            )
                            .max_w(px(164.0))
                            .test_id("recent-book-filter"),
                        )
                        .child(
                            nsh_base::menu_item()
                                .on_select(move || book.set(None))
                                .p_2()
                                .child(text("全部知识库").text_sm().text_color(tokens::fg_primary)),
                        )
                        .child(each(
                            move || store.knowledge_bases(),
                            |b: &crate::domain::KnowledgeBase| b.id,
                            move |b, _| {
                                nsh_base::menu_item()
                                    .on_select(move || book.set(Some(b.with(|b| b.id))))
                                    .p_2()
                                    .child(
                                        text(move || b.with(|b| b.name.clone()))
                                            .text_sm()
                                            .text_color(tokens::fg_primary),
                                    )
                            },
                        )),
                ),
        )
        .child(when(
            move || recent.with(Vec::is_empty),
            || {
                text("暂无文档")
                    .p_4()
                    .text_sm()
                    .text_color(tokens::fg_muted)
            },
        ))
        .child(each(
            recent,
            |row: &crate::domain::RecentDocument| row.id,
            move |row, _| {
                let id = move || row.with(|row| row.id);
                view()
                    .h(px(48.0))
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(tokens::border_subtle)
                    .child(
                        nsh_base::button()
                            .on_press(move || store.open_document(id()))
                            .flex_1()
                            .min_w(px(0.0))
                            .h_full()
                            .flex_row()
                            .items_center()
                            .justify_start()
                            .gap_2()
                            .px_2()
                            .rounded(tokens::radius_row())
                            .text_color(tokens::fg_primary)
                            .hover(|s| s.bg(tokens::control_hover))
                            .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
                            .child(icon(|| Icon::Document, px(16.0)).text_color(tokens::info))
                            .child(
                                text(move || row.with(|row| row.title.clone()))
                                    .text_sm()
                                    .number_of_lines(1),
                            ),
                    )
                    .child(
                        text(move || row.with(|row| row.book_name.clone()))
                            .text_xs()
                            .w(px(108.0))
                            .number_of_lines(1)
                            .text_color(tokens::fg_muted),
                    )
                    .child(
                        text(move || {
                            row.with(|row| {
                                row.updated_at
                                    .get(5..10)
                                    .unwrap_or(&row.updated_at)
                                    .replace('-', "/")
                            })
                        })
                        .text_xs()
                        .text_color(tokens::fg_muted),
                    )
                    .child(
                        crate::ui::chrome::chrome_button(
                            || Icon::Star,
                            || "切换文档收藏".into(),
                            true,
                        )
                        .selected(move || row.with(|row| row.favorited))
                        .selected_style(|s| s.text_color(tokens::warning))
                        .on_press(move || {
                            store.set_document_favorite(id(), !row.with(|row| row.favorited))
                        }),
                    )
            },
        ))
}
