//! Knowledge-controls regressions that `tests/app.rs` does not cover:
//! the shelf book card's secondary menu, the catalog sidebar's
//! 最近删除 section, the book switcher, the book home's dot-leader
//! catalog, the project editor's conditional submit, the reading
//! header's new-tab action, the kanban card's metadata line and its
//! cross-column drag. Helpers are local to this file, matching the
//! per-file convention of `tests/app.rs` and `tests/acceptance.rs`.

use std::path::PathBuf;
use std::time::Duration;

use nsh_app_kanon::AppProps;
use nsh_engine::{EngineConfig, Point, Points, Root, Size};
use nsh_testing::Headless;

fn temp_db(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "kanon-knowledge-{name}-{}-{}.sqlite3",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&path);
    path
}

fn start_sized(path: &std::path::Path, now: &str, width: f32) -> Headless {
    let path = path.to_string_lossy().into_owned();
    let now = now.to_owned();
    Headless::start(EngineConfig {
        viewport: Size::new(Points(width), Points(800.0)),
        ..EngineConfig::new(Root::new::<nsh_app_kanon::App>(move || AppProps {
            workspace_path: Some(path.clone()).into(),
            fixed_now: Some(now.clone()).into(),
        }))
    })
    .expect("start the app")
}

/// Seeds a workspace file through the domain facade, so a UI test starts
/// from a deterministic world. Returns the closure's value.
fn seed<T>(
    path: &std::path::Path,
    now: &str,
    setup: impl FnOnce(&mut nsh_app_kanon::domain::Workspace) -> T,
) -> T {
    use nsh_app_kanon::domain::{FixedClock, LocalDateTime, Workspace};
    let clock = FixedClock::at(LocalDateTime::parse(now).expect("valid fixed datetime"));
    let mut ws = Workspace::open(path, clock).expect("seed workspace");
    let value = setup(&mut ws);
    drop(ws);
    value
}

/// The Knowledge Base shelf: creates a book and lands on its home.
fn create_book(app: &mut Headless, name: &str) -> Result<(), String> {
    app.wait_for("今天")?;
    app.press("id:nav-知识库")?;
    app.wait_for("书架是空的")
        .or_else(|_| app.wait_for("全部知识库"))?;
    app.press("＋ 新建知识库")?;
    app.type_text("text:知识库名称，回车创建", name)?;
    app.press("id:kb-create")?;
    app.wait_for("这本知识库还没有简介。")?;
    Ok(())
}

/// Creates a top-level document through the catalog's top `+`.
fn create_top_document(app: &mut Headless, title: &str) -> Result<(), String> {
    app.press("id:catalog-new")?;
    app.wait_for("在顶部新建文档")?;
    app.press("在顶部新建文档")?;
    app.type_text("text:在目录顶部新建文档，回车创建", title)?;
    app.press("id:catalog-create")?;
    Ok(())
}

/// Creates a top-level grouping node through the catalog's top `+`.
fn create_top_group(app: &mut Headless, title: &str) -> Result<(), String> {
    app.press("id:catalog-new")?;
    app.wait_for("在顶部新建分组")?;
    app.press("在顶部新建分组")?;
    app.type_text("text:在目录顶部新建分组，回车创建", title)?;
    app.press("id:catalog-create")?;
    Ok(())
}

