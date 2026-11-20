//! The Project view (docs/ui-design.md §4.3, issue #8): the header with
//! live progress and the linked Knowledge Base scope, four sub-views
//! (概览与里程碑 / 任务列表 / 看板 / 关联文档), and the Project editor
//! (creation with its ADR-0001 binding choice and rebinding).
//!
//! Every mutation flows through the `Store` into `Workspace`; the
//! sub-view and the open Document of the Linked Documents sub-view are
//! remembered per tab (like the Today view's selected date).

use nsh_base::{menu, menu_item, menu_separator, theme};
use nsh_core::prelude::*;

use crate::domain::{
    LocalDate, Priority, ProjectBinding, ProjectPatch, TaskAggregate, TaskStatus,
};
use crate::kb::{self, CatalogFocus, CatalogNav};
use crate::shell::store::Store;
use crate::shell::tabs::ProjectSection;
use crate::shell::task_row::task_row;
use crate::ui::controls::{
    app_button, app_button_with, empty_state, input_field, panel_icon_button, section_heading,
    submit_field, ButtonKind,
};
use crate::ui::theme as tokens;

/// The inner catalog sidebar's narrow breakpoint (docs/ui-design.md §3.4),
/// mirroring the book shell.
const NARROW_BREAKPOINT: f32 = 960.0;

/// The preset Project colors (user-selected data colors, like the book
/// color).
const COLORS: &[&str] = &["#547093", "#80ad8d", "#c9a168", "#d9977f", "#9a8fb8", "#c17070"];
/// The preset Project emojis.
const EMOJIS: &[&str] = &["📌", "🚀", "🎯", "🛠", "🌱", "📦"];

/// The Project page: header, sub-view switch, and the active sub-view.
pub fn project_view(store: Store, tab_id: u64, project_id: i64) -> impl IntoElement {
    // The sub-view lives on the tab; the computed re-runs on every strip
    // change and the dynamic rebuilds the content only when the section
    // actually changes (docs/ui-design.md §4.3).
    let section = computed(move || store.project_section_of(tab_id));
    view()
        .flex_1()
        .min_w(px(0.0))
        .min_h(px(0.0))
        .flex_col()
        .test_id("project-view")
        .child(when(
            move || store.project(project_id).is_none(),
            move || {
                view().flex_1().items_center().justify_center().child(
                    empty_state("项目不存在", "它可能已被彻底删除；任务与知识库内容不受项目删除影响。"),
                )
            },
        ))
        .child(when(
            move || store.project(project_id).is_some(),
            move || {
                view()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex_col()
                    .child(project_header(store, project_id))
                    .child(sub_view_switch(store, move || section.get()))
                    .child(
                        view()
                            .flex_1()
                            .min_h(px(0.0))
                            .child(dynamic(move || match section.get() {
                                ProjectSection::Overview => overview(store, project_id).into_element(),
                                ProjectSection::Tasks => task_list(store, project_id).into_element(),
                                ProjectSection::Board => board(store, project_id).into_element(),
                                ProjectSection::Documents => {
                                    documents(store, tab_id, project_id).into_element()
                                }
                            })),
                    )
            },
        ))
}

