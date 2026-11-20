//! Domain seam tests: every business rule and the full SQLite schema,
//! exercised through the single public `Workspace` facade with an
//! injected database path and a controlled local clock.

use std::path::PathBuf;
use std::sync::Arc;

use nsh_app_kanon::domain::{
    FixedClock, LocalDate, LocalDateTime, LocalTime, NewTask, Priority, ProjectFilter, TaskFilter,
    TaskPatch, TaskSort, TaskStatus, Workspace,
};

fn at(text: &str) -> Arc<FixedClock> {
    FixedClock::at(LocalDateTime::parse(text).expect("valid fixed datetime"))
}

fn temp_db(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "kanon-test-{name}-{}-{}.sqlite3",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&path);
    path
}

fn workspace_at(name: &str, now: &str) -> (Workspace, PathBuf, Arc<FixedClock>) {
    let path = temp_db(name);
    let clock = at(now);
    let workspace = Workspace::open(&path, clock.clone()).expect("open workspace");
    (workspace, path, clock)
}

fn date(text: &str) -> LocalDate {
    LocalDate::parse(text).unwrap()
}

fn time(text: &str) -> LocalTime {
    LocalTime::parse(text).unwrap()
}

#[test]
fn full_schema_survives_close_and_reopen() {
    let (mut ws, path, _clock) = workspace_at("schema", "2026-11-20 09:00");

    // One of every entity, exactly as ADR-0003 describes.
    let book = ws.create_knowledge_base("产品手册").expect("kb");
    let group = ws.create_catalog_node(book.id, None, "第一章").expect("group");
    let doc = ws.create_document(book.id, Some(group.id), "入门").expect("doc");
    ws.save_document_revision(doc.id, None).expect("auto revision");
    ws.save_document_revision(doc.id, Some("v1.0")).expect("named revision");
    let project = ws.create_project("Kanon", "#ff4f4f", "🚀", Some(book.id), Some(group.id)).expect("project");
    let milestone = ws.create_milestone(project.id, "首发", Some(date("2026-12-01"))).expect("milestone");
    let task = ws
        .create_task(&NewTask {
            title: "写文档".to_owned(),
            priority: Some(Priority::High),
            project_id: Some(project.id),
            scheduled_date: Some(date("2026-11-20")),
            scheduled_time: Some(time("10:00")),
            due_date: Some(date("2026-11-21")),
            due_time: Some(time("18:00")),
            ..Default::default()
        })
        .expect("task");
    let tag = ws.ensure_tag("写作").expect("tag");
    ws.attach_tag(task.task.id, tag.id).expect("attach");
    let subtask = ws.add_subtask(task.task.id, "列大纲").expect("subtask");
    ws.set_subtask_done(subtask.id, true).expect("subtask done");
    let note = ws.create_quick_note("碎片想法").expect("note");
    ws.set_preference("accent", "#00b96b").expect("preference");
    drop(ws);

    // Reopen against the same file: everything is still there.
    let mut ws = Workspace::open(&path, at("2026-11-20 10:00")).expect("reopen");
    let task = ws.task(task.task.id).expect("task loads").expect("present");
    assert_eq!(task.task.title, "写文档");
    assert_eq!(task.task.status, TaskStatus::Todo);
    assert_eq!(task.task.priority, Priority::High);
    assert_eq!(task.task.scheduled_date, Some(date("2026-11-20")));
    assert_eq!(task.task.scheduled_time, Some(time("10:00")));
    assert_eq!(task.task.due_date, Some(date("2026-11-21")));
    assert_eq!(task.task.due_time, Some(time("18:00")));
    assert_eq!(task.project.expect("project ref").name, "Kanon");
    assert_eq!(task.tags.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), vec!["写作"]);
    assert!(task.subtasks[0].done, "subtask state persists");
    assert_eq!(ws.projects().expect("projects").len(), 1);
    assert_eq!(ws.preference("accent").expect("pref"), Some("#00b96b".to_owned()));
    assert_eq!(ws.inbox_tasks().expect("inbox").len(), 0, "project-bound task is not in the inbox");
    assert_eq!(note.id, 1, "quick note row persisted");

    ws.delete_milestone(milestone.id).expect("delete milestone clears task refs (ADR-0001)");
    let task = ws.task(task.task.id).expect("task loads").expect("present");
    assert_eq!(task.task.milestone_id, None, "milestone reference cleared without deleting the task");

    let _ = std::fs::remove_file(path);
}

