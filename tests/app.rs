//! The app-level seam: the real Rust root driven headlessly, verifying
//! the end-to-end Today flow — create, schedule, complete, edit dates —
//! and that every bit of it survives an app restart against the same
//! SQLite workspace.

use std::path::PathBuf;
use std::time::Duration;

use nsh_app_kanon::AppProps;
use nsh_app_kanon::domain::{LocalDate, LocalTime};
use nsh_core::Checked;
use nsh_engine::{EngineConfig, Points, Root, Size};
use nsh_testing::Headless;

fn date(text: &str) -> LocalDate {
    LocalDate::parse(text).unwrap()
}

fn time(text: &str) -> LocalTime {
    LocalTime::parse(text).unwrap()
}

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

/// Seeds a workspace file through the domain facade, so a UI test starts
/// from a deterministic world without driving dozens of interactions.
fn seed(path: &std::path::Path, now: &str, setup: impl FnOnce(&mut nsh_app_kanon::domain::Workspace)) {
    use nsh_app_kanon::domain::{FixedClock, LocalDateTime, Workspace};
    let clock = FixedClock::at(LocalDateTime::parse(now).expect("valid fixed datetime"));
    let mut ws = Workspace::open(path, clock).expect("seed workspace");
    setup(&mut ws);
    drop(ws);
}

fn count_text(app: &Headless, needle: &str) -> usize {
    let mut count = 0;
    for text in app.screen().texts() {
        if text == needle {
            count += 1;
        }
    }
    count
}

