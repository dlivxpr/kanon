//! The app-level seam: the real Rust root driven headlessly, verifying
//! the end-to-end Today flow — create, schedule, complete, edit dates —
//! and that every bit of it survives an app restart against the same
//! SQLite workspace.

use std::path::PathBuf;
use std::time::Duration;

use nsh_app_kanon::AppProps;
use nsh_core::Checked;
use nsh_engine::{EngineConfig, Points, Root, Size};
use nsh_testing::Headless;

fn temp_db(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "kanon-app-test-{name}-{}-{}.sqlite3",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&path);
    path
}

fn start(path: &std::path::Path, now: &str) -> Headless {
    let path = path.to_string_lossy().into_owned();
    let now = now.to_owned();
    Headless::start(EngineConfig {
        viewport: Size::new(Points(1280.0), Points(800.0)),
        ..EngineConfig::new(Root::new::<nsh_app_kanon::App>(move || AppProps {
            workspace_path: Some(path.clone()).into(),
            fixed_now: Some(now.clone()).into(),
        }))
    })
    .expect("start the app")
}

/// Whether the circle of `label` currently reads as checked.
fn circle_checked(app: &Headless, label: &str) -> bool {
    matches!(
        app.screen().accessibility_state(label).expect("the circle's state").checked,
        Some(Checked::True)
    )
}

fn visible_texts(app: &Headless) -> Vec<String> {
    app.screen().texts().iter().map(|text| (*text).to_owned()).collect()
}

