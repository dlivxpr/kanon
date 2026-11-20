//! The Knowledge Base flows (docs/ui-design.md §3.3, §4.4): the shelf
//! with grid/list and favorites, the book-style home page with the
//! dot-leader catalog, and the inner catalog sidebar (book switcher,
//! in-book filtering, locate-current, expand/collapse-all, per-node
//! expand state persisted in SQLite).

use nsh_base::{menu, menu_item, menu_separator, theme};
use nsh_core::prelude::*;

use crate::domain::{BookStats, CatalogEntry, KnowledgeBase, KnowledgeBasePatch};
use crate::shell::store::Store;
use crate::shell::tabs::Route;
use crate::ui::controls::{
    app_button, empty_state, input_field, submit_field, panel_icon_button,
    section_heading, ButtonKind,
};
use crate::ui::theme as tokens;

/// The inner catalog sidebar's width (docs/ui-design.md §3.3).
const CATALOG_W: f32 = 252.0;
/// Below this the catalog overlays the main pane instead of sitting
/// beside it (docs/ui-design.md §3.4).
const NARROW_BREAKPOINT: f32 = 960.0;

/// Which row of the catalog the open tab points at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CatalogFocus {
    /// The book home (`首页`) is showing.
    Home,
    /// A document reading view is showing.
    Document(i64),
}

/// What the catalog's quick-create row is creating, and where.
#[derive(Debug, Clone, Copy, PartialEq)]
enum PendingCreate {
    Document { parent: Option<i64> },
    Group { parent: Option<i64> },
}

/// The catalog tree's shared state, handed to every row.
#[derive(Clone, Copy)]
struct CatalogCtx {
    store: Store,
    focus: CatalogFocus,
    filter: Signal<String>,
    pending: Signal<Option<PendingCreate>>,
    located: Signal<Option<i64>>,
}

// ---------------------------------------------------------------------
// The shelf (全部知识库)
// ---------------------------------------------------------------------

/// The Knowledge Base shelf: grid/list toggle, favorites up top,
/// creation and per-book edit/trash.
pub fn kb_shelf(store: Store) -> impl IntoElement {
    let grid = signal(true);
    let creating = signal(false);
    let draft = signal(String::new());
    let editing = signal(None::<i64>);
    let books = move || store.knowledge_bases();
    let favorites = move || books().into_iter().filter(|kb| kb.favorited).collect::<Vec<_>>();

    view().flex_1().min_h(px(0.0)).child(
        scroll().flex_1().child(
            view()
                .flex_col()
                .gap_4()
                .p_5()
                .max_w(px(1000.0))
                .w_full()
                .self_center()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            text("知识库")
                                .text_size(px(24.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(tokens::fg_primary()),
                        )
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .child(when(
                                    move || grid.get(),
                                    move || panel_icon_button("☰", "切换为列表视图", move || grid.set(false)),
                                ))
                                .child(when(
                                    move || !grid.get(),
                                    move || panel_icon_button("▦", "切换为卡片网格", move || grid.set(true)),
                                ))
                                .child(app_button(ButtonKind::Primary, "＋ 新建知识库", move || {
                                    creating.set(true);
                                })),
                        ),
                )
                .child(when(
                    move || creating.get(),
                    move || {
                        view()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(submit_field(
                                "知识库名称，回车创建",
                                move || draft.get(),
                                move |text: &str| draft.set(text.to_owned()),
                                move |text: &str| {
                                    if let Some(id) = store.create_knowledge_base(text) {
                                        draft.set(String::new());
                                        creating.set(false);
                                        store.open_book(id);
                                    }
                                },
                            ))
                            .child(
                                app_button(ButtonKind::Positive, "创建", move || {
                                    if let Some(id) = store.create_knowledge_base(&draft.get()) {
                                        draft.set(String::new());
                                        creating.set(false);
                                        store.open_book(id);
                                    }
                                })
                                .test_id("kb-create"),
                            )
                            .child(app_button(ButtonKind::Ghost, "取消", move || {
                                creating.set(false);
                                draft.set(String::new());
                            }))
                    },
                ))
                .child(when(
                    move || !favorites().is_empty(),
                    move || {
                        view()
                            .flex_col()
                            .gap_2()
                            .child(section_heading("常用"))
                            .child(dynamic(move || {
                                view().flex_col().gap_2().children(
                                    favorites()
                                        .into_iter()
                                        .map(|kb| book_card(store, kb, grid, editing).into_element())
                                        .collect::<Vec<_>>(),
                                )
                            }))
                    },
                ))
                .child(
                    view()
                        .flex_col()
                        .gap_2()
                        .child(section_heading("全部知识库"))
                        .child(when(
                            move || books().is_empty() && !creating.get(),
                            move || {
                                empty_state(
                                    "书架是空的",
                                    "点击右上角「＋ 新建知识库」创建第一本知识库",
                                )
                            },
                        ))
                        .child(dynamic(move || {
                            view().flex_col().gap_2().children(
                                books()
                                    .into_iter()
                                    .map(|kb| book_card(store, kb, grid, editing).into_element())
                                    .collect::<Vec<_>>(),
                            )
                        })),
                ),
        ),
    )
}

