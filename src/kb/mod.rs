//! The Knowledge Base flows (docs/ui-design.md §3.3, §4.4): the shelf
//! with grid/list and favorites, the book-style home page with the
//! dot-leader catalog, and the inner catalog sidebar (book switcher,
//! in-book filtering, locate-current, expand/collapse-all, per-node
//! expand state persisted in SQLite).

use nsh_base::{context_menu, menu, menu_item, menu_separator, theme};
use nsh_core::prelude::*;
use nsh_core::{TouchEvent, TouchPhase};

use crate::domain::{BookStats, CatalogEntry, KnowledgeBase, KnowledgeBasePatch};
use crate::shell::store::Store;
use crate::shell::tabs::Route;
use crate::ui::controls::{
    ButtonKind, app_button, empty_state, input_field, panel_icon_button, section_heading,
    submit_field,
};
use crate::ui::theme as tokens;

/// The inner catalog sidebar's width (docs/ui-design.md §3.3).
pub const CATALOG_W: f32 = 252.0;
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

/// How the catalog's host view behaves (the book shell versus a
/// Project's Linked Documents sub-view): where "首页" and document rows
/// navigate, and where a root-level `新建文档` lands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CatalogNav {
    /// The book shell: 首页 opens the book home, documents open in
    /// workbench tabs, root-level creations go to the top of the book
    /// root.
    Book,
    /// A Project's Linked Documents sub-view: 首页 clears the open
    /// document back to the scoped book home, documents open inside the
    /// sub-view (per tab), and root-level creations land under the
    /// linked grouping node (`None` when bound to the whole book).
    Project { linked_node: Option<i64> },
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
    nav: CatalogNav,
    filter: Signal<String>,
    pending: Signal<Option<PendingCreate>>,
    /// The row being renamed (`None` while no rename is pending); the
    /// rename field takes the row's place, like `book_edit_form` on the
    /// shelf.
    renaming: Signal<Option<i64>>,
    /// The rename field's draft: only one row renames at a time, so the
    /// rows share it.
    rename_draft: Signal<String>,
    /// Locate-current's target: the row with this node id reveals itself
    /// (expands the chain first, then scrolls itself into view).
    reveal: Signal<Option<i64>>,
    /// The row drag's shared state (measure registry, live target,
    /// dragged node), like the board's `BoardDrag`.
    drag: CatalogDrag,
}

/// Where a dragged row would land on its target (docs/kanon.html's
/// catalog drop: the top quarter inserts before, the center half nests
/// as a child, the bottom quarter inserts after).
#[derive(Debug, Clone, Copy, PartialEq)]
enum DropZone {
    Before,
    Child,
    After,
}

/// The row under the dragged node and the zone the drop commits.
#[derive(Debug, Clone, Copy, PartialEq)]
struct DropTarget {
    node: i64,
    zone: DropZone,
}

/// The catalog row drag's shared state: every mounted row's ref for the
/// pan-start measurement, the measured rectangles for the release
/// hit-test, the dragged node, and the live target for the row
/// highlight. Rows register/unregister themselves; the pan lives on each
/// row like the board card's, committed through
/// `Store::move_catalog_node`.
#[derive(Clone, Copy)]
struct CatalogDrag {
    /// Mounted rows' element refs, keyed by node id.
    rows: Stored<Vec<(i64, ElementRef)>>,
    /// Row bounds in root coordinates, measured when a drag starts.
    rects: Signal<Vec<(i64, Bounds<Points>)>>,
    /// The node being dragged (`None` while no drag runs).
    active: Signal<Option<i64>>,
    /// The drop target under the pointer, for the row highlight.
    target: Signal<Option<DropTarget>>,
}

impl CatalogDrag {
    fn new() -> Self {
        Self {
            rows: stored(Vec::new()),
            rects: signal(Vec::new()),
            active: signal(None),
            target: signal(None),
        }
    }

