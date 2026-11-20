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
        app.screen()
            .accessibility_state(label)
            .expect("the circle's state")
            .checked,
        Some(Checked::True)
    )
}

fn visible_texts(app: &Headless) -> Vec<String> {
    app.screen()
        .texts()
        .iter()
        .map(|text| (*text).to_owned())
        .collect()
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
    assert!(
        circle_checked(&app, "完成 写周报 >> nth=0"),
        "the circle reads as done"
    );
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
    assert!(
        circle_checked(&app, "完成 写周报 >> nth=0"),
        "completion survived the restart"
    );

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
    app.screen()
        .matches(&format!("id:catalog-tree >> text:{text}"))
        .unwrap_or(false)
}

/// Creates a book through the shelf and lands on its book home.
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
/// Returns the closure's value for the caller's assertions.
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
        ws.create_task(&NewTask {
            title: "收集甲".to_owned(),
            ..Default::default()
        })
        .unwrap();
        ws.create_task(&NewTask {
            title: "收集乙".to_owned(),
            ..Default::default()
        })
        .unwrap();
        ws.create_task(&NewTask {
            title: "今日事".to_owned(),
            scheduled_date: Some(date("2026-11-20")),
            ..Default::default()
        })
        .unwrap();
        ws.create_task(&NewTask {
            title: "仅截止".to_owned(),
            due_date: Some(date("2026-11-21")),
            ..Default::default()
        })
        .unwrap();
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
    assert!(
        at("分组一") < at("子二") && at("子二") < at("子一"),
        "children nest at the top: {texts:?}"
    );

    // Deeper nodes start collapsed by design, so the header offers
    // 全部展开 first; collapse-all then hides the children, expand-all
    // brings them back.
    app.press("全部展开")?;
    assert!(
        tree_shows(&app, "子一"),
        "children visible under the expanded group"
    );
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
        ws.create_task(&NewTask {
            title: "将来".to_owned(),
            scheduled_date: Some(date("2026-11-22")),
            ..Default::default()
        })
        .unwrap();
        ws.create_task(&NewTask {
            title: "收进".to_owned(),
            ..Default::default()
        })
        .unwrap();
        // Creation order is the reverse of the priority order.
        ws.create_task(&NewTask {
            title: "低优".to_owned(),
            priority: Some(Priority::Low),
            ..Default::default()
        })
        .unwrap();
        ws.create_task(&NewTask {
            title: "高优".to_owned(),
            priority: Some(Priority::High),
            ..Default::default()
        })
        .unwrap();
        let project = ws
            .create_project("网站建设", "#547093", "🌐", None, None)
            .unwrap();
        ws.create_task(&NewTask {
            title: "项内".to_owned(),
            project_id: Some(project.id),
            ..Default::default()
        })
        .unwrap();
        ws.create_task(&NewTask {
            title: "仅截止".to_owned(),
            due_date: Some(date("2026-11-21")),
            ..Default::default()
        })
        .unwrap();
        let done = ws
            .create_task(&NewTask {
                title: "做完".to_owned(),
                ..Default::default()
            })
            .unwrap()
            .task
            .id;
        ws.update_task(
            done,
            &TaskPatch {
                status: Some(TaskStatus::Done),
                ..Default::default()
            },
        )
        .unwrap();
        let cancelled = ws
            .create_task(&NewTask {
                title: "取消".to_owned(),
                ..Default::default()
            })
            .unwrap()
            .task
            .id;
        ws.update_task(
            cancelled,
            &TaskPatch {
                status: Some(TaskStatus::Cancelled),
                ..Default::default()
            },
        )
        .unwrap();
    });
    let mut app = start(&path, "2026-11-20 09:00");
    app.wait_for("今天")?;
    // Count rows on the Tasks view itself, without the overview panel.
    app.press("切换上下文面板")?;
    app.press("id:nav-待办")?;
    for group in [
        "逾期",
        "今天",
        "即将到来",
        "收件箱",
        "未排期",
        "已完成",
        "已取消",
    ] {
        app.wait_for(&format!("id:tasks-group-{group}"))?;
    }

    // Mutual exclusion: every Task appears in exactly one group.
    let titles = [
        "逾今日",
        "进行事",
        "将来",
        "收进",
        "低优",
        "高优",
        "项内",
        "仅截止",
        "做完",
        "取消",
    ];
    for title in titles {
        assert_eq!(
            count_text(&app, title),
            1,
            "{title} must appear exactly once across all groups"
        );
    }

    // Status filter narrows every group to 进行中 rows.
    app.press("label:状态筛选 进行中")?;
    assert!(app.shows("进行事"));
    assert!(
        !app.shows("逾今日"),
        "todo rows hide under the 进行中 filter"
    );
    assert!(!app.shows("做完"), "done rows hide under the 进行中 filter");
    app.press("label:状态筛选 全部")?;
    assert!(app.shows("逾今日"));

    // Project filter narrows to the one project-bound row.
    app.press("label:项目筛选 网站建设")?;
    assert!(app.shows("项内"));
    assert!(
        !app.shows("收进"),
        "project-less rows hide under a Project filter"
    );
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
    app.wait_for("\u{200b}正文为空")?;

    // Collapse its group: the row disappears…
    app.press("收起 分组一")?;
    assert!(
        !tree_shows(&app, "子文档"),
        "row hidden under the collapsed group"
    );

    // …and locate-current expands the chain back to it.
    app.press("定位当前文档")?;
    assert!(
        tree_shows(&app, "子文档"),
        "locate expanded the chain to the current document"
    );

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
    assert!(
        at("乙") < at("甲"),
        "移到目录顶层 moved 乙 to the top: {texts:?}"
    );

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
    assert!(
        !tree_shows(&app, "子文档"),
        "collapsed subtree stayed collapsed across restart"
    );
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
        ws.create_task(&NewTask {
            title: "结转乙".to_owned(),
            scheduled_date: Some(date("2026-11-19")),
            ..Default::default()
        })
        .unwrap();
    });

    let mut app = start(&path, "2026-11-20 09:00");
    app.wait_for("今天")?;
    app.wait_for("id:carryover")?;
    app.press("id:carryover")?;
    app.wait_for("结转甲")?;
    app.wait_for("截止 2026-11-24 18:30")?;
    assert!(
        !app.shows("07:45"),
        "the stale scheduled time-of-day is cleared"
    );
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
    assert!(
        !app.shows("07:45"),
        "scheduled time-of-day stays cleared across the transition"
    );

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
    assert!(
        app.screen().matches("id:catalog-inline").unwrap_or(false),
        "catalog stays inline at 1280 px"
    );

    // < 960: the catalog turns into an overlay drawer; the main pane
    // keeps its width (the book home is still fully visible).
    app.set_viewport(Size::new(Points(900.0), Points(800.0)))?;
    app.wait_for("响应式")?;
    assert!(
        !app.screen().matches("id:catalog-inline").unwrap_or(false),
        "no inline catalog at 900 px"
    );
    assert!(
        app.shows("这本知识库还没有简介。"),
        "main pane keeps its content"
    );
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
        ws.create_task(&NewTask {
            title: "后日事".to_owned(),
            scheduled_date: Some(date("2026-11-22")),
            ..Default::default()
        })
        .unwrap();
        ws.create_task(&NewTask {
            title: "周五事".to_owned(),
            scheduled_date: Some(date("2026-11-27")),
            ..Default::default()
        })
        .unwrap();
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