/// One book on the shelf: a bordered card in grid mode, a quiet row in
/// list mode. The title opens the book home; the star pins it; the
/// menu edits or trashes it.
fn book_card(
    store: Store,
    kb: KnowledgeBase,
    grid: Signal<bool>,
    editing: Signal<Option<i64>>,
) -> impl IntoElement {
    let id = kb.id;
    let title = kb.name.clone();
    let is_editing = move || editing.get() == Some(id);
    let card = move || {
        let open = move || store.open_book(id);
        view()
            .flex_row()
            .items_center()
            .gap_3()
            .w_full()
            .px_3()
            .py(move || if grid.get() { px(10.0) } else { px(6.0) })
            .rounded(tokens::radius_card())
            .border_1()
            .border_color(move || {
                if grid.get() { tokens::border_subtle() } else { theme().background.opacity(0.0) }
            })
            .bg(move || if grid.get() { theme().card } else { theme().background.opacity(0.0) })
            .hover(|s| s.bg(tokens::control_hover()))
            .child(text(move || store.knowledge_base(id).map(|kb| kb.icon).unwrap_or_else(|| "📕".to_owned())).text_size(px(20.0)))
            .child(
                view()
                    .flex_1()
                    .min_w(px(0.0))
                    .gap_1()
                    .child(
                        nsh_base::button()
                            .on_press(open)
                            .accessibility_label(Some(format!("打开知识库 {title}").into()))
                            .items_center()
                            .child(
                                text(title.clone())
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(tokens::fg_primary())
                                    .number_of_lines(1),
                            ),
                    )
                    .child(
                        text(move || {
                            store
                                .knowledge_base(id)
                                .map(|kb| kb.description)
                                .filter(|desc| !desc.is_empty())
                                .unwrap_or_else(|| "这本知识库还没有简介".to_owned())
                        })
                        .text_size(px(12.0))
                        .text_color(tokens::fg_muted())
                        .number_of_lines(1),
                    )
                    .child(stats_line(store, id)),
            )
            .child(
                nsh_base::button()
                    .on_press(move || {
                        let favorited = store.knowledge_base(id).is_some_and(|kb| kb.favorited);
                        store.set_kb_favorite(id, !favorited);
                    })
                    .accessibility_label(Some(format!("收藏 {title}").into()))
                    .size(px(26.0))
                    .items_center()
                    .justify_center()
                    .rounded(tokens::radius_row())
                    .text_color(move || {
                        if store.knowledge_base(id).is_some_and(|kb| kb.favorited) {
                            tokens::warning()
                        } else {
                            tokens::fg_muted()
                        }
                    })
                    .hover(|s| s.bg(tokens::control_bg()))
                    .child(
                        text(move || {
                            if store.knowledge_base(id).is_some_and(|kb| kb.favorited) {
                                "★"
                            } else {
                                "☆"
                            }
                        })
                        .text_size(px(14.0)),
                    ),
            )
            .child(
                menu()
                    .offset(px(4.0))
                    .popup_style(|s| {
                        s.p_1()
                            .rounded(tokens::radius_card())
                            .bg(theme().popover)
                            .shadow_md()
                            .w(px(180.0))
                    })
                    .trigger(panel_icon_button("⋯", "更多", || {}).accessibility_label(Some(
                        format!("更多 {title}").into(),
                    )))
                    .child(
                        menu_item()
                            .on_select(move || editing.set(Some(id)))
                            .px_3()
                            .py_2()
                            .rounded(tokens::radius_row())
                            .hover(|s| s.bg(tokens::control_hover()))
                            .child(
                                text("编辑信息").text_size(px(13.0)).text_color(tokens::fg_primary()),
                            ),
                    )
                    .child(
                        menu_item()
                            .on_select(move || store.trash_knowledge_base(id))
                            .px_3()
                            .py_2()
                            .rounded(tokens::radius_row())
                            .hover(|s| s.bg(tokens::control_hover()))
                            .child(
                                text("移入回收站")
                                    .text_size(px(13.0))
                                    .text_color(tokens::danger()),
                            ),
                    ),
            )
    };
    when(is_editing, move || view().child(book_edit_form(store, id, editing))).otherwise(card)
}

fn stats_line(store: Store, book_id: i64) -> impl IntoElement {
    text(move || {
        let stats: BookStats = store.book_stats(book_id);
        format!("{} 篇文档 · {} 字", stats.documents, stats.words)
    })
    .text_size(px(11.0))
    .text_color(tokens::fg_muted())
}

/// The inline edit form of one shelf book: name and description.
fn book_edit_form(store: Store, id: i64, editing: Signal<Option<i64>>) -> impl IntoElement {
    let name = signal(store.knowledge_base(id).map(|kb| kb.name).unwrap_or_default());
    let description =
        signal(store.knowledge_base(id).map(|kb| kb.description).unwrap_or_default());
    view()
        .flex_col()
        .gap_2()
        .px_3()
        .py_3()
        .rounded(tokens::radius_card())
        .border_1()
        .border_color(tokens::accent())
        .bg(move || theme().card)
        .child(submit_field(
            "名称",
            move || name.get(),
            move |text: &str| name.set(text.to_owned()),
            move |_: &str| {},
        ))
        .child(submit_field(
            "简介",
            move || description.get(),
            move |text: &str| description.set(text.to_owned()),
            move |_: &str| {},
        ))
        .child(
            view()
                .flex_row()
                .gap_2()
                .child(
                    app_button(ButtonKind::Positive, "保存", move || {
                        store.update_knowledge_base(
                            id,
                            &KnowledgeBasePatch {
                                name: Some(name.get()),
                                description: Some(description.get()),
                                ..Default::default()
                            },
                        );
                        editing.set(None);
                    })
                    .test_id("kb-edit-save"),
                )
                .child(app_button(ButtonKind::Ghost, "取消", move || editing.set(None))),
        )
}

// ---------------------------------------------------------------------
// The book shell: inner catalog sidebar + main content, responsive
// ---------------------------------------------------------------------