#[test]
fn creates_schedules_completes_and_reopens_a_task() {
    let (mut ws, path, _clock) = workspace_at("lifecycle", "2026-11-20 09:00");
    let id = ws
        .create_task(&NewTask { title: "回邮件".to_owned(), scheduled_date: Some(date("2026-11-20")), ..Default::default() })
        .expect("create")
        .task
        .id;
    assert_eq!(ws.tasks_for_date(date("2026-11-20")).expect("today").len(), 1);
    assert_eq!(ws.tasks_for_date(date("2026-11-21")).expect("tomorrow").len(), 0);

    // The quick circle toggles strictly between done and todo.
    let done = ws.toggle_task_circle(id).expect("toggle to done");
    assert_eq!(done.status, TaskStatus::Done);
    assert!(done.completed_at.is_some(), "completion time is recorded");
    let reopened = ws.toggle_task_circle(id).expect("toggle back to todo");
    assert_eq!(reopened.status, TaskStatus::Todo);
    assert!(reopened.completed_at.is_none(), "completion time clears on reopen");

    // do/cancel go through the detail patch, not the circle.
    ws.update_task(id, &TaskPatch { status: Some(TaskStatus::Doing), ..Default::default() }).expect("doing");
    let toggled = ws.toggle_task_circle(id).expect("circle from doing");
    assert_eq!(toggled.status, TaskStatus::Done);
    drop(ws);

    let ws = Workspace::open(&path, at("2026-11-20 10:00")).expect("reopen");
    let task = ws.task(id).expect("load").expect("present");
    assert_eq!(task.task.status, TaskStatus::Done, "status persists across restart");
    assert_eq!(ws.tasks_for_date(date("2026-11-20")).expect("today").len(), 1);
    let _ = std::fs::remove_file(path);
}

#[test]
fn today_query_excludes_cancelled_and_trashed_tasks() {
    let (mut ws, _path, _clock) = workspace_at("today-filter", "2026-11-20 09:00");
    let keep = ws
        .create_task(&NewTask { title: "保留".to_owned(), scheduled_date: Some(date("2026-11-20")), ..Default::default() })
        .expect("keep")
        .task
        .id;
    let cancelled = ws
        .create_task(&NewTask { title: "取消".to_owned(), scheduled_date: Some(date("2026-11-20")), ..Default::default() })
        .expect("cancelled")
        .task
        .id;
    ws.update_task(cancelled, &TaskPatch { status: Some(TaskStatus::Cancelled), ..Default::default() }).unwrap();
    let trashed = ws
        .create_task(&NewTask { title: "已删".to_owned(), scheduled_date: Some(date("2026-11-20")), ..Default::default() })
        .expect("trashed")
        .task
        .id;
    ws.move_task_to_trash(trashed).unwrap();

    let titles = ws
        .tasks_for_date(date("2026-11-20"))
        .expect("today")
        .into_iter()
        .map(|agg| agg.task.title)
        .collect::<Vec<_>>();
    assert_eq!(titles, vec!["保留".to_owned()]);
    assert!(ws.task(keep).expect("keep").is_some());
    assert!(ws.task(trashed).expect("trashed row still exists").is_some(), "trash is a flag, not a delete");
}