// ---------------------------------------------------------------------
// Document reading/editing (issue #6): read/edit switching, checklist
// isolation, auto-save across restart, revision restore, and the
// single-editor lock across tabs.
// ---------------------------------------------------------------------

/// Seeds one book with one document and opens its reading view.
fn open_seeded_document(app: &mut Headless, title: &str) -> Result<(), String> {
    app.wait_for("今天")?;
    app.press("id:nav-知识库")?;
    app.wait_for("全部知识库")?;
    app.press("打开知识库 手册")?;
    app.wait_for("id:catalog-tree")?;
    app.press(&format!("id:catalog-tree >> text:{title}"))?;
    app.wait_for("编辑")?;
    Ok(())
}

#[test]
fn document_reading_editing_and_auto_save_survive_restart() -> Result<(), String> {
    let path = temp_db("doc-autosave");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        let doc = ws.create_document(book.id, None, "周报").expect("doc");
        ws.save_document(doc.id, "周报", "# 本周\n\n- [ ] 收尾")
            .expect("seed body");
    });
    let mut app = start(&path, "2026-11-20 09:00");
    open_seeded_document(&mut app, "周报")?;

    // Reading mode: the committed body, its word count, and no toolbar.
    app.wait_for("本周")?;
    app.wait_for("4 字")?;
    app.wait_for_gone("完成", Duration::from_secs(2))?;

    // Editing mode via the primary button; typing commits continuously.
    app.press("编辑")?;
    app.wait_for("完成")?;
    app.type_text("label:文档正文", "新增进展")?;
    app.wait_for("已自动保存")?;

    // Finish hands the lock back and returns to reading mode.
    app.press("完成")?;
    app.wait_for("编辑")?;

    // Restart against the same workspace: the auto-saved body is back.
    drop(app);
    let mut app = start(&path, "2026-11-20 09:30");
    open_seeded_document(&mut app, "周报")?;
    app.wait_for("新增进展本周")?;
    app.wait_for("8 字")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn reading_checklist_toggle_stays_inside_the_document() -> Result<(), String> {
    let path = temp_db("doc-checklist");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        let doc = ws.create_document(book.id, None, "采购").expect("doc");
        ws.save_document(doc.id, "采购", "- [ ] 买牛奶")
            .expect("seed body");
    });
    let mut app = start(&path, "2026-11-20 09:00");
    open_seeded_document(&mut app, "采购")?;

    // Toggling a reading-mode Checklist Item checks it…
    assert!(
        !matches!(
            app.screen()
                .accessibility_state("清单项")
                .expect("the checkbox state")
                .checked,
            Some(Checked::True)
        ),
        "the seeded item starts unchecked"
    );
    app.press("清单项")?;
    assert!(
        matches!(
            app.screen()
                .accessibility_state("清单项")
                .expect("the checkbox state")
                .checked,
            Some(Checked::True)
        ),
        "reading mode toggles the item in place"
    );

    // …and never creates a global Task (ADR-0002): the Inbox badge stays empty.
    app.wait_for("label:收件箱 0 项任务")?;

    // The toggle is committed: it survives a restart.
    drop(app);
    let mut app = start(&path, "2026-11-20 09:30");
    open_seeded_document(&mut app, "采购")?;
    assert!(
        matches!(
            app.screen()
                .accessibility_state("清单项")
                .expect("the checkbox state")
                .checked,
            Some(Checked::True)
        ),
        "the toggle persisted"
    );
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn single_editor_lock_focuses_the_editing_tab_and_locks_readers() -> Result<(), String> {
    let path = temp_db("doc-lock");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        let doc = ws.create_document(book.id, None, "方案").expect("doc");
        ws.save_document(doc.id, "方案", "- [ ] 锁定项")
            .expect("seed body");
    });
    let mut app = start(&path, "2026-11-20 09:00");
    open_seeded_document(&mut app, "方案")?;

    // Tab 2 opens the same document for reading.
    app.press("在新标签页打开")?;
    app.wait_for("编辑")?;

    // Tab 1 takes the edit lock.
    // The editor holds keyboard focus, so mod+N never reaches the app:
    // switch tabs through the strip chips (both read 标签页 方案).
    app.press("label:标签页 方案 >> nth=0")?;
    app.wait_for("编辑")?;
    app.press("编辑")?;
    app.wait_for("完成")?;

    // Tab 2 reads the same document: it sees the lock banner.
    app.press("label:标签页 方案 >> nth=1")?;
    app.wait_for("此文档正在其他标签页编辑 · 清单项已锁定")?;

    // Requesting Edit from tab 2 focuses the editing tab instead.
    app.press("编辑")?;
    app.wait_for("完成")?;
    app.wait_for_gone(
        "此文档正在其他标签页编辑 · 清单项已锁定",
        Duration::from_secs(2),
    )?;

    // Back on tab 2 (mod+2): the banner is back, and its jump prompt
    // returns focus to the editor tab.
    app.press("label:标签页 方案 >> nth=1")?;
    app.wait_for("此文档正在其他标签页编辑 · 清单项已锁定")?;
    app.press("跳到编辑标签")?;
    app.wait_for("完成")?;
    app.wait_for_gone(
        "此文档正在其他标签页编辑 · 清单项已锁定",
        Duration::from_secs(2),
    )?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn named_version_save_and_restore_round_trips_the_body() -> Result<(), String> {
    let path = temp_db("doc-restore");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        let doc = ws.create_document(book.id, None, "文案").expect("doc");
        ws.save_document(doc.id, "文案", "初稿").expect("seed body");
    });
    let mut app = start(&path, "2026-11-20 09:00");
    open_seeded_document(&mut app, "文案")?;

    // Edit to the second draft and save it as the named version 终稿.
    app.press("编辑")?;
    app.wait_for("完成")?;
    app.type_text("label:文档正文", "第二版")?;
    app.wait_for("已自动保存")?;
    app.press("完成")?;
    app.wait_for("编辑")?;
    app.press("本地历史版本")?;
    app.wait_for("命名当前版本，如「评审版」")?;
    app.type_text("text:命名当前版本，如「评审版」", "终稿")?;
    app.press("保存版本")?;
    app.wait_for("终稿")?;

    // A third draft, saved and finished.
    app.press("关闭历史")?;
    app.press("编辑")?;
    app.wait_for("完成")?;
    app.type_text("label:文档正文", "第三版")?;
    app.wait_for("已自动保存")?;
    app.press("完成")?;
    app.wait_for("第三版第二版初稿")?;

    // Restoring 终稿 brings the second draft back — the finish-to-read
    // snapshot archived 第三版 first, so nothing is lost.
    app.press("本地历史版本")?;
    app.wait_for("恢复 终稿")?;
    app.press("恢复 终稿")?;
    // The reading editor re-keyed with the restored body does not surface
    // its text nodes to the headless screen (engine remount quirk); assert
    // the restore through the committed body on reopen instead.
    app.wait_for_gone("第三版第二版初稿", Duration::from_secs(2))?;

    // The restore is committed and survives a restart.
    drop(app);
    let mut app = start(&path, "2026-11-20 10:00");
    open_seeded_document(&mut app, "文案")?;
    app.wait_for("第二版初稿")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