/// The two-pane body of an open book (or a document of it): the inner
/// catalog sidebar beside the book home / reading view. Under
/// [`NARROW_BREAKPOINT`] the catalog turns into an overlay drawer so
/// the main pane keeps its width (docs/ui-design.md §3.4).
pub fn book_shell(store: Store, book_id: i64, focus: CatalogFocus) -> impl IntoElement {
    let narrow = move || window().width.0 < NARROW_BREAKPOINT;
    let drawer = signal(false);
    let on_close = move || drawer.set(false);
    view()
        .flex_1()
        .min_w(px(0.0))
        .min_h(px(0.0))
        .flex_row()
        .child(when(
            move || !narrow(),
            move || {
                catalog_sidebar(store, book_id, focus, false, on_close)
                    .w(px(CATALOG_W))
                    .test_id("catalog-inline")
            },
        ))
        .child(
            view()
                .flex_1()
                .min_w(px(0.0))
                .min_h(px(0.0))
                .relative()
                .flex_col()
                .child(when(narrow, move || {
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .py_2()
                        .child(app_button(ButtonKind::Soft, "☰ 目录", move || {
                            drawer.set(true);
                        }))
                }))
                .child(match focus {
                    CatalogFocus::Home => book_home(store, book_id).into_element(),
                    CatalogFocus::Document(doc_id) => document_view(store, doc_id).into_element(),
                })
                .child(when(
                    move || narrow() && drawer.get(),
                    move || {
                        view()
                            .absolute()
                            .inset_0()
                            .flex_row()
                            .child(
                                nsh_base::button()
                                    .on_press(on_close)
                                    .flex_1()
                                    .bg(theme().background.opacity(0.35))
                                    .accessibility_label("关闭目录"),
                            )
                            .child(
                                catalog_sidebar(store, book_id, focus, true, on_close)
                                    .w(px(CATALOG_W))
                                    .shadow_lg()
                                    .test_id("catalog-overlay"),
                            )
                    },
                )),
        )
}

// ---------------------------------------------------------------------
// The inner catalog sidebar
// ---------------------------------------------------------------------

