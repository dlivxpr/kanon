//! Planning-controls regression coverage for paths no existing test
//! reaches: the Calendar view's ranges, cross-month stepping, day
//! quick-add and event kinds; the Mini Calendar's month step and its
//! jump to Today; the Task Overview's capture filter and the Quick Note
//! capture; the Tasks view's priority and unassigned-Project filters.
//!
//! Fixtures are isolated per test (a temp SQLite file plus a fixed
//! clock), matching the per-file convention of `tests/app.rs`.

use std::path::PathBuf;

use nsh_app_kanon::AppProps;
use nsh_app_kanon::domain::{LocalDate, LocalTime};
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
        "kanon-planning-{name}-{}-{}.sqlite3",
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

/// Seeds a workspace file through the domain facade, so the test starts
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

#[test]
fn calendar_ranges_step_across_months_and_quick_add_creates_on_the_day() -> Result<(), String> {
    let path = temp_db("calendar-ranges");
    let mut app = start(&path, "2026-11-20 09:00");
    app.wait_for("今天")?;
    app.press("id:nav-日历")?;
    app.wait_for("id:calendar")?;
    app.wait_for("id:calendar-grid")?;

    // Month range: the November grid reaches into both neighbours.
    app.wait_for("2026年11月")?;
    app.wait_for("id:calendar-day-2026-10-26")?;
    app.wait_for("id:calendar-day-2026-12-06")?;
    app.wait_for("id:calendar-day-2026-11-20")?;

    // Week range: one whole week per step, across the month boundary.
    app.press("label:范围 周")?;
    app.wait_for("11月16日 — 11月22日")?;
    app.press("label:下一期")?;
    app.wait_for("11月23日 — 11月29日")?;
    app.press("label:下一期")?;
    app.wait_for("11月30日 — 12月6日")?;

    // Three-day range steps by three days around the same anchor.
    app.press("label:范围 3 天")?;
    app.wait_for("12月4日 — 12月6日")?;
    app.press("label:上一期")?;
    app.wait_for("12月1日 — 12月3日")?;

    // Back to the month range around the stepped anchor.
    app.press("label:范围 月")?;
    app.wait_for("2026年12月")?;
    app.press("label:上一期")?;
    app.wait_for("2026年11月")?;

    // The day number toggles that day's inline quick add.
    app.press("label:2026-11-25 添加任务")?;
    app.wait_for("id:calendar-add-2026-11-25")?;
    app.type_text("text:添加任务，回车保存", "日历新建")?;
    app.press("id:calendar-add-submit")?;
    app.wait_for("任务详情")?;
    app.press("关闭任务详情")?;

    // The created task is scheduled for that day: it shows in Upcoming.
    app.press("id:nav-即将到来")?;
    app.wait_for("日历新建")?;

    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn calendar_marks_due_only_tasks_and_milestones_and_opens_their_pages() -> Result<(), String> {
    use nsh_app_kanon::domain::NewTask;
    let path = temp_db("calendar-events");
    seed(&path, "2026-11-20 09:00", |ws| {
        ws.create_task(&NewTask {
            title: "仅截止".to_owned(),
            due_date: Some(date("2026-11-22")),
            ..Default::default()
        })
        .unwrap();
        ws.create_task(&NewTask {
            title: "排期事".to_owned(),
            scheduled_date: Some(date("2026-11-21")),
            scheduled_time: Some(time("10:30")),
            ..Default::default()
        })
        .unwrap();
        let project = ws
            .create_project("网站", "#547093", "🌐", None, None)
            .unwrap();
        ws.create_milestone(project.id, "上线", Some(date("2026-11-23")))
            .unwrap();
    });

    let mut app = start(&path, "2026-11-20 09:00");
    app.wait_for("今天")?;
    app.press("id:nav-日历")?;
    app.wait_for("id:calendar-grid")?;

    // A due-date-only Task renders on its Due Date as an event, and so
    // does the scheduled Task on its Scheduled Date.
    app.wait_for("label:打开任务 仅截止")?;
    app.wait_for("label:打开任务 排期事")?;
    // The Milestone renders with the ◇ marker under its target date.
    app.wait_for("label:里程碑 上线 · 网站")?;

    // Events open the Task Detail in the right context panel.
    app.press("label:打开任务 排期事")?;
    app.wait_for("任务详情")?;
    app.press("关闭任务详情")?;

    // Milestones open the Project page.
    app.press("label:里程碑 上线 · 网站")?;
    app.wait_for("id:project-view")?;

    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn mini_calendar_steps_months_and_jumps_to_today_with_the_day_selected() -> Result<(), String> {
    use nsh_app_kanon::domain::NewTask;
    let path = temp_db("mini-calendar");
    seed(&path, "2026-11-20 09:00", |ws| {
        ws.create_task(&NewTask {
            title: "十二月事".to_owned(),
            scheduled_date: Some(date("2026-12-02")),
            ..Default::default()
        })
        .unwrap();
    });

    let mut app = start(&path, "2026-11-20 09:00");
    app.wait_for("今天")?;
    // The top bar's panel-mode switch shows the Mini Calendar.
    app.press("label:迷你日历")?;
    app.wait_for("id:mini-calendar")?;
    app.wait_for("2026年11月")?;

    // Step into December and pick the seeded day.
    app.press("label:下一月")?;
    app.wait_for("2026年12月")?;
    app.wait_for("id:mini-day-2026-12-02")?;
    app.press("id:mini-day-2026-12-02")?;

    // The workbench jumps to Today with that date selected.
    app.wait_for("label:选择 2026-12-02")?;
    app.wait_for("12月2日")?;
    app.wait_for("十二月事")?;
    app.press("id:inbox-badge")?;
    app.wait_for("label:筛选收件箱任务")?;
    app.wait_for_gone("id:mini-calendar", std::time::Duration::from_secs(2))?;
    app.wait_for("label:选择 2026-12-02")?;

    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn editing_a_task_title_updates_the_existing_today_row() -> Result<(), String> {
    use nsh_app_kanon::domain::NewTask;
    let path = temp_db("rename-row");
    let now = "2026-11-20 09:00";
    seed(&path, now, |ws| {
        ws.create_task(&NewTask {
            title: "初稿".to_owned(),
            scheduled_date: Some(date("2026-11-20")),
            ..Default::default()
        })
        .unwrap();
    });
    let mut app = start(&path, now);
    app.wait_for("label:打开任务 初稿")?;
    app.press("label:切换上下文面板")?;
    app.wait_for_gone("任务速览", std::time::Duration::from_secs(2))?;
    app.press("label:打开任务 初稿")?;
    app.clear("label:任务标题")?;
    app.type_text("label:任务标题", "终稿")?;
    app.wait_for("label:打开任务 终稿")?;
    app.wait_for("label:完成 终稿")?;
    assert!(!app.shows("label:打开任务 初稿"));
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn task_document_links_open_the_live_document() -> Result<(), String> {
    use nsh_app_kanon::domain::{NewTask, task_card_marker};
    let path = temp_db("task-documents");
    let now = "2026-11-20 09:00";
    seed(&path, now, |ws| {
        let task = ws
            .create_task(&NewTask {
                title: "确认范围".into(),
                scheduled_date: Some(date("2026-11-20")),
                ..Default::default()
            })
            .unwrap();
        let book = ws.create_knowledge_base("资料").unwrap();
        let marker = task_card_marker(task.task.id);
        let live = ws.create_document(book.id, None, "计划文档").unwrap();
        ws.update_document_body_and_references(
            live.id,
            "计划文档",
            &format!("{marker}\n\n{marker}\n\n正文验证"),
            None,
        )
        .unwrap();
        let trashed = ws.create_document(book.id, None, "旧计划").unwrap();
        ws.update_document_body_and_references(trashed.id, "旧计划", &marker, None)
            .unwrap();
        let node = ws.catalog_node_of_document(trashed.id).unwrap().unwrap();
        ws.trash_catalog_node(node.id).unwrap();
    });

    let mut app = start(&path, now);
    app.wait_for("label:关联文档 计划文档")?;
    app.press("label:切换上下文面板")?;
    app.press("label:打开任务 确认范围")?;
    app.wait_for("label:打开文档 计划文档")?;
    assert!(!app.shows("label:打开文档 旧计划"));
    app.press("label:打开文档 计划文档")?;
    app.wait_for("正文验证")?;
    app.press("id:nav-今天")?;
    app.press("label:关联文档 计划文档")?;
    app.wait_for("正文验证")?;
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn overview_filter_switches_groups_and_note_capture_publishes_a_note() -> Result<(), String> {
    use nsh_app_kanon::domain::{NewTask, Priority, TaskPatch, TaskStatus};
    let path = temp_db("overview-filter");
    seed(&path, "2026-11-20 09:00", |ws| {
        ws.create_task(&NewTask {
            title: "紧急事".to_owned(),
            priority: Some(Priority::High),
            due_date: Some(date("2026-11-19")),
            ..Default::default()
        })
        .unwrap();
        ws.create_task(&NewTask {
            title: "平常事".to_owned(),
            due_date: Some(date("2026-11-19")),
            ..Default::default()
        })
        .unwrap();
        let done = ws
            .create_task(&NewTask {
                title: "做完事".to_owned(),
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
    });

    let mut app = start(&path, "2026-11-20 09:00");
    app.wait_for("今天")?;
    // The Inbox is empty; the overview lists both overdue rows.
    app.wait_for("label:收件箱 0 项任务")?;
    app.wait_for("紧急事")?;
    app.wait_for("平常事")?;

    // 高优先级 narrows the overdue group to the high-priority row.
    app.press("label:筛选收件箱任务")?;
    app.wait_for("label:筛选 高优先级")?;
    app.press("label:筛选 高优先级")?;
    app.wait_for("紧急事")?;
    assert!(
        !app.shows("平常事"),
        "the normal-priority row hides under 高优先级"
    );

    // 已完成 replaces the groups with the done group.
    app.press("label:筛选收件箱任务")?;
    app.wait_for("label:筛选 已完成")?;
    app.press("label:筛选 已完成")?;
    app.wait_for("做完事")?;
    assert!(!app.shows("紧急事"), "overdue rows hide under 已完成");

    // 速记小记 publishes a Quick Note without touching the Inbox.
    app.press("label:速记小记")?;
    app.wait_for("保存到小记")?;
    app.type_text("label:速记小记内容", "验收小记一条")?;
    app.press("id:quick-capture-note")?;
    app.wait_for("验收小记一条")?;
    app.wait_for("label:收件箱 0 项任务")?;

    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn tasks_view_priority_and_unassigned_project_filters_narrow_rows() -> Result<(), String> {
    use nsh_app_kanon::domain::{NewTask, Priority};
    let path = temp_db("tasks-filters");
    seed(&path, "2026-11-20 09:00", |ws| {
        ws.create_task(&NewTask {
            title: "高优任务".to_owned(),
            priority: Some(Priority::High),
            ..Default::default()
        })
        .unwrap();
        ws.create_task(&NewTask {
            title: "低优任务".to_owned(),
            priority: Some(Priority::Low),
            ..Default::default()
        })
        .unwrap();
        let project = ws
            .create_project("网站", "#547093", "🌐", None, None)
            .unwrap();
        ws.create_task(&NewTask {
            title: "项内任务".to_owned(),
            project_id: Some(project.id),
            ..Default::default()
        })
        .unwrap();
    });

    let mut app = start(&path, "2026-11-20 09:00");
    app.wait_for("今天")?;
    // Count rows on the Tasks view itself, without the overview panel.
    app.press("切换上下文面板")?;
    app.press("id:nav-待办")?;
    app.wait_for("id:tasks-group-收件箱")?;
    app.wait_for("高优任务")?;
    app.wait_for("低优任务")?;
    app.wait_for("项内任务")?;
    app.wait_for("3 项未完成任务")?;

    // The priority filter narrows every group to high-priority rows.
    app.press("label:优先级筛选 高")?;
    app.wait_for("高优任务")?;
    app.wait_for("3 项未完成任务")?;
    assert!(!app.shows("低优任务"), "low priority hides under 高");
    assert!(
        !app.shows("项内任务"),
        "priority-less project rows hide under 高"
    );
    app.press("label:优先级筛选 全部")?;
    app.wait_for("低优任务")?;

    // The 无项目 scope keeps only project-less rows.
    app.press("label:项目筛选 无项目")?;
    app.wait_for("高优任务")?;
    assert!(
        !app.shows("项内任务"),
        "project-bound rows hide under 无项目"
    );
    app.press("label:项目筛选 全部")?;
    app.wait_for("项内任务")?;

    let _ = std::fs::remove_file(path);
    Ok(())
}