// ---------------------------------------------------------------------
// Issue #7: Checklist → Task promotion, Linked Task Card live sync,
// `@` mentions and backlinks.
// ---------------------------------------------------------------------

#[test]
fn checklist_item_promotes_to_a_linked_task_card_and_undo_keeps_the_task() -> Result<(), String> {
    let path = temp_db("promote-ui");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        let doc = ws.create_document(book.id, None, "周报").expect("doc");
        ws.save_document(doc.id, "周报", "- [ ] 写周报").expect("seed body");
    });
    let mut app = start(&path, "2026-11-20 09:00");
    open_seeded_document(&mut app, "周报")?;

    // Editing mode: the caret starts inside the only Checklist Item, so
    // the promotion chooser opens and targets it.
    app.press("编辑")?;
    app.wait_for("完成")?;
    app.wait_for("转为任务")?;
    app.press("转为任务")?;
    app.wait_for("新任务存入")?;
    app.press("收件箱")?;

    // The Linked Task Card renders the created Task live; the Inbox
    // badge counts it.
    app.wait_for("打开任务 写周报")?;
    assert!(!circle_checked(&app, "完成 写周报"), "the card starts unchecked");
    app.wait_for("label:收件箱 1 项任务")?;

    // Editor Undo after the promotion: the card reverts to a Checklist
    // Item, the reference index drops it — and the Task survives.
    app.press("撤销")?;
    app.wait_for_gone("打开任务 写周报", Duration::from_secs(2))?;
    app.wait_for("label:收件箱 1 项任务")?;

    // Finish, restart: the checklist body persisted and the Task is
    // still in the Inbox.
    app.press("完成")?;
    app.wait_for("编辑")?;
    drop(app);
    let mut app = start(&path, "2026-11-20 09:30");
    open_seeded_document(&mut app, "周报")?;
    app.wait_for("写周报")?;
    app.wait_for("label:收件箱 1 项任务")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn linked_task_card_syncs_from_today_detail_and_survives_restart() -> Result<(), String> {
    let path = temp_db("card-sync-ui");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        let doc = ws.create_document(book.id, None, "周报").expect("doc");
        let task = ws
            .create_task(&nsh_app_kanon::domain::NewTask {
                title: "写周报".to_owned(),
                scheduled_date: Some(date("2026-11-20")),
                ..Default::default()
            })
            .expect("task")
            .task;
        ws.save_document(
            doc.id,
            "周报",
            &format!("{}\n\n- [ ] 普通项", nsh_app_kanon::domain::task_card_marker(task.id)),
        )
        .expect("seed body");
    });
    let mut app = start(&path, "2026-11-20 09:00");
    open_seeded_document(&mut app, "周报")?;
    app.wait_for("打开任务 写周报")?;

    // Pin the document tab so the sidebar opens Today in its own tab.
    app.press("固定标签页 >> nth=0")?;

    // Toggle the Task from its Document card…
    app.press("完成 写周报")?;
    assert!(circle_checked(&app, "完成 写周报"), "the card shows done");

    // …Today reflects the same single source of truth…
    app.press("id:nav-今天")?;
    app.wait_for("id:date-strip")?;
    app.press("完成 写周报 >> nth=0")?;
    app.press("label:标签页 周报 >> nth=0")?;
    assert!(
        !circle_checked(&app, "完成 写周报"),
        "the card follows Today's toggle back to todo"
    );

    // …and a Task Detail edit (a rename) reaches the card immediately.
    app.press("打开任务 写周报")?;
    app.wait_for("任务详情")?;
    app.clear("label:任务标题")?;
    app.type_text("label:任务标题", "写周报 v2")?;
    app.wait_for("写周报 v2")?;
    assert!(
        visible_texts(&app).iter().any(|text| text == "写周报 v2"),
        "the card re-renders the renamed title"
    );

    // The whole state survives a restart.
    drop(app);
    let mut app = start(&path, "2026-11-20 09:30");
    open_seeded_document(&mut app, "周报")?;
    app.wait_for("写周报 v2")?;
    app.wait_for("label:收件箱 0 项任务")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn reading_tab_card_stays_checkable_while_another_tab_edits() -> Result<(), String> {
    let path = temp_db("card-lock-ui");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        let doc = ws.create_document(book.id, None, "方案").expect("doc");
        let task = ws
            .create_task(&nsh_app_kanon::domain::NewTask {
                title: "评审方案".to_owned(),
                scheduled_date: Some(date("2026-11-20")),
                ..Default::default()
            })
            .expect("task")
            .task;
        ws.save_document(
            doc.id,
            "方案",
            &format!("{}\n\n- [ ] 锁定项", nsh_app_kanon::domain::task_card_marker(task.id)),
        )
        .expect("seed body");
    });
    let mut app = start(&path, "2026-11-20 09:00");
    open_seeded_document(&mut app, "方案")?;

    // Tab 2 reads the same Document; tab 1 takes the single-editor lock.
    app.press("在新标签页打开")?;
    app.wait_for("编辑")?;
    app.press("label:标签页 方案 >> nth=0")?;
    app.wait_for("编辑")?;
    app.press("编辑")?;
    app.wait_for("完成")?;

    // Tab 2 (reading): the lock banner shows and its Checklist Items
    // are read-only…
    app.press("label:标签页 方案 >> nth=1")?;
    app.wait_for("此文档正在其他标签页编辑 · 清单项已锁定")?;
    assert!(
        app.screen()
            .accessibility_state("清单项")
            .expect("the checklist state")
            .disabled,
        "reading Checklist Items lock while another tab edits"
    );

    // …but its Linked Task Card stays checkable — a Task write, not a
    // Document write.
    app.press("完成 评审方案")?;
    assert!(circle_checked(&app, "完成 评审方案"), "the card toggles from the reading tab");
    app.wait_for("此文档正在其他标签页编辑 · 清单项已锁定")?;

    // The editing tab agrees without any document write of its own.
    app.press("label:标签页 方案 >> nth=0")?;
    assert!(
        circle_checked(&app, "完成 评审方案"),
        "the editing tab's card sees the same live state"
    );
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn trashed_linked_task_card_degrades_recovers_and_stays_invalid_after_delete() -> Result<(), String> {
    let path = temp_db("card-trash-ui");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        let doc = ws.create_document(book.id, None, "周报").expect("doc");
        let task = ws
            .create_task(&nsh_app_kanon::domain::NewTask { title: "写周报".to_owned(), ..Default::default() })
            .expect("task")
            .task;
        ws.save_document(doc.id, "周报", &nsh_app_kanon::domain::task_card_marker(task.id))
            .expect("seed body");
    });
    let mut app = start(&path, "2026-11-20 09:00");
    open_seeded_document(&mut app, "周报")?;
    app.wait_for("打开任务 写周报")?;

    // Trash the Task from its Task Detail: the card turns into the
    // non-interactive removed-task placeholder in place.
    app.press("打开任务 写周报")?;
    app.wait_for("任务详情")?;
    app.press("移入回收站")?;
    app.wait_for("任务已移除")?;
    app.wait_for_gone("完成 写周报", Duration::from_secs(2))?;

    // Restore it from the Trash screen: the card goes live again.
    app.press("id:nav-回收站")?;
    app.wait_for("写周报")?;
    app.press("恢复")?;
    app.press("id:nav-知识库")?;
    app.wait_for("全部知识库")?;
    app.press("打开知识库 手册")?;
    app.wait_for("id:catalog-tree")?;
    app.press("id:catalog-tree >> text:周报")?;
    app.wait_for("完成 写周报")?;
    assert!(!circle_checked(&app, "完成 写周报"), "the restored card is live and unchecked");

    // Trash again and delete permanently: the invalid placeholder stays.
    app.press("打开任务 写周报")?;
    app.wait_for("任务详情")?;
    app.press("移入回收站")?;
    app.wait_for("任务已移除")?;
    app.press("id:nav-回收站")?;
    app.wait_for("写周报")?;
    app.press("彻底删除")?;
    app.wait_for_gone("写周报", Duration::from_secs(2))?;
    app.press("id:nav-知识库")?;
    app.wait_for("全部知识库")?;
    app.press("打开知识库 手册")?;
    app.wait_for("id:catalog-tree")?;
    app.press("id:catalog-tree >> text:周报")?;
    app.wait_for("任务已移除")?;
    app.wait_for_gone("完成 写周报", Duration::from_secs(2))?;

    // The invalid placeholder survives a restart.
    drop(app);
    let mut app = start(&path, "2026-11-20 09:30");
    open_seeded_document(&mut app, "周报")?;
    app.wait_for("任务已移除")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn inline_mentions_flow_and_feed_the_backlinks_panel() -> Result<(), String> {
    let path = temp_db("mention-ui");
    let (source_id, target_id) = seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        let target = ws.create_document(book.id, None, "手册正文").expect("target");
        let source = ws.create_document(book.id, None, "计划").expect("source");
        let body = format!(
            "前文{}后文{}",
            nsh_app_kanon::domain::mention_marker("document", &target.id.to_string(), "手册正文"),
            nsh_app_kanon::domain::mention_marker("date", "2026-11-21", "2026-11-21"),
        );
        // The editor save path indexes the references; use it in the seed
        // so the backlinks panel has rows from the start.
        ws.update_document_body_and_references(source.id, "计划", &body, None).expect("seed body");
        (source.id, target.id)
    });
    let _ = (source_id, target_id);
    let mut app = start(&path, "2026-11-20 09:00");

    // The mentions flow inline: 前文 and 后文 stay inside ONE paragraph
    // run (no line breaks around the chips), and both chips render
    // their labels as live targets.
    app.wait_for("今天")?;
    app.press("id:nav-知识库")?;
    app.wait_for("全部知识库")?;
    app.press("打开知识库 手册")?;
    app.wait_for("id:catalog-tree")?;
    app.press("id:catalog-tree >> text:计划")?;
    app.wait_for("手册正文")?;
    let texts = visible_texts(&app);
    let paragraph = texts
        .iter()
        .find(|text| text.contains("前文") && text.contains("后文"))
        .expect("前文 and 后文 stay in one paragraph run");
    assert!(!paragraph.contains('\n'), "no line break inside the paragraph: {paragraph:?}");
    assert!(texts.iter().any(|text| text == "手册正文"), "the document chip shows its label: {texts:?}");
    assert!(texts.iter().any(|text| text == "2026-11-21"), "the date chip shows its label: {texts:?}");
    assert!(
        !texts.iter().any(|text| text.contains("已失效")),
        "live targets do not render the invalid placeholder: {texts:?}"
    );

    // The target Document's right panel lists the backlink.
    app.press("id:catalog-tree >> text:手册正文")?;
    app.wait_for("反向链接")?;
    app.wait_for("label:打开文档 计划")?;
    // Jump through the backlink into the source Document.
    app.press("label:打开文档 计划")?;
    app.wait_for("text:手册正文")?;
    app.wait_for("text:2026-11-21")?;

    // Trash the source through its catalog row menu: the backlink
    // degrades but stays listed.
    app.press("节点菜单 >> nth=0")?;
    app.wait_for("移入回收站")?;
    app.press("移入回收站")?;
    app.wait_for("id:catalog-tree")?;
    app.press("id:catalog-tree >> text:手册正文")?;
    app.wait_for("反向链接")?;
    app.wait_for("计划（源文档已删除）")?;

    // Restore the source from the catalog's 最近删除 section: the
    // backlink recovers and jumps again.
    app.press("label:恢复 计划")?;
    app.wait_for_gone("label:恢复 计划", Duration::from_secs(2))?;
    app.press("id:catalog-tree >> text:手册正文")?;
    app.wait_for("label:打开文档 计划")?;
    app.press("label:打开文档 计划")?;
    app.wait_for("text:手册正文")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn mention_chips_degrade_while_the_target_sits_in_trash_and_recover_on_restore(
) -> Result<(), String> {
    let path = temp_db("mention-degrade-ui");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        let target = ws.create_document(book.id, None, "手册正文").expect("target");
        let source = ws.create_document(book.id, None, "计划").expect("source");
        let body = format!(
            "前文{}后文",
            nsh_app_kanon::domain::mention_marker("document", &target.id.to_string(), "手册正文"),
        );
        ws.update_document_body_and_references(source.id, "计划", &body, None).expect("seed body");
    });
    let mut app = start(&path, "2026-11-20 09:00");

    // The mentions live-chip view degrades while its target sits in
    // Trash and recovers on restore.
    app.wait_for("今天")?;
    app.press("id:nav-知识库")?;
    app.wait_for("全部知识库")?;
    app.press("打开知识库 手册")?;
    app.wait_for("id:catalog-tree")?;
    app.press("id:catalog-tree >> text:计划")?;
    // The live target renders its plain label.
    app.wait_for("手册正文")?;
    let texts = visible_texts(&app);
    assert!(
        !texts.iter().any(|text| text.contains("已失效")),
        "a live target shows no invalid placeholder: {texts:?}"
    );

    // Trash the TARGET through its catalog row menu: the chip degrades
    // to the invalid placeholder while the source Document stays put.
    app.press("节点菜单 >> nth=1")?;
    app.wait_for("移入回收站")?;
    app.press("移入回收站")?;
    app.wait_for("手册正文（已失效）")?;

    // Restore the target: the chip recovers automatically.
    app.press("label:恢复 手册正文")?;
    app.wait_for_gone("label:恢复 手册正文", Duration::from_secs(2))?;
    app.wait_for("手册正文")?;
    let texts = visible_texts(&app);
    assert!(
        !texts.iter().any(|text| text.contains("已失效")),
        "the restored target renders the live chip again: {texts:?}"
    );
    let _ = std::fs::remove_file(path);
    Ok(())
}