/// The Yuque-style inner catalog sidebar: book switcher, in-book
/// filter, the top `+` (inserts at the top of the level), 首页, and the
/// mixed grouping/document tree with per-node expand state.
pub fn catalog_sidebar(
    store: Store,
    book_id: i64,
    focus: CatalogFocus,
    overlay: bool,
    on_close: impl Fn() + Copy + 'static,
) -> View {
    let filter = signal(String::new());
    let pending = signal(None::<PendingCreate>);
    let draft = signal(String::new());
    let located = signal(None::<i64>);
    let outline = move || store.catalog_outline(book_id);
    let visible =
        computed(move || visible_entries(&outline(), &filter.get()));
    let all_expanded = move || {
        let entries = outline();
        !entries.is_empty() && entries.iter().all(|entry| entry.node.expanded)
    };

    view()
        .h_full()
        .min_h(px(0.0))
        .flex_col()
        .bg(tokens::panel_subtle())
        .border_r_1()
        .border_color(tokens::border_subtle())
        .child(
            // The book switcher row: quick book switching (+ close when
            // the catalog overlays the main pane).
            view()
                .flex_row()
                .items_center()
                .gap_1()
                .px_2()
                .py_2()
                .child(book_switcher(store, book_id).flex_1().min_w(px(0.0)))
                .child(when(
                    move || overlay,
                    move || panel_icon_button("✕", "收起目录", on_close),
                )),
        )
        .child(
            // The tool row: in-book filter, or the quick-create row
            // while a creation is pending.
            view().px_2().pb_2().child(when(
                move || pending.get().is_some(),
                move || {
                    let kind = pending.get().expect("pending");
                    let (label, placeholder): (String, String) = match kind {
                        PendingCreate::Document { parent } => match parent {
                            Some(parent) => (
                                "新建子文档".to_owned(),
                                format!(
                                    "在「{}」下新建文档，回车创建",
                                    node_title(&outline(), parent)
                                ),
                            ),
                            None => (
                                "新建文档".to_owned(),
                                "在目录顶部新建文档，回车创建".to_owned(),
                            ),
                        },
                        PendingCreate::Group { parent } => match parent {
                            Some(parent) => (
                                "新建子分组".to_owned(),
                                format!(
                                    "在「{}」下新建分组，回车创建",
                                    node_title(&outline(), parent)
                                ),
                            ),
                            None => (
                                "新建分组".to_owned(),
                                "在目录顶部新建分组，回车创建".to_owned(),
                            ),
                        },
                    };
                    view()
                        .flex_col()
                        .gap_1()
                        .child(text(label).text_size(px(11.0)).text_color(tokens::fg_muted()))
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .child(submit_field(
                                    placeholder,
                                    move || draft.get(),
                                    move |text: &str| draft.set(text.to_owned()),
                                    move |text: &str| {
                                        commit_create(store, book_id, kind, text);
                                        draft.set(String::new());
                                        pending.set(None);
                                    },
                                ))
                                .child(
                                    app_button(ButtonKind::Positive, "创建", move || {
                                        commit_create(store, book_id, kind, &draft.get());
                                        draft.set(String::new());
                                        pending.set(None);
                                    })
                                    .test_id("catalog-create"),
                                )
                                .child(panel_icon_button("✕", "取消创建", move || {
                                    pending.set(None);
                                    draft.set(String::new());
                                })),
                        )
                },
            )).child(when(
                move || pending.get().is_none(),
                move || {
                    input_field(
                        "筛选目录",
                        move || filter.get(),
                        move |text: &str| filter.set(text.to_owned()),
                    )
                    .test_id("catalog-filter")
                },
            )),
        )
        .child(
            view()
                .flex_col()
                .gap(px(2.0))
                .px_2()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .child(home_row(store, book_id, focus).flex_1().min_w(px(0.0)))
                        .child(
                            when(
                                move || focus != CatalogFocus::Home,
                                move || {
                                    view().child(panel_icon_button("◎", "定位当前文档", move || {
                                        if let CatalogFocus::Document(doc_id) = focus {
                                            filter.set(String::new());
                                            if let Some(node) = store.catalog_node_of_document(doc_id)
                                            {
                                                located.set(Some(node.id));
                                                for ancestor in ancestor_ids(&outline(), node.id) {
                                                    store.set_node_expanded(ancestor, true);
                                                }
                                                store.set_node_expanded(node.id, true);
                                            }
                                        }
                                    }))
                                },
                            )
                            .otherwise(|| view().w(px(26.0))),
                        )
                        .child(when(
                            move || !all_expanded(),
                            move || {
                                panel_icon_button("⤢", "全部展开", move || {
                                    store.set_catalog_expanded_all(book_id, true)
                                })
                            },
                        ))
                        .child(when(
                            all_expanded,
                            move || {
                                panel_icon_button("⤡", "全部收起", move || {
                                    store.set_catalog_expanded_all(book_id, false)
                                })
                            },
                        )),
                )
                .child(
                    menu()
                        .offset(px(4.0))
                        .popup_style(|s| {
                            s.p_1()
                                .rounded(tokens::radius_card())
                                .bg(theme().popover)
                                .shadow_md()
                                .w(px(200.0))
                        })
                        .trigger(
                            nsh_base::button()
                                .accessibility_label("新建")
                                .test_id("catalog-new")
                                .h(px(28.0))
                                .w_full()
                                .items_center()
                                .justify_center()
                                .gap_1()
                                .rounded(tokens::radius_row())
                                .border_1()
                                .border_color(tokens::border_subtle())
                                .bg(move || theme().background.opacity(0.0))
                                .text_color(tokens::fg_muted())
                                .hover(|s| s.bg(tokens::control_hover()).text_color(tokens::fg_primary()))
                                .child(text("＋").text_size(px(13.0)))
                                .child(text("新建").text_size(px(12.0))),
                        )
                        .child(
                            menu_item()
                                .on_select(move || {
                                    pending.set(Some(PendingCreate::Document { parent: None }));
                                })
                                .px_3()
                                .py_2()
                                .rounded(tokens::radius_row())
                                .hover(|s| s.bg(tokens::control_hover()))
                                .child(
                                    text("在顶部新建文档")
                                        .text_size(px(13.0))
                                        .text_color(tokens::fg_primary()),
                                ),
                        )
                        .child(
                            menu_item()
                                .on_select(move || {
                                    pending.set(Some(PendingCreate::Group { parent: None }));
                                })
                                .px_3()
                                .py_2()
                                .rounded(tokens::radius_row())
                                .hover(|s| s.bg(tokens::control_hover()))
                                .child(
                                    text("在顶部新建分组")
                                        .text_size(px(13.0))
                                        .text_color(tokens::fg_primary()),
                                ),
                        ),
                ),
        )
        .child(
            scroll()
                .flex_1()
                .min_h(px(0.0))
                .child(
                    view()
                        .flex_col()
                        .gap(px(2.0))
                        .p_2()
                        .child(when(
                            move || outline().is_empty(),
                            move || {
                                empty_state(
                                    "这本书还没有目录",
                                    "用上方「＋ 新建」在顶部添加第一篇文档",
                                )
                            },
                        ))
                        .child(each(
                            visible,
                            |entry: &CatalogEntry| entry.node.id,
                            move |entry, index| {
                                catalog_row(
                                    CatalogCtx { store, focus, filter, pending, located },
                                    entry,
                                    index,
                                    move || visible.get(),
                                )
                            },
                        )),
                )
                .test_id("catalog-tree"),
        )
        .child(when(
            move || !store.trashed_catalog_nodes(book_id).is_empty(),
            move || {
                view()
                    .flex_col()
                    .gap_1()
                    .p_2()
                    .border_t_1()
                    .border_color(tokens::border_subtle())
                    .child(section_heading("最近删除"))
                    .child(
                        text("恢复会带回它的整个子树").text_size(px(11.0)).text_color(tokens::fg_muted()),
                    )
                    .child(each(
                        move || store.trashed_catalog_nodes(book_id),
                        |node: &crate::domain::CatalogNode| node.id,
                        move |node, _index| {
                            view()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .px_2()
                                .py_1()
                                .child(
                                    text(move || {
                                        if node.with(|node| node.document_id.is_some()) {
                                            "📄"
                                        } else {
                                            "📁"
                                        }
                                    })
                                    .text_size(px(12.0)),
                                )
                                .child(
                                    text(move || node.with(|node| node.title.clone()))
                                        .text_size(px(12.0))
                                        .text_color(tokens::fg_muted())
                                        .number_of_lines(1)
                                        .flex_1(),
                                )
                                .child(
                                    nsh_base::button()
                                        .on_press(move || {
                                            let id = node.with(|node| node.id);
                                            store.restore_catalog_node(id);
                                        })
                                        .accessibility_label(move || {
                                            Some(format!("恢复 {}", node.with(|node| node.title.clone())).into())
                                        })
                                        .px_2()
                                        .py_1()
                                        .rounded(tokens::radius_row())
                                        .text_color(tokens::info())
                                        .hover(|s| s.bg(tokens::control_hover()))
                                        .child(text("恢复").text_size(px(12.0))),
                                )
                        },
                    ))
            },
        ))
}