    /// Remeasures every mounted row; unmounted rows (filtered or
    /// trashed) drop out of the hit-test.
    fn measure_rows(&self) {
        let rects = self.rects;
        let rows = self.rows.get();
        rects.set(Vec::new());
        spawn(async move {
            let mut measured = Vec::with_capacity(rows.len());
            for (node, reference) in rows {
                if let Some(measurement) = reference.measure().await {
                    measured.push((
                        node,
                        Bounds {
                            origin: measurement.page,
                            size: measurement.frame.size,
                        },
                    ));
                }
            }
            rects.set(measured);
        });
    }
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
    let favorites = move || {
        books()
            .into_iter()
            .filter(|kb| kb.favorited)
            .collect::<Vec<_>>()
    };

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
                                .text_size(px(28.0))
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
                                    move || {
                                        panel_icon_button(
                                            "☰",
                                            "切换为列表视图",
                                            move || grid.set(false),
                                        )
                                    },
                                ))
                                .child(when(
                                    move || !grid.get(),
                                    move || {
                                        panel_icon_button(
                                            "▦",
                                            "切换为卡片网格",
                                            move || grid.set(true),
                                        )
                                    },
                                ))
                                .child(app_button(
                                    ButtonKind::Primary,
                                    "＋ 新建知识库",
                                    move || {
                                        creating.set(true);
                                    },
                                )),
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
                            .child(section_heading("常用知识库"))
                            .child(dynamic(move || {
                                if grid.get() {
                                    view()
                                        .flex_row()
                                        .flex_wrap()
                                        .gap_3()
                                        .children(
                                            favorites()
                                                .into_iter()
                                                .map(|kb| {
                                                    book_card(store, kb, grid, editing)
                                                        .into_element()
                                                })
                                                .collect::<Vec<_>>(),
                                        )
                                        .into_element()
                                } else {
                                    view()
                                        .flex_col()
                                        .gap_2()
                                        .children(
                                            favorites()
                                                .into_iter()
                                                .map(|kb| {
                                                    book_card(store, kb, grid, editing)
                                                        .into_element()
                                                })
                                                .collect::<Vec<_>>(),
                                        )
                                        .into_element()
                                }
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
                            if grid.get() {
                                view()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap_3()
                                    .children(
                                        books()
                                            .into_iter()
                                            .map(|kb| {
                                                book_card(store, kb, grid, editing).into_element()
                                            })
                                            .collect::<Vec<_>>(),
                                    )
                                    .into_element()
                            } else {
                                view()
                                    .flex_col()
                                    .gap_2()
                                    .children(
                                        books()
                                            .into_iter()
                                            .map(|kb| {
                                                book_card(store, kb, grid, editing).into_element()
                                            })
                                            .collect::<Vec<_>>(),
                                    )
                                    .into_element()
                            }
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
    let open = move || store.open_book(id);
    let card_children = move || -> Vec<AnyElement> {
        vec![
            text(move || {
                store
                    .knowledge_base(id)
                    .map(|kb| kb.icon)
                    .unwrap_or_else(|| "📕".to_owned())
            })
            .text_size(px(20.0))
            .into_element(),
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
                .child(stats_line(store, id))
                .into_element(),
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
                )
                .into_element(),
            menu()
                .offset(px(4.0))
                .popup_style(|s| {
                    s.p_1()
                        .rounded(tokens::radius_card())
                        .bg(theme().popover)
                        .shadow_md()
                        .w(px(180.0))
                })
                .trigger(
                    panel_icon_button("⋯", "更多", || {})
                        .accessibility_label(Some(format!("更多 {title}").into())),
                )
                .child(
                    menu_item()
                        .on_select(move || editing.set(Some(id)))
                        .px_3()
                        .py_2()
                        .rounded(tokens::radius_row())
                        .hover(|s| s.bg(tokens::control_hover()))
                        .child(
                            text("编辑信息")
                                .text_size(px(13.0))
                                .text_color(tokens::fg_primary()),
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
                )
                .into_element(),
        ]
    };
    // Grid mode draws the prototype's `.tile`: a fixed-width bordered
    // card; list mode draws its `.listline`: a full-width quiet row.
    let card_chrome = move |tile: bool| {
        let row = view()
            .flex_row()
            .items_center()
            .gap_3()
            .px_3()
            .py(if tile { px(10.0) } else { px(6.0) })
            .rounded(tokens::radius_card())
            .border_1()
            .border_color(if tile {
                tokens::border_subtle()
            } else {
                theme().background.opacity(0.0)
            })
            .bg(if tile {
                theme().card
            } else {
                theme().background.opacity(0.0)
            })
            .hover(|s| s.bg(tokens::control_hover()))
            .children(card_children());
        if tile {
            row.w(px(216.0)).flex_none().into_element()
        } else {
            row.w_full().into_element()
        }
    };
    when(is_editing, move || {
        view().child(book_edit_form(store, id, editing))
    })
    .otherwise(move || {
        // A fresh clone per build: the dynamic factory owns its copy
        // (`otherwise`'s factory stays `Fn`).
        let card_chrome = card_chrome.clone();
        dynamic(move || card_chrome(grid.get()))
    })
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
    let name = signal(
        store
            .knowledge_base(id)
            .map(|kb| kb.name)
            .unwrap_or_default(),
    );
    let description = signal(
        store
            .knowledge_base(id)
            .map(|kb| kb.description)
            .unwrap_or_default(),
    );
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
                .child(app_button(ButtonKind::Ghost, "取消", move || {
                    editing.set(None)
                })),
        )
}

// ---------------------------------------------------------------------
// The book shell: inner catalog sidebar + main content, responsive
// ---------------------------------------------------------------------

/// The two-pane body of an open book (or a document of it): the inner
/// catalog sidebar beside the book home / reading view. Under
/// [`NARROW_BREAKPOINT`] the catalog turns into an overlay drawer so
/// the main pane keeps its width (docs/ui-design.md §3.4).
pub fn book_shell(
    store: Store,
    tab_id: u64,
    book_id: i64,
    focus: CatalogFocus,
    nav: CatalogNav,
) -> impl IntoElement {
    let narrow = move || window().width.0 < NARROW_BREAKPOINT;
    let drawer = signal(false);
    let hidden = move || store.catalog_hidden(tab_id);
    let on_close = move || {
        if narrow() {
            drawer.set(false);
        } else {
            store.set_catalog_hidden(tab_id, true);
        }
    };
    view()
        .flex_1()
        .min_w(px(0.0))
        .min_h(px(0.0))
        .flex_row()
        .child(when(
            move || !narrow() && !hidden(),
            move || {
                catalog_sidebar(store, book_id, focus, false, on_close, nav)
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
                .child(when(
                    move || narrow() || hidden(),
                    move || {
                        catalog_reopen(move || {
                            if narrow() {
                                drawer.set(true);
                            } else {
                                store.set_catalog_hidden(tab_id, false);
                            }
                        })
                    },
                ))
                .child(match focus {
                    CatalogFocus::Home => book_home(store, book_id, None, nav).into_element(),
                    CatalogFocus::Document(doc_id) => {
                        document_view(store, tab_id, doc_id).into_element()
                    }
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
                                catalog_sidebar(store, book_id, focus, true, on_close, nav)
                                    .w(px(CATALOG_W))
                                    .shadow_lg()
                                    .test_id("catalog-overlay"),
                            )
                    },
                )),
        )
}

pub(crate) fn catalog_reopen(open: impl Fn() + 'static) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .px_3()
        .py_2()
        .child(app_button(ButtonKind::Soft, "☰ 目录", open).accessibility_label("展开目录"))
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
    nav: CatalogNav,
) -> View {
    let filter = signal(String::new());
    let pending = signal(None::<PendingCreate>);
    let draft = signal(String::new());
    let renaming = signal(None::<i64>);
    let rename_draft = signal(String::new());
    let reveal = signal(None::<i64>);
    let drag = CatalogDrag::new();
    let dismiss_overlay = move || {
        if overlay {
            on_close();
        }
    };
    let outline = move || {
        let entries = store.catalog_outline(book_id);
        // A Project's Linked Documents sub-view browses only the linked
        // grouping node's subtree, like the prototype's
        // `tree(book, projectGroup)` (docs/kanon.html).
        match nav {
            CatalogNav::Project {
                linked_node: Some(root),
            } => scope_to_subtree(&entries, root),
            _ => entries,
        }
    };
    let visible = computed(move || visible_entries(&outline(), &filter.get()));
    let all_expanded = move || {
        let entries = outline();
        !entries.is_empty() && entries.iter().all(|entry| entry.node.expanded)
    };
    // Where a root-level `新建文档` lands: the top of the book root in a
    // book shell, the linked grouping node in a Project's Linked
    // Documents sub-view (ADR-0001).
    let root_parent: Option<i64> = match nav {
        CatalogNav::Book => None,
        CatalogNav::Project { linked_node } => linked_node,
    };

    view()
        .h_full()
        .min_h(px(0.0))
        .flex_col()
        .bg(tokens::panel_subtle())
        .border_r_1()
        .border_color(tokens::border_subtle())
        .child(
            // The close action folds the inline panel or dismisses the drawer.
            view()
                .flex_row()
                .items_center()
                .gap_1()
                .px_2()
                .py_2()
                .child(book_switcher(store, book_id).flex_1().min_w(px(0.0)))
                .child(panel_icon_button("✕", "收起目录", on_close)),
        )
        .child(
            // The tool row (docs/ui-design.md §3.3): the in-book filter
            // beside the `+` quick-create menu, or the quick-create row
            // itself while a creation is pending.
            view()
                .px_2()
                .pb_2()
                .child(when(
                    move || pending.get().is_some(),
                    move || {
                        let kind = pending.get().expect("pending");
                        let (label, placeholder): (String, String) = match kind {
                            PendingCreate::Document { parent } => match parent.or(root_parent) {
                                Some(parent) => (
                                    "新建文档".to_owned(),
                                    format!(
                                        "在「{}」下新建文档，回车创建",
                                        node_title(store, &outline(), parent)
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
                                        node_title(store, &outline(), parent)
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
                            .child(
                                text(label)
                                    .text_size(px(11.0))
                                    .text_color(tokens::fg_muted()),
                            )
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
                                            commit_create(store, book_id, kind, text, root_parent);
                                            draft.set(String::new());
                                            pending.set(None);
                                        },
                                    ))
                                    .child(
                                        app_button(ButtonKind::Positive, "创建", move || {
                                            commit_create(
                                                store,
                                                book_id,
                                                kind,
                                                &draft.get(),
                                                root_parent,
                                            );
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
                ))
                .child(when(
                    move || pending.get().is_none(),
                    move || {
                        view()
                            .flex_row()
                            .items_center()
                            .gap_1()
                            .child(
                                input_field(
                                    "筛选目录",
                                    move || filter.get(),
                                    move |text: &str| filter.set(text.to_owned()),
                                )
                                .test_id("catalog-filter"),
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
                                        panel_icon_button("＋", "新建", move || {})
                                            .test_id("catalog-new"),
                                    )
                                    .child(
                                        menu_item()
                                            .on_select(move || {
                                                pending.set(Some(PendingCreate::Document {
                                                    parent: None,
                                                }));
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
                                                pending.set(Some(PendingCreate::Group {
                                                    parent: None,
                                                }));
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
                            )
                    },
                )),
        )
        .child(
            view()
                .flex_col()
                .gap(px(2.0))
                .px_2()
                .child(home_row(store, book_id, focus, nav, dismiss_overlay))
                .child(
                    // The 目录 header row (docs/ui-design.md §3.3): the
                    // section title — the linked grouping node's name in a
                    // Project's sub-view — with 定位当前文档 and the
                    // expand/collapse-all pair on the right.
                    view()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .child(
                            text(move || match nav {
                                CatalogNav::Project {
                                    linked_node: Some(node),
                                } => store
                                    .catalog_node(node)
                                    .map(|node| node.title)
                                    .unwrap_or_default(),
                                _ => "目录".to_owned(),
                            })
                            .text_size(px(11.0))
                            .text_color(tokens::fg_muted())
                            .flex_1()
                            .min_w(px(0.0)),
                        )
                        .child(
                            when(
                                move || focus != CatalogFocus::Home,
                                move || {
                                    view().child(panel_icon_button(
                                        "◎",
                                        "定位当前文档",
                                        move || {
                                            if let CatalogFocus::Document(doc_id) = focus {
                                                filter.set(String::new());
                                                if let Some(node) =
                                                    store.catalog_node_of_document(doc_id)
                                                {
                                                    for ancestor in
                                                        ancestor_ids(&outline(), node.id)
                                                    {
                                                        store.set_node_expanded(ancestor, true);
                                                    }
                                                    store.set_node_expanded(node.id, true);
                                                    // The chain is open: the
                                                    // row itself scrolls into
                                                    // view once it renders
                                                    // (docs/kanon.html's
                                                    // locate ends in a
                                                    // scrollIntoView).
                                                    reveal.set(Some(node.id));
                                                }
                                            }
                                        },
                                    ))
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
                        .child(when(all_expanded, move || {
                            panel_icon_button("⤡", "全部收起", move || {
                                store.set_catalog_expanded_all(book_id, false)
                            })
                        })),
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
                            move |entry, _index| {
                                catalog_row(
                                    CatalogCtx {
                                        store,
                                        focus,
                                        nav,
                                        filter,
                                        pending,
                                        renaming,
                                        rename_draft,
                                        reveal,
                                        drag,
                                    },
                                    entry,
                                    outline,
                                    dismiss_overlay,
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
                        text("恢复会带回它的整个子树")
                            .text_size(px(11.0))
                            .text_color(tokens::fg_muted()),
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
                                            Some(
                                                format!(
                                                    "恢复 {}",
                                                    node.with(|node| node.title.clone())
                                                )
                                                .into(),
                                            )
                                        })
                                        .focus_visible(|s| {
                                            s.outline(px(2.0)).outline_color(tokens::accent())
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

fn commit_create(
    store: Store,
    book_id: i64,
    kind: PendingCreate,
    text: &str,
    root_parent: Option<i64>,
) {
    let parent = match kind {
        PendingCreate::Document { parent } => {
            // A Project's Linked Documents view retargets the root-level
            // creation into its linked grouping node (ADR-0001).
            let parent = parent.or(root_parent);
            if let Some(parent) = parent {
                store.create_document_in(book_id, Some(parent), text);
            } else {
                store.create_document_in(book_id, None, text);
            }
            return;
        }
        PendingCreate::Group { parent } => parent.or(root_parent),
    };
    if let Some(parent) = parent {
        store.set_node_expanded(parent, true);
        store.create_group_in(book_id, Some(parent), text);
    } else {
        store.create_group_in(book_id, None, text);
    }
}

/// The title of `id` for inline placeholders: the visible outline
/// first, then the store (a Project's linked grouping node sits above
/// the scoped outline and never appears in it).
fn node_title(store: Store, outline: &[CatalogEntry], id: i64) -> String {
    outline
        .iter()
        .find(|entry| entry.node.id == id)
        .map(|entry| entry.node.title.clone())
        .or_else(|| store.catalog_node(id).map(|node| node.title))
        .unwrap_or_default()
}

/// The catalog row menu's reorder: `delta` of -1/1 moves the node one
/// slot within its level. The sibling ordering is a presentation
/// concern: it is computed here, in the catalog UI layer, and committed
/// through `Store::move_catalog_node`.
fn move_catalog_node_rel(store: Store, id: i64, delta: isize) {
    let (book_id, parent) = {
        let node = store.catalog_node(id);
        match node {
            Some(node) => (node.book_id, node.parent_id),
            None => return,
        }
    };
    let outline = store.catalog_outline(book_id);
    let siblings = outline
        .iter()
        .filter(|entry| entry.node.parent_id == parent)
        .map(|entry| entry.node.id)
        .collect::<Vec<_>>();
    let Some(position) = siblings.iter().position(|sibling| *sibling == id) else {
        return;
    };
    let at = (position as isize + delta).clamp(0, siblings.len() as isize - 1) as usize;
    store.move_catalog_node(id, parent, at);
}

/// The catalog row menu's outdent: move the node to the top of the
/// book's root level.
fn move_catalog_node_to_top(store: Store, id: i64) {
    store.move_catalog_node(id, None, 0);
}

/// The drop target under `at` (root coordinates): the first measured row
/// the point falls in, other than the dragged node and its descendants
/// (a drop there would cycle). The zone follows docs/kanon.html's
/// catalog drop — the top quarter inserts before the row, the center
/// half nests into it, the bottom quarter inserts after.
fn catalog_drop_target(
    rects: &[(i64, Bounds<Points>)],
    outline: &[CatalogEntry],
    dragged: i64,
    at: Point<Points>,
) -> Option<DropTarget> {
    rects
        .iter()
        .find(|(node, bounds)| {
            *node != dragged
                && bounds.contains(&at)
                && !ancestor_ids(outline, *node).contains(&dragged)
        })
        .map(|(node, bounds)| {
            let span = bounds.size.height.0.max(1.0);
            let pos = (at.y - bounds.origin.y).0 / span;
            let zone = if pos <= 0.25 {
                DropZone::Before
            } else if pos >= 0.75 {
                DropZone::After
            } else {
                DropZone::Child
            };
            DropTarget { node: *node, zone }
        })
}

/// Commits a catalog drop through `Store::move_catalog_node`: the child
/// zone appends under the target (expanding it, so the moved node stays
/// visible); the outer zones insert before/after the target among its
/// siblings. Returns false when the store refuses the move (a cycle, a
/// cross-book target), leaving the tree untouched.
fn catalog_commit_drop(
    store: Store,
    outline: &[CatalogEntry],
    dragged: i64,
    target: DropTarget,
) -> bool {
    match target.zone {
        DropZone::Child => {
            let at = outline
                .iter()
                .filter(|entry| entry.node.parent_id == Some(target.node))
                .filter(|entry| entry.node.id != dragged)
                .count();
            let moved = store.move_catalog_node(dragged, Some(target.node), at);
            if moved {
                store.set_node_expanded(target.node, true);
            }
            moved
        }
        DropZone::Before | DropZone::After => {
            let Some(entry) = outline.iter().find(|entry| entry.node.id == target.node) else {
                return false;
            };
            let parent = entry.node.parent_id;
            let siblings = outline
                .iter()
                .filter(|entry| entry.node.parent_id == parent)
                .filter(|entry| entry.node.id != dragged)
                .map(|entry| entry.node.id)
                .collect::<Vec<_>>();
            let Some(at) = siblings.iter().position(|id| *id == target.node) else {
                return false;
            };
            let at = if target.zone == DropZone::Before {
                at
            } else {
                at + 1
            };
            store.move_catalog_node(dragged, parent, at)
        }
    }
}

/// The catalog row's actions, shared by the ⋯ overflow menu and the
/// right-click context menu (docs/kanon.html's `nodeMenu`): a child
/// group, rename, the reorder triple, and trash.
fn catalog_node_menu_items(
    store: Store,
    node_id: impl Fn() -> i64 + Copy + 'static,
    title: impl Fn() -> String + Copy + 'static,
    pending: Signal<Option<PendingCreate>>,
    renaming: Signal<Option<i64>>,
    rename_draft: Signal<String>,
    rename_locked: impl Fn() -> bool + Copy + 'static,
) -> Vec<AnyElement> {
    fn item(
        label: &'static str,
        color: fn() -> AdaptiveColor,
        on_select: impl Fn() + 'static,
    ) -> AnyElement {
        menu_item()
            .on_select(on_select)
            .px_3()
            .py_2()
            .rounded(tokens::radius_row())
            .hover(|s| s.bg(tokens::control_hover()))
            .child(text(label).text_size(px(13.0)).text_color(color()))
            .into_element()
    }
    let separator = || {
        menu_separator()
            .h(px(1.0))
            .my_1()
            .bg(tokens::border_subtle())
            .into_element()
    };
    vec![
        item("新建子分组", tokens::fg_primary, move || {
            let parent = Some(node_id());
            pending.set(Some(PendingCreate::Group { parent }));
        }),
        separator(),
        menu_item()
            .disabled(rename_locked)
            .on_select(move || {
                rename_draft.set(title());
                renaming.set(Some(node_id()));
            })
            .px_3()
            .py_2()
            .rounded(tokens::radius_row())
            .hover(|s| s.bg(tokens::control_hover()))
            .child(
                text("重命名")
                    .text_size(px(13.0))
                    .text_color(tokens::fg_primary()),
            )
            .into_element(),
        item("上移", tokens::fg_primary, move || {
            move_catalog_node_rel(store, node_id(), -1)
        }),
        item("下移", tokens::fg_primary, move || {
            move_catalog_node_rel(store, node_id(), 1)
        }),
        item("移到目录顶层", tokens::fg_primary, move || {
            move_catalog_node_to_top(store, node_id())
        }),
        separator(),
        item("移入回收站", tokens::danger, move || {
            store.trash_catalog_node(node_id())
        }),
    ]
}

fn home_row(
    store: Store,
    book_id: i64,
    focus: CatalogFocus,
    nav: CatalogNav,
    dismiss_overlay: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    let active = focus == CatalogFocus::Home;
    nsh_base::button()
        .on_press(move || {
            dismiss_overlay();
            match nav {
                CatalogNav::Book => store.open_book(book_id),
                CatalogNav::Project { .. } => store.set_project_document(None),
            }
        })
        .accessibility_label("首页")
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
        .h(px(30.0))
        .w_full()
        .flex_row()
        .justify_start()
        .items_center()
        .gap_2()
        .px_2()
        .rounded(tokens::radius_row())
        .bg(move || {
            if active {
                tokens::accent_soft()
            } else {
                theme().background.opacity(0.0)
            }
        })
        .text_color(move || {
            if active {
                tokens::accent()
            } else {
                tokens::fg_muted()
            }
        })
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
                .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                .h(px(30.0))
                .w_full()
                .flex_row()
                .justify_start()
                .min_w(px(0.0))
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
                    .min_w(px(0.0))
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
                            format!(
                                "{} {}",
                                kb.with(|kb| kb.icon.clone()),
                                kb.with(|kb| kb.name.clone())
                            )
                        })
                        .text_size(px(13.0))
                        .text_color(tokens::fg_primary()),
                    )
            },
        ))
        .child(
            menu_separator()
                .h(px(1.0))
                .my_1()
                .bg(tokens::border_subtle()),
        )
        .child(
            menu_item()
                .on_select(move || store.open_or_focus(Route::KnowledgeBases))
                .px_3()
                .py_2()
                .rounded(tokens::radius_row())
                .hover(|s| s.bg(tokens::control_hover()))
                .child(
                    text("＋ 新建知识库")
                        .text_size(px(13.0))
                        .text_color(tokens::accent()),
                ),
        )
}

/// One row of the catalog tree: caret (when it has children), icon and
/// title; the hover `＋` nests a new child document under it, the menu
/// creates a child group, reorders/reparents, or trashes the subtree.
fn catalog_row(
    ctx: CatalogCtx,
    entry: ReadSignal<CatalogEntry>,
    outline: impl Fn() -> Vec<CatalogEntry> + Copy + 'static,
    dismiss_overlay: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let CatalogCtx {
        store,
        focus,
        nav,
        filter,
        pending,
        renaming,
        rename_draft,
        reveal,
        drag,
    } = ctx;
    let node_id = move || entry.with(|entry| entry.node.id);
    let is_document = move || entry.with(|entry| entry.node.document_id.is_some());
    let depth = move || entry.with(|entry| entry.depth);
    // Children exist in the FULL outline. `visible` hides collapsed
    // subtrees, so judging by it drops the caret the moment a node
    // collapses — and then it can never re-expand.
    let has_children = move || {
        let id = node_id();
        outline()
            .iter()
            .any(|entry| entry.node.parent_id == Some(id))
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
    let renaming_this = move || renaming.get() == Some(node_id());
    // Renaming a Document whose edit session is live would fight the
    // editor's auto-save (the store refuses it); the menu item stays
    // disabled meanwhile, so the gesture never dead-ends.
    let rename_locked = move || {
        entry
            .with(|entry| entry.node.document_id)
            .is_some_and(|doc_id| store.editing_locked(doc_id))
    };
    // Locate-current scrolls the open document's row into sight. The
    // target row does the reveal itself: it mounts only once the
    // expanded chain renders, so a signal drives it instead of the
    // header button reaching into the tree.
    let row_ref = element_ref();
    effect(move || {
        if reveal.get() == Some(node_id()) {
            reveal.set(None);
            row_ref.reveal(RevealOptions::default().vertical(ScrollAlignment::Nearest));
        }
    });
    // The drag (docs/kanon.html's catalog drop): the row pans with the
    // pointer (its values move it), the rows measured at pan start
    // hit-test the release — top quarter before, center half child,
    // bottom quarter after — and a release nowhere, a cancel, or an
    // invalid target springs back and changes nothing. Only the desktop
    // arms the pan, so touch drags keep scrolling the tree.
    let row_id = node_id();
    drag.rows.with(|rows| rows.push((row_id, row_ref)));
    cleanup(move || {
        drag.rows
            .try_with(|rows| rows.retain(|(_, reference)| *reference != row_ref));
    });
    let drag_x = animated_value(0.0);
    let drag_y = animated_value(0.0);
    let down = signal(None::<Point<Points>>);
    let dragging_this = move || drag.active.get() == Some(node_id());
    let zone = move || {
        drag.target
            .get()
            .and_then(|target| (target.node == node_id()).then_some(target.zone))
    };

    // One item set for both menus (the row's ⋯ overflow and the
    // right-click context menu): a pointer user and a keyboard user
    // see identical node actions.
    let menu_items = move || {
        catalog_node_menu_items(
            store,
            node_id,
            title,
            pending,
            renaming,
            rename_draft,
            rename_locked,
        )
    };
    let pan_armed = move || is_desktop() && !renaming_this();
    let row = view()
        .element_ref(&row_ref)
        .test_id("catalog-row")
        .flex_row()
        .items_center()
        .gap_1()
        .pl(move || px(depth() as f32 * 12.0))
        .pr_1()
        .h(px(30.0))
        .rounded(tokens::radius_row())
        .bind_translate_x(&drag_x)
        .bind_translate_y(&drag_y)
        .pan_values(Some(&drag_x), Some(&drag_y))
        .pan_enabled(pan_armed)
        .on_touch(move |touch: &TouchEvent| match touch.phase {
            TouchPhase::Start => down.set(touch.points.first().map(|p| p.page)),
            TouchPhase::Move | TouchPhase::End => {
                if drag.active.get_untracked() == Some(row_id)
                    && let Some(at) = touch.points.first().map(|p| p.page)
                {
                    drag.target.set(drag.rects.with_untracked(|rects| {
                        catalog_drop_target(rects, &outline(), row_id, at)
                    }));
                }
            }
            TouchPhase::Cancel => drag.target.set(None),
        })
        .on_pan_start(move |_: &PanEvent| {
            drag.active.set(Some(node_id()));
            drag.measure_rows();
        })
        .on_pan_end(move |event: &PanEvent| {
            let released = down
                .get()
                .map(|origin| origin + event.translation)
                .filter(|_| !event.cancelled);
            let target = released
                .and_then(|at| catalog_drop_target(&drag.rects.get(), &outline(), node_id(), at));
            let committed = target
                .is_some_and(|target| catalog_commit_drop(store, &outline(), node_id(), target));
            drag.active.set(None);
            drag.target.set(None);
            if committed || reduce_motion() {
                drag_x.set(0.0);
                drag_y.set(0.0);
            } else {
                drag_x.spring(0.0, Spring::default());
                drag_y.spring(0.0, Spring::default());
            }
        })
        // The dragged row lifts above its neighbors; the drop target
        // answers the zone — a child nest tints the row, the outer
        // quarters draw the accent edge (the prototype's `.over`).
        .z_index(move || if dragging_this() { 10 } else { 0 })
        // The current document's row highlights; locate-current expands
        // the chain to it, so no separate marker is needed (a stale one
        // would keep the old row lit after switching documents).
        .bg(move || {
            if zone() == Some(DropZone::Child) || is_current() {
                tokens::accent_soft()
            } else if dragging_this() {
                tokens::panel_raised()
            } else {
                theme().background.opacity(0.0)
            }
        })
        .border_t(move || {
            if zone() == Some(DropZone::Before) {
                px(2.0)
            } else {
                px(0.0)
            }
        })
        .border_b(move || {
            if zone() == Some(DropZone::After) {
                px(2.0)
            } else {
                px(0.0)
            }
        })
        .border_color(move || match zone() {
            Some(DropZone::Before) | Some(DropZone::After) => tokens::accent(),
            _ => tokens::border_subtle(),
        })
        .hover(|s| s.bg(tokens::control_hover()))
        .child(view().w(px(26.0)).child(when(
            move || has_children() && !filtering(),
            move || {
                view()
                    .child(
                        panel_icon_button(
                            move || if expanded() { "▾" } else { "▸" },
                            "切换展开",
                            move || store.set_node_expanded(node_id(), !expanded()),
                        )
                        .accessibility_label(move || {
                            Some(
                                format!("{} {}", if expanded() { "收起" } else { "展开" }, title())
                                    .into(),
                            )
                        }),
                    )
                    .accessibility_state(move || nsh_core::AccessibilityState {
                        expanded: Some(expanded()),
                        ..Default::default()
                    })
            },
        )))
        // The rename field takes the title's place while its row
        // renames (docs/kanon.html's node menu leads with 重命名);
        // Enter commits, ✕ cancels, and a refused or unchanged rename
        // just closes again.
        .child(
            when(renaming_this, move || {
                view()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(
                        input_field(
                            "重命名，回车确认",
                            move || rename_draft.get(),
                            move |text: &str| rename_draft.set(text.to_owned()),
                        )
                        .label("重命名，回车确认")
                        .auto_focus(true)
                        .on_submit(move |text: &str| {
                            if !text.trim().is_empty() {
                                store.rename_catalog_node(node_id(), text);
                                renaming.set(None);
                            }
                        }),
                    )
                    .child(panel_icon_button("✕", "取消重命名", move || {
                        renaming.set(None)
                    }))
            })
            .otherwise(move || {
                nsh_base::button()
                    .on_press(move || {
                        let (doc, id, was_expanded) = entry.with(|entry| {
                            (entry.node.document_id, entry.node.id, entry.node.expanded)
                        });
                        if let Some(doc_id) = doc {
                            dismiss_overlay();
                            match nav {
                                CatalogNav::Book => store.open_document(doc_id),
                                CatalogNav::Project { .. } => {
                                    store.set_project_document(Some(doc_id))
                                }
                            }
                        } else {
                            store.set_node_expanded(id, !was_expanded);
                        }
                    })
                    .accessibility_label(move || Some(title().into()))
                    .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                    .flex_1()
                    .min_w(px(0.0))
                    .flex_row()
                    .justify_start()
                    .items_center()
                    .gap_2()
                    .child(
                        text(move || if is_document() { "📄" } else { "📁" }).text_size(px(13.0)),
                    )
                    .child(
                        text(title)
                            .text_size(px(13.0))
                            .number_of_lines(1)
                            .text_color(tokens::fg_primary()),
                    )
            }),
        )
        .child(when(
            move || !renaming_this(),
            move || {
                nsh_base::button()
                    .on_press(move || {
                        let parent = Some(node_id());
                        pending.set(Some(PendingCreate::Document { parent }));
                    })
                    .accessibility_label("新建子文档")
                    .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                    .size(px(22.0))
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .text_color(tokens::fg_muted())
                    .hover(|s| s.bg(tokens::control_hover()).text_color(tokens::accent()))
                    .child(text("＋").text_size(px(12.0)))
            },
        ))
        .child(when(
            move || !renaming_this(),
            move || {
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
                    .children(menu_items())
            },
        ));

    // The right-click menu (docs/kanon.html's `nodeMenu`): the same
    // actions at the pointer, the row its trigger; the ⋯ menu above
    // stays as the explicit overflow path.
    context_menu()
        .offset(px(4.0))
        .popup_style(|s| {
            s.p_1()
                .rounded(tokens::radius_card())
                .bg(theme().popover)
                .shadow_md()
                .w(px(180.0))
        })
        .trigger(row)
        .children(menu_items())
}

// ---------------------------------------------------------------------
// The book-style home page
// ---------------------------------------------------------------------

/// The book-style home (docs/ui-design.md §4.4.2, docs/kanon.html
/// `bookHome`): a centered paper card on the subtle canvas — the big
/// book icon, live (subtree-scoped) stats, the favorite action, the
/// description, and the collapsible dot-leader catalog. `scope` narrows
/// the page to a grouping node's subtree (a Project's Linked Documents
/// sub-view); `nav` decides where documents open.
pub fn book_home(
    store: Store,
    book_id: i64,
    scope: Option<i64>,
    nav: CatalogNav,
) -> impl IntoElement {
    let editing = signal(None::<i64>);
    let outline = move || {
        let entries = store.catalog_outline(book_id);
        match scope {
            Some(root) => scope_to_subtree(&entries, root),
            None => entries,
        }
    };
    // Scoped stats over the same subtree the dot-leader catalog lists.
    let stats = move || {
        let mut documents = 0_usize;
        let mut words = 0_usize;
        for entry in outline() {
            if let Some(doc_id) = entry.node.document_id
                && let Some(doc) = store.document(doc_id)
            {
                documents += 1;
                words += doc.word_count as usize;
            }
        }
        (documents, words)
    };
    let title = move || match scope {
        Some(root) => store
            .catalog_node(root)
            .map(|node| node.title)
            .unwrap_or_default(),
        None => store
            .knowledge_base(book_id)
            .map(|kb| kb.name)
            .unwrap_or_else(|| "知识库".to_owned()),
    };
    view().flex_1().min_h(px(0.0)).child(
        // The book-page canvas (docs/kanon.html `.book-page`).
        scroll().flex_1().bg(tokens::panel_subtle()).child(
            view()
                .flex_col()
                .gap_4()
                .p_5()
                .w_full()
                .max_w(px(1000.0))
                .self_center()
                .child(
                    // The paper itself (docs/kanon.html `.book-paper`).
                    view()
                        .flex_col()
                        .gap_4()
                        .w_full()
                        .rounded(tokens::radius_card())
                        .border_1()
                        .border_color(tokens::border_subtle())
                        .bg(move || theme().card)
                        .p_6()
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
                                            text(title)
                                                .text_size(px(28.0))
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(tokens::fg_primary()),
                                        )
                                        .child(
                                            text(move || {
                                                let (documents, words) = stats();
                                                format!("{} 篇文档 · {} 字", documents, words)
                                            })
                                            .text_size(px(13.0))
                                            .text_color(tokens::fg_muted()),
                                        ),
                                )
                                .child(
                                    nsh_base::button()
                                        .on_press(move || {
                                            let favorited = store
                                                .knowledge_base(book_id)
                                                .is_some_and(|kb| kb.favorited);
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
                                            if store
                                                .knowledge_base(book_id)
                                                .is_some_and(|kb| kb.favorited)
                                            {
                                                tokens::warning()
                                            } else {
                                                tokens::fg_muted()
                                            }
                                        })
                                        .hover(|s| s.bg(tokens::control_hover()))
                                        .child(
                                            text(move || {
                                                if store
                                                    .knowledge_base(book_id)
                                                    .is_some_and(|kb| kb.favorited)
                                                {
                                                    "★ 已收藏"
                                                } else {
                                                    "☆ 收藏"
                                                }
                                            })
                                            .text_size(px(13.0)),
                                        ),
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
                                                .unwrap_or_else(|| {
                                                    "这本知识库还没有简介。".to_owned()
                                                })
                                        })
                                        .text_size(px(14.0))
                                        .text_color(
                                            move || {
                                                if has_description {
                                                    tokens::fg_body()
                                                } else {
                                                    tokens::fg_muted()
                                                }
                                            },
                                        ),
                                    )
                                    .child(app_button(
                                        ButtonKind::Ghost,
                                        "编辑信息",
                                        move || {
                                            editing.set(Some(book_id));
                                        },
                                    ))
                            },
                        ))
                        .child(when(
                            move || editing.get().is_some(),
                            move || book_edit_form(store, book_id, editing),
                        ))
                        .child(home_catalog(store, book_id, scope, nav)),
                ),
        ),
    )
}

/// The dot-leader catalog of the book home (docs/kanon.html
/// `.book-line`): one row per visible node of the (scoped) subtree, the
/// title on the left, its update date — or `分组` for a pure grouping
/// node — on the right, a dotted leader running between. The 目录
/// header carries the create action, like the prototype.
fn home_catalog(
    store: Store,
    book_id: i64,
    scope: Option<i64>,
    nav: CatalogNav,
) -> impl IntoElement {
    let outline = move || {
        let entries = store.catalog_outline(book_id);
        match scope {
            Some(root) => scope_to_subtree(&entries, root),
            None => entries,
        }
    };
    let visible = move || visible_entries(&outline(), "");
    // A Project's Linked Documents sub-view names where its new
    // documents land (ADR-0001); the book shell doesn't need the note.
    let scope_hint = move || match nav {
        CatalogNav::Project { linked_node } => {
            store.bump.get();
            match linked_node {
                Some(node) => {
                    let title = store
                        .catalog_node(node)
                        .map(|node| node.title)
                        .unwrap_or_default();
                    format!("新建文档将落在关联目录「{}」下", title)
                }
                None => {
                    let book = store
                        .knowledge_base(book_id)
                        .map(|kb| kb.name)
                        .unwrap_or_else(|| "知识库".to_owned());
                    format!("新建文档将落在知识库「{}」的目录顶部", book)
                }
            }
        }
        CatalogNav::Book => String::new(),
    };
    let create_document = move || {
        if let Some(id) = store.create_document_in(book_id, scope, "未命名文档") {
            match nav {
                CatalogNav::Book => store.open_document(id),
                CatalogNav::Project { .. } => store.set_project_document(Some(id)),
            }
            let tab_id = store
                .strip
                .with_untracked(|strip| strip.tabs[strip.active].id);
            store.request_edit(tab_id, id);
        }
    };
    view()
        .flex_col()
        .gap_1()
        .test_id("home-catalog")
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .pb_2()
                .border_b_1()
                .border_color(tokens::border_subtle())
                .child(section_heading("目录"))
                .child(match nav {
                    // The Project sub-view keeps its own selector (the
                    // Linked Documents creation path is covered by the
                    // behavior tests).
                    CatalogNav::Project { .. } => {
                        app_button(ButtonKind::Positive, "＋ 新建文档", create_document)
                            .test_id("project-doc-create")
                            .into_element()
                    }
                    CatalogNav::Book => {
                        app_button(ButtonKind::Positive, "＋ 新建文档", create_document)
                            .into_element()
                    }
                }),
        )
        .child(when(
            move || !scope_hint().is_empty(),
            move || {
                text(scope_hint)
                    .text_size(px(11.0))
                    .text_color(tokens::fg_muted())
                    .px_1()
            },
        ))
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
                    entry.with(|entry| {
                        if entry.node.document_id.is_some() {
                            entry
                                .node
                                .updated_at
                                .split(' ')
                                .next()
                                .unwrap_or("")
                                .to_owned()
                        } else {
                            "分组".to_owned()
                        }
                    })
                };
                view()
                    .flex_row()
                    .items_end()
                    .gap_2()
                    .pl(move || px(depth() as f32 * 15.0))
                    .py(px(4.0))
                    .child(
                        nsh_base::button()
                            .on_press(move || {
                                let (doc, id, was_expanded) = entry.with(|entry| {
                                    (entry.node.document_id, entry.node.id, entry.node.expanded)
                                });
                                if let Some(doc_id) = doc {
                                    match nav {
                                        CatalogNav::Book => store.open_document(doc_id),
                                        CatalogNav::Project { .. } => {
                                            store.set_project_document(Some(doc_id))
                                        }
                                    }
                                } else {
                                    store.set_node_expanded(id, !was_expanded);
                                }
                            })
                            .accessibility_label(move || Some(title().into()))
                            .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                            .flex_row()
                            .items_center()
                            .justify_start()
                            .gap_2()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(
                                text(move || {
                                    if is_document() {
                                        "📄".to_owned()
                                    } else {
                                        "📁".to_owned()
                                    }
                                })
                                .text_size(px(13.0)),
                            )
                            .child(
                                text(title)
                                    .text_size(px(14.0))
                                    .text_color(tokens::fg_primary())
                                    .min_w(px(0.0))
                                    .number_of_lines(1),
                            )
                            .child(
                                view()
                                    .flex_1()
                                    .min_w(px(12.0))
                                    .border_b_1()
                                    .border_dotted()
                                    .border_color(tokens::border_default()),
                            )
                            .child(
                                text(updated)
                                    .text_size(px(11.0))
                                    .flex_none()
                                    .text_color(tokens::fg_muted()),
                            ),
                    )
                    .child(when(
                        move || !is_document(),
                        move || {
                            panel_icon_button(
                                move || {
                                    if entry.with(|entry| entry.node.expanded) {
                                        "▾"
                                    } else {
                                        "▸"
                                    }
                                },
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
/// A Document's reading/editing screen (docs/ui-design.md §4.4): the
/// block editor with auto-save, local history, and the single-editor
/// lock ships in milestone #6 — this is its workbench mount point.
pub fn document_view(store: Store, tab_id: u64, doc_id: i64) -> impl IntoElement {
    crate::document::document_view(store, tab_id, doc_id)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TrashPurge {
    Book(i64),
    Project(i64),
    Task(i64),
    Note(i64),
    Node(i64),
}

impl TrashPurge {
    fn warning(self) -> (&'static str, &'static str) {
        match self {
            Self::Book(_) => ("彻底删除知识库？", "知识库及其中的目录和文档将无法恢复。"),
            Self::Project(_) => (
                "彻底删除项目？",
                "项目无法恢复。任务将解除归属，知识库和文档不受影响。",
            ),
            Self::Task(_) => (
                "彻底删除任务？",
                "任务无法恢复；文档中的关联任务卡片将失效。",
            ),
            Self::Note(_) => ("彻底删除小记？", "删除后无法恢复。"),
            Self::Node(_) => ("彻底删除目录？", "该目录、下级目录及其文档将无法恢复。"),
        }
    }

    fn confirm(self, store: Store) -> bool {
        match self {
            Self::Book(id) => store.delete_knowledge_base_permanent(id),
            Self::Project(id) => store.delete_project_permanent(id),
            Self::Task(id) => store.delete_task_permanent(id),
            Self::Note(id) => return store.delete_quick_note_permanent(id),
            Self::Node(id) => store.delete_catalog_node_permanent(id),
        }
        true
    }
}

/// The Trash screen: trashed Knowledge Bases, Projects and catalog node
/// subtrees, each with restore and permanent-delete actions. Permanent
/// deletion applies the ADR-0001 scope rules: a Project's Tasks are
/// detached (never deleted), and a catalog node's subtree deletion
/// resets bound Projects to unbound.
pub fn trash_view(store: Store) -> impl IntoElement {
    let books = move || store.trashed_knowledge_bases();
    let projects = move || store.trashed_projects();
    let nodes = move || store.trashed_catalog_nodes_all();
    let task_rows = move || store.trashed_tasks();
    let notes = computed(move || store.trashed_quick_notes());
    let purge = signal(None::<TrashPurge>);
    let empty = move || {
        books().is_empty()
            && projects().is_empty()
            && nodes().is_empty()
            && task_rows().is_empty()
            && notes.with(Vec::is_empty)
    };
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
                    text("回收站")
                        .text_size(px(28.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(tokens::fg_primary()),
                )
                .child(when(empty, move || {
                    empty_state(
                        "回收站是空的",
                        "移入回收站的知识库、项目、目录、任务和小记会在这里出现",
                    )
                }))
                .child(when(
                    move || !books().is_empty(),
                    move || {
                        view()
                            .flex_col()
                            .gap_2()
                            .child(section_heading("知识库"))
                            .child(dynamic(move || {
                                view().flex_col().gap_2().children(
                                    books()
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
                                                    app_button(
                                                        ButtonKind::Soft,
                                                        "恢复",
                                                        move || {
                                                            store.restore_knowledge_base(id);
                                                        },
                                                    )
                                                    .test_id(&format!("kb-restore-{id}")),
                                                )
                                                .child(
                                                    app_button(
                                                        ButtonKind::Danger,
                                                        "彻底删除",
                                                        move || {
                                                            purge.set(Some(TrashPurge::Book(id)))
                                                        },
                                                    )
                                                    .test_id(&format!("kb-delete-{id}")),
                                                )
                                                .into_element()
                                        })
                                        .collect::<Vec<_>>(),
                                )
                            }))
                    },
                ))
                .child(when(
                    move || !projects().is_empty(),
                    move || {
                        view()
                            .flex_col()
                            .gap_2()
                            .child(section_heading("项目"))
                            .child(
                                text("彻底删除项目会解除任务归属，但不会删除任务、知识库或文档。")
                                    .text_size(px(11.0))
                                    .text_color(tokens::fg_muted()),
                            )
                            .child(dynamic(move || {
                                view().flex_col().gap_2().children(
                                    projects()
                                        .into_iter()
                                        .map(|project| {
                                            let id = project.id;
                                            let title = project.name.clone();
                                            let emoji = project.emoji.clone();
                                            view()
                                                .flex_row()
                                                .items_center()
                                                .gap_3()
                                                .px_3()
                                                .py_2()
                                                .rounded(tokens::radius_row())
                                                .border_1()
                                                .border_color(tokens::border_subtle())
                                                .child(text(emoji).text_size(px(18.0)))
                                                .child(
                                                    text(title)
                                                        .text_size(px(14.0))
                                                        .text_color(tokens::fg_primary())
                                                        .flex_1()
                                                        .number_of_lines(1),
                                                )
                                                .child(
                                                    app_button(
                                                        ButtonKind::Soft,
                                                        "恢复",
                                                        move || {
                                                            store.restore_project(id);
                                                        },
                                                    )
                                                    .test_id(&format!("project-restore-{id}")),
                                                )
                                                .child(
                                                    app_button(
                                                        ButtonKind::Danger,
                                                        "彻底删除",
                                                        move || {
                                                            purge.set(Some(TrashPurge::Project(id)))
                                                        },
                                                    )
                                                    .test_id(&format!("project-delete-{id}")),
                                                )
                                                .into_element()
                                        })
                                        .collect::<Vec<_>>(),
                                )
                            }))
                    },
                ))
                .child(when(
                    move || !task_rows().is_empty(),
                    move || {
                        view()
                            .flex_col()
                            .gap_2()
                            .child(section_heading("任务"))
                            .child(dynamic(move || {
                                view().flex_col().gap_2().children(
                                    task_rows()
                                        .into_iter()
                                        .map(|aggregate| {
                                            let id = aggregate.task.id;
                                            let title = aggregate.task.title.clone();
                                            view()
                                                .flex_row()
                                                .items_center()
                                                .gap_3()
                                                .px_3()
                                                .py_2()
                                                .rounded(tokens::radius_row())
                                                .border_1()
                                                .border_color(tokens::border_subtle())
                                                .child(
                                                    text("☑".to_owned())
                                                        .text_size(px(16.0))
                                                        .text_color(tokens::fg_muted()),
                                                )
                                                .child(
                                                    text(title)
                                                        .text_size(px(14.0))
                                                        .text_color(tokens::fg_primary())
                                                        .flex_1()
                                                        .number_of_lines(1),
                                                )
                                                .child(
                                                    app_button(
                                                        ButtonKind::Soft,
                                                        "恢复",
                                                        move || {
                                                            store.restore_task(id);
                                                        },
                                                    )
                                                    .test_id(&format!("task-restore-{id}")),
                                                )
                                                .child(
                                                    app_button(
                                                        ButtonKind::Danger,
                                                        "彻底删除",
                                                        move || {
                                                            purge.set(Some(TrashPurge::Task(id)))
                                                        },
                                                    )
                                                    .test_id(&format!("task-delete-{id}")),
                                                )
                                                .into_element()
                                        })
                                        .collect::<Vec<_>>(),
                                )
                            }))
                    },
                ))
                .child(when(
                    move || !notes.with(Vec::is_empty),
                    move || {
                        view()
                            .flex_col()
                            .gap_2()
                            .child(section_heading("小记"))
                            .child(each(
                                notes,
                                |note| note.id,
                                move |note, _| {
                                    let id = note.with(|note| note.id);
                                    view()
                                        .flex_row()
                                        .items_center()
                                        .gap_3()
                                        .px_3()
                                        .py_2()
                                        .rounded(tokens::radius_row())
                                        .border_1()
                                        .border_color(tokens::border_subtle)
                                        .child(
                                            text(move || {
                                                note.with(|note| {
                                                    note.content
                                                        .lines()
                                                        .next()
                                                        .unwrap_or_default()
                                                        .to_owned()
                                                })
                                            })
                                            .flex_1()
                                            .min_w(px(0.0))
                                            .number_of_lines(1)
                                            .text_sm()
                                            .text_color(tokens::fg_primary),
                                        )
                                        .child(
                                            app_button(ButtonKind::Soft, "恢复", move || {
                                                store.restore_quick_note(id)
                                            })
                                            .test_id(&format!("note-restore-{id}")),
                                        )
                                        .child(
                                            app_button(
                                                ButtonKind::Danger,
                                                "彻底删除",
                                                move || purge.set(Some(TrashPurge::Note(id))),
                                            )
                                            .test_id(&format!("note-delete-{id}")),
                                        )
                                },
                            ))
                    },
                ))
                .child(when(
                    move || !nodes().is_empty(),
                    move || {
                        view()
                            .flex_col()
                            .gap_2()
                            .child(section_heading("目录"))
                            .child(
                                text(
                                    "彻底删除目录会同时删除其子树与文档；绑定它的项目将解除绑定。",
                                )
                                .text_size(px(11.0))
                                .text_color(tokens::fg_muted()),
                            )
                            .child(dynamic(move || {
                                view().flex_col().gap_2().children(
                                    nodes()
                                        .into_iter()
                                        .map(|node| {
                                            let id = node.id;
                                            view()
                                                .flex_row()
                                                .items_center()
                                                .gap_3()
                                                .px_3()
                                                .py_2()
                                                .rounded(tokens::radius_row())
                                                .border_1()
                                                .border_color(tokens::border_subtle())
                                                .child(
                                                    text(if node.document_id.is_some() {
                                                        "📄"
                                                    } else {
                                                        "📁"
                                                    })
                                                    .text_size(px(18.0)),
                                                )
                                                .child(
                                                    text(node.title.clone())
                                                        .text_size(px(14.0))
                                                        .text_color(tokens::fg_primary())
                                                        .flex_1()
                                                        .number_of_lines(1),
                                                )
                                                .child(
                                                    app_button(
                                                        ButtonKind::Soft,
                                                        "恢复",
                                                        move || {
                                                            store.restore_catalog_node(id);
                                                        },
                                                    )
                                                    .test_id(&format!("node-restore-{id}")),
                                                )
                                                .child(
                                                    app_button(
                                                        ButtonKind::Danger,
                                                        "彻底删除",
                                                        move || {
                                                            purge.set(Some(TrashPurge::Node(id)))
                                                        },
                                                    )
                                                    .test_id(&format!("node-delete-{id}")),
                                                )
                                                .into_element()
                                        })
                                        .collect::<Vec<_>>(),
                                )
                            }))
                    },
                ))
                .child(dynamic(move || {
                    purge.get().map(|target| {
                        let (title, warning) = target.warning();
                        nsh_base::dialog()
                            .label(title)
                            .on_cancel(move || purge.set(None))
                            .flex_1()
                            .p_4()
                            .items_center()
                            .justify_center()
                            .bg(tokens::overlay)
                            .child(
                                view()
                                    .w_full()
                                    .max_w(px(420.0))
                                    .gap_3()
                                    .p_5()
                                    .rounded(tokens::radius_card())
                                    .bg(tokens::panel_raised)
                                    .child(section_heading(title))
                                    .child(text(warning).text_sm().text_color(tokens::fg_muted))
                                    .child(
                                        view()
                                            .flex_row()
                                            .justify_end()
                                            .gap_2()
                                            .child(app_button(
                                                ButtonKind::Ghost,
                                                "取消",
                                                move || purge.set(None),
                                            ))
                                            .child(app_button(
                                                ButtonKind::Danger,
                                                "确认删除",
                                                move || {
                                                    if target.confirm(store) {
                                                        purge.set(None);
                                                    }
                                                },
                                            )),
                                    ),
                            )
                    })
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

/// The subtree under `root` with depths re-based to the root's
/// children: a Project's Linked Documents sub-view and the scoped book
/// home list from the root's children downward, like the prototype's
/// `tree(book, root)` (docs/kanon.html).
fn scope_to_subtree(entries: &[CatalogEntry], root: i64) -> Vec<CatalogEntry> {
    let Some(pos) = entries.iter().position(|entry| entry.node.id == root) else {
        return Vec::new();
    };
    let base = entries[pos].depth;
    let mut scoped = Vec::new();
    for entry in &entries[pos + 1..] {
        if entry.depth <= base {
            break;
        }
        let mut entry = entry.clone();
        entry.depth -= base + 1;
        scoped.push(entry);
    }
    scoped
}
