//! The end-to-end first-slice acceptance journey (issue #9), driven
//! headlessly through the real Rust root:
//!
//! open a Project → enter its linked Knowledge Base Document → edit and
//! promote a Checklist Item to a Task → schedule it for Today → complete
//! it in Today → verify the synchronized completion on the Document's
//! Linked Task Card AND the Project Task List → restart the app against
//! the same SQLite workspace → verify full consistency.
//!
//! Helpers are intentionally local to this file, matching the per-file
//! convention of `tests/app.rs` and `tests/workspace.rs`.

use std::path::PathBuf;
use std::time::Duration;

use nsh_app_kanon::AppProps;
use nsh_core::Checked;
use nsh_engine::{EngineConfig, Points, Root, Size};
use nsh_testing::Headless;

fn temp_db(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "kanon-acceptance-{name}-{}-{}.sqlite3",
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

/// Seeds a workspace file through the domain facade, so the journey
/// starts from a deterministic world. Returns the closure's value.
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

/// Opens the only project's page through the sidebar.
fn open_project(app: &mut Headless) -> Result<(), String> {
    app.wait_for("今天")?;
    app.press("id:sidebar-project")?;
    app.wait_for("id:project-view")?;
    Ok(())
}

/// From the project page, enters its linked Document's reading view.
fn open_linked_document(app: &mut Headless, title: &str) -> Result<(), String> {
    app.press("关联文档")?;
    app.wait_for("id:catalog-tree")?;
    app.press(&format!("id:catalog-tree >> text:{title}"))?;
    app.wait_for("编辑")?;
    Ok(())
}

#[test]
fn project_to_document_to_task_journey_survives_restart() -> Result<(), String> {
    let path = temp_db("first-slice");
    seed(&path, "2026-11-20 09:00", |ws| {
        let book = ws.create_knowledge_base("发布会手册").expect("kb");
        let group = ws.create_catalog_node(book.id, None, "第一章").expect("group");
        ws.create_project("发布会", "#547093", "🚀", Some(book.id), Some(group.id))
            .expect("project");
        let doc = ws
            .create_document(book.id, Some(group.id), "筹备清单")
            .expect("doc");
        ws.save_document(doc.id, "筹备清单", "- [ ] 确认场地")
            .expect("seed body");
    });

    let mut app = start_sized(&path, "2026-11-20 09:00", 1440.0);

    // 1. Open the Project from the sidebar.
    open_project(&mut app)?;
    app.wait_for("📚 发布会手册 / 第一章")?;

    // 2. Enter its linked Knowledge Base Document.
    open_linked_document(&mut app, "筹备清单")?;
    app.wait_for("确认场地")?;

    // 3. Edit and promote the Checklist Item to a Task in this Project.
    app.press("编辑")?;
    app.wait_for("完成")?;
    app.wait_for("转为任务")?;
    app.press("转为任务")?;
    app.wait_for("新任务存入")?;
    app.press("项目 · 发布会")?;
    app.wait_for("打开任务 确认场地")?;
    assert!(
        !circle_checked(&app, "完成 确认场地"),
        "the Linked Task Card starts unchecked"
    );
    app.wait_for("label:收件箱 0 项任务")?;

    // 4. Schedule it for Today through the Task Detail.
    app.press("打开任务 确认场地")?;
    app.wait_for("任务详情")?;
    app.type_text("text:排期日 (YYYY-MM-DD)", "2026-11-20")?;
    app.wait_for("label:排期日")?;
    app.press("切换上下文面板")?;
    app.wait_for_gone("任务速览", Duration::from_secs(2))?;

    // 5. Complete it in Today.
    app.press("id:nav-今天")?;
    app.wait_for("id:date-strip")?;
    app.wait_for("完成 确认场地 >> nth=0")?;
    app.press("完成 确认场地 >> nth=0")?;
    assert!(
        circle_checked(&app, "完成 确认场地 >> nth=0"),
        "Today shows the completed circle"
    );

    // 6. The Document's Linked Task Card shows the same completion.
    open_project(&mut app)?;
    open_linked_document(&mut app, "筹备清单")?;
    assert!(
        circle_checked(&app, "完成 确认场地"),
        "the Document's Linked Task Card shows the synchronized completion"
    );

    // 7. The Project Task List shows the same completion.
    app.press("任务列表")?;
    app.wait_for("已完成")?;
    app.wait_for("完成 确认场地")?;
    assert!(
        circle_checked(&app, "完成 确认场地"),
        "the Project Task List shows the synchronized completion"
    );

    // 8. Restart against the same SQLite workspace: full consistency.
    drop(app);
    let mut app = start_sized(&path, "2026-11-20 09:30", 1440.0);

    // Today still lists it as completed.
    app.wait_for("今天")?;
    app.wait_for("id:date-strip")?;
    assert!(
        circle_checked(&app, "完成 确认场地 >> nth=0"),
        "the completion survived the restart on Today"
    );

    // The Document's Linked Task Card still agrees.
    open_project(&mut app)?;
    open_linked_document(&mut app, "筹备清单")?;
    assert!(
        circle_checked(&app, "完成 确认场地"),
        "the Document's card survived the restart"
    );

    // The Project Task List still agrees.
    app.press("任务列表")?;
    app.wait_for("已完成")?;
    app.wait_for("完成 确认场地")?;
    assert!(
        circle_checked(&app, "完成 确认场地"),
        "the Project Task List survived the restart"
    );

    let _ = std::fs::remove_file(path);
    Ok(())
}