fn commit_create(store: Store, book_id: i64, kind: PendingCreate, text: &str) {
    let parent = match kind {
        PendingCreate::Document { parent } => {
            if let Some(parent) = parent {
                store.create_document_in(book_id, Some(parent), text);
            } else {
                store.create_document_in(book_id, None, text);
            }
            return;
        }
        PendingCreate::Group { parent } => parent,
    };
    if let Some(parent) = parent {
        store.set_node_expanded(parent, true);
        store.create_group_in(book_id, Some(parent), text);
    } else {
        store.create_group_in(book_id, None, text);
    }
}

fn node_title(outline: &[CatalogEntry], id: i64) -> String {
    outline
        .iter()
        .find(|entry| entry.node.id == id)
        .map(|entry| entry.node.title.clone())
        .unwrap_or_default()
}

fn home_row(store: Store, book_id: i64, focus: CatalogFocus) -> nsh_base::ButtonElement {
    let active = focus == CatalogFocus::Home;
    nsh_base::button()
        .on_press(move || store.open_book(book_id))
        .accessibility_label("首页")
        .h(px(30.0))
        .w_full()
        .items_center()
        .gap_2()
        .px_2()
        .rounded(tokens::radius_row())
        .bg(move || if active { tokens::accent_soft() } else { theme().background.opacity(0.0) })
        .text_color(move || if active { tokens::accent() } else { tokens::fg_muted() })
        .hover(|s| s.bg(tokens::control_hover()))
        .child(text("🏠").text_size(px(13.0)))
        .child(text("首页").text_size(px(13.0)))
}

