//! Domain seam tests: every business rule and the full SQLite schema,
//! exercised through the single public `Workspace` facade with an
//! injected database path and a controlled local clock.

use std::path::PathBuf;
use std::sync::Arc;

use nsh_app_kanon::domain::{
    FixedClock, KnowledgeBasePatch, LocalDate, LocalDateTime, LocalTime, NewTask, Priority,
    TaskPatch, TaskStatus, Workspace, WorkspaceError,
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