#[test]
fn floating_local_time_overdue_boundaries_through_the_workspace() {
    let clock = at("2026-11-20 23:59");
    let mut ws = Workspace::open_in_memory_with_clock(clock.clone()).expect("workspace");

    // Date-only due date: not overdue on its day, overdue the next midnight.
    let id = ws
        .create_task(&NewTask {
            title: "日期截止".to_owned(),
            due_date: Some(date("2026-11-20")),
            ..Default::default()
        })
        .expect("create")
        .task
        .id;
    clock.advance_to(LocalDateTime::parse("2026-11-20 23:59").unwrap());
    assert!(!ws.task(id).unwrap().unwrap().task.is_overdue_at(ws.now()));
    clock.advance_to(LocalDateTime::parse("2026-11-21 00:00").unwrap());
    assert!(ws.task(id).unwrap().unwrap().task.is_overdue_at(ws.now()));
    assert_eq!(ws.task_groups().expect("groups").overdue.len(), 1);

    // Timed due date: overdue only after the local cutoff.
    let timed = ws
        .create_task(&NewTask {
            title: "时分截止".to_owned(),
            due_date: Some(date("2026-11-21")),
            due_time: Some(time("09:30")),
            ..Default::default()
        })
        .expect("create")
        .task
        .id;
    clock.advance_to(LocalDateTime::parse("2026-11-21 09:30").unwrap());
    assert!(!ws.task(timed).unwrap().unwrap().task.is_overdue_at(ws.now()));
    clock.advance_to(LocalDateTime::parse("2026-11-21 09:31").unwrap());
    assert!(ws.task(timed).unwrap().unwrap().task.is_overdue_at(ws.now()));
    assert_eq!(ws.task_groups().expect("groups").overdue.len(), 2);
}

#[test]
fn carryover_moves_past_active_tasks_and_never_touches_due() {
    let clock = at("2026-11-20 08:00");
    let mut ws = Workspace::open_in_memory_with_clock(clock).expect("workspace");
    let carry = ws
        .create_task(&NewTask {
            title: "昨天的事".to_owned(),
            scheduled_date: Some(date("2026-11-18")),
            scheduled_time: Some(time("07:30")),
            due_date: Some(date("2026-11-25")),
            due_time: Some(time("12:00")),
            ..Default::default()
        })
        .expect("carry")
        .task
        .id;
    let done_yesterday = ws
        .create_task(&NewTask {
            title: "昨天已完成".to_owned(),
            scheduled_date: Some(date("2026-11-19")),
            ..Default::default()
        })
        .expect("done")
        .task
        .id;
    ws.update_task(done_yesterday, &TaskPatch { status: Some(TaskStatus::Done), ..Default::default() }).unwrap();
    let cancelled_past = ws
        .create_task(&NewTask {
            title: "已取消".to_owned(),
            scheduled_date: Some(date("2026-11-17")),
            ..Default::default()
        })
        .expect("cancelled")
        .task
        .id;
    ws.update_task(cancelled_past, &TaskPatch { status: Some(TaskStatus::Cancelled), ..Default::default() }).unwrap();
    let today = ws
        .create_task(&NewTask { title: "今天的事".to_owned(), scheduled_date: Some(date("2026-11-20")), ..Default::default() })
        .expect("today")
        .task
        .id;

    assert_eq!(ws.carryover_count().expect("count"), 1);
    assert_eq!(ws.carryover().expect("carryover"), 1);

    let carried = ws.task(carry).expect("load").expect("present").task;
    assert_eq!(carried.scheduled_date, Some(date("2026-11-20")), "rescheduled onto today");
    assert_eq!(carried.scheduled_time, None, "stale time-of-day cleared");
    assert_eq!(carried.due_date, Some(date("2026-11-25")), "due date untouched");
    assert_eq!(carried.due_time, Some(time("12:00")), "due time untouched");
    let untouched = ws.task(today).expect("load").expect("present").task;
    assert_eq!(untouched.scheduled_date, Some(date("2026-11-20")));
}

