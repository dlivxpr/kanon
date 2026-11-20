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