// ---------------------------------------------------------------------
// Projects (issue #8, ADR-0001)
// ---------------------------------------------------------------------

/// Opens the only project's page through the sidebar.
fn open_project(app: &mut Headless) -> Result<(), String> {
    app.wait_for("今天")?;
    app.press("id:sidebar-project")?;
    app.wait_for("id:project-view")?;
    Ok(())
}

/// Trashes the project's linked grouping node through the book's catalog
/// row menu.
fn trash_linked_group(app: &mut Headless) -> Result<(), String> {
    app.press("id:catalog-tree >> label:第一章")?;
    app.wait_for("节点菜单")?;
    app.press("id:catalog-tree >> label:节点菜单")?;
    app.wait_for("移入回收站")?;
    app.press("移入回收站")?;
    Ok(())
}

#[test]
fn creating_a_project_with_a_same_named_knowledge_base_lists_both() -> Result<(), String> {
    let path = temp_db("proj-create");
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);

    // Create from the sidebar with the one-click same-named binding.
    app.wait_for("今天")?;
    app.press("id:project-new")?;
    app.wait_for("新建项目")?;
    app.type_text("text:项目名称", "发布会")?;
    app.press("新建同名知识库")?;
    app.press("id:project-save")?;

    // The project view opens, bound to the fresh book.
    app.wait_for("id:project-view")?;
    app.wait_for("📚 发布会")?;
    app.wait_for("还没有关联知识库").expect_err("bound projects skip the unbound state");

    // The same-named Knowledge Base exists on the shelf.
    app.press("id:nav-知识库")?;
    app.wait_for("打开知识库 发布会")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn project_sub_views_show_metrics_quick_add_milestones_and_kanban_moves() -> Result<(), String> {
    let path = temp_db("proj-subviews");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        let group = ws.create_catalog_node(book.id, None, "第一章").expect("group");
        let project = ws
            .create_project("改版", "#547093", "🚀", Some(book.id), Some(group.id))
            .expect("project");
        let milestone = ws.create_milestone(project.id, "阶段一", None).expect("milestone");
        let task = ws
            .create_task(&nsh_app_kanon::domain::NewTask {
                title: "画稿".to_owned(),
                project_id: Some(project.id),
                ..Default::default()
            })
            .expect("task");
        ws.update_task(
            task.task.id,
            &nsh_app_kanon::domain::TaskPatch {
                milestone_id: Some(Some(milestone.id)),
                ..Default::default()
            },
        )
        .expect("assign milestone");
    });
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    open_project(&mut app)?;

    // Overview: metrics and the milestone with its live progress.
    app.wait_for("总任务")?;
    app.wait_for("关联文档")?;
    app.wait_for("阶段一")?;
    app.wait_for("0/1 项完成")?;

    // Add a milestone inline.
    app.type_text("text:里程碑名称，回车添加", "上线")?;
    app.press("id:milestone-add")?;
    app.wait_for("上线")?;

    // Task List: the inline quick-add lands in the project.
    app.press("任务列表")?;
    app.type_text("text:添加项目任务，回车创建", "走查")?;
    app.press("id:project-quick-add")?;
    app.wait_for("走查")?;

    // Kanban: move the new card across lanes.
    app.press("看板")?;
    app.wait_for("id:kanban-todo")?;
    app.press("移动 走查")?;
    app.wait_for("移到进行中")?;
    app.press("移到进行中")?;
    app.wait_for("id:kanban-doing >> text:走查")?;

    // Deleting the milestone clears the reference without deleting the
    // task: the progress line is gone, and 画稿 still lists in the Task
    // List sub-view.
    app.press("概览与里程碑")?;
    app.press("删除里程碑 >> nth=0")?;
    app.wait_for_gone("0/1 项完成", Duration::from_secs(2))?;
    app.press("任务列表")?;
    app.wait_for("画稿")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn kanban_shows_the_cancel_column_only_while_it_holds_cards() -> Result<(), String> {
    let path = temp_db("proj-kanban-cancel");
    seed(&path, "2026-11-20 09:00", |ws| {
        let project = ws.create_project("小项目", "#547093", "📌", None, None).expect("project");
        ws.create_task(&nsh_app_kanon::domain::NewTask {
            title: "任务甲".to_owned(),
            project_id: Some(project.id),
            ..Default::default()
        })
        .expect("task");
        ws.create_task(&nsh_app_kanon::domain::NewTask {
            title: "任务乙".to_owned(),
            project_id: Some(project.id),
            ..Default::default()
        })
        .expect("task");
    });
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    open_project(&mut app)?;
    app.press("看板")?;

    // Two todo cards, no cancel column yet.
    app.wait_for("id:kanban-todo")?;
    app.wait_for("任务甲")?;
    app.wait_for("任务乙")?;
    app.wait_for("id:kanban-cancel").expect_err("the cancel lane hides while empty");

    // Move one card to cancel: the optional column appears with it.
    app.press("移动 任务乙")?;
    app.wait_for("移到已取消")?;
    app.press("移到已取消")?;
    app.wait_for("id:kanban-cancel >> text:任务乙")?;

    // Move it back: the column disappears again.
    app.press("移动 任务乙")?;
    app.wait_for("移到待办")?;
    app.press("移到待办")?;
    app.wait_for("id:kanban-todo >> text:任务乙")?;
    app.wait_for_gone("id:kanban-cancel", Duration::from_secs(2))?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn documents_created_from_linked_documents_land_in_the_linked_scope() -> Result<(), String> {
    let path = temp_db("proj-doc-scope");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        let group = ws.create_catalog_node(book.id, None, "第一章").expect("group");
        ws.create_project("分组项目", "#547093", "📌", Some(book.id), Some(group.id))
            .expect("project");
        ws.create_project("整本项目", "#547093", "📌", Some(book.id), None)
            .expect("project");
    });
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);

    // Bound to a grouping node: the landing names it, and the new
    // document appears inside it.
    app.wait_for("今天")?;
    app.press("id:sidebar-project >> nth=0")?;
    app.wait_for("id:project-view")?;
    app.press("关联文档")?;
    app.wait_for("新建文档将落在关联目录「第一章」下")?;
    app.press("id:project-doc-create")?;
    app.wait_for("未命名文档")?;
    assert!(
        tree_shows(&app, "未命名文档"),
        "the project document renders in the reused catalog tree"
    );
    // The document opened inside the project tab (the sub-view switch
    // stays visible next to the canvas).
    app.wait_for("概览与里程碑")?;

    // Bound to the whole book: the document lands at the root top.
    app.press("id:sidebar-project >> nth=1")?;
    app.wait_for("整本项目")?;
    app.press("关联文档")?;
    app.wait_for("新建文档将落在知识库「手册」的目录顶部")?;
    app.press("id:project-doc-create")?;
    app.wait_for("未命名文档")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn trashing_a_project_keeps_its_tasks_visible_and_restore_is_lossless() -> Result<(), String> {
    let path = temp_db("proj-trash-ui");
    seed(&path, "2026-11-20 09:00", |ws| {
        let project = ws.create_project("过渡项目", "#547093", "📌", None, None).expect("project");
        ws.create_task(&nsh_app_kanon::domain::NewTask {
            title: "收尾任务".to_owned(),
            project_id: Some(project.id),
            ..Default::default()
        })
        .expect("task");
    });
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    open_project(&mut app)?;

    // Trash it from the project menu; the tab falls back to 待办.
    app.press("项目菜单")?;
    app.wait_for("移入回收站")?;
    app.press("移入回收站")?;
    app.wait_for("id:tasks-group-未排期")?;

    // The task stays visible in the regular task views (ADR-0001).
    app.wait_for("收尾任务")?;

    // Restore from the Trash screen; the task never lost its assignment.
    app.press("id:nav-回收站")?;
    app.wait_for("过渡项目")?;
    app.press("id:project-restore-1")?;
    app.wait_for("回收站是空的")?;
    app.press("id:sidebar-project")?;
    app.wait_for("过渡项目")?;
    app.press("任务列表")?;
    app.wait_for("收尾任务")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn permanent_project_deletion_detaches_tasks_but_preserves_the_knowledge_base() -> Result<(), String> {
    let path = temp_db("proj-delete-ui");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("沉淀").expect("kb");
        ws.create_document(book.id, None, "纪要").expect("doc");
        let project = ws.create_project("短项目", "#547093", "📌", Some(book.id), None).expect("project");
        ws.create_task(&nsh_app_kanon::domain::NewTask {
            title: "孤儿任务".to_owned(),
            project_id: Some(project.id),
            ..Default::default()
        })
        .expect("task");
    });
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    open_project(&mut app)?;
    app.press("项目菜单")?;
    app.wait_for("移入回收站")?;
    app.press("移入回收站")?;

    app.press("id:nav-回收站")?;
    app.wait_for("短项目")?;
    app.press("id:project-delete-1")?;
    app.wait_for_gone("短项目", Duration::from_secs(2))?;

    // The task survives, now without a project…
    app.press("id:nav-待办")?;
    app.wait_for("孤儿任务")?;
    app.wait_for("text:短项目").expect_err("the row no longer names a project");

    // …and the Knowledge Base with its document is intact.
    app.press("id:nav-知识库")?;
    app.wait_for("打开知识库 沉淀")?;
    app.press("打开知识库 沉淀")?;
    app.wait_for("1 篇文档 · 0 字")?;
    app.wait_for("纪要")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn trashing_the_linked_grouping_node_shows_the_recoverable_notice() -> Result<(), String> {
    let path = temp_db("proj-node-trash-ui");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        let group = ws.create_catalog_node(book.id, None, "第一章").expect("group");
        ws.create_project("绑定项目", "#547093", "📌", Some(book.id), Some(group.id))
            .expect("project");
    });
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    open_project(&mut app)?;
    app.press("关联文档")?;
    app.wait_for("id:catalog-tree")?;

    // Trash the linked node from the book's catalog.
    trash_linked_group(&mut app)?;

    // The project's binding survives with the recoverable notice (the
    // Linked Documents sub-view explains the state and offers actions).
    app.press("id:sidebar-project")?;
    app.wait_for("id:project-view")?;
    app.press("关联文档")?;
    app.wait_for("关联目录「第一章」在回收站中")?;
    app.wait_for("id:project-restore-node")?;
    app.wait_for("id:project-rebind")?;

    // One-click restore brings the linked tree back.
    app.press("id:project-restore-node")?;
    app.wait_for("id:catalog-tree")?;
    app.wait_for("新建文档将落在关联目录「第一章」下")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn permanently_deleting_the_linked_grouping_node_resets_the_project_to_unbound() -> Result<(), String> {
    let path = temp_db("proj-node-delete-ui");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        let group = ws.create_catalog_node(book.id, None, "第一章").expect("group");
        ws.create_project("绑定项目", "#547093", "📌", Some(book.id), Some(group.id))
            .expect("project");
    });
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    open_project(&mut app)?;
    app.press("关联文档")?;
    app.wait_for("id:catalog-tree")?;
    trash_linked_group(&mut app)?;

    // Permanently delete the node from the Trash screen (目录 section).
    app.press("id:nav-回收站")?;
    app.wait_for("第一章")?;
    app.press("id:node-delete-1")?;
    app.wait_for_gone("第一章", Duration::from_secs(2))?;

    // The project is cleanly unbound — never widened to the whole book.
    app.press("id:sidebar-project")?;
    app.wait_for("id:project-view")?;
    app.press("关联文档")?;
    app.wait_for("还没有关联知识库")?;
    app.wait_for("id:project-bind")?;
    app.wait_for("id:project-bind-new-book")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn rebinding_a_project_to_a_grouping_node_scopes_new_documents() -> Result<(), String> {
    let path = temp_db("proj-rebind-ui");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("手册").expect("kb");
        ws.create_catalog_node(book.id, None, "第一章").expect("group");
        ws.create_project("未绑定", "#547093", "📌", None, None).expect("project");
    });
    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);
    open_project(&mut app)?;

    // The unbound state offers the binding actions.
    app.press("关联文档")?;
    app.wait_for("还没有关联知识库")?;

    // Rebind through the editor to the book's grouping node.
    app.press("id:project-bind")?;
    app.wait_for("编辑项目")?;
    app.press("关联目录分组")?;
    app.press("id:project-book-picker")?;
    app.wait_for("手册")?;
    app.press("手册")?;
    app.press("id:project-node-picker")?;
    app.wait_for("第一章")?;
    app.press("第一章")?;
    app.press("id:project-save")?;

    // Back on the project, the linked scope names the node.
    app.wait_for("id:project-view")?;
    app.wait_for("📚 手册 / 第一章")?;
    app.press("关联文档")?;
    app.wait_for("新建文档将落在关联目录「第一章」下")?;
    let _ = std::fs::remove_file(path);
    Ok(())
}