#[test]
fn inbox_predicate_is_strict_and_groups_are_mutually_exclusive() {
    let clock = at("2026-11-20 09:00");
    let mut ws = Workspace::open_in_memory_with_clock(clock).expect("workspace");

    let inbox = ws.create_task(&NewTask { title: "收件".to_owned(), ..Default::default() }).expect("inbox").task.id;
    let project = ws.create_project("项目", "#547093", "📌", None, None).expect("project");
    let bound = ws
        .create_task(&NewTask { title: "项目任务".to_owned(), project_id: Some(project.id), ..Default::default() })
        .expect("bound")
        .task
        .id;
    let due_only = ws
        .create_task(&NewTask { title: "只有截止".to_owned(), due_date: Some(date("2026-11-30")), ..Default::default() })
        .expect("due only")
        .task
        .id;
    let upcoming = ws
        .create_task(&NewTask { title: "明天".to_owned(), scheduled_date: Some(date("2026-11-21")), ..Default::default() })
        .expect("upcoming")
        .task
        .id;
    let today = ws
        .create_task(&NewTask { title: "今天".to_owned(), scheduled_date: Some(date("2026-11-20")), ..Default::default() })
        .expect("today")
        .task
        .id;
    let overdue = ws
        .create_task(&NewTask {
            title: "逾期".to_owned(),
            scheduled_date: Some(date("2026-11-20")),
            due_date: Some(date("2026-11-19")),
            ..Default::default()
        })
        .expect("overdue")
        .task
        .id;
    let finished = ws
        .create_task(&NewTask { title: "完成".to_owned(), scheduled_date: Some(date("2026-11-20")), ..Default::default() })
        .expect("finished")
        .task
        .id;
    ws.update_task(finished, &TaskPatch { status: Some(TaskStatus::Done), ..Default::default() }).unwrap();

    let inbox_ids = ws.inbox_tasks().expect("inbox").into_iter().map(|a| a.task.id).collect::<Vec<_>>();
    assert_eq!(inbox_ids, vec![inbox], "inbox holds exactly the one strict match");

    let groups = ws.task_groups().expect("groups");
    let ids = |list: &[nsh_app_kanon::domain::TaskAggregate]| list.iter().map(|a| a.task.id).collect::<Vec<_>>();
    assert_eq!(ids(&groups.overdue), vec![overdue], "overdue wins over today");
    assert_eq!(ids(&groups.today), vec![today], "a task never appears twice");
    assert_eq!(ids(&groups.upcoming), vec![upcoming]);
    assert_eq!(ids(&groups.inbox), vec![inbox]);
    assert_eq!(ids(&groups.unscheduled), vec![bound, due_only], "project-bound and due-only fall to unscheduled");
    assert_eq!(ids(&groups.done), vec![finished]);

    let all = [
        ids(&groups.overdue),
        ids(&groups.today),
        ids(&groups.upcoming),
        ids(&groups.inbox),
        ids(&groups.unscheduled),
        ids(&groups.done),
        ids(&groups.cancelled),
    ]
    .concat();
    let mut unique = all.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(all.len(), unique.len(), "strict mutual exclusion across every group");
}