#[test]
fn creating_scheduling_completing_and_editing_a_task_survives_restart() -> Result<(), String> {
    let path = temp_db("restart");
    let mut app = start(&path, "2026-11-20 09:00");

    // Kanon opens on Today.
    app.wait_for("今天")?;
    app.wait_for("id:date-strip")?;

    // Quick-add a task for today.
    app.type_text("text:添加任务，回车保存到所选日期", "写周报")?;
    app.press("id:quick-add")?;
    app.wait_for("写周报")?;

    // Its detail opens in the right context panel; give it a due date in
    // the past so the overdue badge lights up.
    app.wait_for("任务详情")?;
    app.type_text("text:截止日 (YYYY-MM-DD)", "2026-11-19")?;
    app.wait_for("逾期")?;

    // The quick circle toggles strictly done ↔ todo.
    app.press("完成 写周报 >> nth=0")?;
    assert!(circle_checked(&app, "完成 写周报 >> nth=0"), "the circle reads as done");
    // A done task is no longer overdue (the predicate counts active only).
    app.wait_for_gone("逾期", Duration::from_secs(2))?;

    // Close the panel and collapse the sidebar; both persist.
    app.press("切换上下文面板")?;
    app.wait_for_gone("任务速览", Duration::from_secs(2))?;
    app.press("切换侧栏")?;
    assert!(
        !visible_texts(&app).iter().any(|text| text == "知识库"),
        "the expanded sidebar's labels disappear when it collapses"
    );
    drop(app);

    // Restart against the same workspace.
    let mut app = start(&path, "2026-11-20 09:30");
    app.wait_for("今天")?;
    app.wait_for("写周报")?;
    app.wait_for("截止 2026-11-19")?;

    // The preference state persisted: no expanded-sidebar labels, no panel.
    assert!(
        !visible_texts(&app).iter().any(|text| text == "知识库"),
        "sidebar stays collapsed across restart"
    );
    assert!(!app.shows("任务速览"), "panel stays closed across restart");
    assert!(circle_checked(&app, "完成 写周报 >> nth=0"), "completion survived the restart");

    // The task's completion persisted too.
    app.press("打开任务 写周报")?;
    app.wait_for("已完成")?;

    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn today_view_toggles_the_circle_strictly_between_done_and_todo() -> Result<(), String> {
    let path = temp_db("circle");
    let mut app = start(&path, "2026-11-20 09:00");
    app.wait_for("今天")?;
    app.type_text("text:添加任务，回车保存到所选日期", "买牛奶")?;
    app.press("id:quick-add")?;
    app.wait_for("买牛奶")?;

    // Creating a task opened its detail; close the panel for one circle.
    app.press("关闭任务详情")?;
    app.press("切换上下文面板")?;

    app.press("完成 买牛奶")?;
    assert!(circle_checked(&app, "完成 买牛奶"), "todo → done");
    app.press("打开任务 买牛奶")?;
    app.wait_for("已完成")?;
    app.press("关闭任务详情")?;
    app.press("切换上下文面板")?;

    app.press("完成 买牛奶")?;
    assert!(!circle_checked(&app, "完成 买牛奶"), "done → todo again");
    app.press("打开任务 买牛奶")?;
    app.wait_for("任务详情")?;

    let _ = std::fs::remove_file(path);
    Ok(())
}

// ---------------------------------------------------------------------
// Knowledge Base flows (issue #5)
// ---------------------------------------------------------------------

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

/// The catalog tree's scoped match helpers.
fn tree_shows(app: &Headless, text: &str) -> bool {
    app.screen().matches(&format!("id:catalog-tree >> text:{text}")).unwrap_or(false)
}

/// Creates a book through the shelf and lands on its book home.
fn create_book(app: &mut Headless, name: &str) -> Result<(), String> {
    app.wait_for("今天")?;
    app.press("id:nav-知识库")?;
    app.wait_for("书架是空的").or_else(|_| app.wait_for("全部知识库"))?;
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

#[test]
fn kb_shelf_creates_opens_and_edits_a_book() -> Result<(), String> {
    let path = temp_db("kb-crud");
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);

    create_book(&mut app, "产品手册")?;
    // The creation landed on the book home with live (empty) stats.
    app.wait_for("产品手册")?;
    app.wait_for("0 篇文档 · 0 字")?;

    // Edit the name and description inline.
    app.press("编辑信息")?;
    app.clear("label:名称")?;
    app.type_text("label:名称", "产品手册 v2")?;
    app.clear("label:简介")?;
    app.type_text("label:简介", "对外发布的版本")?;
    app.press("id:kb-edit-save")?;
    app.wait_for("产品手册 v2")?;
    app.wait_for("对外发布的版本")?;

    // Favorite it; the shelf lists it under 常用.
    app.press("收藏本书")?;
    app.press("id:nav-知识库")?;
    app.wait_for("常用")?;
    app.wait_for("产品手册 v2")?;
    app.wait_for("0 篇文档 · 0 字")?;

    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn catalog_inserts_at_the_top_nests_children_and_filters() -> Result<(), String> {
    let path = temp_db("kb-catalog");
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    create_book(&mut app, "目录规则")?;

    // Top `+` inserts at the TOP of the level: newest first.
    create_top_document(&mut app, "甲")?;
    app.wait_for("id:catalog-tree >> text:甲")?;
    create_top_document(&mut app, "乙")?;
    create_top_document(&mut app, "丙")?;
    let texts = visible_texts(&app);
    let at = |needle: &str| texts.iter().position(|t| t == needle).expect("present");
    assert!(
        at("丙") < at("乙") && at("乙") < at("甲"),
        "top insertion keeps newest first: {texts:?}"
    );

    // A top-level group; child creation nests under it, also at the top.
    app.press("id:catalog-new")?;
    app.wait_for("在顶部新建分组")?;
    app.press("在顶部新建分组")?;
    app.type_text("text:在目录顶部新建分组，回车创建", "分组一")?;
    app.press("id:catalog-create")?;
    app.wait_for("id:catalog-tree >> text:分组一")?;

    app.press("新建子文档 >> nth=0")?; // 分组一's row action
    app.type_text("text:在「分组一」下新建文档，回车创建", "子一")?;
    app.press("id:catalog-create")?;
    app.press("新建子文档 >> nth=0")?;
    app.type_text("text:在「分组一」下新建文档，回车创建", "子二")?;
    app.press("id:catalog-create")?;
    let texts = visible_texts(&app);
    let at = |needle: &str| texts.iter().position(|t| t == needle).expect("present");
    assert!(at("分组一") < at("子二") && at("子二") < at("子一"), "children nest at the top: {texts:?}");

    // Deeper nodes start collapsed by design, so the header offers
    // 全部展开 first; collapse-all then hides the children, expand-all
    // brings them back.
    app.press("全部展开")?;
    assert!(tree_shows(&app, "子一"), "children visible under the expanded group");
    app.press("全部收起")?;
    assert!(!tree_shows(&app, "子一"), "collapsed subtree hidden");
    app.press("全部展开")?;
    assert!(tree_shows(&app, "子一"), "expanded subtree visible");

    // In-book filtering keeps only the matching rows.
    app.type_text("text:筛选目录", "丙")?;
    assert!(tree_shows(&app, "丙"));
    assert!(!tree_shows(&app, "甲"), "filtered out");

    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn catalog_locates_the_current_document() -> Result<(), String> {
    let path = temp_db("kb-locate");
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    create_book(&mut app, "定位规则")?;
    create_top_document(&mut app, "甲")?;
    app.press("id:catalog-new")?;
    app.wait_for("在顶部新建分组")?;
    app.press("在顶部新建分组")?;
    app.type_text("text:在目录顶部新建分组，回车创建", "分组一")?;
    app.press("id:catalog-create")?;
    app.press("新建子文档 >> nth=0")?;
    app.type_text("text:在「分组一」下新建文档，回车创建", "子文档")?;
    app.press("id:catalog-create")?;

    // Open the child document; its row highlights in the catalog.
    app.press("id:catalog-tree >> text:子文档")?;
    app.wait_for("子文档")?;
    app.wait_for("正文是空的")?;

    // Collapse its group: the row disappears…
    app.press("收起 分组一")?;
    assert!(!tree_shows(&app, "子文档"), "row hidden under the collapsed group");

    // …and locate-current expands the chain back to it.
    app.press("定位当前文档")?;
    assert!(tree_shows(&app, "子文档"), "locate expanded the chain to the current document");

    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn catalog_menu_reorders_and_reparents_without_cycles() -> Result<(), String> {
    let path = temp_db("kb-move-ui");
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    create_book(&mut app, "移动规则")?;
    create_top_document(&mut app, "甲")?;
    create_top_document(&mut app, "乙")?;

    // 下移 乙 below 甲.
    app.press("节点菜单 >> nth=0")?;
    app.wait_for("移到目录顶层")?;
    app.press("下移")?;
    let texts = visible_texts(&app);
    let at = |needle: &str| texts.iter().position(|t| t == needle).expect("present");
    assert!(at("甲") < at("乙"), "下移 moved 乙 below 甲: {texts:?}");

    // 移到目录顶层 puts the first node at the root top.
    app.press("节点菜单 >> nth=1")?;
    app.wait_for("移到目录顶层")?;
    app.press("移到目录顶层")?;
    let texts = visible_texts(&app);
    let at = |needle: &str| texts.iter().position(|t| t == needle).expect("present");
    assert!(at("乙") < at("甲"), "移到目录顶层 moved 乙 to the top: {texts:?}");

    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn trash_and_restore_a_knowledge_base() -> Result<(), String> {
    let path = temp_db("kb-trash-ui");
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    create_book(&mut app, "临时库")?;

    // Trash it from the shelf.
    app.press("id:nav-知识库")?;
    app.wait_for("打开知识库 临时库")?;
    app.press("更多 临时库")?;
    app.wait_for("移入回收站")?;
    app.press("移入回收站")?;
    app.wait_for_gone("打开知识库 临时库", Duration::from_secs(2))?;

    // The tab that showed the book fell back to the shelf.
    app.wait_for("书架是空的")?;

    // Restore it from the Trash screen.
    app.press("id:nav-回收站")?;
    app.wait_for("临时库")?;
    app.press("恢复 >> nth=0")?;
    app.wait_for_gone("恢复", Duration::from_secs(2))?;
    app.press("id:nav-知识库")?;
    app.wait_for("打开知识库 临时库")?;

    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn knowledge_base_flow_survives_restart() -> Result<(), String> {
    let path = temp_db("kb-restart");
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    create_book(&mut app, "持久化")?;
    create_top_document(&mut app, "甲")?;
    create_top_document(&mut app, "乙")?;
    app.press("id:catalog-new")?;
    app.wait_for("在顶部新建分组")?;
    app.press("在顶部新建分组")?;
    app.type_text("text:在目录顶部新建分组，回车创建", "分组一")?;
    app.press("id:catalog-create")?;
    app.press("新建子文档 >> nth=0")?;
    app.type_text("text:在「分组一」下新建文档，回车创建", "子文档")?;
    app.press("id:catalog-create")?;
    app.press("收藏本书")?;
    // The child node starts collapsed by design: expand everything
    // first, then collapse it all — the state that must persist.
    app.press("全部展开")?;
    app.press("全部收起")?;
    drop(app);

    // Restart against the same workspace: shelf, tree, favorites and
    // expansion state all come back from SQLite.
    let mut app = start_sized(&path, "2026-11-20 09:30", 1440.0);
    app.wait_for("今天")?;
    app.press("id:nav-知识库")?;
    app.wait_for("常用")?;
    app.wait_for("打开知识库 持久化")?;
    app.wait_for("3 篇文档 · 0 字")?;

    // The favorite shows in both 常用 and 全部知识库; open the first.
    app.press("打开知识库 持久化 >> nth=0")?;
    app.wait_for("持久化")?;
    // The collapse-all state persisted: the group hides its child.
    assert!(tree_shows(&app, "分组一"), "root nodes stay visible");
    assert!(!tree_shows(&app, "子文档"), "collapsed subtree stayed collapsed across restart");
    // …until expanded again.
    app.press("全部展开")?;
    assert!(tree_shows(&app, "子文档"), "expand-all after restart");

    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn responsive_rules_collapse_sidebar_and_overlay_catalog() -> Result<(), String> {
    let path = temp_db("kb-responsive");
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    create_book(&mut app, "响应式")?;

    // Wide: outer sidebar expanded, catalog inline. (「即将到来」 only
    // exists as a sidebar label, unlike 「今天」 which the open tab also
    // shows in the tab strip.)
    assert!(app.shows("即将到来"), "sidebar labels at 1440 px");
    assert!(app.screen().matches("id:catalog-inline").unwrap_or(false));

    // 960–1439: the outer sidebar auto-collapses to the icon rail; the
    // catalog stays inline.
    app.set_viewport(Size::new(Points(1280.0), Points(800.0)))?;
    app.wait_for_gone("text:即将到来", Duration::from_secs(2))?;
    assert!(app.screen().matches("id:catalog-inline").unwrap_or(false), "catalog stays inline at 1280 px");

    // < 960: the catalog turns into an overlay drawer; the main pane
    // keeps its width (the book home is still fully visible).
    app.set_viewport(Size::new(Points(900.0), Points(800.0)))?;
    app.wait_for("响应式")?;
    assert!(!app.screen().matches("id:catalog-inline").unwrap_or(false), "no inline catalog at 900 px");
    assert!(app.shows("这本知识库还没有简介。"), "main pane keeps its content");
    app.press("☰ 目录")?;
    app.wait_for("id:catalog-overlay")?;
    app.press("关闭目录")?;
    app.wait_for_gone("id:catalog-overlay", Duration::from_secs(2))?;

    let _ = std::fs::remove_file(path);
    Ok(())
}


