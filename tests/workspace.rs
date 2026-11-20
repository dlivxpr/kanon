//! Domain seam tests: every business rule and the full SQLite schema,
//! exercised through the single public `Workspace` facade with an
//! injected database path and a controlled local clock.

use std::path::PathBuf;
use std::sync::Arc;

use nsh_app_kanon::domain::{
    FixedClock, LocalDate, LocalDateTime, LocalTime, NewTask, Priority, TaskPatch, TaskStatus,
    Workspace, count_words,
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
fn revisions_snapshot_and_restore_only_document_content_and_reference_positions() {
    let (mut ws, path, clock) = workspace_at("doc-restore", "2026-11-20 09:00");
    let book = ws.create_knowledge_base("手册").expect("kb");
    let doc = ws.create_document(book.id, None, "方案").expect("doc");

    // v1: the body mentions Kanon right away; the mention is indexed.
    ws.update_document_body(doc.id, "方案 v1", "和 Kanon 一起\n\n尾部", None).expect("v1 body");
    let mention = ws
        .add_document_reference(doc.id, "mention", "document", Some(7), "Kanon", 2)
        .expect("mention index");
    let auto = ws.save_document_revision(doc.id, None).expect("auto snapshot");
    assert_eq!(auto.name, None, "no name records an automatic snapshot");
    let v1 = ws.save_document_revision(doc.id, Some("v1")).expect("named version");
    assert_eq!(v1.name.as_deref(), Some("v1"));

    // The body keeps moving after v1; a live Task is referenced too.
    clock.advance_to(LocalDateTime::parse("2026-11-20 10:00").unwrap());
    ws.update_document_body(doc.id, "方案 v2", "开头很长的一段\n\n和 Kanon 一起", None).expect("v2 body");
    ws.save_document_revision(doc.id, None).expect("another auto snapshot");
    let task = ws
        .create_task(&NewTask { title: "被引用的任务".to_owned(), ..Default::default() })
        .expect("task")
        .task;
    let card = ws
        .add_document_reference(doc.id, "task_card", "task", Some(task.id), "被引用的任务", 14)
        .expect("card index");

    // Restoring v1 puts back exactly title/body and re-anchors the
    // references' positions against the restored text; nothing else moves.
    let restored = ws.restore_document_revision(v1.id).expect("restore v1");
    assert_eq!(restored.title, "方案 v1");
    assert_eq!(restored.body, "和 Kanon 一起\n\n尾部");
    assert_eq!(restored.word_count, count_words("和 Kanon 一起\n\n尾部"));

    let refs = ws.document_references(doc.id).expect("refs");
    let mention = refs.iter().find(|r| r.id == mention.id).expect("mention survives");
    assert_eq!(mention.position, 2, "Kanon starts at UTF-16 offset 2 in the restored body");
    let card = refs.iter().find(|r| r.id == card.id).expect("card survives");
    assert_eq!(card.position, 14, "labels absent from the restored body keep their position");

    // The Task itself is never mutated by a document restore.
    let untouched = ws.task(task.id).expect("load").expect("present").task;
    assert_eq!(untouched.title, "被引用的任务");
    assert_eq!(untouched.status, TaskStatus::Todo);
    drop(ws);

    // The restore is durable across a restart.
    let ws = Workspace::open(&path, at("2026-11-20 10:30")).expect("reopen");
    assert_eq!(ws.document(doc.id).expect("load").expect("present").body, "和 Kanon 一起\n\n尾部");
    let _ = std::fs::remove_file(path);
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