#[test]
fn inbox_triage_removes_task_and_updates_the_inbox_count() {
    let clock = at("2026-11-20 09:00");
    let mut ws = Workspace::open_in_memory_with_clock(clock).expect("workspace");
    let by_project = ws.create_task(&NewTask { title: "分项目".to_owned(), ..Default::default() }).expect("a").task.id;
    let by_schedule = ws.create_task(&NewTask { title: "排期".to_owned(), ..Default::default() }).expect("b").task.id;
    let by_due = ws.create_task(&NewTask { title: "截止".to_owned(), ..Default::default() }).expect("c").task.id;
    assert_eq!(ws.inbox_count().expect("count"), 3);
    assert_eq!(ws.inbox_tasks().expect("inbox").len(), 3);

    let project = ws.create_project("项目", "#547093", "📌", None, None).expect("project");
    ws.update_task(by_project, &TaskPatch { project_id: Some(Some(project.id)), ..Default::default() }).unwrap();
    assert_eq!(ws.inbox_count().expect("count"), 2, "assigning a Project leaves the Inbox at once");
    assert!(ws.inbox_tasks().expect("inbox").iter().all(|a| a.task.id != by_project));

    ws.update_task(by_schedule, &TaskPatch { scheduled_date: Some(Some(date("2026-11-20"))), ..Default::default() }).unwrap();
    assert_eq!(ws.inbox_count().expect("count"), 1, "assigning a Scheduled Date leaves the Inbox at once");

    ws.update_task(by_due, &TaskPatch { due_date: Some(Some(date("2026-11-25"))), ..Default::default() }).unwrap();
    assert_eq!(ws.inbox_count().expect("count"), 0, "assigning a Due Date leaves the Inbox at once");
    assert!(ws.inbox_tasks().expect("inbox").is_empty());

    // Completion and cancellation also drop a task from the strict Inbox.
    let finishing = ws.create_task(&NewTask { title: "将完成".to_owned(), ..Default::default() }).expect("d").task.id;
    assert_eq!(ws.inbox_count().expect("count"), 1);
    ws.update_task(finishing, &TaskPatch { status: Some(TaskStatus::Done), ..Default::default() }).unwrap();
    assert_eq!(ws.inbox_count().expect("count"), 0, "done tasks are not Inbox-shaped");
}

#[test]
fn carryover_preserves_due_across_local_clock_transitions() {
    let clock = at("2026-11-20 08:00");
    let mut ws = Workspace::open_in_memory_with_clock(clock.clone()).expect("workspace");
    let carry = ws
        .create_task(&NewTask {
            title: "连续结转".to_owned(),
            scheduled_date: Some(date("2026-11-19")),
            scheduled_time: Some(time("07:30")),
            due_date: Some(date("2026-11-25")),
            due_time: Some(time("12:00")),
            ..Default::default()
        })
        .expect("carry")
        .task
        .id;
    let today = ws
        .create_task(&NewTask { title: "当天".to_owned(), scheduled_date: Some(date("2026-11-20")), ..Default::default() })
        .expect("today")
        .task
        .id;

    assert_eq!(ws.carryover().expect("first carryover"), 1);
    let task = ws.task(carry).expect("load").expect("present").task;
    assert_eq!(task.scheduled_date, Some(date("2026-11-20")));
    assert_eq!(task.scheduled_time, None, "stale scheduled time-of-day cleared");
    assert_eq!(task.due_date, Some(date("2026-11-25")), "due date untouched");
    assert_eq!(task.due_time, Some(time("12:00")), "due time untouched");

    // Midnight passes: everything scheduled before the new day moves again,
    // and the Due Date/Time still stay exactly as promised.
    clock.advance_to(LocalDateTime::parse("2026-11-21 00:00").unwrap());
    assert_eq!(ws.carryover_count().expect("count"), 2);
    assert_eq!(ws.carryover().expect("second carryover"), 2);
    let task = ws.task(carry).expect("load").expect("present").task;
    assert_eq!(task.scheduled_date, Some(date("2026-11-21")));
    assert_eq!(task.scheduled_time, None, "scheduled time-of-day stays cleared");
    assert_eq!(task.due_date, Some(date("2026-11-25")), "due date survives the clock transition");
    assert_eq!(task.due_time, Some(time("12:00")), "due time survives the clock transition");
    assert_eq!(ws.task(today).expect("load").expect("present").task.scheduled_date, Some(date("2026-11-21")));
}

