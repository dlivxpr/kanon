//! The Collections/Quick-Notes control seam: the real Rust root driven
//! headlessly through the 开始 / 收藏 / 标签 / 小记 surfaces and the
//! Trash screen, filling the UI-regression gaps the note domain rules
//! need (publish, tags, checklist toggle, search, layout, pin, archive,
//! trash, restore, purge). The document favorite and recycle models are
//! proven by `tests/app.rs` (`recent_browsing_survives_restart_without_
//! listing_unvisited_documents`, `trash_and_restore_a_knowledge_base`)
//! and `tests/workspace.rs`; this file drives only the controls those
//! tests do not reach. Every fixture is an isolated temp workspace.

use std::path::PathBuf;
use std::time::Duration;

use nsh_app_kanon::AppProps;
use nsh_core::Checked;
use nsh_engine::{EngineConfig, Points, Root, Size};
use nsh_testing::Headless;

fn temp_db(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "kanon-collections-{name}-{}-{}.sqlite3",
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

/// Seeds a workspace file through the domain facade, so a UI test starts
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

fn open_quick_notes(app: &mut Headless) -> Result<(), String> {
    app.wait_for("id:nav-小记")?;
    app.press("id:nav-小记")?;
    app.wait_for("id:quick-notes-heading")?;
    Ok(())
}

#[test]
fn publishing_a_quick_note_with_tags_persists_across_restart() -> Result<(), String> {
    let path = temp_db("note-publish");
    let now = "2026-11-20 09:00";
    {
        let mut app = start(&path, now);
        open_quick_notes(&mut app)?;
        // The empty stream says where the first note comes from.
        app.wait_for("还没有小记")?;
        // The composer refuses an empty publish.
        assert!(
            !app.screen().enabled("id:publish-note")?,
            "发布 starts disabled on an empty draft"
        );
        app.type_text("label:小记正文", "想法一")?;
        app.type_text("label:小记标签", "设计, 田野")?;
        assert!(
            app.screen().enabled("id:publish-note")?,
            "发布 enables once the draft has text"
        );
        app.press("id:publish-note")?;
        app.wait_for("想法一")?;
        // The Tag chips render on the card.
        app.wait_for("#设计")?;
        app.wait_for("#田野")?;
    }

    let mut app = start(&path, now);
    open_quick_notes(&mut app)?;
    app.wait_for("想法一")?;
    app.wait_for("#设计")?;
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn checklist_lines_toggle_and_pinned_notes_head_the_stream_across_restart() -> Result<(), String> {
    let path = temp_db("note-check-pin");
    let now = "2026-11-20 09:00";
    seed(&path, now, |ws| {
        ws.publish_quick_note("- [ ] 买菜\n- [x] 打扫", &[])
            .expect("note");
    });

    {
        let mut app = start(&path, now);
        open_quick_notes(&mut app)?;
        assert!(
            !circle_checked(&app, "买菜"),
            "the seeded line starts unchecked"
        );
        assert!(circle_checked(&app, "打扫"), "the [x] line starts checked");
        app.press("买菜")?;
        assert!(circle_checked(&app, "买菜"), "the toggle commits at once");

        // Pin through the card menu.
        app.press("label:/小记操作/")?;
        app.press("role:menuitem[置顶]")?;
        app.wait_for("已置顶")?;
    }

    let mut app = start(&path, now);
    open_quick_notes(&mut app)?;
    assert!(
        circle_checked(&app, "买菜"),
        "the checked line survives a restart"
    );
    app.wait_for("已置顶")?;
    // 取消置顶 through the same menu keeps working after the restart.
    app.press("label:/小记操作/")?;
    app.press("role:menuitem[取消置顶]")?;
    app.wait_for_gone("已置顶", Duration::from_secs(2))?;
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn search_filters_content_and_tag_names_with_honest_empty_states() -> Result<(), String> {
    let path = temp_db("note-search");
    let now = "2026-11-20 09:00";
    seed(&path, now, |ws| {
        ws.publish_quick_note("买牛奶", &[]).expect("note one");
        ws.publish_quick_note("写周报", &["写作".to_owned()])
            .expect("note two");
    });

    let mut app = start(&path, now);
    open_quick_notes(&mut app)?;
    app.wait_for("买牛奶")?;
    app.wait_for("写周报")?;
    // The small-note page opens with no inspector obscuring its results.
    app.wait_for_gone("id:context-pane", Duration::from_secs(2))?;

    // Content match narrows the stream.
    app.type_text("text:搜索小记或标签…", "牛奶")?;
    app.wait_for_gone("写周报", Duration::from_secs(2))?;
    assert!(app.shows("买牛奶"));

    // Tag-name match finds the other card; no-match says so honestly.
    app.clear("text:牛奶")?;
    app.type_text("text:搜索小记或标签…", "写作")?;
    app.wait_for_gone("买牛奶", Duration::from_secs(2))?;
    app.wait_for("#写作")?;
    app.clear("text:写作")?;
    app.type_text("text:搜索小记或标签…", "不存在的词")?;
    app.wait_for("没有匹配的小记")?;
    app.press("label:全局搜索")?;
    app.type_text("label:搜索工作区", "写作")?;
    app.press("text:写周报")?;
    app.wait_for("label:返回全部小记")?;
    app.wait_for("text:写周报")?;
    app.wait_for_gone("label:搜索工作区", Duration::from_secs(2))?;
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn the_stream_layout_switch_persists_in_workspace_preferences() -> Result<(), String> {
    let path = temp_db("note-layout");
    let now = "2026-11-20 09:00";
    {
        let mut app = start(&path, now);
        open_quick_notes(&mut app)?;
        assert!(
            app.screen().selected("label:时间流")?,
            "时间流 is the default layout"
        );
        app.press("label:网格")?;
        assert!(
            app.screen().selected("label:网格")?,
            "网格 takes the selection"
        );
    }

    let mut app = start(&path, now);
    open_quick_notes(&mut app)?;
    assert!(
        app.screen().selected("label:网格")?,
        "the layout preference survives a restart (ADR-0003)"
    );
    app.press("label:时间流")?;
    assert!(app.screen().selected("label:时间流")?);
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn converting_a_note_archives_it_with_readonly_checklist_lines() -> Result<(), String> {
    let path = temp_db("note-convert");
    let now = "2026-11-20 09:00";
    seed(&path, now, |ws| {
        ws.create_knowledge_base("会议资料").expect("book").id
    });

    let mut app = start(&path, now);
    open_quick_notes(&mut app)?;
    app.type_text("label:小记正文", "清单\n- [x] 已完成\n- [ ] 待办")?;
    app.type_text("label:小记标签", "写作")?;
    app.press("id:publish-note")?;
    app.wait_for("清单")?;

    // 转为知识库文档: pick the seeded book in the dialog (scoped to the
    // dialog; the sidebar lists the same book name).
    app.press("label:/小记操作/")?;
    app.press("role:menuitem[转为知识库文档]")?;
    app.wait_for("role:dialog[转为知识库文档]")?;
    app.press("role:dialog[转为知识库文档] >> label:会议资料")?;
    // The conversion opens the new Document's reading view.
    app.wait_for("收藏文档")?;

    // Back on 小记 the active stream is empty; the archive holds the
    // note with its Tags and its checklist frozen as text.
    app.press("id:nav-小记")?;
    app.wait_for("还没有小记")?;
    app.press("已归档")?;
    app.wait_for("清单")?;
    app.wait_for("#写作")?;
    app.wait_for("☑ 已完成")?;
    app.wait_for("☐ 待办")?;
    assert!(
        !app.shows("label:/小记操作/"),
        "an archived note has no action menu"
    );
    assert!(
        !app.screen().matches("label:已完成")?,
        "archived checklist lines are plain text, not circles"
    );
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn a_trashed_note_restores_losslessly_and_purges_permanently() -> Result<(), String> {
    let path = temp_db("note-trash");
    let now = "2026-11-20 09:00";
    seed(&path, now, |ws| {
        ws.publish_quick_note("待删小记", &[]).expect("note");
    });

    let mut app = start(&path, now);
    open_quick_notes(&mut app)?;
    app.wait_for("待删小记")?;
    app.press("label:/小记操作/")?;
    app.press("role:menuitem[移入回收站]")?;
    app.wait_for("还没有小记")?;

    // The Trash screen lists the note; 恢复 puts it back.
    app.press("id:nav-回收站")?;
    app.wait_for("回收站")?;
    app.wait_for("待删小记")?;
    app.press("恢复")?;
    app.wait_for("回收站是空的")?;
    app.press("id:nav-小记")?;
    app.wait_for("待删小记")?;

    // 彻底删除 asks first, then the note is gone for good.
    app.press("label:/小记操作/")?;
    app.press("role:menuitem[移入回收站]")?;
    app.wait_for("还没有小记")?;
    app.press("id:nav-回收站")?;
    app.wait_for("待删小记")?;
    app.press("彻底删除")?;
    app.wait_for("确认删除")?;
    app.press("确认删除")?;
    app.wait_for("回收站是空的")?;
    drop(app);

    let mut app = start(&path, now);
    // Wait for the first render before pressing: a fresh engine starts
    // from an empty accessibility tree.
    app.wait_for("id:nav-回收站")?;
    app.press("id:nav-回收站")?;
    app.wait_for("回收站是空的")?;
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn ctrl_enter_publishes_only_while_the_composer_is_focused() -> Result<(), String> {
    let path = temp_db("note-shortcut");
    let now = "2026-11-20 09:00";
    let mut app = start(&path, now);
    open_quick_notes(&mut app)?;
    app.type_text("label:小记正文", "快捷键草稿")?;

    // Focus moves to the stream search: Ctrl/⌘+Enter here must NOT
    // publish the composer draft (the prototype binds the shortcut to
    // the composer form only).
    app.type_text("text:搜索小记或标签…", "随便")?;
    app.press_key("mod+enter")?;
    assert!(
        !app.shows("快捷键草稿"),
        "Ctrl+Enter outside the composer does not publish the draft"
    );

    // Back inside the composer card (its tag field) the shortcut
    // publishes, exactly like the prototype's #noteForm.
    app.clear("text:随便")?;
    app.type_text("label:小记标签", "快捷键")?;
    app.press_key("mod+enter")?;
    app.wait_for("快捷键草稿")?;
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn the_right_panel_quick_capture_publishes_tagless_notes() -> Result<(), String> {
    let path = temp_db("note-capture");
    let now = "2026-11-20 09:00";
    seed(&path, now, |ws| {
        ws.publish_quick_note("另一条小记\n另一条详情", &[])
            .expect("other note");
    });
    let mut app = start(&path, now);
    open_quick_notes(&mut app)?;

    // The overview panel's capture switch flips to the note box.
    app.press("id:inbox-badge")?;
    app.press("速记小记")?;
    assert!(
        !app.screen().enabled("id:quick-capture-note")?,
        "保存小记 starts disabled on an empty draft"
    );
    app.type_text("label:速记小记内容", "右栏想法")?;
    app.press("id:quick-capture-note")?;
    app.wait_for("右栏想法")?;
    // 最近小记 shows the fragment next to the box.
    app.wait_for("最近小记")?;
    app.press("label:右栏想法")?;
    app.wait_for("label:返回全部小记")?;
    app.wait_for_gone("text:另一条详情", Duration::from_secs(2))?;
    app.press("label:后退")?;
    app.wait_for("text:另一条详情")?;
    app.press("label:前进")?;
    app.wait_for_gone("text:另一条详情", Duration::from_secs(2))?;
    app.press("label:返回全部小记")?;
    app.wait_for("text:另一条详情")?;
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn favorites_bucket_lists_books_and_documents_and_unfavorites() -> Result<(), String> {
    let path = temp_db("favorites-view");
    let now = "2026-11-20 09:00";
    seed(&path, now, |ws| {
        let book = ws.create_knowledge_base("常用书目").expect("book");
        ws.set_knowledge_base_favorite(book.id, true)
            .expect("fav book");
        let document = ws
            .create_document(book.id, None, "收藏的目标")
            .expect("document");
        ws.set_document_favorite(document.id, true)
            .expect("fav document");
        ws.create_knowledge_base("其他书目").expect("other book");
    });

    let mut app = start(&path, now);
    app.wait_for("id:nav-收藏")?;
    app.press("id:nav-收藏")?;
    app.wait_for("id:favorites-heading")?;
    app.wait_for("收藏 常用书目")?;
    app.wait_for("收藏 收藏的目标（常用书目）")?;
    assert!(
        !app.screen().matches("label:收藏 其他书目")?,
        "unfavorited rows never appear in the bucket (the sidebar may still name the book)"
    );

    // The star unfavorites a row; the bucket re-binds without it.
    app.press("label:取消收藏 收藏的目标")?;
    app.wait_for_gone("收藏 收藏的目标（常用书目）", Duration::from_secs(2))?;
    app.press("label:取消收藏 常用书目")?;
    app.wait_for("暂无收藏")?;
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn project_star_persists_and_its_favorite_opens_the_project() -> Result<(), String> {
    let path = temp_db("favorite-project");
    let now = "2026-11-20 09:00";
    seed(&path, now, |ws| {
        let book = ws.create_knowledge_base("收藏的手册").unwrap();
        ws.set_knowledge_base_favorite(book.id, true).unwrap();
        ws.create_project("收藏的项目", "#547093", "", None, None)
            .unwrap();
    });
    {
        let mut app = start(&path, now);
        app.wait_for("id:sidebar-project")?;
        app.press("id:sidebar-project")?;
        app.press("label:收藏项目")?;
        assert!(app.screen().selected("label:收藏项目")?);
    }
    let mut app = start(&path, now);
    app.wait_for("id:nav-收藏")?;
    app.press("id:nav-收藏")?;
    app.wait_for("label:收藏 收藏的项目")?;
    app.wait_for("label:收藏 收藏的手册")?;
    app.press("label:筛选收藏：项目")?;
    app.wait_for_gone("label:收藏 收藏的手册", Duration::from_secs(2))?;
    app.press("label:收藏 收藏的项目")?;
    app.wait_for("id:project-view")?;
    assert!(app.screen().selected("label:收藏项目")?);
    app.press("id:nav-收藏")?;
    app.press("label:取消收藏 收藏的项目")?;
    app.wait_for_gone("label:收藏 收藏的项目", Duration::from_secs(2))?;
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn width_setting_applies_to_document_reading_and_editing() -> Result<(), String> {
    let path = temp_db("document-width");
    let now = "2026-11-20 09:00";
    seed(&path, now, |ws| {
        let book = ws.create_knowledge_base("纸面资料库").unwrap();
        ws.create_document(book.id, None, "宽度验收文档").unwrap();
        ws.set_preference("context_document_panel_open", "false")
            .unwrap();
    });
    let path_text = path.to_string_lossy().into_owned();
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1760.0), Points(900.0)),
        ..EngineConfig::new(Root::new::<nsh_app_kanon::App>(move || AppProps {
            workspace_path: Some(path_text.clone()).into(),
            fixed_now: Some(now.to_owned()).into(),
        }))
    })
    .expect("start document width fixture");
    let paper_width = |app: &Headless| {
        app.screen()
            .layout_nodes()
            .into_iter()
            .find(|node| node.test_id.as_deref() == Some("document-paper"))
            .expect("visible paper")
            .bounds
            .size
            .width
            .0
    };
    app.wait_for("label:全局搜索")?;
    app.press("label:全局搜索")?;
    app.type_text("label:搜索工作区", "宽度验收文档")?;
    app.press("text:宽度验收文档")?;
    app.wait_for("id:document-paper")?;
    assert_eq!(paper_width(&app), 860.0);

    app.press("id:nav-设置")?;
    app.press("label:编辑与文档")?;
    app.press("label:文档页面宽度")?;
    app.press("text:宽屏画布")?;
    app.press("label:后退")?;
    app.wait_for("id:document-paper")?;
    assert_eq!(paper_width(&app), 1120.0);
    app.press("label:编辑")?;
    app.wait_for("label:文档正文")?;
    assert_eq!(paper_width(&app), 1120.0);
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn importing_markdown_adds_a_document_to_the_chosen_book_without_rebinding_ids()
-> Result<(), String> {
    let path = temp_db("import");
    let now = "2026-11-20 09:00";
    seed(&path, now, |ws| {
        ws.create_knowledge_base("别的知识库").unwrap();
        ws.create_knowledge_base("导入目的地").unwrap();
        ws.create_task(&nsh_app_kanon::domain::NewTask {
            title: "不应建立关联的任务".to_owned(),
            ..Default::default()
        })
        .unwrap();
    });
    let input_dir = path.with_extension("fixture");
    std::fs::create_dir(&input_dir).unwrap();
    let input = input_dir.join("访谈纪要.md");
    std::fs::write(&input, "\u{feff}## 新导入的正文\n\n@task(1)\n").unwrap();
    let mut app = start(&path, now);
    app.wait_for("id:nav-设置")?;
    app.on_pick_paths(vec![input])?;
    app.press("id:nav-设置")?;
    app.press("label:数据与备份")?;
    app.press("label:导入到知识库…")?;
    app.press("role:menuitem[导入目的地]")?;
    app.wait_for("label:标签页 访谈纪要")?;
    app.wait_for("text:新导入的正文")?;
    app.wait_for("text:＠task(1)")?;
    assert!(!app.shows("text:不应建立关联的任务"));
    drop(app);
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_dir_all(input_dir);
    Ok(())
}

#[test]
fn tag_cloud_lists_live_rows_across_modules_and_navigates() -> Result<(), String> {
    let path = temp_db("tags-view");
    let now = "2026-11-20 09:00";
    seed(&path, now, |ws| {
        let tag = ws.ensure_tag("设计").expect("tag");
        let task = ws
            .create_task(&nsh_app_kanon::domain::NewTask {
                title: "画原型".to_owned(),
                ..Default::default()
            })
            .expect("task");
        ws.attach_tag(task.task.id, tag.id).expect("attach");
        ws.publish_quick_note("小记灵感", &["设计".to_owned()])
            .expect("note");
        let book = ws.create_knowledge_base("资料库").expect("book");
        let source = ws
            .publish_quick_note("文档素材", &["设计".to_owned()])
            .expect("note to convert");
        ws.convert_quick_note_to_document(source.id, book.id)
            .expect("document carrying the tag");
        ws.ensure_tag("田野").expect("an unused tag");
    });

    let mut app = start(&path, now);
    app.wait_for("id:nav-标签")?;
    app.press("id:nav-标签")?;
    app.wait_for("id:tags-heading")?;
    // Every tag shows a chip, even one with no live rows.
    app.wait_for("label:查看标签 设计")?;
    app.wait_for("label:查看标签 田野")?;

    // Picking 设计 lists its live Task, Document and Quick Note.
    app.press("label:查看标签 设计")?;
    app.wait_for("#设计")?;
    app.wait_for("相关内容")?;
    app.wait_for("画原型")?;
    app.wait_for("小记灵感")?;
    app.wait_for("文档素材")?;
    app.press("label:后退")?;
    app.wait_for_gone("相关内容", Duration::from_secs(2))?;
    app.press("label:前进")?;
    app.wait_for("label:标签页 #设计")?;
    app.wait_for("label:打开 小记灵感")?;
    // A tag whose rows all left shows the honest empty bucket.
    app.press("label:查看标签 田野")?;
    app.wait_for("暂无内容")?;

    // Rows navigate to their modules.
    app.press("label:查看标签 设计")?;
    app.wait_for("画原型")?;
    app.press("label:打开 画原型")?;
    app.wait_for("任务详情")?;
    app.press("id:nav-标签")?;
    app.wait_for("id:tags-heading")?;
    // Pick a different tag, then return to 设计 before following its note.
    app.press("label:查看标签 田野")?;
    app.wait_for("暂无内容")?;
    app.press("label:查看标签 设计")?;
    app.wait_for("小记灵感")?;
    app.press("label:打开 小记灵感")?;
    app.wait_for("id:quick-notes-heading")?;
    app.wait_for("label:返回全部小记")?;
    app.wait_for_gone("text:文档素材", Duration::from_secs(2))?;
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn renaming_to_an_existing_tag_keeps_associations_across_restart() -> Result<(), String> {
    let path = temp_db("tag-merge");
    let now = "2026-11-20 09:00";
    seed(&path, now, |ws| {
        ws.publish_quick_note("共同关联", &["草案".into(), "计划".into()])
            .unwrap();
        let source = ws.publish_quick_note("关联文档", &["草案".into()]).unwrap();
        let book = ws.create_knowledge_base("资料库").unwrap();
        ws.convert_quick_note_to_document(source.id, book.id)
            .unwrap();
    });

    let mut app = start(&path, now);
    app.wait_for("id:nav-标签")?;
    app.press("id:nav-标签")?;
    app.press("label:查看标签 草案")?;
    app.press("label:重命名标签")?;
    app.clear("label:新名称")?;
    assert!(!app.screen().enabled("label:保存标签")?);
    app.type_text("label:新名称", "计划")?;
    app.press("label:保存标签")?;
    app.wait_for_gone("label:查看标签 草案", Duration::from_secs(2))?;
    app.wait_for("label:打开 关联文档")?;
    drop(app);

    let mut app = start(&path, now);
    app.wait_for("id:nav-标签")?;
    app.press("id:nav-标签")?;
    app.press("label:查看标签 计划")?;
    assert!(!app.shows("label:查看标签 草案"));
    app.wait_for("label:打开 关联文档")?;
    app.press("label:打开 共同关联")?;
    app.wait_for("id:quick-notes-heading")?;
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn start_page_launchers_digest_recent_scopes_and_book_filter() -> Result<(), String> {
    let path = temp_db("start-controls");
    let now = "2026-11-20 09:00";
    seed(&path, now, |ws| {
        let manual = ws.create_knowledge_base("手册").expect("book one");
        ws.create_document(manual.id, None, "入门指南")
            .expect("doc a");
        let essays = ws.create_knowledge_base("随笔").expect("book two");
        ws.create_document(essays.id, None, "散步记录")
            .expect("doc b");
        ws.create_task(&nsh_app_kanon::domain::NewTask {
            title: "今日排期但已逾期".to_owned(),
            scheduled_date: nsh_app_kanon::domain::LocalDate::parse("2026-11-20"),
            due_date: nsh_app_kanon::domain::LocalDate::parse("2026-11-19"),
            ..Default::default()
        })
        .unwrap();
    });

    let mut app = start(&path, now);
    app.wait_for("id:nav-开始")?;
    app.press("id:nav-开始")?;
    app.wait_for("id:start-heading")?;
    // The digest card names its counts (one full text) and jumps to Today.
    app.wait_for("今日速览")?;
    app.wait_for("text:1 项待办 · 0 个项目 · 1 项逾期")?;
    // The four launchers open their destinations (scoped to the
    // launcher row: the sidebar also names 新建知识库/新建项目).
    app.press("id:start-launchers >> label:模板中心")?;
    app.wait_for("role:dialog[模板中心]")?;
    app.press("关闭")?;
    app.press("id:start-launchers >> label:新建文档")?;
    app.wait_for("role:dialog[新建文档]")?;
    app.press("关闭")?;
    app.press("id:start-launchers >> label:新建知识库")?;
    app.wait_for("role:dialog[新建知识库]")?;
    app.press("关闭")?;
    app.press("id:start-launchers >> label:新建项目")?;
    app.wait_for("label:标签页 项目")?;

    // The recent table lists both documents under 最近编辑.
    app.press("id:nav-开始")?;
    app.wait_for("最近文档")?;
    app.wait_for("入门指南")?;
    app.wait_for("散步记录")?;

    // The book filter narrows the table and names itself on the trigger.
    app.press("id:recent-book-filter")?;
    app.press("role:menuitem[手册]")?;
    app.wait_for_gone("散步记录", Duration::from_secs(2))?;
    app.wait_for("入门指南")?;
    app.press("id:recent-book-filter")?;
    app.press("role:menuitem[全部知识库]")?;
    app.wait_for("散步记录")?;

    // The 已收藏 scope lists only the starred row (the first row is the
    // newest by id: 散步记录).
    app.press("label:切换文档收藏 >> nth=0")?;
    app.press("已收藏")?;
    app.wait_for("散步记录")?;
    app.wait_for_gone("入门指南", Duration::from_secs(2))?;
    drop(app);
    let _ = std::fs::remove_file(path);
    Ok(())
}