/// The header: emoji, name, description, the live progress bar, the
/// linked scope chip, and the project menu (edit / rebind / trash).
fn project_header(store: Store, project_id: i64) -> impl IntoElement {
    let tasks = move || store.project_tasks(project_id);
    let progress = move || {
        let tasks = tasks();
        let done = tasks.iter().filter(|a| a.task.status == TaskStatus::Done).count();
        (done, tasks.len())
    };
    view()
        .flex_col()
        .gap_2()
        .px_5()
        .pt_4()
        .pb_3()
        .border_b_1()
        .border_color(tokens::border_subtle())
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_3()
                .child(
                    text(move || store.project(project_id).map(|p| p.emoji).unwrap_or_else(|| "📌".into()))
                        .text_size(px(30.0)),
                )
                .child(
                    view()
                        .flex_1()
                        .min_w(px(0.0))
                        .gap_1()
                        .child(
                            text(move || {
                                store.project(project_id).map(|p| p.name).unwrap_or_default()
                            })
                            .text_size(px(22.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(tokens::fg_primary()),
                        )
                        .child(
                            text(move || {
                                store
                                    .project(project_id)
                                    .map(|p| p.description)
                                    .filter(|d| !d.is_empty())
                                    .unwrap_or_else(|| "这个项目还没有简介。".to_owned())
                            })
                            .text_size(px(12.0))
                            .text_color(tokens::fg_muted())
                            .number_of_lines(2),
                        ),
                )
                .child(project_menu(store, project_id)),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_3()
                .child(dynamic(move || {
                    // A `when` branch is built only when its condition flips;
                    // the filled width must re-derive from `progress()` on every
                    // workspace change, so the bar is a dynamic region.
                    let (done, total) = progress();
                    view()
                        .flex_1()
                        .min_w(px(0.0))
                        .h(px(6.0))
                        .flex_row()
                        .overflow_hidden()
                        .rounded_full()
                        .bg(tokens::control_bg())
                        .child(when(
                            move || done > 0,
                            move || {
                                view()
                                    .h_full()
                                    .rounded_full()
                                    .bg(tokens::positive())
                                    .flex_grow(done as f32)
                                    .child(view().flex_grow((total - done) as f32))
                            },
                        ))
                }))
                .child(
                    text(move || {
                        let (done, total) = progress();
                        let percent = (done * 100).checked_div(total).unwrap_or(0);
                        format!("{}%", percent)
                    })
                    .text_size(px(12.0))
                    .text_color(tokens::fg_muted()),
                )
                .child(scope_chip(store, project_id)),
        )
}

/// Where the project's documents live: a chip naming the linked book (or
/// the linked grouping node inside it); unbound projects invite binding.
fn scope_chip(store: Store, project_id: i64) -> impl IntoElement {
    let target = move || store.project_document_target(project_id);
    view().child(when(
        move || target().is_none(),
        move || {
            app_button(ButtonKind::Soft, "未关联知识库", move || {
                store.open_project_editor(Some(project_id));
            })
        },
    )).child(when(
        move || target().is_some(),
        move || {
            let label = move || -> SharedString {
                match target() {
                    Some(target) => {
                        let book = store
                            .knowledge_base(target.book_id)
                            .map(|kb| kb.name)
                            .unwrap_or_else(|| "知识库".to_owned());
                        match target.group_node_id {
                            Some(node_id) => {
                                let node = store
                                    .catalog_node(node_id)
                                    .map(|node| node.title)
                                    .unwrap_or_default();
                                format!("📚 {} / {}", book, node)
                            }
                            None => format!("📚 {}", book),
                        }
                    }
                    None => String::new(),
                }
                .into()
            };
            app_button_with(ButtonKind::Ghost, label, move || {
                if let Some(target) = target() {
                    store.open_book(target.book_id);
                }
            })
        },
    ))
}

fn project_menu(store: Store, project_id: i64) -> impl IntoElement {
    menu()
        .offset(px(4.0))
        .popup_style(|s| {
            s.p_1()
                .rounded(tokens::radius_card())
                .bg(theme().popover)
                .shadow_md()
                .w(px(180.0))
        })
        .trigger(panel_icon_button("⋯", "项目菜单", || {}))
        .child(
            menu_item()
                .on_select(move || store.open_project_editor(Some(project_id)))
                .px_3()
                .py_2()
                .rounded(tokens::radius_row())
                .hover(|s| s.bg(tokens::control_hover()))
                .child(text("编辑与关联").text_size(px(13.0)).text_color(tokens::fg_primary())),
        )
        .child(menu_separator().h(px(1.0)).my_1().bg(tokens::border_subtle()))
        .child(
            menu_item()
                .on_select(move || store.trash_project(project_id))
                .px_3()
                .py_2()
                .rounded(tokens::radius_row())
                .hover(|s| s.bg(tokens::control_hover()))
                .child(text("移入回收站").text_size(px(13.0)).text_color(tokens::danger())),
        )
}

/// The four sub-view pills (docs/ui-design.md §4.3).
fn sub_view_switch(
    store: Store,
    section: impl Fn() -> ProjectSection + Copy + 'static,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_1()
        .px_5()
        .py_2()
        .border_b_1()
        .border_color(tokens::border_subtle())
        .children(
            [
                ProjectSection::Overview,
                ProjectSection::Tasks,
                ProjectSection::Board,
                ProjectSection::Documents,
            ]
            .iter()
            .map(|candidate| {
                let candidate = *candidate;
                let active = move || section() == candidate;
                view()
                    .child(
                        nsh_base::button()
                            .on_press(move || store.set_project_section(candidate))
                            .accessibility_label(candidate.label())
                            .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                            .h(px(28.0))
                            .px_3()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .border_1()
                            .border_color(move || {
                                if active() { tokens::accent() } else { tokens::border_subtle() }
                            })
                            .bg(move || {
                                if active() { tokens::accent_soft() } else { theme().background.opacity(0.0) }
                            })
                            .text_color(move || {
                                if active() { tokens::accent() } else { tokens::fg_body() }
                            })
                            .hover(|s| s.bg(tokens::control_hover()))
                            .child(text(candidate.label()).text_size(px(12.0))),
                    )
                    .accessibility_state(move || {
                        nsh_core::AccessibilityState { selected: active(), ..Default::default() }
                    })
                    .into_element()
            })
            .collect::<Vec<_>>(),
        )
}

// ---------------------------------------------------------------------
// 概览与里程碑
// ---------------------------------------------------------------------

fn overview(store: Store, project_id: i64) -> impl IntoElement {
    view().flex_1().min_h(px(0.0)).child(
        scroll().flex_1().child(
            view()
                .flex_col()
                .gap_4()
                .p_5()
                .max_w(px(860.0))
                .w_full()
                .self_center()
                .child(metrics(store, project_id))
                .child(milestones(store, project_id)),
        ),
    )
}

/// One key metric card of the Overview.
fn metric_card(
    label: &'static str,
    value: impl Fn() -> usize + Copy + 'static,
) -> impl IntoElement {
    view()
        .flex_1()
        .flex_col()
        .gap_1()
        .px_3()
        .py_3()
        .rounded(tokens::radius_card())
        .border_1()
        .border_color(tokens::border_subtle())
        .bg(move || theme().card)
        .child(
            text(move || format!("{}", value()))
                .text_size(px(20.0))
                .font_weight(FontWeight::BOLD)
                .text_color(tokens::fg_primary()),
        )
        .child(text(label).text_size(px(11.0)).text_color(tokens::fg_muted()))
}

/// The key metric cards: 总任务 / 进行中 / 已完成 / 关联文档.
fn metrics(store: Store, project_id: i64) -> impl IntoElement {
    let tasks = move || store.project_tasks(project_id);
    view()
        .flex_row()
        .gap_2()
        .child(metric_card("总任务", move || tasks().len()))
        .child(metric_card("进行中", move || {
            tasks().iter().filter(|a| a.task.status == TaskStatus::Doing).count()
        }))
        .child(metric_card("已完成", move || {
            tasks().iter().filter(|a| a.task.status == TaskStatus::Done).count()
        }))
        .child(metric_card("关联文档", move || {
            store.project_document_count(project_id)
        }))
}

/// The milestone list: one row per milestone with its target date, live
/// progress and the assigned Tasks, plus the inline add row. Deleting a
/// milestone clears the tasks' reference without deleting them
/// (ADR-0001).
fn milestones(store: Store, project_id: i64) -> impl IntoElement {
    let list = move || store.milestones(project_id);
    let tasks = move || store.project_tasks(project_id);
    view()
        .flex_col()
        .gap_2()
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(section_heading("里程碑"))
                .child(
                    text(move || format!("{}", list().len()))
                        .text_size(px(11.0))
                        .text_color(tokens::fg_muted()),
                ),
        )
        .child(when(
            move || list().is_empty(),
            move || empty_state("还没有里程碑", "在下方添加第一个阶段性目标。"),
        ))
        .child(dynamic(move || {
            view().flex_col().gap_1().children(
                list()
                    .into_iter()
                    .map(|milestone| {
                        milestone_row(store, project_id, milestone.id, tasks).into_element()
                    })
                    .collect::<Vec<_>>(),
            )
        }))
        .child(milestone_add_form(store, project_id))
}