#[test]
fn upcoming_lists_future_scheduled_tasks_in_date_order() {
    let clock = at("2026-11-20 09:00");
    let mut ws = Workspace::open_in_memory_with_clock(clock).expect("workspace");
    let base = |title: &str, scheduled: &str| NewTask {
        title: title.to_owned(),
        scheduled_date: Some(date(scheduled)),
        ..Default::default()
    };
    ws.create_task(&base("过去", "2026-11-19")).expect("past");
    ws.create_task(&base("今天", "2026-11-20")).expect("today");
    let third = ws.create_task(&base("第三天", "2026-11-23")).expect("d3").task.id;
    let early = ws
        .create_task(&NewTask { title: "当天早".to_owned(), scheduled_date: Some(date("2026-11-21")), scheduled_time: Some(time("09:00")), ..Default::default() })
        .expect("early")
        .task
        .id;
    let late = ws
        .create_task(&NewTask { title: "当天晚".to_owned(), scheduled_date: Some(date("2026-11-21")), scheduled_time: Some(time("18:00")), ..Default::default() })
        .expect("late")
        .task
        .id;
    let cancelled = ws.create_task(&base("已取消", "2026-11-22")).expect("cancelled").task.id;
    ws.update_task(cancelled, &TaskPatch { status: Some(TaskStatus::Cancelled), ..Default::default() }).unwrap();
    let trashed = ws.create_task(&base("已删", "2026-11-22")).expect("trashed").task.id;
    ws.move_task_to_trash(trashed).unwrap();

    let titles = ws
        .upcoming_tasks()
        .expect("upcoming")
        .into_iter()
        .map(|a| a.task.title)
        .collect::<Vec<_>>();
    assert_eq!(
        titles,
        vec!["当天早".to_owned(), "当天晚".to_owned(), "第三天".to_owned()],
        "future Scheduled-Date tasks only, ordered by date then time-of-day"
    );
    let ids = ws.upcoming_tasks().expect("upcoming").into_iter().map(|a| a.task.id).collect::<Vec<_>>();
    assert_eq!(ids, vec![early, late, third]);
}

