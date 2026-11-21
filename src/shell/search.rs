//! Local navigation and commands in the workbench's single search surface.

use nsh_base::CommandItem;
use nsh_core::prelude::*;

use crate::ui::chrome::search_dialog;

use super::{Store, create::Create, quick_notes::note_preview, tabs::Route};

#[derive(Clone, PartialEq, Eq)]
enum Target {
    Route(Route),
    Task(i64),
    Create(Create),
    TogglePin,
    ReopenTab,
    NewTab,
    NewProject,
}

pub(super) fn command_palette(
    store: Store,
    open: Signal<bool>,
    create: Signal<Option<Create>>,
) -> impl IntoElement {
    let query = signal(SharedString::default());
    let results = computed(move || {
        let value = query.get();
        let query = value.trim();
        let needle = query.trim_start_matches('>').trim().to_lowercase();
        let commands_only = query.starts_with('>');
        let matches = |name: &str| name.to_lowercase().contains(&needle);
        let mut results = Vec::new();
        if !commands_only {
            for route in [
                Route::Start,
                Route::Today,
                Route::Upcoming,
                Route::Tasks,
                Route::Calendar,
                Route::QuickNotes,
                Route::Favorites,
                Route::Tags,
                Route::KnowledgeBases,
                Route::Trash,
                Route::Settings,
            ] {
                let title = route.title();
                if matches(&title) {
                    results.push((Target::Route(route), title, "工作区"));
                }
            }
            if !needle.is_empty() {
                results.extend(
                    store
                        .search_documents(&needle, 12)
                        .into_iter()
                        .map(|(id, title)| (Target::Route(Route::Document(id)), title, "文档")),
                );
                results.extend(
                    store
                        .knowledge_bases()
                        .into_iter()
                        .filter(|book| matches(&book.name))
                        .take(8)
                        .map(|book| (Target::Route(Route::Book(book.id)), book.name, "知识库")),
                );
                results.extend(
                    store
                        .projects()
                        .into_iter()
                        .filter(|project| matches(&project.name))
                        .take(8)
                        .map(|project| {
                            (
                                Target::Route(Route::Project(project.id)),
                                project.name,
                                "项目",
                            )
                        }),
                );
                results.extend(
                    store
                        .live_tasks()
                        .into_iter()
                        .filter(|task| {
                            matches(&task.task.title)
                                || matches(&task.task.memo)
                                || task.tags.iter().any(|tag| matches(&tag.name))
                        })
                        .take(8)
                        .map(|task| (Target::Task(task.task.id), task.task.title, "任务")),
                );
                results.extend(
                    store
                        .quick_notes()
                        .into_iter()
                        .filter(|note| {
                            matches(&note.content) || note.tags.iter().any(|tag| matches(&tag.name))
                        })
                        .take(8)
                        .map(|note| {
                            (
                                Target::Route(Route::QuickNote(note.id)),
                                note_preview(&note.content, 70),
                                "小记",
                            )
                        }),
                );
            }
        }
        let pinned = store.strip.with(|strip| strip.tabs[strip.active].pinned);
        for (command, title) in [
            (Target::NewTab, "新建标签页"),
            (Target::NewProject, "新建项目"),
            (Target::Create(Create::Task), "新建任务"),
            (Target::Create(Create::Book), "新建知识库"),
            (Target::Create(Create::Document(None)), "新建文档"),
            (Target::Create(Create::Templates), "打开模板中心"),
            (Target::Route(Route::QuickNotes), "新建小记"),
            (
                Target::TogglePin,
                if pinned {
                    "取消固定标签页"
                } else {
                    "固定标签页"
                },
            ),
            (Target::ReopenTab, "恢复关闭的标签页"),
        ] {
            if matches(title) {
                results.push((command, title.to_owned(), "命令"));
            }
        }
        results
    });
    search_dialog(
        move || {
            results.with(|results| {
                results
                    .iter()
                    .enumerate()
                    .map(|(i, (_, title, group))| {
                        CommandItem::new(i.to_string(), title.clone()).group(*group)
                    })
                    .collect()
            })
        },
        move |value| query.set(value.clone()),
        move |id| {
            let Ok(index) = id.parse::<usize>() else {
                return;
            };
            let Some(target) = results
                .with_untracked(|results| results.get(index).map(|(target, ..)| target.clone()))
            else {
                return;
            };
            open.set(false);
            match target {
                Target::Route(Route::Document(id)) => store.open_document(id),
                Target::Route(Route::Book(id)) => store.open_book(id),
                Target::Route(Route::Project(id)) => store.open_project(id),
                Target::Route(route) => store.open_or_focus(route),
                Target::Task(id) => store.open_task(id),
                Target::NewTab => store.open_new_tab(),
                Target::NewProject => store.open_project_editor(None),
                Target::Create(form) => create.set(Some(form)),
                Target::TogglePin => store.toggle_pin(
                    store
                        .strip
                        .with_untracked(|strip| strip.tabs[strip.active].id),
                ),
                Target::ReopenTab => store.reopen_tab(),
            }
        },
        move || open.set(false),
    )
}