fn milestone_row(
    store: Store,
    project_id: i64,
    milestone_id: i64,
    tasks: impl Fn() -> Vec<TaskAggregate> + Copy + 'static,
) -> impl IntoElement {
    let assigned = move || {
        tasks()
            .into_iter()
            .filter(|a| a.task.milestone_id == Some(milestone_id))
            .collect::<Vec<_>>()
    };
    let progress = move || {
        let assigned = assigned();
        let done = assigned
            .iter()
            .filter(|a| a.task.status == TaskStatus::Done)
            .count();
        (done, assigned.len())
    };
    view()
        .flex_col()
        .gap_1()
        .px_3()
        .py_2()
        .rounded(tokens::radius_card())
        .border_1()
        .border_color(tokens::border_subtle())
        .bg(move || theme().card)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(move || {
                        store
                            .milestones(project_id)
                            .into_iter()
                            .find(|m| m.id == milestone_id)
                            .map(|m| m.title)
                            .unwrap_or_default()
                    })
                    .text_size(px(14.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(tokens::fg_primary())
                    .flex_1()
                    .number_of_lines(1),
                )
                .child(
                    text(move || {
                        store
                            .milestones(project_id)
                            .into_iter()
                            .find(|m| m.id == milestone_id)
                            .and_then(|m| m.target_date)
                            .map(|d| d.to_string())
                            .unwrap_or_default()
                    })
                    .text_size(px(11.0))
                    .text_color(tokens::fg_muted()),
                )
                .child(
                    text(move || {
                        let (done, total) = progress();
                        format!("{}/{} 项完成", done, total)
                    })
                    .text_size(px(11.0))
                    .text_color(tokens::fg_muted()),
                )
                .child(
                    nsh_base::button()
                        .on_press(move || store.delete_milestone(milestone_id))
                        .accessibility_label("删除里程碑")
                        .size(px(22.0))
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .text_color(tokens::fg_muted())
                        .hover(|s| s.bg(tokens::control_hover()).text_color(tokens::danger()))
                        .child(text("✕").text_size(px(11.0))),
                ),
        )
        .child(when(
            move || assigned().is_empty(),
            move || {
                text("在任务详情中为任务选择此里程碑。")
                    .text_size(px(11.0))
                    .text_color(tokens::fg_muted())
            },
        ))
        .child(dynamic(move || {
            view().flex_col().gap(px(2.0)).children(
                assigned()
                    .into_iter()
                    .map(|aggregate| task_row(store, aggregate).into_element())
                    .collect::<Vec<_>>(),
            )
        }))
}

fn milestone_add_form(store: Store, project_id: i64) -> impl IntoElement {
    let title = signal(String::new());
    let date = signal(String::new());
    let submit = move || {
        let parsed = LocalDate::parse(&date.get());
        if date.get().trim().is_empty() {
            store.create_milestone(project_id, &title.get(), None);
            title.set(String::new());
        } else if parsed.is_some() {
            store.create_milestone(project_id, &title.get(), parsed);
            title.set(String::new());
            date.set(String::new());
        }
    };
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(submit_field(
            "里程碑名称，回车添加",
            move || title.get(),
            move |text: &str| title.set(text.to_owned()),
            move |_: &str| submit(),
        ))
        .child(input_field(
            "目标日期（可选，YYYY-MM-DD）",
            move || date.get(),
            move |text: &str| date.set(text.to_owned()),
        ))
        .child(
            app_button(ButtonKind::Positive, "添加里程碑", submit)
                .test_id("milestone-add"),
        )
}

// ---------------------------------------------------------------------
// 任务列表
// ---------------------------------------------------------------------

/// The Task List sub-view: inline quick-add plus the tasks grouped by
/// status.
fn task_list(store: Store, project_id: i64) -> impl IntoElement {
    let draft = signal(String::new());
    let tasks = move || store.project_tasks(project_id);
    let group = move |status: TaskStatus| {
        tasks()
            .into_iter()
            .filter(|a| a.task.status == status)
            .collect::<Vec<_>>()
    };
    view()
        .flex_1()
        .min_h(px(0.0))
        .child(
            view()
                .flex_col()
                .gap_2()
                .p_5()
                .max_w(px(860.0))
                .w_full()
                .self_center()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(submit_field(
                            "添加项目任务，回车创建",
                            move || draft.get(),
                            move |text: &str| draft.set(text.to_owned()),
                            move |text: &str| {
                                if store.create_project_task(project_id, text).is_some() {
                                    draft.set(String::new());
                                }
                            },
                        ))
                        .child(
                            app_button(ButtonKind::Positive, "添加任务", move || {
                                if store.create_project_task(project_id, &draft.get()).is_some() {
                                    draft.set(String::new());
                                }
                            })
                            .test_id("project-quick-add"),
                        ),
                )
                .child(when(
                    move || tasks().is_empty(),
                    move || empty_state("这个项目还没有任务", "在上方输入一条任务开始推进项目。"),
                ))
                .child(status_group(store, "待办", move || group(TaskStatus::Todo)))
                .child(status_group(store, "进行中", move || group(TaskStatus::Doing)))
                .child(status_group(store, "已完成", move || group(TaskStatus::Done)))
                .child(status_group(store, "已取消", move || group(TaskStatus::Cancelled))),
        )
}

/// One status group of the Task List sub-view.
fn status_group(
    store: Store,
    title: &'static str,
    items: impl Fn() -> Vec<TaskAggregate> + Copy + 'static,
) -> impl IntoElement {
    view().flex_col().gap_1().child(when(
        move || !items().is_empty(),
        move || {
            view()
                .flex_col()
                .gap_1()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(section_heading(title))
                        .child(
                            text(move || format!("{}", items().len()))
                                .text_size(px(11.0))
                                .text_color(tokens::fg_muted()),
                        ),
                )
                .child(dynamic(move || {
                    view().flex_col().gap(px(2.0)).children(
                        items()
                            .into_iter()
                            .map(|aggregate| task_row(store, aggregate).into_element())
                            .collect::<Vec<_>>(),
                    )
                }))
        },
    ))
}