#[test]
fn shelf_menu_edits_book_info_inline() -> Result<(), String> {
    let path = temp_db("kb-shelf-menu");
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    create_book(&mut app, "货架")?;
    app.press("id:nav-知识库")?;
    app.wait_for("打开知识库 货架")?;

    // The card's ⋯ menu edits the book inline on the shelf (the book
    // home has its own 编辑信息; this is the card form).
    app.press("更多 货架")?;
    app.wait_for("移入回收站")?;
    app.press("编辑信息")?;
    app.wait_for("id:kb-edit-save")?;
    app.clear("label:名称")?;
    app.type_text("label:名称", "货架 v2")?;
    app.clear("label:简介")?;
    app.type_text("label:简介", "第二版")?;
    app.press("id:kb-edit-save")?;
    app.wait_for("打开知识库 货架 v2")?;

    // The description follows on the card.
    app.wait_for("第二版")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn catalog_trash_section_in_the_sidebar_restores_nodes() -> Result<(), String> {
    let path = temp_db("kb-side-trash");
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    create_book(&mut app, "回收角")?;
    create_top_document(&mut app, "废弃稿")?;
    app.wait_for("id:catalog-tree >> text:废弃稿")?;

    // Trash the node from its row menu.
    app.press("节点菜单 >> nth=0")?;
    app.wait_for("移到目录顶层")?;
    app.press("移入回收站")?;
    app.wait_for_gone("id:catalog-tree >> text:废弃稿", Duration::from_secs(2))?;

    // The sidebar's own 最近删除 section offers the restore.
    app.wait_for("最近删除")?;
    app.press("恢复 废弃稿")?;
    app.wait_for("id:catalog-tree >> text:废弃稿")?;
    app.wait_for_gone("最近删除", Duration::from_secs(2))?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn book_switcher_jumps_between_books() -> Result<(), String> {
    let path = temp_db("kb-switcher");
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    create_book(&mut app, "甲库")?;
    create_top_document(&mut app, "甲文")?;
    app.press("id:nav-知识库")?;
    create_book(&mut app, "乙库")?;
    app.wait_for("这本知识库还没有简介。")?;

    // The switcher lists every book; picking 甲库 replaces the shell.
    app.press("id:book-switcher")?;
    app.wait_for("＋ 新建知识库")?;
    app.press("切换到 甲库")?;
    app.wait_for("甲库")?;
    app.wait_for("id:catalog-tree >> text:甲文")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn book_home_catalog_toggles_groups_and_opens_documents() -> Result<(), String> {
    let path = temp_db("kb-home-catalog");
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    create_book(&mut app, "书目首页")?;
    create_top_document(&mut app, "文稿甲")?;
    create_top_group(&mut app, "分组一")?;
    app.press("新建子文档 >> nth=0")?;
    app.type_text("text:在「分组一」下新建文档，回车创建", "子文档")?;
    app.press("id:catalog-create")?;
    app.wait_for("id:catalog-tree >> text:子文档")?;

    // The book home's dot-leader catalog mirrors the tree.
    app.press("首页")?;
    app.wait_for("id:home-catalog")?;
    app.wait_for("id:home-catalog >> text:子文档")?;

    // Collapsing the group on the home page hides the child row…
    app.press("id:home-catalog >> label:切换展开")?;
    app.wait_for_gone("id:home-catalog >> text:子文档", Duration::from_secs(2))?;
    // …and expanding brings it back (the child stays in the sidebar).
    app.press("id:home-catalog >> label:切换展开")?;
    app.wait_for("id:home-catalog >> text:子文档")?;

    // A document row opens the reading view in this tab.
    app.press("id:home-catalog >> text:文稿甲")?;
    app.wait_for("编辑")?;
    app.wait_for("id:catalog-tree >> text:文稿甲")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn project_editor_disables_submit_until_the_form_commits() -> Result<(), String> {
    let path = temp_db("proj-editor-disabled");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        ws.create_catalog_node(book.id, None, "第一章")
            .expect("group");
        ws.create_knowledge_base("第二本手册").expect("second book");
    });
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);

    // No name: the submit stays disabled.
    app.wait_for("今天")?;
    app.press("id:project-new")?;
    app.wait_for("新建项目")?;
    app.press("创建项目")
        .expect_err("an empty name keeps the submit disabled");

    // WholeBook without a chosen book: still disabled…
    app.type_text("text:项目名称", "整本绑定")?;
    app.press("关联整本知识库")?;
    app.press("创建项目")
        .expect_err("an unchosen book keeps the submit disabled");
    // …until the book is picked.
    app.press("id:project-book-picker")?;
    app.press("id:project-bind-book-1")?;
    app.press("id:project-save")?;
    app.wait_for("id:project-view")?;
    app.wait_for("📚 手册")?;

    // Node mode without a chosen grouping node: disabled, then committable.
    app.press("id:project-new")?;
    app.wait_for("新建项目")?;
    app.type_text("text:项目名称", "分组绑定")?;
    app.press("关联目录分组")?;
    app.press("创建项目")
        .expect_err("an unchosen grouping node keeps the submit disabled");
    app.press("id:project-book-picker")?;
    app.press("id:project-bind-book-1")?;
    app.press("id:project-node-picker")?;
    app.press("id:project-bind-node-1")?;
    app.press("id:project-book-picker")?;
    app.press("id:project-bind-book-2")?;
    assert!(
        !app.screen().enabled("id:project-save")?,
        "changing the book clears the old grouping node"
    );
    app.press("id:project-book-picker")?;
    app.press("id:project-bind-book-1")?;
    app.press("id:project-node-picker")?;
    app.press("id:project-bind-node-1")?;
    app.press("id:project-save")?;
    app.wait_for("id:project-view")?;
    app.wait_for("📚 手册 / 第一章")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn reading_view_opens_the_document_in_a_new_tab() -> Result<(), String> {
    let path = temp_db("doc-new-tab");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        let doc = ws.create_document(book.id, None, "周报").expect("doc");
        ws.save_document(doc.id, "周报", "# 本周进展")
            .expect("seed body");
    });
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    app.wait_for("今天")?;
    app.press("id:nav-知识库")?;
    app.wait_for("全部知识库")?;
    app.press("打开知识库 手册")?;
    app.wait_for("id:catalog-tree")?;
    app.press("id:catalog-tree >> text:周报")?;
    app.wait_for("编辑")?;

    // The header action opens the same document in a second work tab.
    app.press("在新标签页打开")?;
    app.wait_for("本周进展")?;
    app.press("标签页 周报 >> nth=1")?;
    app.wait_for("编辑")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn reading_menu_creates_a_template_sibling_without_changing_the_original() -> Result<(), String> {
    let path = temp_db("reading-template");
    let now = "2026-11-20 09:00";
    let (book, group, original) = seed(&path, now, |ws| {
        let book = ws.create_knowledge_base("手册").unwrap();
        let group = ws.create_catalog_node(book.id, None, "会议").unwrap();
        let original = ws
            .create_document(book.id, Some(group.id), "现有文档")
            .unwrap();
        (book.id, group.id, original.id)
    });
    let mut app = start_sized(&path, now, 1440.0);
    app.wait_for("label:全局搜索")?;
    app.press("label:全局搜索")?;
    app.type_text("label:搜索工作区", "现有文档")?;
    app.press("text:现有文档")?;
    app.press("label:更多")?;
    app.press("role:menuitem[从模板新建…]")?;
    app.press("role:button[会议纪要]")?;
    app.wait_for("label:标签页 会议纪要")?;
    app.wait_for("label:文档正文")?;
    app.wait_for("下一步行动")?;
    drop(app);
    seed(&path, now, |ws| {
        let entries = ws.catalog_outline(book).unwrap();
        assert!(
            entries
                .iter()
                .any(|e| e.node.title == "会议纪要" && e.node.parent_id == Some(group))
        );
        assert!(ws.document(original).unwrap().unwrap().body.is_empty());
    });
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn reading_menu_trashes_only_after_confirmation() -> Result<(), String> {
    let path = temp_db("reading-trash");
    let now = "2026-11-20 09:00";
    seed(&path, now, |ws| {
        let book = ws.create_knowledge_base("手册").unwrap();
        ws.create_document(book.id, None, "待回收文档").unwrap();
    });
    let mut app = start_sized(&path, now, 1440.0);
    app.wait_for("label:全局搜索")?;
    app.press("label:全局搜索")?;
    app.type_text("label:搜索工作区", "待回收文档")?;
    app.press("text:待回收文档")?;
    app.press("label:更多")?;
    app.press("role:menuitem[移入回收站]")?;
    app.press("role:button[取消]")?;
    app.wait_for("label:标签页 待回收文档")?;

    app.press("label:更多")?;
    app.press("role:menuitem[移入回收站]")?;
    app.press("role:button[确认移入回收站]")?;
    app.wait_for_gone("label:标签页 待回收文档", Duration::from_secs(2))?;
    app.press("id:nav-回收站")?;
    app.wait_for("待回收文档")?;
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn blank_document_starts_in_edit_mode_and_persists_an_applied_template() -> Result<(), String> {
    let path = temp_db("blank-template");
    let now = "2026-11-20 09:00";
    {
        let mut app = start_sized(&path, now, 1440.0);
        create_book(&mut app, "手册")?;
        app.press("＋ 新建文档")?;
        app.wait_for("完成")?;
        app.press("label:套用模板 会议纪要")?;
        app.wait_for("下一步行动")?;
        app.wait_for_gone("label:套用模板 会议纪要", std::time::Duration::from_secs(2))?;
        app.press("完成")?;
        app.wait_for("编辑")?;
    }
    let mut app = start_sized(&path, now, 1440.0);
    app.wait_for("id:nav-知识库")?;
    app.press("id:nav-知识库")?;
    app.press("打开知识库 手册")?;
    app.press("id:catalog-tree >> text:未命名文档")?;
    app.wait_for("下一步行动")?;
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

/// Opens the seeded (single) Project's 看板 sub-view.
fn open_project_board(app: &mut Headless) -> Result<(), String> {
    app.wait_for("今天")?;
    app.press("id:sidebar-project")?;
    app.wait_for("id:project-view")?;
    app.press("看板")?;
    app.wait_for("id:kanban-todo")?;
    Ok(())
}

#[test]
fn kanban_cards_show_due_priority_subtasks_and_the_linked_document() -> Result<(), String> {
    let path = temp_db("proj-kanban-meta");
    seed(&path, "2026-11-20 09:00", |ws| {
        use nsh_app_kanon::domain::{LocalDate, NewTask, Priority};
        let book = ws.create_knowledge_base("手册").expect("kb");
        let project = ws
            .create_project("改版", "#547093", "🚀", None, None)
            .expect("project");
        let rich = ws
            .create_task(&NewTask {
                title: "画稿".to_owned(),
                priority: Some(Priority::High),
                project_id: Some(project.id),
                due_date: LocalDate::parse("2026-11-19"),
                ..Default::default()
            })
            .expect("task");
        // A Document embedding the task's card (ADR-0002) is the linked
        // document the card's meta line badges.
        let doc = ws.create_document(book.id, None, "验收清单").expect("doc");
        ws.update_document_body_and_references(
            doc.id,
            "验收清单",
            &format!("检查项\n@task({})", rich.task.id),
            None,
        )
        .expect("indexed linked task card");
        let first = ws.add_subtask(rich.task.id, "线框").expect("subtask");
        ws.add_subtask(rich.task.id, "配色").expect("subtask");
        ws.set_subtask_done(first.id, true).expect("done subtask");
        ws.create_task(&NewTask {
            title: "走查".to_owned(),
            project_id: Some(project.id),
            ..Default::default()
        })
        .expect("plain task");
    });
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    open_project_board(&mut app)?;

    // The metadata line (docs/ui-design.md §4.3): Due indicator,
    // priority marker, subtask progress, linked-document chip.
    app.wait_for("id:kanban-todo >> text:画稿")?;
    app.wait_for("id:kanban-todo >> text:2026-11-19")?;
    app.wait_for("id:kanban-todo >> text:高优先级")?;
    app.wait_for("id:kanban-todo >> text:1/2")?;
    app.wait_for("id:kanban-todo >> label:关联文档 验收清单")?;
    // The plain card carries no metadata line.
    app.wait_for("id:kanban-todo >> text:走查")?;

    // The chip opens the linked Document in this tab.
    app.press("id:kanban-todo >> label:关联文档 验收清单")?;
    app.wait_for("编辑")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn kanban_drag_moves_a_card_across_lanes_and_stays_put_on_a_short_drag() -> Result<(), String> {
    let path = temp_db("proj-kanban-drag");
    seed(&path, "2026-11-20 09:00", |ws| {
        let project = ws
            .create_project("改版", "#547093", "🚀", None, None)
            .expect("project");
        for title in ["画稿", "走查"] {
            ws.create_task(&nsh_app_kanon::domain::NewTask {
                title: title.to_owned(),
                project_id: Some(project.id),
                ..Default::default()
            })
            .expect("task");
        }
    });
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    open_project_board(&mut app)?;
    app.wait_for("id:kanban-todo >> text:画稿")?;

    // The lanes are a few hundred points each at this width: dragging the
    // card 400 points right releases it above the 进行中 lane.
    app.drag("text:画稿", Point::new(Points(400.0), Points(0.0)))?;
    app.wait_for("id:kanban-doing >> text:画稿")?;
    app.wait_for_gone("id:kanban-todo >> text:画稿", Duration::from_secs(2))?;

    // A short drag that releases inside the same lane keeps the status
    // (the card springs back where it was).
    app.drag(
        "id:kanban-doing >> text:画稿",
        Point::new(Points(20.0), Points(0.0)),
    )?;
    app.wait_for("id:kanban-doing >> text:画稿")?;
    app.wait_for_gone("id:kanban-todo >> text:画稿", Duration::from_secs(2))?;

    // The 移动 menu stays as the keyboard path (docs/ui-design.md §4.3).
    app.press("移动 走查")?;
    app.wait_for("移到进行中")?;
    app.press("移到进行中")?;
    app.wait_for("id:kanban-doing >> text:走查")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}