#[test]
fn inbox_badge_counts_strict_inbox_tasks_and_triage_updates_it() -> Result<(), String> {
    use nsh_app_kanon::domain::NewTask;
    let path = temp_db("inbox-badge");
    seed(&path, "2026-11-20 09:00", |ws| {
        ws.create_task(&NewTask { title: "收集甲".to_owned(), ..Default::default() }).unwrap();
        ws.create_task(&NewTask { title: "收集乙".to_owned(), ..Default::default() }).unwrap();
        ws.create_task(&NewTask { title: "今日事".to_owned(), scheduled_date: Some(date("2026-11-20")), ..Default::default() }).unwrap();
        ws.create_task(&NewTask { title: "仅截止".to_owned(), due_date: Some(date("2026-11-21")), ..Default::default() }).unwrap();
    });
    let mut app = start(&path, "2026-11-20 09:00");
    app.wait_for("今天")?;
    // Only the two strict Inbox tasks count: scheduled and due-only rows do not.
    app.wait_for("label:收件箱 2 项任务")?;

    // Triage one through the Task Detail: a Scheduled Date removes it immediately.
    app.press("打开任务 收集甲")?;
    app.wait_for("任务详情")?;
    app.type_text("label:排期日", "2026-11-20")?;
    app.wait_for("label:收件箱 1 项任务")?;

    // Triaging the second via a Due Date empties the badge.
    app.press("关闭任务详情")?;
    app.press("打开任务 收集乙")?;
    app.wait_for("任务详情")?;
    app.type_text("label:截止日", "2026-11-22")?;
    app.wait_for("label:收件箱 0 项任务")?;

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
fn tasks_view_groups_without_duplicates_and_filters_and_sorts() -> Result<(), String> {
    use nsh_app_kanon::domain::{NewTask, Priority, TaskPatch, TaskStatus};
    let path = temp_db("tasks-view");
    seed(&path, "2026-11-20 09:00", |ws| {
        // Overdue wins over Today on the same row.
        ws.create_task(&NewTask {
            title: "逾今日".to_owned(),
            scheduled_date: Some(date("2026-11-20")),
            due_date: Some(date("2026-11-19")),
            ..Default::default()
        })
        .unwrap();
        ws.create_task(&NewTask {
            title: "进行事".to_owned(),
            status: Some(TaskStatus::Doing),
            scheduled_date: Some(date("2026-11-20")),
            ..Default::default()
        })
        .unwrap();
        ws.create_task(&NewTask { title: "将来".to_owned(), scheduled_date: Some(date("2026-11-22")), ..Default::default() }).unwrap();
        ws.create_task(&NewTask { title: "收进".to_owned(), ..Default::default() }).unwrap();
        // Creation order is the reverse of the priority order.
        ws.create_task(&NewTask { title: "低优".to_owned(), priority: Some(Priority::Low), ..Default::default() }).unwrap();
        ws.create_task(&NewTask { title: "高优".to_owned(), priority: Some(Priority::High), ..Default::default() }).unwrap();
        let project = ws.create_project("网站建设", "#547093", "🌐", None, None).unwrap();
        ws.create_task(&NewTask { title: "项内".to_owned(), project_id: Some(project.id), ..Default::default() }).unwrap();
        ws.create_task(&NewTask { title: "仅截止".to_owned(), due_date: Some(date("2026-11-21")), ..Default::default() }).unwrap();
        let done = ws.create_task(&NewTask { title: "做完".to_owned(), ..Default::default() }).unwrap().task.id;
        ws.update_task(done, &TaskPatch { status: Some(TaskStatus::Done), ..Default::default() }).unwrap();
        let cancelled = ws.create_task(&NewTask { title: "取消".to_owned(), ..Default::default() }).unwrap().task.id;
        ws.update_task(cancelled, &TaskPatch { status: Some(TaskStatus::Cancelled), ..Default::default() }).unwrap();
    });
    let mut app = start(&path, "2026-11-20 09:00");
    app.wait_for("今天")?;
    // Count rows on the Tasks view itself, without the overview panel.
    app.press("切换上下文面板")?;
    app.press("id:nav-待办")?;
    for group in ["逾期", "今天", "即将到来", "收件箱", "未排期", "已完成", "已取消"] {
        app.wait_for(&format!("id:tasks-group-{group}"))?;
    }

    // Mutual exclusion: every Task appears in exactly one group.
    let titles = ["逾今日", "进行事", "将来", "收进", "低优", "高优", "项内", "仅截止", "做完", "取消"];
    for title in titles {
        assert_eq!(count_text(&app, title), 1, "{title} must appear exactly once across all groups");
    }

    // Status filter narrows every group to 进行中 rows.
    app.press("label:状态筛选 进行中")?;
    assert!(app.shows("进行事"));
    assert!(!app.shows("逾今日"), "todo rows hide under the 进行中 filter");
    assert!(!app.shows("做完"), "done rows hide under the 进行中 filter");
    app.press("label:状态筛选 全部")?;
    assert!(app.shows("逾今日"));

    // Project filter narrows to the one project-bound row.
    app.press("label:项目筛选 网站建设")?;
    assert!(app.shows("项内"));
    assert!(!app.shows("收进"), "project-less rows hide under a Project filter");
    app.press("label:项目筛选 全部")?;

    // Sorting by priority re-orders the inbox rows (creation was 低优, 高优).
    let texts = visible_texts(&app);
    let low_at = texts.iter().position(|text| text == "低优").unwrap();
    let high_at = texts.iter().position(|text| text == "高优").unwrap();
    assert!(low_at < high_at, "manual order keeps creation order");
    app.press("label:排序方式 优先级")?;
    let texts = visible_texts(&app);
    let low_at = texts.iter().position(|text| text == "低优").unwrap();
    let high_at = texts.iter().position(|text| text == "高优").unwrap();
    assert!(high_at < low_at, "priority sort lists 高优 before 低优");

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
fn carryover_preserves_due_after_a_clock_transition() -> Result<(), String> {
    use nsh_app_kanon::domain::NewTask;
    let path = temp_db("carryover-due");
    seed(&path, "2026-11-20 09:00", |ws| {
        ws.create_task(&NewTask {
            title: "结转甲".to_owned(),
            scheduled_date: Some(date("2026-11-19")),
            scheduled_time: Some(time("07:45")),
            due_date: Some(date("2026-11-24")),
            due_time: Some(time("18:30")),
            ..Default::default()
        })
        .unwrap();
        ws.create_task(&NewTask { title: "结转乙".to_owned(), scheduled_date: Some(date("2026-11-19")), ..Default::default() }).unwrap();
    });

    let mut app = start(&path, "2026-11-20 09:00");
    app.wait_for("今天")?;
    app.wait_for("id:carryover")?;
    app.press("id:carryover")?;
    app.wait_for("结转甲")?;
    app.wait_for("截止 2026-11-24 18:30")?;
    assert!(!app.shows("07:45"), "the stale scheduled time-of-day is cleared");
    drop(app);

    // A new day starts: the carried tasks are past-scheduled again, and
    // Carryover still leaves the Due Date/Time untouched.
    let mut app = start(&path, "2026-11-21 08:00");
    app.wait_for("今天")?;
    app.wait_for("id:carryover")?;
    app.press("id:carryover")?;
    app.wait_for("结转甲")?;
    app.wait_for("结转乙")?;
    app.wait_for("截止 2026-11-24 18:30")?;
    assert!(!app.shows("07:45"), "scheduled time-of-day stays cleared across the transition");

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

#[test]
fn upcoming_view_switches_between_list_and_board_with_due_indicators() -> Result<(), String> {
    use nsh_app_kanon::domain::NewTask;
    let path = temp_db("upcoming");
    seed(&path, "2026-11-20 09:00", |ws| {
        ws.create_task(&NewTask {
            title: "明日事".to_owned(),
            scheduled_date: Some(date("2026-11-21")),
            due_date: Some(date("2026-11-22")),
            ..Default::default()
        })
        .unwrap();
        ws.create_task(&NewTask { title: "后日事".to_owned(), scheduled_date: Some(date("2026-11-22")), ..Default::default() }).unwrap();
        ws.create_task(&NewTask { title: "周五事".to_owned(), scheduled_date: Some(date("2026-11-27")), ..Default::default() }).unwrap();
    });
    let heading = LocalDate::parse("2026-11-21").unwrap().long_label();

    let mut app = start(&path, "2026-11-20 09:00");
    app.wait_for("今天")?;
    app.press("id:nav-即将到来")?;

    // 列表 mode: day sections with the Due indicator inline.
    app.wait_for("id:upcoming-list")?;
    app.wait_for(&heading)?;
    assert!(app.shows("明日事"));
    assert!(app.shows("后日事"));
    assert!(app.shows("周五事"));
    app.wait_for("截止 2026-11-22")?;

    // 看板 mode: one column per day, same rows and Due indicators.
    app.press("label:视图 看板")?;
    app.wait_for("id:upcoming-board")?;
    app.wait_for(&heading)?;
    assert!(app.shows("明日事"));
    assert!(app.shows("截止 2026-11-22"));

    // Back to 列表.
    app.press("label:视图 列表")?;
    app.wait_for("id:upcoming-list")?;

    let _ = std::fs::remove_file(path);
    Ok(())
}