// ---------------------------------------------------------------------
// 看板
// ---------------------------------------------------------------------

/// The kanban: one lane per status (`todo`/`doing`/`done`, plus `cancel`
/// only while the project holds cancelled tasks — the optional column).
/// Cards move across lanes through their 移动 menu.
fn board(store: Store, project_id: i64) -> impl IntoElement {
    let tasks = move || store.project_tasks(project_id);
    let lane = move |status: TaskStatus| {
        tasks()
            .into_iter()
            .filter(|a| a.task.status == status)
            .collect::<Vec<_>>()
    };
    view().flex_1().min_w(px(0.0)).min_h(px(0.0)).child(
        scroll()
            .flex_1()
            .child(
                view()
                    .flex_row()
                    .items_start()
                    .gap_3()
                    .p_5()
                    .h_full()
                    .child(lane_column(store, "待办", TaskStatus::Todo, move || {
                        lane(TaskStatus::Todo)
                    }))
                    .child(lane_column(store, "进行中", TaskStatus::Doing, move || {
                        lane(TaskStatus::Doing)
                    }))
                    .child(lane_column(store, "已完成", TaskStatus::Done, move || {
                        lane(TaskStatus::Done)
                    }))
                    .child(when(
                        move || !lane(TaskStatus::Cancelled).is_empty(),
                        move || {
                            lane_column(store, "已取消", TaskStatus::Cancelled, move || {
                                lane(TaskStatus::Cancelled)
                            })
                        },
                    )),
            ),
    )
}

fn lane_column(
    store: Store,
    title: &'static str,
    status: TaskStatus,
    items: impl Fn() -> Vec<TaskAggregate> + Copy + 'static,
) -> impl IntoElement {
    view()
        .w(px(248.0))
        .flex_col()
        .gap_2()
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .px_1()
                .child(
                    text(title)
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens::fg_primary()),
                )
                .child(
                    text(move || format!("{}", items().len()))
                        .text_size(px(11.0))
                        .text_color(tokens::fg_muted()),
                ),
        )
        .child(
            view()
                .flex_col()
                .gap_2()
                .min_h(px(80.0))
                .p_2()
                .rounded(tokens::radius_card())
                .bg(tokens::panel_subtle())
                .test_id(&format!("kanban-{}", status.as_str()))
                .child(when(
                    move || items().is_empty(),
                    move || {
                        text("暂无任务")
                            .text_size(px(12.0))
                            .text_color(tokens::fg_muted())
                            .p_2()
                    },
                ))
                .child(dynamic(move || {
                    view().flex_col().gap_2().children(
                        items()
                            .into_iter()
                            .map(|aggregate| {
                                board_card(store, aggregate, status).into_element()
                            })
                            .collect::<Vec<_>>(),
                    )
                })),
        )
}

/// One kanban card: title (opens the Task Detail), the meta line, and
/// the move menu to change its status.
fn board_card(
    store: Store,
    aggregate: TaskAggregate,
    current: TaskStatus,
) -> impl IntoElement {
    let id = aggregate.task.id;
    let title = aggregate.task.title.clone();
    let due = aggregate.task.due_date;
    let (done_subs, total_subs) =
        crate::domain::Task::subtask_progress(&aggregate.subtasks);
    view()
        .flex_col()
        .gap_1()
        .p_2()
        .rounded(tokens::radius_row())
        .border_1()
        .border_color(tokens::border_subtle())
        .bg(move || theme().card)
        .child(
            view()
                .flex_row()
                .items_start()
                .gap_1()
                .child(
                    nsh_base::button()
                        .on_press(move || store.open_task(id))
                        .accessibility_label(Some(format!("打开任务 {}", title).into()))
                        .flex_1()
                        .min_w(px(0.0))
                        .items_start()
                        .child(
                            text(title.clone())
                                .text_size(px(13.0))
                                .text_color(tokens::fg_primary())
                                .number_of_lines(2),
                        ),
                )
                .child(card_move_menu(store, id, current, title)),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .flex_wrap()
                .child(when(
                    move || due.is_some(),
                    move || {
                        text(move || due.map(|d| d.to_string()).unwrap_or_default())
                            .text_size(px(11.0))
                            .text_color(tokens::fg_muted())
                    },
                ))
                .child(when(
                    move || aggregate.task.priority == Priority::High,
                    || text("高优先级").text_size(px(11.0)).text_color(tokens::warning()),
                ))
                .child(when(
                    move || total_subs > 0,
                    move || {
                        text(format!("{}/{}", done_subs, total_subs))
                            .text_size(px(11.0))
                            .text_color(tokens::fg_muted())
                    },
                )),
        )
}

/// The card's move menu: every status but the current one.
fn card_move_menu(
    store: Store,
    task_id: i64,
    current: TaskStatus,
    title: String,
) -> impl IntoElement {
    let targets = [
        TaskStatus::Todo,
        TaskStatus::Doing,
        TaskStatus::Done,
        TaskStatus::Cancelled,
    ]
    .into_iter()
    .filter(|status| *status != current)
    .collect::<Vec<_>>();
    menu()
        .offset(px(4.0))
        .popup_style(|s| {
            s.p_1()
                .rounded(tokens::radius_card())
                .bg(theme().popover)
                .shadow_md()
                .w(px(160.0))
        })
        .trigger(
            nsh_base::button()
                .accessibility_label(Some(format!("移动 {}", title).into()))
                .size(px(26.0))
                .items_center()
                .justify_center()
                .rounded(move || theme().radius)
                .bg(move || theme().background.opacity(0.0))
                .text_color(tokens::fg_muted())
                .hover(|s| s.bg(tokens::control_hover()).text_color(tokens::fg_primary()))
                .active(|s| s.opacity(0.7))
                .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent()))
                .child(text("⋯").text_size(px(13.0))),
        )
        .children(
            targets
                .into_iter()
                .map(|status| {
                    menu_item()
                        .on_select(move || store.set_task_status(task_id, status))
                        .px_3()
                        .py_2()
                        .rounded(tokens::radius_row())
                        .hover(|s| s.bg(tokens::control_hover()))
                        .child(
                            text(format!("移到{}", status.label()))
                                .text_size(px(13.0))
                                .text_color(tokens::fg_primary()),
                        )
                })
                .collect::<Vec<_>>(),
        )
}

