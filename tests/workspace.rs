//! Domain seam tests: every business rule and the full SQLite schema,
//! exercised through the single public `Workspace` facade with an
//! injected database path and a controlled local clock.

use std::path::PathBuf;
use std::sync::Arc;

use nsh_app_kanon::domain::{
    FixedClock, KnowledgeBasePatch, LocalDate, LocalDateTime, LocalTime, NewTask, Priority,
    ProjectBinding, ProjectFilter, ProjectPatch, TaskFilter, TaskPatch, TaskSort, TaskStatus,
    Workspace, WorkspaceError, count_words,
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

#[test]
fn document_body_update_persists_word_count_and_survives_reopen() {
    let (mut ws, path, _clock) = workspace_at("doc-body", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("kb");
    let doc = ws.create_document(book.id, None, "入门").expect("doc");

    let saved = ws
        .update_document_body(doc.id, "入门", "# 你好 world\n\n- [ ] 清单项", None)
        .expect("save body");
    assert_eq!(saved.word_count, 6, "two CJK heading words + world + three CJK item words");
    assert_ne!(saved.updated_at, "", "every commit stamps updated_at");

    // A checklist rewrite is body-only persistence: it never invents Tasks
    // (ADR-0002), so the workspace stays task-free.
    let rewritten = ws
        .update_document_body(doc.id, "入门", "# 你好 world\n\n- [x] 清单项", None)
        .expect("rewrite body");
    assert_eq!(rewritten.body, "# 你好 world\n\n- [x] 清单项");
    assert_eq!(ws.task_groups().expect("groups").total_active(), 0, "no global Task appears");
    drop(ws);

    let ws = Workspace::open(&path, at("2026-11-20 09:30")).expect("reopen");
    let reopened = ws.document(doc.id).expect("load").expect("present");
    assert_eq!(reopened.body, "# 你好 world\n\n- [x] 清单项", "the body survives a restart");
    assert_eq!(reopened.word_count, 6);
    let _ = std::fs::remove_file(path);
}

#[test]
fn optimistic_body_update_rejects_stale_base_and_retry_commits() {
    let (mut ws, _path, _clock) = workspace_at("doc-stale", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("kb");
    let doc = ws.create_document(book.id, None, "入门").expect("doc");
    let base = ws.document(doc.id).expect("load").expect("present").version;

    // A concurrent writer (another tab, another process) commits first.
    ws.update_document_body(doc.id, "入门", "别人的修改", None).expect("concurrent write");

    // The editor's auto-save still speaks from the stale base: rejected,
    // so the draft is never silently lost over someone else's commit.
    let stale = ws.update_document_body(doc.id, "入门", "我的草稿", Some(base));
    assert!(matches!(stale, Err(nsh_app_kanon::domain::WorkspaceError::StaleDocument)), "stale base refuses");

    // Retry against the fresh base commits the same draft.
    let fresh = ws.document(doc.id).expect("load").expect("present").version;
    let committed = ws
        .update_document_body(doc.id, "入门", "我的草稿", Some(fresh))
        .expect("retry commits");
    assert_eq!(committed.body, "我的草稿");
    assert_eq!(committed.title, "入门");
    assert!(committed.version > fresh, "a successful commit bumps the version");
}

#[test]
fn revisions_snapshot_and_restore_only_document_content_and_rebuilt_references() {
    let (mut ws, path, clock) = workspace_at("doc-restore", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("kb");
    let doc = ws.create_document(book.id, None, "方案").expect("doc");

    // A live Task the v1 body links with a Linked Task Card.
    let task = ws
        .create_task(&NewTask { title: "被引用的任务".to_owned(), ..Default::default() })
        .expect("task")
        .task;

    // v1: the body holds a real mention marker and a real card marker;
    // both are indexed projections of the body.
    let v1_body = format!(
        "和 {} 一起\n\n{} 尾部",
        nsh_app_kanon::domain::mention_marker("document", "7", "Kanon"),
        nsh_app_kanon::domain::task_card_marker(task.id)
    );
    ws.update_document_body_and_references(doc.id, "方案 v1", &v1_body, None).expect("v1 body");
    let auto = ws.save_document_revision(doc.id, None).expect("auto snapshot");
    assert_eq!(auto.name, None, "no name records an automatic snapshot");
    let v1 = ws.save_document_revision(doc.id, Some("v1")).expect("named version");
    assert_eq!(v1.name.as_deref(), Some("v1"));
    let expected_mention_pos = nsh_app_kanon::domain::scan_mention_markers(&v1_body)[0].position;
    let expected_card_pos = nsh_app_kanon::domain::scan_task_card_markers(&v1_body)[0].1;

    // The body keeps moving after v1; both markers disappear from the
    // live body (the index reconciles them away).
    clock.advance_to(LocalDateTime::parse("2026-11-20 10:00").unwrap());
    ws.update_document_body_and_references(doc.id, "方案 v2", "开头很长的一段\n\n没有引用了", None)
        .expect("v2 body");
    ws.save_document_revision(doc.id, None).expect("another auto snapshot");
    assert!(
        ws.document_references(doc.id).expect("refs").is_empty(),
        "editing the markers away reconciled the index"
    );

    // Restoring v1 puts back exactly title/body and REBUILDS the
    // reference index from the restored body's markers — rollback
    // recreates the cards and mentions with their anchor positions.
    let restored = ws.restore_document_revision(v1.id).expect("restore v1");
    assert_eq!(restored.title, "方案 v1");
    assert_eq!(restored.body, v1_body);
    assert_eq!(restored.word_count, count_words(&v1_body));

    let refs = ws.document_references(doc.id).expect("refs");
    assert_eq!(refs.len(), 2, "the restored body's markers are indexed again");
    let mention = refs.iter().find(|r| r.ref_type == "mention").expect("mention re-indexed");
    assert_eq!(mention.target_kind, "document");
    assert_eq!(mention.target_id, Some(7));
    assert_eq!(mention.target_label, "Kanon");
    assert_eq!(mention.position, expected_mention_pos as i64, "anchored where the marker sits");
    let card = refs.iter().find(|r| r.ref_type == "task_card").expect("card re-indexed");
    assert_eq!(card.target_id, Some(task.id));
    assert_eq!(card.target_label, "被引用的任务", "card labels denormalize the Task's title");
    assert_eq!(card.position, expected_card_pos as i64);

    // The Task itself is never mutated by a document restore.
    let untouched = ws.task(task.id).expect("load").expect("present").task;
    assert_eq!(untouched.title, "被引用的任务");
    assert_eq!(untouched.status, TaskStatus::Todo);
    drop(ws);

    // The restore is durable across a restart.
    let ws = Workspace::open(&path, at("2026-11-20 10:30")).expect("reopen");
    assert_eq!(ws.document(doc.id).expect("load").expect("present").body, v1_body);
    assert_eq!(ws.document_references(doc.id).expect("refs").len(), 2);
    let _ = std::fs::remove_file(path);
}

#[test]
fn create_document_rejects_an_unknown_knowledge_base() {
    let (mut ws, _path, _clock) = workspace_at("orphan-doc", "2026-11-20 09:00");
    let err = ws.create_document(999, None, "孤儿").expect_err("no book 999");
    assert!(matches!(err, WorkspaceError::NotFound(_)), "unknown book: {err}");
    // The enforced foreign key is the second line of defense.
    let err = ws.create_catalog_node(999, None, "孤儿分组").expect_err("no book 999");
    assert!(matches!(err, WorkspaceError::NotFound(_)), "unknown book: {err}");
    let book = ws.create_knowledge_base("手册").expect("kb");
    let doc = ws.create_document(book.id, None, "正常").expect("doc in a real book");
    assert_eq!(doc.book_id, book.id);
}

#[test]
fn binding_a_new_book_creates_project_and_book_atomically() {
    let (mut ws, path, _clock) = workspace_at("new-book-binding", "2026-11-20 09:00");
    let project = ws
        .create_project_with_binding("Kanon 手册", "#547093", "📕", ProjectBinding::NewBook)
        .expect("project + same-named book in one commit");
    let target = ws.project_document_target(project.id).expect("target").expect("bound");
    let book = ws.knowledge_base(target.book_id).expect("book").expect("present");
    assert_eq!(book.name, "Kanon 手册", "the book is same-named");

    drop(ws);
    let ws = Workspace::open(&path, at("2026-11-20 10:00")).expect("reopen");
    let project = ws.project(project.id).expect("load").expect("present");
    assert_eq!(project.linked_book_id, Some(target.book_id), "the binding persisted");
    assert_eq!(ws.knowledge_bases().expect("books").len(), 1);
    let _ = std::fs::remove_file(path);
}

#[test]
fn mention_markers_round_trip_labels_with_special_characters() {
    use nsh_app_kanon::domain::{mention_marker, parse_mention_marker, scan_mention_markers};
    // The writer sanitizes exactly what the parser rejects: `)`, `{`,
    // `}` and `@` in a label survive the archive round trip.
    let marker = mention_marker("document", "7", "a)b{c}d@e");
    let (parsed, len) = parse_mention_marker(&marker).expect("the sanitized marker parses");
    assert_eq!(len, marker.len(), "the whole marker is consumed");
    assert_eq!(parsed.kind, "document");
    assert_eq!(parsed.target_id, Some(7));
    assert_eq!(parsed.label, "a）b｛c｝d＠e");

    let body = format!("前文{marker}后文");
    let found = scan_mention_markers(&body);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].position, 2, "counted in UTF-16 code units");
    assert_eq!(found[0].label, "a）b｛c｝d＠e");
}

#[test]
fn revision_listing_orders_newest_first_and_separates_named_from_auto() {
    let (mut ws, _path, clock) = workspace_at("doc-history", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("kb");
    let doc = ws.create_document(book.id, None, "日志").expect("doc");

    ws.save_document_revision(doc.id, None).expect("auto 1");
    clock.advance_to(LocalDateTime::parse("2026-11-20 09:10").unwrap());
    ws.save_document_revision(doc.id, Some("评审版")).expect("named");
    clock.advance_to(LocalDateTime::parse("2026-11-20 09:20").unwrap());
    ws.save_document_revision(doc.id, None).expect("auto 2");

    let revisions = ws.document_revisions(doc.id).expect("list");
    assert_eq!(revisions.len(), 3);
    assert_eq!(revisions[0].name, None, "newest first: the second auto snapshot");
    assert_eq!(revisions[1].name.as_deref(), Some("评审版"));
    assert_eq!(revisions[2].name, None);
    assert!(revisions.windows(2).all(|w| w[0].created_at >= w[1].created_at), "ordered for the history panel");
}

#[test]
fn version2_migration_adds_document_version_to_existing_databases() {
    let path = temp_db("migrate-v2");
    // A version-1 database, as shipped by the previous milestone: a
    // documents table without the optimistic-locking column.
    {
        let conn = rusqlite::Connection::open(&path).expect("raw v1 db");
        conn.execute_batch(
            "CREATE TABLE knowledge_bases (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, description TEXT NOT NULL DEFAULT '', icon TEXT NOT NULL DEFAULT '📕', color TEXT NOT NULL DEFAULT '#547093', favorited INTEGER NOT NULL DEFAULT 0, sort_order INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, trashed_at TEXT);
             CREATE TABLE documents (id INTEGER PRIMARY KEY AUTOINCREMENT, book_id INTEGER NOT NULL, title TEXT NOT NULL, body TEXT NOT NULL DEFAULT '', word_count INTEGER NOT NULL DEFAULT 0, favorited INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, trashed_at TEXT);
             INSERT INTO knowledge_bases (name, created_at, updated_at) VALUES ('旧库', '2026-11-20 09:00', '2026-11-20 09:00');
             INSERT INTO documents (book_id, title, created_at, updated_at) VALUES (1, '旧文档', '2026-11-20 09:00', '2026-11-20 09:00');
             PRAGMA user_version = 1;",
        )
        .expect("v1 schema with data");
    }
    // The current facade opens it, migrates in place, and keeps the data.
    let mut ws = Workspace::open(&path, at("2026-11-20 10:00")).expect("open migrates");
    let doc = ws.document(1).expect("load").expect("present");
    assert_eq!(doc.title, "旧文档");
    assert_eq!(doc.version, 0, "existing rows start at version 0");
    let saved = ws.update_document_body(1, "旧文档", "新内容", Some(doc.version)).expect("optimistic write");
    assert_eq!(saved.version, 1);
    assert_eq!(ws.document(1).expect("reload").expect("present").body, "新内容");
    let _ = std::fs::remove_file(path);
}

#[test]
fn knowledge_base_shelf_favorites_first_and_edits_fields() {
    let (mut ws, _path, _clock) = workspace_at("kb-shelf", "2026-11-20 09:00");
    let first = ws.create_knowledge_base("产品手册").expect("create first");
    let second = ws.create_knowledge_base("研究笔记").expect("create second");

    // Default shelf order follows creation; unfavorite state is off.
    let names = |ws: &Workspace| {
        ws.knowledge_bases().expect("list").into_iter().map(|kb| kb.name).collect::<Vec<_>>()
    };
    assert_eq!(names(&ws), vec!["产品手册".to_owned(), "研究笔记".to_owned()]);

    ws.set_knowledge_base_favorite(second.id, true).expect("favorite");
    assert_eq!(names(&ws), vec!["研究笔记".to_owned(), "产品手册".to_owned()], "favorites lead the shelf");

    let edited = ws
        .update_knowledge_base(
            first.id,
            &KnowledgeBasePatch {
                name: Some("产品手册 v2".to_owned()),
                description: Some("对外发布的版本".to_owned()),
                icon: Some("📗".to_owned()),
                color: Some("#00b96b".to_owned()),
            },
        )
        .expect("patch");
    assert_eq!(edited.name, "产品手册 v2");
    assert_eq!(edited.description, "对外发布的版本");
    assert_eq!(edited.icon, "📗");
    assert_eq!(edited.color, "#00b96b");
}

#[test]
fn catalog_creates_at_the_top_of_its_level_and_nests_children() {
    let (mut ws, _path, _clock) = workspace_at("kb-top-insert", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("目录规则").expect("book");

    let a = ws.create_document(book.id, None, "甲").expect("doc 甲");
    let b = ws.create_document(book.id, None, "乙").expect("doc 乙");
    let g = ws.create_catalog_node(book.id, None, "分组一").expect("group");
    let titles = |ws: &Workspace| {
        ws.catalog_outline(book.id)
            .expect("outline")
            .into_iter()
            .map(|entry| (entry.node.title, entry.depth))
            .collect::<Vec<_>>()
    };
    // Every creation lands at the TOP of its level: newest first.
    assert_eq!(
        titles(&ws),
        vec![
            ("分组一".to_owned(), 0),
            ("乙".to_owned(), 0),
            ("甲".to_owned(), 0),
        ]
    );

    // Child creation nests under the selected node, also at the top.
    ws.create_document(book.id, Some(g.id), "子一").expect("child 1");
    ws.create_document(book.id, Some(g.id), "子二").expect("child 2");
    let _ = (a, b);
    assert_eq!(
        titles(&ws),
        vec![
            ("分组一".to_owned(), 0),
            ("子二".to_owned(), 1),
            ("子一".to_owned(), 1),
            ("乙".to_owned(), 0),
            ("甲".to_owned(), 0),
        ],
        "children nest under their parent at the top of its level"
    );
}

#[test]
fn catalog_move_reparents_reorders_and_rejects_cycles() {
    let (mut ws, _path, _clock) = workspace_at("kb-move", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("移动规则").expect("book");
    let group = ws.create_catalog_node(book.id, None, "分组").expect("group");
    let a_doc = ws.create_document(book.id, Some(group.id), "甲").expect("甲");
    let b_doc = ws.create_document(book.id, Some(group.id), "乙").expect("乙");
    let node_of = |ws: &Workspace, doc_id: i64| {
        ws.catalog_node_of_document(doc_id).expect("lookup").expect("node").id
    };
    let a = node_of(&ws, a_doc.id);
    let b = node_of(&ws, b_doc.id);

    // Reorder within the level: 甲 after 乙.
    ws.move_catalog_node(a, Some(group.id), 1).expect("reorder");
    let level_titles = |ws: &Workspace| {
        ws.catalog_outline(book.id)
            .expect("outline")
            .into_iter()
            .filter(|entry| entry.depth == 1)
            .map(|entry| entry.node.title)
            .collect::<Vec<_>>()
    };
    assert_eq!(level_titles(&ws), vec!["乙".to_owned(), "甲".to_owned()]);

    // Reparent to the root at the top.
    ws.move_catalog_node(b, None, 0).expect("reparent to root");
    assert_eq!(level_titles(&ws), vec!["甲".to_owned()]);
    let roots = ws.catalog_outline(book.id).expect("outline");
    assert_eq!(roots.first().expect("top").node.title, "乙", "moved node lands at the top of the root level");

    // Cycles: a node cannot move into itself or any descendant.
    let err = ws.move_catalog_node(group.id, Some(group.id), 0).expect_err("into itself");
    assert!(matches!(err, WorkspaceError::InvalidMove(_)), "self move rejected: {err}");
    let err = ws.move_catalog_node(group.id, Some(a), 0).expect_err("into a descendant");
    assert!(matches!(err, WorkspaceError::InvalidMove(_)), "descendant move rejected: {err}");
    // And the tree is untouched after both rejections.
    assert_eq!(ws.catalog_outline(book.id).expect("outline").len(), 3);
}

#[test]
fn catalog_trash_and_restore_moves_the_whole_subtree() {
    let (mut ws, path, _clock) = workspace_at("kb-trash", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("回收规则").expect("book");
    let group = ws.create_catalog_node(book.id, None, "分组").expect("group");
    let doc = ws.create_document(book.id, Some(group.id), "子文档").expect("doc");
    let lone = ws.create_document(book.id, None, "独立文档").expect("lone");

    // Trash the group: the descendant goes with it, the sibling stays.
    ws.trash_catalog_node(group.id).expect("trash group");
    let remaining = ws
        .catalog_outline(book.id)
        .expect("outline")
        .into_iter()
        .map(|entry| entry.node.title)
        .collect::<Vec<_>>();
    assert_eq!(remaining, vec!["独立文档".to_owned()]);
    assert_eq!(ws.book_stats(book.id).expect("stats").documents, 1, "trashed document leaves the live counts");

    // Restore brings the whole subtree back (sibling order is the
    // top-insertion order the tree had before trashing).
    ws.restore_catalog_node(group.id).expect("restore group");
    let restored = ws
        .catalog_outline(book.id)
        .expect("outline")
        .into_iter()
        .map(|entry| (entry.node.title, entry.depth))
        .collect::<Vec<_>>();
    assert_eq!(
        restored,
        vec![("独立文档".to_owned(), 0), ("分组".to_owned(), 0), ("子文档".to_owned(), 1)]
    );
    assert_eq!(ws.book_stats(book.id).expect("stats").documents, 2);
    drop(ws);

    // The Trash flags survive a restart (restore entry point after reopen).
    let mut ws = Workspace::open(&path, at("2026-11-20 10:00")).expect("reopen");
    ws.trash_catalog_node(lone.id).expect("trash after reopen");
    assert_eq!(ws.catalog_outline(book.id).expect("outline").len(), 2);
    let _ = doc;
}

#[test]
fn knowledge_base_trash_restore_and_permanent_delete_unbinds_projects() {
    let (mut ws, path, _clock) = workspace_at("kb-book-trash", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("整本回收").expect("book");
    let group = ws.create_catalog_node(book.id, None, "分组").expect("group");
    ws.create_document(book.id, Some(group.id), "文档").expect("doc");
    let bound = ws.create_project("关联项目", "#547093", "📌", Some(book.id), Some(group.id)).expect("project");

    // Trash the book: whole subtree leaves the shelf and the live tree.
    ws.trash_knowledge_base(book.id).expect("trash book");
    assert!(ws.knowledge_bases().expect("shelf").is_empty(), "trashed book leaves the shelf");
    assert!(ws.catalog_outline(book.id).expect("outline").is_empty());
    assert!(ws.knowledge_base(book.id).expect("row").is_some(), "the row itself remains for restore");

    // The binding survives trashing (ADR-0001: restore brings it back).
    let project = ws.projects().expect("projects").into_iter().find(|p| p.id == bound.id).expect("project");
    assert_eq!(project.linked_book_id, Some(book.id));

    // Restore: everything returns.
    ws.restore_knowledge_base(book.id).expect("restore book");
    assert_eq!(ws.knowledge_bases().expect("shelf").len(), 1);
    assert_eq!(ws.catalog_outline(book.id).expect("outline").len(), 2);

    // Permanent delete: rows are gone and the project falls back to unbound.
    ws.delete_knowledge_base_permanent(book.id).expect("delete");
    assert!(ws.knowledge_base(book.id).expect("gone").is_none());
    assert!(ws.catalog_outline(book.id).expect("empty").is_empty());
    let project = ws.projects().expect("projects").into_iter().find(|p| p.id == bound.id).expect("project");
    assert_eq!(project.linked_book_id, None, "permanent delete unbinds the project");
    assert_eq!(project.linked_group_node_id, None);
    drop(ws);

    // Deletion persists across restart.
    let ws = Workspace::open(&path, at("2026-11-20 10:00")).expect("reopen");
    assert!(ws.knowledge_bases().expect("shelf").is_empty());
}

#[test]
fn catalog_expand_state_persists_and_first_level_defaults_expanded() {
    let (mut ws, path, _clock) = workspace_at("kb-expand", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("展开规则").expect("book");
    let group = ws.create_catalog_node(book.id, None, "分组").expect("group");
    let doc = ws.create_document(book.id, Some(group.id), "子文档").expect("doc");

    // First-open default: the first level starts expanded, deeper collapsed.
    let doc_node = ws.catalog_node_of_document(doc.id).expect("lookup").expect("node").id;
    let outline = ws.catalog_outline(book.id).expect("outline");
    assert!(outline.iter().find(|e| e.node.id == group.id).expect("group").node.expanded);
    assert!(!outline.iter().find(|e| e.node.id == doc_node).expect("doc").node.expanded);

    // Toggling persists per node.
    ws.set_catalog_node_expanded(doc_node, true).expect("expand doc");
    ws.set_catalog_node_expanded(group.id, false).expect("collapse group");
    drop(ws);

    let ws = Workspace::open(&path, at("2026-11-20 10:00")).expect("reopen");
    let outline = ws.catalog_outline(book.id).expect("outline");
    assert!(!outline.iter().find(|e| e.node.id == group.id).expect("group").node.expanded, "collapse persisted");
    assert!(outline.iter().find(|e| e.node.id == doc_node).expect("doc").node.expanded, "expand persisted");
}

#[test]
fn book_stats_count_live_documents_and_words() {
    let (mut ws, _path, _clock) = workspace_at("kb-stats", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("统计").expect("book");
    let empty = ws.book_stats(book.id).expect("stats");
    assert_eq!(empty, nsh_app_kanon::domain::BookStats { documents: 0, words: 0 });

    let doc = ws.create_document(book.id, None, "正文").expect("doc");
    ws.save_document(doc.id, "正文", "hello world 你好").expect("save");
    let stats = ws.book_stats(book.id).expect("stats");
    assert_eq!(stats.documents, 1);
    assert_eq!(stats.words, 4, "two latin words + two CJK characters");
}

// ---------------------------------------------------------------------
// Issue #7: Checklist-to-Task promotion, Linked Task Cards, `@` mentions
// ---------------------------------------------------------------------

#[test]
fn promote_checklist_item_creates_task_replaces_body_and_indexes_reference_atomically() {
    let (mut ws, path, _clock) = workspace_at("promote", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("kb");
    let doc = ws.create_document(book.id, None, "周报").expect("doc");
    ws.update_document_body(doc.id, "周报", "- [ ] 写周报\n\n尾部段落", None).expect("seed body");

    // The editor replaced the item with a placeholder card and serialized.
    let editor_body = "@task(0)\n\n尾部段落".to_owned();
    let project = ws.create_project("Kanon", "#ff4f4f", "🚀", None, None).expect("project");
    let base = ws.document(doc.id).expect("load").expect("present").version;
    let promotion = ws
        .promote_checklist_item(doc.id, Some(base), &editor_body, "写周报", Some(project.id))
        .expect("promotion commits");

    // All three landed: the Task (bound to its Project)…
    let task = ws.task(promotion.task_id).expect("load").expect("present").task;
    assert_eq!(task.title, "写周报");
    assert_eq!(task.status, TaskStatus::Todo);
    assert_eq!(task.project_id, Some(project.id));

    // …the Document body with the real card marker (version bumped once)…
    let committed = ws.document(doc.id).expect("load").expect("present");
    assert_eq!(promotion.document, committed);
    assert_eq!(committed.body, format!("@task({})\n\n尾部段落", promotion.task_id));
    assert_eq!(committed.version, base + 1, "the promotion advances the lock once");
    assert_eq!(committed.word_count, count_words(&committed.body));

    // …and the reference index row, anchored at the marker.
    let reference = &promotion.reference;
    assert_eq!(reference.ref_type, "task_card");
    assert_eq!(reference.target_kind, "task");
    assert_eq!(reference.target_id, Some(promotion.task_id));
    assert_eq!(reference.target_label, "写周报");
    assert_eq!(reference.position, 0);
    assert_eq!(ws.document_references(doc.id).expect("refs"), vec![reference.clone()]);

    // Inbox promotion (no Project) is the other target choice; the
    // editor body still carries the first document's card.
    let editor_body = format!("{}\n\n开头\n\n@task(0)", nsh_app_kanon::domain::task_card_marker(promotion.task_id));
    let second = ws
        .promote_checklist_item(doc.id, Some(committed.version), &editor_body, "回邮件", None)
        .expect("inbox promotion");
    let inbox_task = ws.task(second.task_id).expect("load").expect("present").task;
    assert!(inbox_task.is_inbox_shape(), "no Project and no dates lands in the Inbox");

    // The whole promotion survives a restart.
    drop(ws);
    let ws = Workspace::open(&path, at("2026-11-20 09:30")).expect("reopen");
    let committed = ws.document(doc.id).expect("load").expect("present");
    assert_eq!(
        committed.body,
        format!("{}\n\n开头\n\n@task({})", nsh_app_kanon::domain::task_card_marker(promotion.task_id), second.task_id)
    );
    assert_eq!(ws.document_references(doc.id).expect("refs").len(), 2);
    let _ = std::fs::remove_file(path);
}

// ---------------------------------------------------------------------
// Projects and Milestones (issue #8, ADR-0001)
// ---------------------------------------------------------------------

#[test]
fn project_trash_keeps_task_assignment_and_restore_is_lossless() {
    let (mut ws, path, _clock) = workspace_at("proj-trash", "2026-11-20 09:00");
    let project = ws.create_project("网站改版", "#547093", "📌", None, None).expect("project");
    let task = ws
        .create_task(&NewTask { title: "改首页".to_owned(), project_id: Some(project.id), ..Default::default() })
        .expect("task");

    // Trash: the Project leaves the list but its Tasks stay assigned.
    ws.trash_project(project.id).expect("trash");
    assert!(ws.projects().expect("active").is_empty(), "trashed project leaves the active list");
    assert_eq!(ws.trashed_projects().expect("trashed").len(), 1, "the row remains for restore");
    let kept = ws.task(task.task.id).expect("task").expect("present");
    assert_eq!(kept.task.project_id, Some(project.id), "trashing keeps the task assignment (ADR-0001)");
    assert!(ws.task_groups().expect("groups").unscheduled.iter().any(|a| a.task.id == task.task.id),
        "the task stays visible in the regular task views");

    // Restore is lossless: the task never lost its assignment.
    ws.restore_project(project.id).expect("restore");
    assert_eq!(ws.projects().expect("active").len(), 1);
    let restored = ws.task(task.task.id).expect("task").expect("present");
    assert_eq!(restored.task.project_id, Some(project.id));
    drop(ws);

    let ws = Workspace::open(&path, at("2026-11-20 10:00")).expect("reopen");
    assert_eq!(ws.projects().expect("active").len(), 1, "the project persisted across restart");
    let _ = std::fs::remove_file(path);
}

#[test]
fn promotion_rolls_back_everything_on_stale_base_or_missing_placeholder() {
    let (mut ws, _path, _clock) = workspace_at("promote-rollback", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("kb");
    let doc = ws.create_document(book.id, None, "周报").expect("doc");
    ws.update_document_body(doc.id, "周报", "- [ ] 写周报", None).expect("seed body");

    // A stale optimistic base: the version moved on (another writer
    // committed first). Nothing — not even the Task — may survive.
    ws.update_document_body(doc.id, "周报", "- [ ] 写周报（改）", None).expect("concurrent write");
    let editor_body = "@task(0)".to_owned();
    let err = ws
        .promote_checklist_item(doc.id, Some(0), &editor_body, "写周报", None)
        .expect_err("stale base refuses");
    assert!(matches!(err, WorkspaceError::StaleDocument));
    assert_eq!(ws.task(1).expect("query"), None, "the rolled-back Task left no row");
    assert_eq!(
        ws.document(doc.id).expect("load").expect("present").body,
        "- [ ] 写周报（改）",
        "the Document is untouched"
    );
    assert!(ws.document_references(doc.id).expect("refs").is_empty());

    // A body without the placeholder is rejected before anything writes.
    let err = ws
        .promote_checklist_item(doc.id, Some(2), "没有占位符", "写周报", None)
        .expect_err("missing placeholder refuses");
    assert!(matches!(err, WorkspaceError::InvalidPromotion(_)));
    assert!(ws.inbox_tasks().expect("inbox").is_empty(), "no Task was created");

    // An empty title never promotes either.
    let err = ws
        .promote_checklist_item(doc.id, Some(2), "@task(0)", "  ", None)
        .expect_err("empty title refuses");
    assert!(matches!(err, WorkspaceError::InvalidPromotion(_)));
}

#[test]
fn editor_save_reconciles_card_and_mention_references() {
    let (mut ws, _path, _clock) = workspace_at("reconcile", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("kb");
    let target = ws.create_document(book.id, None, "手册正文").expect("target doc");
    let task = ws
        .create_task(&NewTask { title: "被引用的任务".to_owned(), ..Default::default() })
        .expect("task")
        .task;
    let doc = ws.create_document(book.id, None, "计划").expect("doc");

    // A body with every marker kind saves with a fully-built index.
    let body = format!(
        "{}\n\n前文{}后文\n\n{}",
        nsh_app_kanon::domain::task_card_marker(task.id),
        nsh_app_kanon::domain::mention_marker("document", &target.id.to_string(), "手册正文"),
        nsh_app_kanon::domain::mention_marker("date", "2026-11-21", "2026-11-21"),
    );
    ws.update_document_body_and_references(doc.id, "计划", &body, None).expect("save with markers");
    let refs = ws.document_references(doc.id).expect("refs");
    assert_eq!(refs.len(), 3);
    let card = refs.iter().find(|r| r.ref_type == "task_card").expect("card row");
    assert_eq!(card.target_id, Some(task.id));
    assert_eq!(card.target_label, "被引用的任务", "the label denormalizes the live title");
    assert_eq!(card.position, 0);
    let mention = refs.iter().find(|r| r.ref_type == "mention" && r.target_kind == "document").expect("mention row");
    assert_eq!(mention.target_id, Some(target.id));
    assert_eq!(mention.position, 12, "the 8-unit card marker, two newlines and 前文 precede it");
    let date = refs.iter().find(|r| r.target_kind == "date").expect("date row");
    assert_eq!(date.target_id, None);
    assert_eq!(date.target_label, "2026-11-21");

    // Re-saving the same body is idempotent (no duplicate rows).
    ws.update_document_body_and_references(doc.id, "计划", &body, None).expect("resave");
    assert_eq!(ws.document_references(doc.id).expect("refs").len(), 3);

    // A body whose markers were removed (editor Undo) drops the rows in
    // the same commit — the Tasks and Documents they pointed at survive.
    ws.update_document_body_and_references(doc.id, "计划", "干净的正文", None).expect("save without markers");
    assert!(ws.document_references(doc.id).expect("refs").is_empty());
    assert!(ws.task(task.id).expect("task").is_some());
    assert!(ws.document(target.id).expect("doc").is_some());

    // Moving a marker re-anchors its position, keeping the row.
    let moved = format!("挪走\n\n{}", nsh_app_kanon::domain::task_card_marker(task.id));
    ws.update_document_body_and_references(doc.id, "计划", &moved, None).expect("resave moved");
    let refs = ws.document_references(doc.id).expect("refs");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].position, 4, "挪走 and two newlines precede the marker");
}

#[test]
fn undoing_a_promotion_keeps_the_task_and_its_later_edits() {
    let (mut ws, _path, _clock) = workspace_at("undo-boundary", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("kb");
    let doc = ws.create_document(book.id, None, "周报").expect("doc");
    ws.update_document_body(doc.id, "周报", "- [ ] 写周报", None).expect("seed body");

    let base = ws.document(doc.id).expect("load").expect("present").version;
    let promotion = ws
        .promote_checklist_item(doc.id, Some(base), "@task(0)", "写周报", None)
        .expect("promote");

    // The Task earns edits after the promotion…
    ws.update_task(
        promotion.task_id,
        &TaskPatch { status: Some(TaskStatus::Doing), priority: Some(Priority::High), ..Default::default() },
    )
    .expect("edit the task");

    // …editor Undo then reverts only the Document (card → checklist item)
    // and the reference index: the body no longer holds the marker.
    ws.update_document_body_and_references(doc.id, "周报", "- [ ] 写周报", None).expect("undo save");
    assert!(ws.document_references(doc.id).expect("refs").is_empty(), "the index row went with the undo");

    // The Task and its edits are intact.
    let task = ws.task(promotion.task_id).expect("load").expect("present").task;
    assert_eq!(task.title, "写周报");
    assert_eq!(task.status, TaskStatus::Doing);
    assert_eq!(task.priority, Priority::High);
}

#[test]
fn task_trash_lifecycle_keeps_the_card_reference_and_backlinks_degrade() {
    let (mut ws, _path, _clock) = workspace_at("trash-lifecycle", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("kb");
    let doc = ws.create_document(book.id, None, "周报").expect("doc");
    let promotion = ws
        .promote_checklist_item(doc.id, Some(0), "@task(0)", "写周报", None)
        .expect("promote");

    // Trash: the reference stays (the card renders the removed-task
    // placeholder from it); the Task row is excluded from live queries
    // but still loads.
    ws.move_task_to_trash(promotion.task_id).expect("trash");
    assert_eq!(ws.trashed_tasks().expect("trashed").len(), 1);
    assert_eq!(ws.inbox_tasks().expect("inbox"), vec![]);
    let trashed = ws.task(promotion.task_id).expect("load").expect("present").task;
    assert!(trashed.trashed_at.is_some(), "the row carries its trash stamp");
    assert_eq!(ws.document_references(doc.id).expect("refs").len(), 1, "the index row survives Trash");

    // A mention of this Task from another Document back-links; both the
    // original card and the mention report while the target is trashed.
    let other = ws.create_document(book.id, None, "计划").expect("other doc");
    let body = format!("看 {}", nsh_app_kanon::domain::mention_marker("task", &promotion.task_id.to_string(), "写周报"));
    ws.update_document_body_and_references(other.id, "计划", &body, None).expect("mention save");
    let backlinks = ws.documents_referencing("task", promotion.task_id).expect("backlinks");
    assert_eq!(backlinks.len(), 2, "the card reference and the inline mention both target the Task");
    let from_other = backlinks.iter().find(|b| b.source_document_id == other.id).expect("the mention row");
    assert_eq!(from_other.source_title, "计划");
    assert!(!from_other.source_trashed);

    // Restore: live behavior recovers wholesale.
    ws.restore_task(promotion.task_id).expect("restore");
    assert!(ws.task(promotion.task_id).expect("load").expect("present").task.trashed_at.is_none());
    assert_eq!(ws.inbox_tasks().expect("inbox").len(), 1);

    // Permanent deletion: the Task row is gone, the invalid placeholder
    // stays — both the card reference in the body and its index row.
    ws.move_task_to_trash(promotion.task_id).expect("trash again");
    ws.delete_task_permanent(promotion.task_id).expect("delete");
    assert_eq!(ws.task(promotion.task_id).expect("load"), None);
    let refs = ws.document_references(doc.id).expect("refs");
    assert_eq!(refs.len(), 1, "the card reference is preserved as invalid");
    assert_eq!(refs[0].target_label, "写周报", "its last known label survives");

    // The backlinks of the other Document degrade (source still live,
    // target gone): they still report so the panel can render the state.
    let backlinks = ws.documents_referencing("task", promotion.task_id).expect("backlinks");
    assert_eq!(backlinks.len(), 2);
}

#[test]
fn document_backlinks_track_source_trash_and_recover_on_restore() {
    let (mut ws, _path, _clock) = workspace_at("doc-backlinks", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("kb");
    let source = ws.create_document(book.id, None, "计划").expect("source");
    let target = ws.create_document(book.id, None, "手册正文").expect("target");
    let body = format!("前文{}后文", nsh_app_kanon::domain::mention_marker("document", &target.id.to_string(), "手册正文"));
    ws.update_document_body_and_references(source.id, "计划", &body, None).expect("save");

    let backlinks = ws.documents_referencing("document", target.id).expect("backlinks");
    assert_eq!(backlinks.len(), 1);
    assert_eq!(backlinks[0].source_title, "计划");
    assert!(!backlinks[0].source_trashed);

    // Trashing the source degrades the backlink; it still reports.
    ws.trash_catalog_node(ws.catalog_node_of_document(source.id).expect("node").expect("present").id)
        .expect("trash the source");
    let backlinks = ws.documents_referencing("document", target.id).expect("backlinks");
    assert_eq!(backlinks.len(), 1);
    assert!(backlinks[0].source_trashed, "the panel renders the degraded state");

    // Restoring the source recovers the live backlink.
    let node = ws.catalog_node_of_document(source.id).expect("node").expect("present");
    ws.restore_catalog_node(node.id).expect("restore the source");
    let backlinks = ws.documents_referencing("document", target.id).expect("backlinks");
    assert_eq!(backlinks.len(), 1);
    assert!(!backlinks[0].source_trashed);

    // Permanently deleting the source removes its reference rows with it.
    let book_id = book.id;
    ws.trash_knowledge_base(book_id).expect("trash the book");
    ws.delete_knowledge_base_permanent(book_id).expect("delete the book");
    assert!(ws.documents_referencing("document", target.id).expect("backlinks").is_empty());
}

#[test]
fn project_permanent_delete_detaches_tasks_but_preserves_knowledge_bases() {
    let (mut ws, path, _clock) = workspace_at("proj-delete", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("沉淀").expect("book");
    let doc = ws.create_document(book.id, None, "纪要").expect("doc");
    let project = ws.create_project("短项目", "#547093", "📌", Some(book.id), None).expect("project");
    let milestone = ws.create_milestone(project.id, "收尾", Some(date("2026-11-25"))).expect("milestone");
    let task = ws
        .create_task(&NewTask { title: "收尾工作".to_owned(), project_id: Some(project.id), ..Default::default() })
        .expect("task");
    ws.update_task(task.task.id, &TaskPatch { milestone_id: Some(Some(milestone.id)), ..Default::default() }).expect("assign milestone");

    ws.trash_project(project.id).expect("trash");
    ws.delete_project_permanent(project.id).expect("permanent delete");

    // The task survives, detached; the milestone is gone with the project.
    let after = ws.task(task.task.id).expect("task").expect("present");
    assert_eq!(after.task.project_id, None, "permanent delete detaches tasks (ADR-0001)");
    assert_eq!(after.task.milestone_id, None, "the milestone went away with the project");
    assert!(ws.milestones(project.id).expect("milestones").is_empty());
    assert!(ws.trashed_projects().expect("trashed").is_empty());

    // The Knowledge Base and its Document are untouched.
    assert_eq!(ws.knowledge_bases().expect("shelf").len(), 1);
    assert!(ws.document(doc.id).expect("doc").is_some(), "documents are preserved");

    drop(ws);
    let ws = Workspace::open(&path, at("2026-11-20 10:00")).expect("reopen");
    assert_eq!(ws.task(task.task.id).expect("task").expect("present").task.project_id, None);
    assert_eq!(ws.knowledge_bases().expect("shelf").len(), 1);
}

#[test]
fn milestone_delete_clears_task_references_without_deleting_tasks() {
    let (mut ws, _path, _clock) = workspace_at("milestone-delete", "2026-11-20 09:00");
    let project = ws.create_project("项目", "#547093", "📌", None, None).expect("project");
    let keep = ws.create_milestone(project.id, "保留", None).expect("milestone");
    let gone = ws.create_milestone(project.id, "删除", Some(date("2026-12-01"))).expect("milestone");
    let task = ws
        .create_task(&NewTask { title: "任务".to_owned(), project_id: Some(project.id), ..Default::default() })
        .expect("task");
    ws.update_task(task.task.id, &TaskPatch { milestone_id: Some(Some(gone.id)), ..Default::default() }).expect("assign");

    assert_eq!(ws.milestones(project.id).expect("list").len(), 2);
    ws.delete_milestone(gone.id).expect("delete");
    assert_eq!(ws.milestones(project.id).expect("list").len(), 1);
    assert_eq!(ws.milestones(project.id).expect("list")[0].id, keep.id);
    let after = ws.task(task.task.id).expect("task").expect("present");
    assert_eq!(after.task.milestone_id, None, "the task survives, its milestone reference cleared");
}

#[test]
fn document_created_for_a_project_lands_under_the_linked_grouping_node() {
    let (mut ws, _path, _clock) = workspace_at("proj-doc-placement", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("book");
    let group = ws.create_catalog_node(book.id, None, "第一章").expect("group");
    let bound = ws.create_project("绑定分组", "#547093", "📌", Some(book.id), Some(group.id)).expect("project");
    let whole = ws.create_project("绑定整本", "#547093", "📌", Some(book.id), None).expect("project");
    let free = ws.create_project("未关联", "#547093", "📌", None, None).expect("project");

    // Bound to a grouping node: the new document lands inside it.
    let doc = ws.create_document_for_project(bound.id, "项目文档").expect("doc");
    let node = ws.catalog_node_of_document(doc.id).expect("lookup").expect("node");
    assert_eq!(node.parent_id, Some(group.id), "created under the linked grouping node");
    assert_eq!(node.book_id, book.id);

    // Bound to the whole book: the new document lands at the top of the root.
    let root_doc = ws.create_document_for_project(whole.id, "全书文档").expect("doc");
    let root_node = ws.catalog_node_of_document(root_doc.id).expect("lookup").expect("node");
    assert_eq!(root_node.parent_id, None, "created at the book root");
    let outline = ws.catalog_outline(book.id).expect("outline");
    assert_eq!(outline[0].node.document_id, Some(root_doc.id), "inserted at the TOP of the root");

    // Unbound: creation is refused (the UI offers binding first).
    assert!(ws.create_document_for_project(free.id, "无处安放").is_err());
}

#[test]
fn document_for_project_refuses_a_trashed_grouping_node() {
    let (mut ws, _path, _clock) = workspace_at("proj-doc-trashed", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("book");
    let group = ws.create_catalog_node(book.id, None, "第一章").expect("group");
    let project = ws.create_project("项目", "#547093", "📌", Some(book.id), Some(group.id)).expect("project");

    ws.trash_catalog_node(group.id).expect("trash the linked node");
    // The binding is kept (notice state) but placement must not silently
    // widen to the whole book.
    assert!(ws.create_document_for_project(project.id, "新文档").is_err());
}

#[test]
fn trashing_a_linked_grouping_node_keeps_binding_and_permanent_delete_resets_unbound() {
    let (mut ws, path, _clock) = workspace_at("proj-node-lifecycle", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("book");
    let group = ws.create_catalog_node(book.id, None, "第一章").expect("group");
    let project = ws.create_project("项目", "#547093", "📌", Some(book.id), Some(group.id)).expect("project");

    // Trash the linked node: the project keeps the binding (recoverable notice).
    ws.trash_catalog_node(group.id).expect("trash node");
    let found = ws.project(project.id).expect("project").expect("present");
    assert_eq!(found.linked_group_node_id, Some(group.id), "trashing keeps the binding");
    assert_eq!(found.linked_book_id, Some(book.id));
    assert!(ws.catalog_node_is_trashed(group.id).expect("state"), "the notice reads the node's trash state");

    // Restore brings the binding back to life.
    ws.restore_catalog_node(group.id).expect("restore node");
    assert!(!ws.catalog_node_is_trashed(group.id).expect("state"));
    let target = ws.project_document_target(project.id).expect("target").expect("bound");
    assert_eq!(target.book_id, book.id);
    assert_eq!(target.group_node_id, Some(group.id));

    // Permanent delete: the project resets cleanly to unbound — never
    // widening to the whole Knowledge Base.
    ws.trash_catalog_node(group.id).expect("trash again");
    ws.delete_catalog_node_permanent(group.id).expect("permanent delete");
    let after = ws.project(project.id).expect("project").expect("present");
    assert_eq!(after.linked_group_node_id, None, "reset to unbound");
    assert_eq!(after.linked_book_id, None, "not widened to the whole book");

    drop(ws);
    let ws = Workspace::open(&path, at("2026-11-20 10:00")).expect("reopen");
    let after = ws.project(project.id).expect("project").expect("present");
    assert_eq!(after.linked_book_id, None);
}

#[test]
fn permanent_delete_of_a_catalog_node_removes_its_subtree_and_documents() {
    let (mut ws, path, _clock) = workspace_at("node-permanent", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("book");
    let group = ws.create_catalog_node(book.id, None, "第一章").expect("group");
    let child = ws.create_document(book.id, Some(group.id), "子文档").expect("child");
    let lone = ws.create_document(book.id, None, "独立文档").expect("lone");
    ws.save_document(child.id, "子文档", "内容").expect("body");
    ws.save_document_revision(child.id, None).expect("revision");

    ws.trash_catalog_node(group.id).expect("trash");
    ws.delete_catalog_node_permanent(group.id).expect("permanent delete");

    assert!(ws.catalog_node(group.id).expect("gone").is_none(), "the node row is gone");
    assert!(ws.document(child.id).expect("gone").is_none(), "the subtree's document is gone");
    assert!(ws.document_revisions(child.id).expect("revisions").is_empty());
    let remaining = ws.catalog_outline(book.id).expect("outline");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].node.document_id, Some(lone.id), "siblings survive");

    drop(ws);
    let ws = Workspace::open(&path, at("2026-11-20 10:00")).expect("reopen");
    assert_eq!(ws.catalog_outline(book.id).expect("outline").len(), 1);
}

#[test]
fn project_update_rebinds_and_edits_without_touching_tasks() {
    let (mut ws, _path, _clock) = workspace_at("proj-rebind", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("book");
    let group = ws.create_catalog_node(book.id, None, "第一章").expect("group");
    let project = ws.create_project("项目", "#547093", "📌", None, None).expect("project");
    let task = ws
        .create_task(&NewTask { title: "任务".to_owned(), project_id: Some(project.id), ..Default::default() })
        .expect("task");

    ws.update_project(project.id, &ProjectPatch {
        description: Some("新目标".to_owned()),
        linked_book_id: Some(Some(book.id)),
        linked_group_node_id: Some(Some(group.id)),
        ..Default::default()
    })
    .expect("rebind");
    let found = ws.project(project.id).expect("project").expect("present");
    assert_eq!(found.description, "新目标");
    assert_eq!(found.linked_book_id, Some(book.id));
    assert_eq!(found.linked_group_node_id, Some(group.id));

    // Rebind back to unbound clears both columns.
    ws.update_project(project.id, &ProjectPatch {
        linked_book_id: Some(None),
        linked_group_node_id: Some(None),
        ..Default::default()
    })
    .expect("unbind");
    let found = ws.project(project.id).expect("project").expect("present");
    assert_eq!(found.linked_book_id, None);
    assert_eq!(found.linked_group_node_id, None);

    // Tasks never move on a rebind.
    assert_eq!(ws.task(task.task.id).expect("task").expect("present").task.project_id, Some(project.id));
}

#[test]
fn project_tasks_lists_every_assignment_in_status_order() {
    let (mut ws, _path, _clock) = workspace_at("proj-tasks", "2026-11-20 09:00");
    let project = ws.create_project("项目", "#547093", "📌", None, None).expect("project");
    let other = ws.create_project("别的项目", "#547093", "📌", None, None).expect("other");
    let milestone = ws.create_milestone(project.id, "阶段一", None).expect("milestone");
    let doing = ws
        .create_task(&NewTask { title: "进行中".to_owned(), project_id: Some(project.id), status: Some(TaskStatus::Doing), ..Default::default() })
        .expect("task");
    ws.create_task(&NewTask { title: "待办".to_owned(), project_id: Some(project.id), ..Default::default() }).expect("task");
    ws.create_task(&NewTask { title: "别的".to_owned(), project_id: Some(other.id), ..Default::default() }).expect("task");
    ws.update_task(doing.task.id, &TaskPatch { milestone_id: Some(Some(milestone.id)), ..Default::default() }).expect("assign");

    let tasks = ws.project_tasks(project.id).expect("tasks");
    let titles = tasks.iter().map(|a| a.task.title.as_str()).collect::<Vec<_>>();
    assert_eq!(titles, vec!["待办", "进行中"], "status order, other projects excluded");
    assert_eq!(tasks[1].task.milestone_id, Some(milestone.id));
    assert_eq!(tasks.len(), 2);
}

#[test]
fn create_project_with_binding_resolves_the_node_book_and_creates_a_same_named_book() {
    let (mut ws, _path, _clock) = workspace_at("proj-create-binding", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("book");
    let group = ws.create_catalog_node(book.id, None, "第一章").expect("group");

    // Binding a node stores the node's book alongside.
    let bound = ws
        .create_project_with_binding("节点项目", "#547093", "📌", ProjectBinding::Node(group.id))
        .expect("project");
    assert_eq!(bound.linked_book_id, Some(book.id));
    assert_eq!(bound.linked_group_node_id, Some(group.id));

    // One-click: the same-named Knowledge Base is created with the project.
    let made = ws
        .create_project_with_binding("同名项目", "#547093", "📌", ProjectBinding::NewBook)
        .expect("project");
    let named = ws.knowledge_bases().expect("shelf").into_iter().find(|kb| kb.name == "同名项目");
    assert!(named.is_some(), "the same-named book exists");
    assert_eq!(made.linked_book_id, named.map(|kb| kb.id));
    assert_eq!(ws.knowledge_bases().expect("shelf").len(), 2);

    // Refused bindings: missing book, trashed book, trashed node.
    assert!(ws.create_project_with_binding("坏", "#547093", "📌", ProjectBinding::Book(999)).is_err());
    ws.trash_knowledge_base(book.id).expect("trash book");
    assert!(ws.create_project_with_binding("坏", "#547093", "📌", ProjectBinding::Book(book.id)).is_err());
    assert!(ws.create_project_with_binding("坏", "#547093", "📌", ProjectBinding::Node(group.id)).is_err());
}