#[test]
fn task_filter_and_sort_narrow_and_reorder_groups() {
    let clock = at("2026-11-20 09:00");
    let mut ws = Workspace::open_in_memory_with_clock(clock).expect("workspace");
    // Creation order is deliberately the reverse of the priority order.
    let low = ws
        .create_task(&NewTask { title: "低优".to_owned(), priority: Some(Priority::Low), ..Default::default() })
        .expect("low")
        .task
        .id;
    let high = ws
        .create_task(&NewTask { title: "高优".to_owned(), priority: Some(Priority::High), ..Default::default() })
        .expect("high")
        .task
        .id;
    // Due-only tasks (no Project, no Scheduled Date) land in 未排期; their
    // creation order is the reverse of their Due Date order.
    let late_due = ws
        .create_task(&NewTask {
            title: "晚截止".to_owned(),
            priority: Some(Priority::High),
            due_date: Some(date("2026-11-28")),
            ..Default::default()
        })
        .expect("late due")
        .task
        .id;
    let early_due = ws
        .create_task(&NewTask {
            title: "早截止".to_owned(),
            priority: Some(Priority::Low),
            due_date: Some(date("2026-11-22")),
            ..Default::default()
        })
        .expect("early due")
        .task
        .id;
    let project = ws.create_project("项目", "#547093", "📌", None, None).expect("project");
    let bound = ws
        .create_task(&NewTask { title: "项内".to_owned(), project_id: Some(project.id), ..Default::default() })
        .expect("bound")
        .task
        .id;
    let finished = ws.create_task(&NewTask { title: "做完".to_owned(), ..Default::default() }).expect("done").task.id;
    ws.update_task(finished, &TaskPatch { status: Some(TaskStatus::Done), ..Default::default() }).unwrap();

    let groups = ws.task_groups().expect("groups");
    let titles = |list: &[nsh_app_kanon::domain::TaskAggregate]| {
        list.iter().map(|a| a.task.title.clone()).collect::<Vec<_>>()
    };

    // Manual order is the creation order.
    assert_eq!(titles(&groups.inbox), vec!["低优".to_owned(), "高优".to_owned()]);

    // Priority sort flips the inbox; Due Date sort is independent of it.
    assert_eq!(titles(&groups.sorted(TaskSort::Priority).inbox), vec!["高优".to_owned(), "低优".to_owned()]);
    assert_eq!(
        titles(&groups.sorted(TaskSort::DueDate).unscheduled),
        vec!["早截止".to_owned(), "晚截止".to_owned(), "项内".to_owned()],
        "due-date order, rows without a Due Date last"
    );
    assert_eq!(
        titles(&groups.sorted(TaskSort::Priority).unscheduled),
        vec!["晚截止".to_owned(), "早截止".to_owned(), "项内".to_owned()],
        "priority order, rows without a priority last"
    );

    // Filters narrow every group; mutual exclusion is preserved.
    let only_done = groups.filtered(&TaskFilter { status: Some(TaskStatus::Done), ..Default::default() });
    assert_eq!(only_done.total(), 1);
    assert_eq!(titles(&only_done.done), vec!["做完".to_owned()]);

    let only_high = groups.filtered(&TaskFilter { priority: Some(Priority::High), ..Default::default() });
    assert_eq!(titles(&only_high.inbox), vec!["高优".to_owned()]);

    let in_project = groups.filtered(&TaskFilter {
        project: ProjectFilter::Project(project.id),
        ..Default::default()
    });
    assert_eq!(in_project.total(), 1);
    assert_eq!(titles(&in_project.unscheduled), vec!["项内".to_owned()]);

    let unassigned = groups.filtered(&TaskFilter { project: ProjectFilter::Unassigned, ..Default::default() });
    assert_eq!(unassigned.total(), 5, "everything except the project-bound task");
    assert!(!titles(&unassigned.unscheduled).is_empty(), "due-only rows have no Project either");

    // Filter and sort compose.
    let sorted = groups
        .filtered(&TaskFilter { project: ProjectFilter::Unassigned, ..Default::default() })
        .sorted(TaskSort::Priority);
    assert_eq!(titles(&sorted.inbox), vec!["高优".to_owned(), "低优".to_owned()]);
    let _ = (low, high, late_due, early_due, bound);
}

#[test]
fn task_patch_sets_and_clears_independent_dates() {
    let (mut ws, _path, _clock) = workspace_at("patch", "2026-11-20 09:00");
    let id = ws.create_task(&NewTask { title: "日程".to_owned(), ..Default::default() }).expect("create").task.id;

    ws.update_task(
        id,
        &TaskPatch {
            scheduled_date: Some(Some(date("2026-11-22"))),
            scheduled_time: Some(Some(time("09:00"))),
            due_date: Some(Some(date("2026-11-23"))),
            due_time: Some(Some(time("17:00"))),
            ..Default::default()
        },
    )
    .expect("set all four");
    let task = ws.task(id).unwrap().unwrap().task;
    assert_eq!(task.scheduled_date, Some(date("2026-11-22")));
    assert_eq!(task.scheduled_time, Some(time("09:00")));
    assert_eq!(task.due_date, Some(date("2026-11-23")));
    assert_eq!(task.due_time, Some(time("17:00")));

    // Clearing the scheduled date must leave the due date alone.
    ws.update_task(id, &TaskPatch { scheduled_date: Some(None), scheduled_time: Some(None), ..Default::default() })
        .expect("clear scheduled");
    let task = ws.task(id).unwrap().unwrap().task;
    assert_eq!(task.scheduled_date, None);
    assert_eq!(task.scheduled_time, None);
    assert_eq!(task.due_date, Some(date("2026-11-23")), "due date independent");
    assert_eq!(task.due_time, Some(time("17:00")), "due time independent");
}