// ---------------------------------------------------------------------
// 关联文档
// ---------------------------------------------------------------------

/// The Linked Documents sub-view (ADR-0001). Bound projects reuse the
/// Knowledge Base catalog sidebar plus the document canvas; the unbound
/// and trashed-link states explain themselves with their actions.
fn documents(store: Store, tab_id: u64, project_id: i64) -> impl IntoElement {
    view()
        .flex_1()
        .min_w(px(0.0))
        .min_h(px(0.0))
        .child(when(
            move || store.project_document_target(project_id).is_none(),
            move || unbound_documents(store, project_id),
        ))
        .child(when(
            move || {
                store.project_document_target(project_id).is_some()
                    && store.project_linked_node_trashed(project_id)
            },
            move || trashed_link_notice(store, project_id),
        ))
        .child(when(
            move || {
                store.project_document_target(project_id).is_some()
                    && !store.project_linked_node_trashed(project_id)
            },
            move || linked_documents(store, tab_id, project_id),
        ))
}

/// The unbound state: offer binding an existing book/node or the
/// one-click same-named book (ADR-0001).
fn unbound_documents(store: Store, project_id: i64) -> impl IntoElement {
    let name = move || store.project(project_id).map(|p| p.name).unwrap_or_default();
    view().flex_1().items_center().justify_center().child(
        view()
            .flex_col()
            .gap_3()
            .items_center()
            .max_w(px(420.0))
            .child(
                text("还没有关联知识库")
                    .text_size(px(18.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(tokens::fg_primary()),
            )
            .child(
                text("项目本身不存放文档：把项目关联到一本知识库或其中一个目录分组，这里就会出现项目的文档树。")
                    .text_size(px(12.0))
                    .text_color(tokens::fg_muted()),
            )
            .child(
                view()
                    .flex_row()
                    .gap_2()
                    .child(
                        app_button(ButtonKind::Primary, "绑定已有知识库", move || {
                            store.open_project_editor(Some(project_id));
                        })
                        .test_id("project-bind"),
                    )
                    .child(
                        app_button(ButtonKind::Soft, "一键创建同名知识库", move || {
                            store.bind_project_new_book(project_id);
                        })
                        .test_id("project-bind-new-book"),
                    ),
            )
            .child(
                text(move || format!("将创建与项目同名的知识库「{}」", name()))
                    .text_size(px(11.0))
                    .text_color(tokens::fg_muted()),
            ),
    )
}

/// The recoverable-binding notice: the linked grouping node sits in
/// Trash; the binding is kept, and the user can restore it or rebind
/// (ADR-0001).
fn trashed_link_notice(store: Store, project_id: i64) -> impl IntoElement {
    let node = move || {
        store
            .project(project_id)
            .and_then(|project| project.linked_group_node_id)
            .and_then(|node_id| store.catalog_node(node_id))
    };
    view().flex_1().items_center().justify_center().child(
        view()
            .flex_col()
            .gap_3()
            .items_center()
            .max_w(px(460.0))
            .px_5()
            .py_4()
            .rounded(tokens::radius_card())
            .border_1()
            .border_color(tokens::warning())
            .bg(move || theme().card)
            .child(
                text("⚠️")
                    .text_size(px(28.0)),
            )
            .child(
                text(move || {
                    let title = node().map(|node| node.title).unwrap_or_default();
                    format!("关联目录「{}」在回收站中", title)
                })
                .text_size(px(16.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tokens::fg_primary()),
            )
            .child(
                text("项目仍保持绑定。恢复目录即可继续在此新建项目文档；也可以改绑到其他知识库或目录分组。")
                    .text_size(px(12.0))
                    .text_color(tokens::fg_muted()),
            )
            .child(
                view()
                    .flex_row()
                    .gap_2()
                    .child(
                        app_button(ButtonKind::Positive, "恢复目录", move || {
                            if let Some(node) = node() {
                                store.restore_catalog_node(node.id);
                            }
                        })
                        .test_id("project-restore-node"),
                    )
                    .child(
                        app_button(ButtonKind::Soft, "改绑", move || {
                            store.open_project_editor(Some(project_id));
                        })
                        .test_id("project-rebind"),
                    ),
            ),
    )
}

/// The bound state: the catalog sidebar on the left, the document
/// canvas (or the landing) on the right — the book shell's layout with
/// the Project's navigation rules (docs/ui-design.md §3.4).
fn linked_documents(store: Store, tab_id: u64, project_id: i64) -> impl IntoElement {
    let narrow = move || window().width.0 < NARROW_BREAKPOINT;
    let drawer = signal(false);
    let on_close = move || drawer.set(false);
    view()
        .flex_1()
        .min_w(px(0.0))
        .min_h(px(0.0))
        .flex_row()
        .child(when(
            move || !narrow(),
            move || {
                each(
                    move || vec![linked_catalog_key(store, tab_id, project_id)],
                    |key: &(i64, Option<i64>, Option<i64>)| *key,
                    move |key, _index| {
                        let (book_id, group_node, doc) = key.get();
                        kb::catalog_sidebar(
                            store,
                            book_id,
                            match doc {
                                Some(doc) => CatalogFocus::Document(doc),
                                None => CatalogFocus::Home,
                            },
                            false,
                            on_close,
                            CatalogNav::Project { linked_node: group_node },
                        )
                        .w(px(kb::CATALOG_W))
                        .test_id("catalog-inline")
                        .into_element()
                    },
                )
            },
        ))
        .child(
            view()
                .flex_1()
                .min_w(px(0.0))
                .min_h(px(0.0))
                .relative()
                .flex_col()
                .child(when(narrow, move || {
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .py_2()
                        .child(app_button(ButtonKind::Soft, "☰ 目录", move || {
                            drawer.set(true);
                        }))
                }))
                .child(each(
                    move || visible_document(store, tab_id).into_iter().collect::<Vec<_>>(),
                    |doc_id: &i64| *doc_id,
                    move |doc_id, _index| {
                        kb::document_view(store, tab_id, doc_id.get()).into_element()
                    },
                ))
                .child(when(
                    move || visible_document(store, tab_id).is_none(),
                    move || landing(store, project_id),
                ))
                .child(when(
                    move || narrow() && drawer.get(),
                    move || {
                        view()
                            .absolute()
                            .inset_0()
                            .flex_row()
                            .child(
                                nsh_base::button()
                                    .on_press(on_close)
                                    .flex_1()
                                    .bg(theme().background.opacity(0.35))
                                    .accessibility_label("关闭目录"),
                            )
                            .child(
                                each(
                                    move || vec![linked_catalog_key(store, tab_id, project_id)],
                                    |key: &(i64, Option<i64>, Option<i64>)| *key,
                                    move |key, _index| {
                                        let (book_id, group_node, doc) = key.get();
                                        kb::catalog_sidebar(
                                            store,
                                            book_id,
                                            match doc {
                                                Some(doc) => CatalogFocus::Document(doc),
                                                None => CatalogFocus::Home,
                                            },
                                            true,
                                            on_close,
                                            CatalogNav::Project { linked_node: group_node },
                                        )
                                        .w(px(kb::CATALOG_W))
                                        .shadow_lg()
                                        .test_id("catalog-overlay")
                                        .into_element()
                                    },
                                )
                            )
                    },
                )),
        )
}

/// The keyed identity of the Linked Documents chrome: the binding
/// target (book + grouping node) and the open document. The sidebar and
/// the canvas rebuild when — and only when — this identity changes
/// (switching documents inside the sub-view, rebinding): a `when`
/// branch would bake the first visible document until the condition
/// flips back.
fn linked_catalog_key(
    store: Store,
    tab_id: u64,
    project_id: i64,
) -> (i64, Option<i64>, Option<i64>) {
    let target = store.project_document_target(project_id);
    let book_id = target.map(|target| target.book_id).unwrap_or_default();
    let group_node = target.and_then(|target| target.group_node_id);
    (book_id, group_node, visible_document(store, tab_id))
}

/// The document open in this tab's Linked Documents sub-view, when it
/// still exists and its tree row is not trashed.
fn visible_document(store: Store, tab_id: u64) -> Option<i64> {
    let doc_id = store.project_document_of(tab_id)?;
    store.document(doc_id)?;
    let node = store.catalog_node_of_document(doc_id)?;
    if store.catalog_node_is_trashed(node.id) {
        return None;
    }
    Some(doc_id)
}

/// The Linked Documents landing: where new documents land and a button
/// that creates one in the linked scope (ADR-0001).
fn landing(store: Store, project_id: i64) -> impl IntoElement {
    let scope = move || match store.project_document_target(project_id) {
        Some(target) => match target.group_node_id {
            Some(node_id) => {
                let node = store
                    .catalog_node(node_id)
                    .map(|node| node.title)
                    .unwrap_or_default();
                format!("新建文档将落在关联目录「{}」下", node)
            }
            None => {
                let book = store
                    .knowledge_base(target.book_id)
                    .map(|kb| kb.name)
                    .unwrap_or_else(|| "知识库".to_owned());
                format!("新建文档将落在知识库「{}」的目录顶部", book)
            }
        },
        None => String::new(),
    };
    view().flex_1().items_center().justify_center().child(
        view()
            .flex_col()
            .gap_3()
            .items_center()
            .max_w(px(460.0))
            .child(
                text("关联文档")
                    .text_size(px(18.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(tokens::fg_primary()),
            )
            .child(
                text("从左侧目录打开文档，或在此为项目新建一篇文档。")
                    .text_size(px(12.0))
                    .text_color(tokens::fg_muted()),
            )
            .child(
                app_button(ButtonKind::Primary, "＋ 新建文档", move || {
                    if let Some(doc_id) = store.create_document_for_project(project_id, "未命名文档") {
                        store.set_project_document(Some(doc_id));
                    }
                })
                .test_id("project-doc-create"),
            )
            .child(text(scope).text_size(px(11.0)).text_color(tokens::fg_muted())),
    )
}

// ---------------------------------------------------------------------
// The Project editor (creation and rebinding)
// ---------------------------------------------------------------------

/// How the editor binds the Project's documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindMode {
    None,
    WholeBook,
    Node,
    NewBook,
}

/// The full-page Project editor: name, description, color/emoji, and the
/// ADR-0001 binding choice (none / a book / a grouping node / a new
/// same-named book). `editing == None` creates; `Some(id)` rebinds.
pub fn project_editor_view(store: Store, editing: Option<i64>) -> impl IntoElement {
    let existing = editing.and_then(|id| store.project(id));
    let name = signal(existing.as_ref().map(|p| p.name.clone()).unwrap_or_default());
    let description =
        signal(existing.as_ref().map(|p| p.description.clone()).unwrap_or_default());
    let color = signal(
        existing
            .as_ref()
            .map(|p| p.color.clone())
            .unwrap_or_else(|| COLORS[0].to_owned()),
    );
    let emoji = signal(
        existing
            .as_ref()
            .map(|p| p.emoji.clone())
            .unwrap_or_else(|| EMOJIS[0].to_owned()),
    );
    let mode = signal(match existing.as_ref().and_then(|p| p.linked_group_node_id) {
        Some(_) => BindMode::Node,
        None => match existing.as_ref().and_then(|p| p.linked_book_id) {
            Some(_) => BindMode::WholeBook,
            None => BindMode::None,
        },
    });
    let book_sel = signal(existing.as_ref().and_then(|p| p.linked_book_id));
    let node_sel = signal(existing.as_ref().and_then(|p| p.linked_group_node_id));
    let title = if editing.is_some() { "编辑项目" } else { "新建项目" };

    view().flex_1().min_h(px(0.0)).child(
        scroll().flex_1().child(
            view()
                .flex_col()
                .gap_4()
                .p_5()
                .max_w(px(640.0))
                .w_full()
                .self_center()
                .child(
                    text(title)
                        .text_size(px(22.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(tokens::fg_primary()),
                )
                .child(
                    view()
                        .flex_col()
                        .gap_1()
                        .child(section_heading("名称"))
                        .child(input_field(
                            "项目名称",
                            move || name.get(),
                            move |text: &str| name.set(text.to_owned()),
                        )),
                )
                .child(
                    view()
                        .flex_col()
                        .gap_1()
                        .child(section_heading("简介"))
                        .child(input_field(
                            "这个项目要达成什么？",
                            move || description.get(),
                            move |text: &str| description.set(text.to_owned()),
                        )),
                )
                .child(
                    view()
                        .flex_row()
                        .gap_4()
                        .child(picker("颜色", COLORS, color, |c| c.to_owned()))
                        .child(picker("图标", EMOJIS, emoji, |e| e.to_owned())),
                )
                .child(binding_section(store, mode, book_sel, node_sel))
                .child(
                    view()
                        .flex_row()
                        .gap_2()
                        .child(
                            app_button(ButtonKind::Primary, if editing.is_some() { "保存" } else { "创建项目" }, move || {
                                submit_editor(
                                    store,
                                    &EditorDraft {
                                        editing,
                                        name: name.get(),
                                        description: description.get(),
                                        color: color.get(),
                                        emoji: emoji.get(),
                                        mode: mode.get(),
                                        book_sel: book_sel.get(),
                                        node_sel: node_sel.get(),
                                    },
                                );
                            })
                            .test_id("project-save"),
                        )
                        .child(app_button(ButtonKind::Ghost, "取消", move || {
                            cancel_editor(store, editing);
                        })),
                ),
        ),
    )
}

/// A labeled row of preset choices (color dots / emoji tiles).
fn picker(
    label: &'static str,
    choices: &'static [&'static str],
    selected: Signal<String>,
    name: impl Fn(&'static str) -> String + Copy + 'static,
) -> impl IntoElement {
    view().flex_col().gap_1().child(section_heading(label)).child(
        view()
            .flex_row()
            .gap_2()
            .children(
                choices
                    .iter()
                    .map(|choice| {
                        let choice = *choice;
                        let is_selected = move || selected.get() == name(choice);
                        nsh_base::button()
                            .on_press(move || selected.set(name(choice)))
                            .accessibility_label(name(choice))
                            .size(px(26.0))
                            .items_center()
                            .justify_center()
                            .rounded(tokens::radius_row())
                            .border_1()
                            .border_color(move || {
                                if is_selected() {
                                    tokens::accent()
                                } else {
                                    tokens::border_subtle()
                                }
                            })
                            .bg(move || {
                                if is_selected() {
                                    tokens::accent_soft()
                                } else {
                                    theme().background.opacity(0.0)
                                }
                            })
                            .child(text(choice).text_size(px(13.0)))
                    })
                    .collect::<Vec<_>>(),
            ),
    )
}

/// The binding choice: 暂不关联 / 整本知识库 / 目录分组 / 新建同名知识库,
/// with the book and grouping-node pickers.
fn binding_section(
    store: Store,
    mode: Signal<BindMode>,
    book_sel: Signal<Option<i64>>,
    node_sel: Signal<Option<i64>>,
) -> impl IntoElement {
    view()
        .flex_col()
        .gap_2()
        .child(section_heading("关联文档来源（ADR-0001）"))
        .child(
            view()
                .flex_row()
                .gap_2()
                .flex_wrap()
                .children(
                    [
                        (BindMode::None, "暂不关联"),
                        (BindMode::WholeBook, "关联整本知识库"),
                        (BindMode::Node, "关联目录分组"),
                        (BindMode::NewBook, "新建同名知识库"),
                    ]
                    .iter()
                    .map(|(candidate, label)| {
                        let candidate = *candidate;
                        let active = move || mode.get() == candidate;
                        nsh_base::button()
                            .on_press(move || mode.set(candidate))
                            .accessibility_label(Some((*label).into()))
                            .h(px(28.0))
                            .px_3()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .border_1()
                            .border_color(move || {
                                if active() { tokens::accent() } else { tokens::border_subtle() }
                            })
                            .bg(move || {
                                if active() {
                                    tokens::accent_soft()
                                } else {
                                    theme().background.opacity(0.0)
                                }
                            })
                            .text_color(move || {
                                if active() { tokens::accent() } else { tokens::fg_body() }
                            })
                            .hover(|s| s.bg(tokens::control_hover()))
                            .child(text(*label).text_size(px(12.0)))
                    })
                    .collect::<Vec<_>>(),
                ),
        )
        .child(when(
            move || matches!(mode.get(), BindMode::WholeBook | BindMode::Node),
            move || book_picker(store, book_sel),
        ))
        .child(when(
            move || mode.get() == BindMode::Node,
            move || node_picker(store, book_sel, node_sel),
        ))
        .child(when(
            move || mode.get() == BindMode::NewBook,
            move || {
                text("创建项目时同时新建一本同名知识库，项目的文档默认落在它的目录顶部。")
                    .text_size(px(11.0))
                    .text_color(tokens::fg_muted())
            },
        ))
}

/// The book dropdown for the WholeBook/Node modes.
fn book_picker(store: Store, book_sel: Signal<Option<i64>>) -> impl IntoElement {
    let label = move || {
        book_sel
            .get()
            .and_then(|id| store.knowledge_base(id))
            .map(|kb| kb.name)
            .unwrap_or_else(|| "选择知识库".to_owned())
    };
    view().flex_col().gap_1().child(
        menu()
            .offset(px(4.0))
            .popup_style(|s| {
                s.p_1()
                    .rounded(tokens::radius_card())
                    .bg(theme().popover)
                    .shadow_md()
                    .w(px(240.0))
            })
            .trigger(
                nsh_base::button()
                    .accessibility_label("选择知识库")
                    .test_id("project-book-picker")
                    .h(px(30.0))
                    .items_center()
                    .gap_2()
                    .px_3()
                    .rounded(tokens::radius_row())
                    .border_1()
                    .border_color(tokens::border_subtle())
                    .bg(move || theme().background.opacity(0.0))
                    .text_color(tokens::fg_primary())
                    .hover(|s| s.bg(tokens::control_hover()))
                    .child(text(label).text_size(px(13.0)).flex_1())
                    .child(text("▾").text_size(px(11.0)).text_color(tokens::fg_muted())),
            )
            .child(each(
                move || store.knowledge_bases(),
                |kb: &crate::domain::KnowledgeBase| kb.id,
                move |kb, _index| {
                    menu_item()
                        .on_select(move || book_sel.set(Some(kb.with(|kb| kb.id))))
                        .px_3()
                        .py_2()
                        .rounded(tokens::radius_row())
                        .hover(|s| s.bg(tokens::control_hover()))
                        .child(
                            text(move || kb.with(|kb| kb.name.clone()))
                                .text_size(px(13.0))
                                .text_color(tokens::fg_primary()),
                        )
                },
            )),
    )
}

/// The grouping-node dropdown of the chosen book (flattened with depth
/// indentation; only pure grouping nodes can bind a Project). The list
/// binds to `book_sel` so choosing a book refreshes the popup.
fn node_picker(
    store: Store,
    book_sel: Signal<Option<i64>>,
    node_sel: Signal<Option<i64>>,
) -> impl IntoElement {
    let nodes = move || {
        book_sel
            .get()
            .map(|book_id| {
                store
                    .catalog_outline(book_id)
                    .into_iter()
                    .filter(|entry| entry.node.document_id.is_none())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let label = move || {
        node_sel
            .get()
            .and_then(|id| store.catalog_node(id))
            .map(|node| node.title)
            .unwrap_or_else(|| "选择目录分组".to_owned())
    };
    view().flex_col().gap_1().child(
        menu()
            .offset(px(4.0))
            .popup_style(|s| {
                s.p_1()
                    .rounded(tokens::radius_card())
                    .bg(theme().popover)
                    .shadow_md()
                    .w(px(240.0))
            })
            .trigger(
                nsh_base::button()
                    .accessibility_label("选择目录分组")
                    .test_id("project-node-picker")
                    .h(px(30.0))
                    .items_center()
                    .gap_2()
                    .px_3()
                    .rounded(tokens::radius_row())
                    .border_1()
                    .border_color(tokens::border_subtle())
                    .bg(move || theme().background.opacity(0.0))
                    .text_color(tokens::fg_primary())
                    .hover(|s| s.bg(tokens::control_hover()))
                    .child(text(label).text_size(px(13.0)).flex_1())
                    .child(text("▾").text_size(px(11.0)).text_color(tokens::fg_muted())),
            )
            .child(when(
                move || nodes().is_empty(),
                move || {
                    text("这本知识库还没有目录分组")
                        .text_size(px(12.0))
                        .text_color(tokens::fg_muted())
                        .px_3()
                        .py_2()
                },
            ))
            .child(each(
                nodes,
                |entry: &crate::domain::CatalogEntry| entry.node.id,
                move |entry, _index| {
                    menu_item()
                        .on_select(move || node_sel.set(Some(entry.with(|entry| entry.node.id))))
                        .px_3()
                        .py_2()
                        .rounded(tokens::radius_row())
                        .hover(|s| s.bg(tokens::control_hover()))
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .pl(move || px(entry.with(|entry| entry.depth as f32 * 12.0)))
                                .child(
                                    text(move || entry.with(|entry| entry.node.title.clone()))
                                        .text_size(px(13.0))
                                        .text_color(tokens::fg_primary()),
                                ),
                        )
                },
            )),
    )
}

/// The editor form's committed state.
struct EditorDraft {
    editing: Option<i64>,
    name: String,
    description: String,
    color: String,
    emoji: String,
    mode: BindMode,
    book_sel: Option<i64>,
    node_sel: Option<i64>,
}

/// Commits the editor: creates (with the binding) or rebinds, then lands
/// on the Project page.
fn submit_editor(store: Store, draft: &EditorDraft) {
    let binding = match draft.mode {
        BindMode::None => ProjectBinding::None,
        BindMode::WholeBook => match draft.book_sel {
            Some(book_id) => ProjectBinding::Book(book_id),
            None => return,
        },
        BindMode::Node => match draft.node_sel {
            Some(node_id) => ProjectBinding::Node(node_id),
            None => return,
        },
        BindMode::NewBook => ProjectBinding::NewBook,
    };
    match draft.editing {
        Some(id) => {
            let saved = if draft.mode == BindMode::NewBook {
                // One-click binding on an existing project: save the
                // fields first so the new book takes the (possibly new)
                // project name, then create and bind it.
                if store.update_project(
                    id,
                    &ProjectPatch {
                        name: Some(draft.name.trim().to_owned()),
                        description: Some(draft.description.clone()),
                        color: Some(draft.color.clone()),
                        emoji: Some(draft.emoji.clone()),
                        ..Default::default()
                    },
                ) {
                    store.bind_project_new_book(id)
                } else {
                    false
                }
            } else {
                store.update_project(
                    id,
                    &ProjectPatch {
                        name: Some(draft.name.trim().to_owned()),
                        description: Some(draft.description.clone()),
                        color: Some(draft.color.clone()),
                        emoji: Some(draft.emoji.clone()),
                        linked_book_id: Some(binding_book(store, binding)),
                        linked_group_node_id: Some(binding_node(binding)),
                    },
                )
            };
            if saved {
                store.open_project(id);
            }
        }
        None => {
            if let Some(id) = store.create_project_with_binding(
                &draft.name,
                &draft.color,
                &draft.emoji,
                binding,
            ) {
                store.open_project(id);
            }
        }
    }
}

/// The rebind's book column: a node binding stores the node's book.
fn binding_book(store: Store, binding: ProjectBinding) -> Option<i64> {
    match binding {
        ProjectBinding::None => None,
        ProjectBinding::Book(book_id) => Some(book_id),
        ProjectBinding::Node(node_id) => store
            .catalog_node(node_id)
            .map(|node| node.book_id),
        ProjectBinding::NewBook => None,
    }
}

fn binding_node(binding: ProjectBinding) -> Option<i64> {
    match binding {
        ProjectBinding::Node(node_id) => Some(node_id),
        _ => None,
    }
}

fn cancel_editor(store: Store, editing: Option<i64>) {
    match editing {
        Some(id) => store.open_project(id),
        None => store.open_or_focus(crate::shell::tabs::Route::Tasks),
    }
}