fn book_switcher(store: Store, book_id: i64) -> nsh_base::MenuElement {
    menu()
        .offset(px(4.0))
        .popup_style(|s| {
            s.p_1()
                .rounded(tokens::radius_card())
                .bg(theme().popover)
                .shadow_md()
                .max_w(px(260.0))
        })
        .trigger(
            nsh_base::button()
                .accessibility_label("切换知识库")
                .test_id("book-switcher")
                .h(px(30.0))
                .w_full()
                .items_center()
                .gap_2()
                .px_2()
                .rounded(tokens::radius_row())
                .bg(move || theme().background.opacity(0.0))
                .text_color(tokens::fg_primary())
                .hover(|s| s.bg(tokens::control_hover()))
                .child(
                    text(move || {
                        store
                            .knowledge_base(book_id)
                            .map(|kb| kb.icon)
                            .unwrap_or_else(|| "📕".to_owned())
                    })
                    .text_size(px(13.0)),
                )
                .child(
                    text(move || {
                        store
                            .knowledge_base(book_id)
                            .map(|kb| kb.name)
                            .unwrap_or_else(|| "知识库".to_owned())
                    })
                    .text_size(px(13.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .number_of_lines(1)
                    .flex_1(),
                )
                .child(text("▾").text_size(px(11.0)).text_color(tokens::fg_muted())),
        )
        .child(each(
            move || store.knowledge_bases(),
            |kb: &KnowledgeBase| kb.id,
            move |kb, _index| {
                menu_item()
                    .on_select(move || {
                        let id = kb.with(|kb| kb.id);
                        if id != book_id {
                            store.open_book(id);
                        }
                    })
                    .accessibility_label(move || {
                        Some(format!("切换到 {}", kb.with(|kb| kb.name.clone())).into())
                    })
                    .px_3()
                    .py_2()
                    .rounded(tokens::radius_row())
                    .hover(|s| s.bg(tokens::control_hover()))
                    .child(
                        text(move || {
                            format!("{} {}", kb.with(|kb| kb.icon.clone()), kb.with(|kb| kb.name.clone()))
                        })
                        .text_size(px(13.0))
                        .text_color(tokens::fg_primary()),
                    )
            },
        ))
        .child(menu_separator().h(px(1.0)).my_1().bg(tokens::border_subtle()))
        .child(
            menu_item()
                .on_select(move || store.open_or_focus(Route::KnowledgeBases))
                .px_3()
                .py_2()
                .rounded(tokens::radius_row())
                .hover(|s| s.bg(tokens::control_hover()))
                .child(text("＋ 新建知识库").text_size(px(13.0)).text_color(tokens::accent())),
        )
}

/// One row of the catalog tree: caret (when it has children), icon and
/// title; the hover `＋` nests a new child document under it, the menu
/// creates a child group, reorders/reparents, or trashes the subtree.
fn catalog_row(
    ctx: CatalogCtx,
    entry: ReadSignal<CatalogEntry>,
    index: ReadSignal<usize>,
    visible: impl Fn() -> Vec<CatalogEntry> + Copy + 'static,
) -> impl IntoElement {
    let CatalogCtx { store, focus, filter, pending, located } = ctx;
    let node_id = move || entry.with(|entry| entry.node.id);
    let is_document = move || entry.with(|entry| entry.node.document_id.is_some());
    let depth = move || entry.with(|entry| entry.depth);
    let has_children = move || {
        let list = visible();
        let index = index.get();
        matches!((list.get(index), list.get(index + 1)), (Some(a), Some(b)) if b.depth == a.depth + 1)
    };
    let filtering = move || !filter.get().is_empty();
    let expanded = move || entry.with(|entry| entry.node.expanded);
    let is_current = move || {
        entry.with(|entry| match focus {
            CatalogFocus::Document(doc_id) => entry.node.document_id == Some(doc_id),
            CatalogFocus::Home => false,
        })
    };
    let title = move || entry.with(|entry| entry.node.title.clone());

    view()
        .flex_row()
        .items_center()
        .gap_1()
        .pl(move || px(depth() as f32 * 14.0))
        .pr_1()
        .h(px(30.0))
        .rounded(tokens::radius_row())
        .bg(move || {
            if located.get() == Some(node_id()) || is_current() {
                tokens::accent_soft()
            } else {
                theme().background.opacity(0.0)
            }
        })
        .hover(|s| s.bg(tokens::control_hover()))
        .child(
            view().w(px(26.0)).child(when(
                move || has_children() && !filtering(),
                move || {
                    panel_icon_button(
                        move || if expanded() { "▾" } else { "▸" },
                        "切换展开",
                        move || store.set_node_expanded(node_id(), !expanded()),
                    )
                    .accessibility_label(move || {
                        Some(format!("{} {}", if expanded() { "收起" } else { "展开" }, title()).into())
                    })
                },
            )),
        )
        .child(
            nsh_base::button()
                .on_press(move || {
                    let (doc, id, was_expanded) =
                        entry.with(|entry| (entry.node.document_id, entry.node.id, entry.node.expanded));
                    if let Some(doc_id) = doc {
                        store.open_document(doc_id);
                    } else {
                        store.set_node_expanded(id, !was_expanded);
                    }
                })
                .accessibility_label(move || Some(title().into()))
                .flex_1()
                .min_w(px(0.0))
                .items_center()
                .gap_2()
                .child(text(move || if is_document() { "📄" } else { "📁" }).text_size(px(13.0)))
                .child(
                    text(title)
                        .text_size(px(13.0))
                        .number_of_lines(1)
                        .text_color(tokens::fg_primary()),
                ),
        )
        .child(
            nsh_base::button()
                .on_press(move || {
                    let parent = Some(node_id());
                    pending.set(Some(PendingCreate::Document { parent }));
                })
                .accessibility_label("新建子文档")
                .size(px(22.0))
                .items_center()
                .justify_center()
                .rounded_full()
                .text_color(tokens::fg_muted())
                .hover(|s| s.bg(tokens::control_hover()).text_color(tokens::accent()))
                .child(text("＋").text_size(px(12.0))),
        )
        .child(
            menu()
                .offset(px(4.0))
                .popup_style(|s| {
                    s.p_1()
                        .rounded(tokens::radius_card())
                        .bg(theme().popover)
                        .shadow_md()
                        .w(px(180.0))
                })
                .trigger(panel_icon_button("⋯", "节点菜单", || {}))
                .child(
                    menu_item()
                        .on_select(move || {
                            let parent = Some(node_id());
                            pending.set(Some(PendingCreate::Group { parent }));
                        })
                        .px_3()
                        .py_2()
                        .rounded(tokens::radius_row())
                        .hover(|s| s.bg(tokens::control_hover()))
                        .child(
                            text("新建子分组").text_size(px(13.0)).text_color(tokens::fg_primary()),
                        ),
                )
                .child(menu_separator().h(px(1.0)).my_1().bg(tokens::border_subtle()))
                .child(
                    menu_item()
                        .on_select(move || store.move_catalog_node_rel(node_id(), -1))
                        .px_3()
                        .py_2()
                        .rounded(tokens::radius_row())
                        .hover(|s| s.bg(tokens::control_hover()))
                        .child(text("上移").text_size(px(13.0)).text_color(tokens::fg_primary())),
                )
                .child(
                    menu_item()
                        .on_select(move || store.move_catalog_node_rel(node_id(), 1))
                        .px_3()
                        .py_2()
                        .rounded(tokens::radius_row())
                        .hover(|s| s.bg(tokens::control_hover()))
                        .child(text("下移").text_size(px(13.0)).text_color(tokens::fg_primary())),
                )
                .child(
                    menu_item()
                        .on_select(move || store.move_catalog_node_to_top(node_id()))
                        .px_3()
                        .py_2()
                        .rounded(tokens::radius_row())
                        .hover(|s| s.bg(tokens::control_hover()))
                        .child(
                            text("移到目录顶层").text_size(px(13.0)).text_color(tokens::fg_primary()),
                        ),
                )
                .child(menu_separator().h(px(1.0)).my_1().bg(tokens::border_subtle()))
                .child(
                    menu_item()
                        .on_select(move || store.trash_catalog_node(node_id()))
                        .px_3()
                        .py_2()
                        .rounded(tokens::radius_row())
                        .hover(|s| s.bg(tokens::control_hover()))
                        .child(
                            text("移入回收站").text_size(px(13.0)).text_color(tokens::danger()),
                        ),
                ),
        )
}

// ---------------------------------------------------------------------
// The book-style home page
// ---------------------------------------------------------------------

/// The book-style home (docs/ui-design.md §4.4.2): a centered paper
/// card with the big book icon, live stats, favorite and create
/// actions, the description, and the collapsible dot-leader catalog.
pub fn book_home(store: Store, book_id: i64) -> impl IntoElement {
    let editing = signal(None::<i64>);
    view().flex_1().min_h(px(0.0)).child(
        scroll().flex_1().child(
            view()
                .flex_col()
                .gap_4()
                .p_5()
                .max_w(px(1000.0))
                .w_full()
                .self_center()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_4()
                        .child(
                            text(move || {
                                store
                                    .knowledge_base(book_id)
                                    .map(|kb| kb.icon)
                                    .unwrap_or_else(|| "📕".to_owned())
                            })
                            .text_size(px(44.0)),
                        )
                        .child(
                            view()
                                .flex_1()
                                .min_w(px(0.0))
                                .gap_1()
                                .child(
                                    text(move || {
                                        store
                                            .knowledge_base(book_id)
                                            .map(|kb| kb.name)
                                            .unwrap_or_else(|| "知识库".to_owned())
                                    })
                                    .text_size(px(28.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(tokens::fg_primary()),
                                )
                                .child(
                                    text(move || {
                                        let stats: BookStats = store.book_stats(book_id);
                                        format!("{} 篇文档 · {} 字", stats.documents, stats.words)
                                    })
                                    .text_size(px(13.0))
                                    .text_color(tokens::fg_muted()),
                                ),
                        )
                        .child(
                            nsh_base::button()
                                .on_press(move || {
                                    let favorited =
                                        store.knowledge_base(book_id).is_some_and(|kb| kb.favorited);
                                    store.set_kb_favorite(book_id, !favorited);
                                })
                                .accessibility_label("收藏本书")
                                .h(px(32.0))
                                .px_3()
                                .items_center()
                                .gap_1()
                                .rounded_full()
                                .border_1()
                                .border_color(tokens::border_subtle())
                                .text_color(move || {
                                    if store.knowledge_base(book_id).is_some_and(|kb| kb.favorited) {
                                        tokens::warning()
                                    } else {
                                        tokens::fg_muted()
                                    }
                                })
                                .hover(|s| s.bg(tokens::control_hover()))
                                .child(
                                    text(move || {
                                        if store.knowledge_base(book_id).is_some_and(|kb| kb.favorited)
                                        {
                                            "★ 已收藏"
                                        } else {
                                            "☆ 收藏"
                                        }
                                    })
                                    .text_size(px(13.0)),
                                ),
                        )
                        .child(
                            app_button(ButtonKind::Positive, "＋ 新建文档", move || {
                                if let Some(id) = store.create_document_in(book_id, None, "未命名文档") {
                                    store.open_document(id);
                                }
                            }),
                        ),
                )
                .child(when(
                    move || editing.get().is_none(),
                    move || {
                        let has_description = store
                            .knowledge_base(book_id)
                            .is_some_and(|kb| !kb.description.is_empty());
                        view()
                            .flex_col()
                            .gap_2()
                            .items_start()
                            .child(
                                text(move || {
                                    store
                                        .knowledge_base(book_id)
                                        .map(|kb| kb.description)
                                        .filter(|desc| !desc.is_empty())
                                        .unwrap_or_else(|| "这本知识库还没有简介。".to_owned())
                                })
                                .text_size(px(14.0))
                                .text_color(move || {
                                    if has_description {
                                        tokens::fg_body()
                                    } else {
                                        tokens::fg_muted()
                                    }
                                }),
                            )
                            .child(app_button(ButtonKind::Ghost, "编辑信息", move || {
                                editing.set(Some(book_id));
                            }))
                    },
                ))
                .child(when(move || editing.get().is_some(), move || {
                    book_edit_form(store, book_id, editing)
                }))
                .child(home_catalog(store, book_id)),
        ),
    )
}

/// The dot-leader catalog preview of the book home: one row per visible
/// node, title on the left, its update date on the right, a dotted
/// leader running between.
fn home_catalog(store: Store, book_id: i64) -> impl IntoElement {
    let visible = move || visible_entries(&store.catalog_outline(book_id), "");
    view()
        .flex_col()
        .gap_1()
        .child(section_heading("目录"))
        .child(when(
            move || visible().is_empty(),
            move || empty_state("这本书还没有目录", "点击上方「＋ 新建文档」开始"),
        ))
        .child(each(
            visible,
            |entry: &CatalogEntry| entry.node.id,
            move |entry, _index| {
                let title = move || entry.with(|entry| entry.node.title.clone());
                let depth = move || entry.with(|entry| entry.depth);
                let is_document = move || entry.with(|entry| entry.node.document_id.is_some());
                let updated = move || {
                    entry
                        .with(|entry| entry.node.updated_at.split(' ').next().unwrap_or("").to_owned())
                };
                view()
                    .flex_row()
                    .items_end()
                    .gap_2()
                    .pl(move || px(depth() as f32 * 16.0))
                    .py(px(4.0))
                    .child(
                        nsh_base::button()
                            .on_press(move || {
                                let (doc, id, was_expanded) = entry.with(|entry| {
                                    (entry.node.document_id, entry.node.id, entry.node.expanded)
                                });
                                if let Some(doc_id) = doc {
                                    store.open_document(doc_id);
                                } else {
                                    store.set_node_expanded(id, !was_expanded);
                                }
                            })
                            .accessibility_label(move || Some(title().into()))
                            .items_end()
                            .gap_2()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(
                                text(move || if is_document() { "📄".to_owned() } else { "📁".to_owned() })
                                    .text_size(px(13.0)),
                            )
                            .child(
                                text(title)
                                    .text_size(px(14.0))
                                    .text_color(tokens::fg_primary())
                                    .number_of_lines(1),
                            )
                            .child(
                                view()
                                    .flex_1()
                                    .min_w(px(12.0))
                                    .border_b_1()
                                    .border_dotted()
                                    .border_color(tokens::border_default())
                                    .mb(px(4.0)),
                            )
                            .child(
                                text(updated).text_size(px(11.0)).text_color(tokens::fg_muted()),
                            ),
                    )
                    .child(when(
                        move || !is_document(),
                        move || {
                            panel_icon_button(
                                move || if entry.with(|entry| entry.node.expanded) { "▾" } else { "▸" },
                                "切换展开",
                                move || {
                                    let (id, was_expanded) =
                                        entry.with(|entry| (entry.node.id, entry.node.expanded));
                                    store.set_node_expanded(id, !was_expanded);
                                },
                            )
                        },
                    ))
            },
        ))
}

// ---------------------------------------------------------------------
// The document reading view
// ---------------------------------------------------------------------

/// A Document's reading view (docs/ui-design.md §4.4.3, content only):
/// title, body paragraphs, and the word count. The block editor itself
/// ships in milestone #6.
pub fn document_view(store: Store, doc_id: i64) -> impl IntoElement {
    view().flex_1().min_h(px(0.0)).child(
        scroll().flex_1().child(
            view()
                .flex_col()
                .gap_4()
                .p_5()
                .max_w(px(780.0))
                .w_full()
                .self_center()
                .child(
                    text(move || {
                        store
                            .document(doc_id)
                            .map(|doc| doc.title)
                            .unwrap_or_else(|| "文档".to_owned())
                    })
                    .text_size(px(32.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(tokens::fg_primary()),
                )
                .child(when(
                    move || store.document(doc_id).is_some_and(|doc| doc.body.trim().is_empty()),
                    move || {
                        empty_state(
                            "正文是空的",
                            "块级编辑器将在后续里程碑交付；当前为阅读视图",
                        )
                    },
                ))
                .child(dynamic(move || {
                    let body = store.document(doc_id).map(|doc| doc.body).unwrap_or_default();
                    view().flex_col().gap_3().children(
                        body.split('\n')
                            .filter(|line| !line.trim().is_empty())
                            .map(|line| {
                                text(line.to_owned())
                                    .text_size(px(15.0))
                                    .text_color(tokens::fg_body())
                                    .into_element()
                            })
                            .collect::<Vec<_>>(),
                    )
                }))
                .child(
                    text(move || {
                        let words = store
                            .document(doc_id)
                            .map(|doc| crate::domain::count_words(&doc.body))
                            .unwrap_or_default();
                        format!("{} 字", words)
                    })
                    .text_size(px(11.0))
                    .text_color(tokens::fg_muted()),
                ),
        ),
    )
}

// ---------------------------------------------------------------------
// The Trash screen
// ---------------------------------------------------------------------

/// The Trash screen: trashed Knowledge Bases with restore and
/// permanent-delete actions. Catalog node trash/restore is exercised
/// through the workspace and drives the same subtree rules; the full
/// Trash milestone polishes this screen further.
pub fn trash_view(store: Store) -> impl IntoElement {
    let rows = move || store.trashed_knowledge_bases();
    view().flex_1().min_h(px(0.0)).child(
        scroll().flex_1().child(
            view()
                .flex_col()
                .gap_3()
                .p_5()
                .max_w(px(860.0))
                .w_full()
                .self_center()
                .child(
                    text("回收站")
                        .text_size(px(24.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(tokens::fg_primary()),
                )
                .child(when(
                    move || rows().is_empty(),
                    move || empty_state("回收站是空的", "移入回收站的知识库会在这里出现"),
                ))
                .child(dynamic(move || {
                    view().flex_col().gap_2().children(
                        rows()
                            .into_iter()
                            .map(|kb| {
                                let id = kb.id;
                                let title = kb.name.clone();
                                view()
                                    .flex_row()
                                    .items_center()
                                    .gap_3()
                                    .px_3()
                                    .py_2()
                                    .rounded(tokens::radius_row())
                                    .border_1()
                                    .border_color(tokens::border_subtle())
                                    .child(text(kb.icon.clone()).text_size(px(18.0)))
                                    .child(
                                        text(title)
                                            .text_size(px(14.0))
                                            .text_color(tokens::fg_primary())
                                            .flex_1()
                                            .number_of_lines(1),
                                    )
                                    .child(
                                        app_button(ButtonKind::Soft, "恢复", move || {
                                            store.restore_knowledge_base(id);
                                        })
                                        .test_id(&format!("kb-restore-{id}")),
                                    )
                                    .child(
                                        app_button(ButtonKind::Danger, "彻底删除", move || {
                                            store.delete_knowledge_base_permanent(id);
                                        })
                                        .test_id(&format!("kb-delete-{id}")),
                                    )
                                    .into_element()
                            })
                            .collect::<Vec<_>>(),
                    )
                })),
        ),
    )
}

// ---------------------------------------------------------------------
// Tree rules (pure)
// ---------------------------------------------------------------------

/// Which outline entries render: everything under an expanded chain,
/// or the flat title matches while filtering.
fn visible_entries(entries: &[CatalogEntry], filter: &str) -> Vec<CatalogEntry> {
    if !filter.is_empty() {
        let needle = filter.to_lowercase();
        return entries
            .iter()
            .filter(|entry| entry.node.title.to_lowercase().contains(&needle))
            .cloned()
            .collect();
    }
    let mut visible = Vec::new();
    // The pre-order walk carries each ancestor's expand flag at its
    // depth; a chain stops rendering the moment one link collapses.
    let mut chain: Vec<bool> = Vec::new();
    for entry in entries {
        chain.truncate(entry.depth);
        if chain.iter().all(|expanded| *expanded) {
            visible.push(entry.clone());
        }
        chain.push(entry.node.expanded);
    }
    visible
}

/// The ancestor ids of `target` along the pre-order outline (nearest
/// first), for locate-current's expand pass.
fn ancestor_ids(entries: &[CatalogEntry], target: i64) -> Vec<i64> {
    let Some(pos) = entries.iter().position(|entry| entry.node.id == target) else {
        return Vec::new();
    };
    let mut ancestors = Vec::new();
    let mut want = entries[pos].depth;
    for entry in entries[..pos].iter().rev() {
        if entry.depth + 1 == want {
            ancestors.push(entry.node.id);
            want = entry.depth;
        }
    }
    ancestors
}
