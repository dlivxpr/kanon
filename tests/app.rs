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
