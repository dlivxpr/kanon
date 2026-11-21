mod backup;
mod text_import;

use nsh_base::{ButtonElement, button, theme};
use nsh_core::file_dialog::{FileFilter, SaveDialog};
use nsh_core::{platform::Appearance, prelude::*};

use crate::ui::{
    chrome::{Icon, icon},
    controls::{ButtonKind, app_button, section_heading},
    theme as tokens,
};

use super::{
    Store,
    calendar::PREF_WEEK_START,
    sidebar::NAV,
    store::{
        PREF_CALENDAR_RANGE, PREF_CARRYOVER, PREF_DOCUMENT_WIDTH, PREF_OUTLINE, PREF_START_PAGE,
        PREF_WORD_COUNT,
    },
    tabs::Route,
};

/// The workspace-preference key of the main accent preset (one of
/// `tokens::ACCENT_PRESETS`; `theme.json`'s coral when unset).
pub(crate) const PREF_ACCENT: &str = "accent";
pub(super) const PREF_UI_FONT: &str = "ui_font";
pub(super) const PREF_TEXT_SIZE: &str = "text_size";
pub(super) const PREF_DENSITY: &str = "density";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    General,
    Appearance,
    Editor,
    Notifications,
    Data,
    Shortcuts,
    About,
}

const PAGES: [(Page, &str, Icon); 7] = [
    (Page::General, "通用", Icon::Settings),
    (Page::Appearance, "外观", Icon::Sun),
    (Page::Editor, "编辑与文档", Icon::Document),
    (Page::Notifications, "通知与提醒", Icon::Clock),
    (Page::Data, "数据与备份", Icon::Local),
    (Page::Shortcuts, "快捷键", Icon::Outline),
    (Page::About, "关于", Icon::Brand),
];

pub fn settings_view(store: Store) -> impl IntoElement {
    let page = signal(Page::General);
    let failed = signal(false);
    let compact = move || {
        let width = window().width.0;
        width < 1024.0 || (width < 1280.0 && store.panel_visible())
    };
    view()
        .flex_1()
        .min_h(px(0.0))
        .p_5()
        .gap_4()
        .w_full()
        .max_w(px(960.0))
        .self_center()
        .child(
            view()
                .gap_1()
                .child(
                    text("设置")
                        .role(AccessibilityRole::Header)
                        .text_3xl()
                        .font_weight(FontWeight::BOLD)
                        .text_color(tokens::fg_primary),
                )
                .child(
                    text("自定义你的 Kanon 工作区")
                        .text_sm()
                        .text_color(tokens::fg_muted),
                ),
        )
        .child(
            view()
                .flex_1()
                .min_h(px(0.0))
                .flex_row()
                .gap_6()
                .style_when(compact, |s| s.flex_col().items_stretch())
                .child(
                    view()
                        .flex_col()
                        .gap_1()
                        .w(px(176.0))
                        .flex_shrink(0.0)
                        .style_when(compact, |s| s.w_full().flex_row().flex_wrap())
                        .children(PAGES.map(|(item, title, glyph)| {
                            button()
                                .h(px(32.0))
                                .px_2()
                                .rounded(tokens::radius_row())
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .accessibility_label(title)
                                .on_press(move || {
                                    failed.set(false);
                                    page.set(item);
                                })
                                .selected(move || page.get() == item)
                                .selected_style(|s| {
                                    s.bg(tokens::control_bg).text_color(tokens::accent_ink)
                                })
                                .text_color(tokens::fg_body)
                                .hover(|s| s.bg(tokens::control_hover))
                                .active(|s| s.opacity(0.75))
                                .transition_colors()
                                .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
                                .focus_visible(|s| {
                                    s.outline(px(2.0)).outline_color(tokens::accent_ink)
                                })
                                .child(icon(move || glyph, px(15.0)))
                                .child(text(title).text_sm())
                        })),
                )
                .child(dynamic(move || {
                    let group = page.get();
                    scroll().flex_1().min_w(px(0.0)).min_h(px(0.0)).child(
                        view()
                            .gap_3()
                            .px_1()
                            .pb_2()
                            .child(
                                settings_page(store, group, failed)
                                    .bind_motion(&crate::ui::motion::reveal()),
                            )
                            .child(when(failed, || {
                                text("无法保存设置。请检查工作区是否可写，然后重试。")
                                    .text_sm()
                                    .text_color(tokens::danger)
                            })),
                    )
                })),
        )
}

fn settings_page(store: Store, page: Page, failed: Signal<bool>) -> View {
    let heading = PAGES
        .iter()
        .find(|(item, _, _)| *item == page)
        .map(|(_, title, _)| *title)
        .unwrap_or("设置");
    let description = match page {
        Page::General => "配置启动行为、日历排期习惯与日常交互偏好。",
        Page::Appearance => "外观统一应用于窗口、侧栏和内容区。",
        Page::Editor => "调整文档纸面和小记的默认布局。",
        Page::Notifications => "管理任务提醒和逾期结转提示。",
        Page::Data => "知识库、文档、项目、任务与小记均保存在本机。",
        Page::Shortcuts => "用键盘快速完成导航、切换与写作操作。",
        Page::About => "把知识库、待办与项目收拢在一处安静的本地工作台。",
    };
    let content = view()
        .gap_3()
        .child(section_heading(heading))
        .child(text(description).text_sm().text_color(tokens::fg_muted));
    match page {
        Page::General => content
            .child(preference_row(
                "启动页",
                "重新打开 Kanon 时默认显示的页面。",
                choices(
                    "启动页",
                    store,
                    failed,
                    PREF_START_PAGE,
                    "today",
                    &[
                        ("today", "今天"),
                        ("start", "开始"),
                        ("tasks", "待办"),
                        ("calendar", "日历"),
                        ("notes", "小记"),
                    ],
                ),
            ))
            .child(preference_row(
                "默认日历视图",
                "进入日历时默认显示的时间范围。",
                choices(
                    "默认日历视图",
                    store,
                    failed,
                    PREF_CALENDAR_RANGE,
                    "month",
                    &[
                        ("month", "月视图"),
                        ("week", "周视图"),
                        ("three", "3 天视图"),
                    ],
                ),
            ))
            .child(preference_row(
                "每周起始日",
                "同时应用于主日历与右侧迷你日历。",
                choices(
                    "每周起始日",
                    store,
                    failed,
                    PREF_WEEK_START,
                    "monday",
                    &[("monday", "周一"), ("sunday", "周日")],
                ),
            ))
            .child(preference_row(
                "时间格式",
                "控制任务与日程中的时间显示格式。",
                choices(
                    "时间格式",
                    store,
                    failed,
                    super::store::preferences::PREF_TIME_FORMAT,
                    "24h",
                    &[
                        ("24h", "24 小时制（14:00）"),
                        ("12h", "12 小时制（下午 2:00）"),
                    ],
                ),
            ))
            .child(preference_row(
                "侧栏项目与知识库在新标签打开",
                "开启后会保留当前标签页，在新标签打开项目或知识库。",
                preference_switch(
                    store,
                    failed,
                    super::store::preferences::PREF_OPEN_IN_NEW_TAB,
                    false,
                    "侧栏项目与知识库在新标签打开",
                ),
            ))
            .child(preference_row(
                "任务完成音效",
                "将任务标为完成时播放简短提示音。",
                preference_switch(
                    store,
                    failed,
                    super::store::preferences::PREF_COMPLETION_SOUND,
                    true,
                    "任务完成音效",
                ),
            ))
            .child(preference_row(
                "开机自动启动",
                "登录系统后打开此 Kanon。系统审批状态以系统设置为准。",
                launch_at_login_control(),
            )),
        Page::Appearance => content
            .child(
                view().flex_row().gap_2().children(
                    [
                        (Appearance::Light, "浅色"),
                        (Appearance::Dark, "深色"),
                        (Appearance::System, "跟随系统"),
                    ]
                    .map(|(appearance, title)| theme_option(store, failed, appearance, title)),
                ),
            )
            .child(preference_row(
                "主强调色",
                "用于激活高亮、日期标记与主交互反馈。",
                accent_choices(store, failed),
            ))
            .child(preference_row(
                "文字大小",
                "调整界面常用控件和文档文字的大小。",
                choices(
                    "文字大小",
                    store,
                    failed,
                    PREF_TEXT_SIZE,
                    "standard",
                    &[
                        (
                            tokens::TEXT_SIZE_PRESETS[1].key,
                            tokens::TEXT_SIZE_PRESETS[1].label,
                        ),
                        (
                            tokens::TEXT_SIZE_PRESETS[0].key,
                            tokens::TEXT_SIZE_PRESETS[0].label,
                        ),
                        (
                            tokens::TEXT_SIZE_PRESETS[2].key,
                            tokens::TEXT_SIZE_PRESETS[2].label,
                        ),
                    ],
                ),
            ))
            .child(preference_row(
                "界面字体",
                "选择工作台和阅读纸面的字体风格。",
                choices(
                    "界面字体",
                    store,
                    failed,
                    PREF_UI_FONT,
                    "system",
                    const {
                        &[
                            (tokens::UiFont::ALL[0].key(), tokens::UiFont::ALL[0].label()),
                            (tokens::UiFont::ALL[1].key(), tokens::UiFont::ALL[1].label()),
                            (tokens::UiFont::ALL[2].key(), tokens::UiFont::ALL[2].label()),
                        ]
                    },
                ),
            ))
            .child(preference_row(
                "信息密度",
                "紧凑模式减少列表与控件留白。",
                choices(
                    "信息密度",
                    store,
                    failed,
                    PREF_DENSITY,
                    "comfortable",
                    const {
                        &[
                            (
                                tokens::Density::ALL[0].key(),
                                tokens::Density::ALL[0].label(),
                            ),
                            (
                                tokens::Density::ALL[1].key(),
                                tokens::Density::ALL[1].label(),
                            ),
                        ]
                    },
                ),
            ))
            .child(section_heading("侧栏主导航入口"))
            .child(view().children(NAV.iter().map(|(route, key)| {
                let title = route.title();
                let description = match route {
                    Route::Today => "日常聚焦入口，建议保持开启。",
                    _ => "隐藏后仍可通过搜索与命令打开。",
                };
                let label = format!("显示{title}");
                preference_row(
                    title,
                    description,
                    preference_switch(store, failed, key, true, label),
                )
            }))),
        Page::Editor => content
            .child(preference_row(
                "文档页面宽度",
                "同时应用于文档的阅读和编辑界面。",
                choices(
                    "文档页面宽度",
                    store,
                    failed,
                    PREF_DOCUMENT_WIDTH,
                    "standard",
                    &[("standard", "标准纸面"), ("wide", "宽屏画布")],
                ),
            ))
            .child(preference_row(
                "显示文档字数统计",
                "在阅读纸面左下方显示正文字数。",
                preference_switch(store, failed, PREF_WORD_COUNT, true, "显示文档字数统计"),
            ))
            .child(preference_row(
                "阅读态悬浮大纲入口",
                "收起右侧面板后，在阅读纸面旁显示大纲。",
                preference_switch(store, failed, PREF_OUTLINE, true, "阅读态悬浮大纲入口"),
            ))
            .child(preference_row(
                "小记默认布局",
                "切换小记的时间流或网格排列。",
                choices(
                    "小记默认布局",
                    store,
                    failed,
                    "quick_notes_layout",
                    "stream",
                    &[("stream", "时间流"), ("grid", "网格")],
                ),
            )),
        Page::Notifications => notification_page(store, content, failed),
        Page::Data => data_page(store, content),
        Page::Shortcuts => {
            let modifier = if cfg!(target_os = "macos") {
                "⌘"
            } else {
                "Ctrl"
            };
            content.children(
                [
                    (Some("K"), "打开全局搜索与命令面板"),
                    (Some("T"), "新建工作标签页"),
                    (Some("Shift + T"), "恢复刚关闭的标签页"),
                    (Some("W"), "关闭当前标签页"),
                    (Some("Tab"), "切换到下一个工作标签页"),
                    (Some("1–9"), "切换到对应标签页"),
                    (Some("\\"), "折叠或展开主导航"),
                    (Some("."), "展开或收起上下文面板"),
                    (Some("E"), "在文档页面切换阅读态与编辑态"),
                    (None, "用 / 打开块插入菜单，用 @ 提及对象"),
                    (Some("Enter"), "在小记输入框快速发布"),
                ]
                .map(|(key, title)| {
                    view()
                        .flex_row()
                        .justify_between()
                        .items_center()
                        .gap_2()
                        .py_2()
                        .border_b_1()
                        .border_color(tokens::border_subtle)
                        .child(text(title).text_sm().text_color(tokens::fg_body))
                        .child(key.map(|key| {
                            text(format!("{modifier} + {key}"))
                                .text_xs()
                                .px_2()
                                .py_1()
                                .rounded(tokens::radius_row())
                                .bg(tokens::control_bg)
                                .text_color(tokens::fg_primary)
                        }))
                }),
            )
        }
        Page::About => about_page(content),
    }
}

fn notification_page(store: Store, content: View, failed: Signal<bool>) -> View {
    use super::reminders::{DEFAULT_DIGEST_TIME, PREF_DIGEST, PREF_DIGEST_TIME, PREF_DUE_REMINDER};
    use nsh_core::notifications::NotificationPermission;

    let permission = signal(None);
    spawn(async move { permission.set(Some(notification_permission().await)) });
    let digest_enabled = move || store.preference(PREF_DIGEST).as_deref() == Some("true");
    content
        .child(preference_row(
            "系统通知",
            "Kanon 打开时发送，关闭应用后不会在后台运行。",
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(move || match permission.get() {
                        None => "正在读取权限…",
                        Some(NotificationPermission::Granted) => "已允许",
                        Some(NotificationPermission::Denied) => "已关闭，请在系统设置中允许",
                        Some(NotificationPermission::Default) => "尚未授权",
                        _ => "当前运行环境不支持",
                    })
                    .text_sm()
                    .text_color(tokens::fg_muted),
                )
                .child(when(
                    move || permission.get() == Some(NotificationPermission::Default),
                    move || {
                        app_button(ButtonKind::Soft, "允许通知", move || {
                            spawn(async move {
                                permission.set(Some(request_notification_permission().await))
                            });
                        })
                    },
                )),
        ))
        .child(preference_row(
            "逾期任务结转提醒条",
            "在「今天」展示之前未完成的排期任务。",
            preference_switch(store, failed, PREF_CARRYOVER, true, "逾期任务结转提醒条"),
        ))
        .child(preference_row(
            "每日晨间概览",
            "在设定的时间汇总当日待办与项目里程碑。",
            preference_switch(store, failed, PREF_DIGEST, false, "每日晨间概览"),
        ))
        .child(preference_row(
            "晨间概览时间",
            "按本地时间发送。",
            nsh_base::time_field()
                .accessibility_label("晨间概览时间")
                .disabled(move || !digest_enabled())
                .disabled_style(|s| s.opacity(0.55))
                .value(move || {
                    let saved = store.preference(PREF_DIGEST_TIME);
                    let time = saved
                        .as_deref()
                        .and_then(crate::domain::LocalTime::parse)
                        .unwrap_or(DEFAULT_DIGEST_TIME);
                    time.to_string().parse().ok().map(nsh_base::TimeOfDay)
                })
                .on_change(move |time: &nsh_base::TimeOfDay| {
                    failed.set(
                        !store
                            .set_preference(PREF_DIGEST_TIME, &time.0.format("%H:%M").to_string()),
                    );
                })
                .h_8()
                .px_2()
                .items_center()
                .rounded(tokens::radius_row())
                .bg(tokens::control_bg)
                .segment_class(
                    "min-w-5 rounded-sm px-1 focus:bg-accent-soft focus:text-accent-ink",
                ),
        ))
        .child(preference_row(
            "任务到期前提醒",
            "只提醒设置了具体截止时间的未完成任务。",
            choices(
                "任务到期前提醒",
                store,
                failed,
                PREF_DUE_REMINDER,
                "off",
                &[
                    ("off", "关闭"),
                    ("0m", "准时提醒"),
                    ("15m", "提前 15 分钟"),
                    ("30m", "提前 30 分钟"),
                    ("60m", "提前 1 小时"),
                ],
            ),
        ))
}

fn about_page(content: View) -> View {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../../nsh.json")).expect("valid nsh.json");
    let version = config["version"].as_str().unwrap_or_default();
    let build = &config["build"];
    let id = config["id"].as_str().unwrap_or_default().to_owned();
    let notes_title = format!("更新日志 · v{version}");
    let version = format!("v{version} · Build {build}");
    let version_info = format!("Kanon {version} · {id}");
    let copied = signal(false);
    let notes_open = signal(false);
    let release_failed = signal(false);
    content
        .child(
            settings_card()
                .flex_row()
                .items_center()
                .gap_3()
                .child(
                    svg(nsh_core::element::ImageSrc::svg_markup(include_str!(
                        "../../assets/kanon.svg"
                    )))
                    .size(px(48.0))
                    .flex_none(),
                )
                .child(
                    view()
                        .flex_1()
                        .gap_1()
                        .child(section_heading("关于 Kanon"))
                        .child(
                            text("本地优先 · 知识库、项目与待办")
                                .text_sm()
                                .text_color(tokens::fg_muted),
                        ),
                ),
        )
        .child(preference_row(
            "软件更新",
            "当前构建未配置自动更新，发布信息以项目发布页为准。",
            view()
                .flex_row()
                .flex_wrap()
                .gap_2()
                .child(app_button(ButtonKind::Soft, "更新日志", move || {
                    notes_open.set(true)
                }))
                .child(app_button(ButtonKind::Soft, "查看发布", move || {
                    spawn(async move {
                        release_failed
                            .set(!open_url("https://github.com/dlivxpr/kanon/releases").await);
                    });
                })),
        ))
        .child(when(release_failed, || {
            text("无法打开浏览器。请访问 github.com/dlivxpr/kanon/releases。")
                .text_sm()
                .text_color(tokens::fg_muted)
        }))
        .child(
            settings_card()
                .child(
                    view()
                        .flex_row()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(section_heading("版本与构建信息"))
                        .child(app_button(
                            ButtonKind::Soft,
                            "复制版本信息",
                            move || {
                                set_clipboard(version_info.clone());
                                copied.set(true);
                            },
                        )),
                )
                .children(
                    [
                        ("当前版本", version),
                        ("应用标识", id),
                        ("数据模式", "本地优先 · 离线存储".to_owned()),
                    ]
                    .map(|(name, value)| {
                        view()
                            .flex_row()
                            .flex_wrap()
                            .justify_between()
                            .gap_2()
                            .child(text(name).text_sm().text_color(tokens::fg_muted))
                            .child(text(value).text_sm().text_color(tokens::fg_primary))
                    }),
                )
                .child(when(copied, || {
                    text("已复制版本信息")
                        .text_sm()
                        .text_color(tokens::positive)
                })),
        )
        .child(when(notes_open, move || {
            crate::ui::controls::dialog_card(
                notes_title.clone(),
                "工作台交互动效、单一收件箱入口，以及重新整理的设置页。",
                AccessibilityRole::Dialog,
                move || notes_open.set(false),
                app_button(ButtonKind::Soft, "关闭", move || notes_open.set(false)),
            )
        }))
}

fn settings_card() -> View {
    view()
        .gap_2()
        .p_4()
        .rounded(tokens::radius_card())
        .bg(tokens::panel_subtle)
        .border_1()
        .border_color(tokens::border_subtle)
}

fn data_page(store: Store, content: View) -> View {
    let busy = signal(false);
    let notice = signal(None::<(bool, String)>);
    let export = move || {
        busy.set(true);
        notice.set(None);
        spawn(async move {
            let result = pick_save_path(
                SaveDialog::new()
                    .title("导出工作区备份")
                    .button_label("导出")
                    .file_name(format!("kanon-{}.sqlite3", store.now().date))
                    .filter(FileFilter::new("Kanon 工作区备份", ["sqlite3"])),
            )
            .await;
            let status = match result {
                Ok(Some(path)) => match store.export_backup(&path) {
                    Ok(()) => Some((true, format!("备份已保存到 {}", path.display()))),
                    Err(crate::domain::WorkspaceError::Io(error))
                        if error.kind() == std::io::ErrorKind::AlreadyExists =>
                    {
                        Some((false, "文件已存在。请选择一个新文件名。".to_owned()))
                    }
                    Err(error) => {
                        log::warn!("workspace backup failed: {error}");
                        Some((
                            false,
                            "无法保存备份。请检查目标文件夹是否可写，然后重试。".to_owned(),
                        ))
                    }
                },
                Ok(None) => None,
                Err(error) => {
                    log::warn!("workspace backup picker failed: {error}");
                    Some((false, "无法打开保存对话框。请重试。".to_owned()))
                }
            };
            notice.set(status);
            busy.set(false);
        });
    };
    content
        .child(workspace_status_card(store))
        .child(preference_row(
            "导出完整备份（SQLite）",
            "包含知识库、文档、项目、任务、小记、历史版本、回收站和偏好设置。请选择一个新文件名。",
            app_button(ButtonKind::Primary, "导出备份", export)
                .disabled(busy)
                .self_start(),
        ))
        .child(preference_row(
            "恢复工作区备份",
            "选择备份并保存为新文件。副本会在独立窗口打开，原工作区与备份不变。",
            app_button(ButtonKind::Soft, "恢复到新副本", move || {
                backup::restore(busy, notice)
            })
            .disabled(busy)
            .self_start(),
        ))
        .child(preference_row(
            "导入文本文档",
            "将 Markdown 或 UTF-8 文本作为新文档导入，不覆盖现有内容。",
            text_import::import_control(store, busy, notice),
        ))
        .child(dynamic(move || {
            notice.get().map(|(success, message)| {
                text(message).text_sm().text_color(move || {
                    if success {
                        tokens::positive()
                    } else {
                        tokens::danger()
                    }
                })
            })
        }))
}

fn workspace_status_card(store: Store) -> View {
    let status = computed(move || store.workspace_status());
    type Count = fn(&crate::domain::WorkspaceStatus) -> usize;
    let stats: [(&str, Count); 5] = [
        ("知识库", |s| s.knowledge_bases),
        ("文档", |s| s.documents),
        ("项目", |s| s.projects),
        ("任务", |s| s.tasks),
        ("小记", |s| s.quick_notes),
    ];
    settings_card()
        .child(section_heading("本地工作区状态"))
        .child(
            text("数据不会上传。备份可单独打开，不会覆盖当前工作区。")
                .text_sm()
                .text_color(tokens::fg_muted),
        )
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .gap_2()
                .children(stats.map(|(name, count)| {
                    view()
                        .flex_1()
                        .min_w(px(64.0))
                        .p_2()
                        .gap_1()
                        .rounded(tokens::radius_row())
                        .bg(tokens::panel_raised)
                        .child(
                            text(move || status.with(|s| count(s).to_string()))
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(tokens::fg_primary),
                        )
                        .child(text(name).text_sm().text_color(tokens::fg_muted))
                })),
        )
}

fn preference_row(
    title: impl Into<SharedString>,
    description: &'static str,
    control: impl IntoElement,
) -> View {
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap_x_4()
        .gap_y_2()
        .py_3()
        .style_when(tokens::is_compact(), |s| s.py_2())
        .border_b_1()
        .border_color(tokens::border_subtle)
        .child(
            view()
                .flex_1()
                .min_w(px(164.0))
                .gap_1()
                .child(
                    text(title.into())
                        .text_size(tokens::text_px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(tokens::fg_primary),
                )
                .child(
                    text(description)
                        .text_size(tokens::text_px(12.0))
                        .text_color(tokens::fg_muted),
                ),
        )
        .child(view().max_w(percentage(1.0)).child(control))
}

fn choices(
    label: &'static str,
    store: Store,
    failed: Signal<bool>,
    key: &'static str,
    default: &'static str,
    items: &'static [(&'static str, &'static str)],
) -> impl IntoElement {
    let current = computed(move || store.preference(key));
    crate::ui::settings::setting_select(
        label,
        items,
        move || {
            current.with(|pref| {
                let value = pref.as_deref().unwrap_or(default);
                items
                    .iter()
                    .find(|&&(key, _)| key == value)
                    .map_or(default, |&(key, _)| key)
                    .into()
            })
        },
        move |value| failed.set(!store.set_preference(key, value)),
    )
}

fn launch_at_login_control() -> impl IntoElement {
    use nsh_core::desktop::LaunchAtLoginError;
    use nsh_core::element::desktop::{launch_at_login, set_launch_at_login};

    let enabled = signal(None);
    let busy = signal(false);
    let notice = signal("");
    spawn(async move { enabled.set(Some(launch_at_login().await)) });
    view()
        .items_end()
        .gap_1()
        .child(crate::ui::settings::setting_switch(
            "开机自动启动".into(),
            move || enabled.get() == Some(true),
            move || busy.get() || enabled.get().is_none(),
            move |next| {
                busy.set(true);
                notice.set("");
                spawn(async move {
                    let result = set_launch_at_login(next).await;
                    let actual = launch_at_login().await;
                    enabled.set(Some(actual));
                    notice.set(match result {
                        Ok(()) if actual == next => "",
                        Ok(()) => "请在系统的登录项设置中确认。",
                        Err(LaunchAtLoginError::Unavailable) => "当前运行方式不支持开机启动。",
                        Err(error) => {
                            log::warn!("launch at login preference failed: {error}");
                            "未能更改。请检查系统的登录项设置。"
                        }
                    });
                    busy.set(false);
                });
            },
        ))
        .child(when(
            move || !notice.get().is_empty(),
            move || {
                text(move || notice.get())
                    .text_sm()
                    .text_color(tokens::fg_muted)
            },
        ))
}

fn preference_switch(
    store: Store,
    failed: Signal<bool>,
    key: &'static str,
    default: bool,
    label: impl Into<SharedString>,
) -> impl IntoElement {
    crate::ui::settings::setting_switch(
        label.into(),
        move || {
            store
                .preference(key)
                .as_deref()
                .map_or(default, |value| value == "true")
        },
        || false,
        move |checked| {
            failed.set(!store.set_preference(key, if checked { "true" } else { "false" }))
        },
    )
}

/// The main-accent swatches: one disc per preset of
/// `tokens::ACCENT_PRESETS`. The selected disc wears the accent-ink ring
/// and a check glyph, so the state never rests on color alone; pressing
/// a swatch persists its key, which the shell subscribes to apply the
/// preset (apply and restore share that one path).
fn accent_choices(store: Store, failed: Signal<bool>) -> View {
    let current = computed(move || store.preference(PREF_ACCENT));
    view()
        .flex_row()
        .gap_3()
        .items_center()
        .children(tokens::ACCENT_PRESETS.iter().map(|preset| {
            let key = preset.key;
            let selected = move || {
                current.with(|pref| pref.as_deref().unwrap_or(tokens::DEFAULT_ACCENT_KEY) == key)
            };
            button()
                .size(px(26.0))
                .items_center()
                .justify_center()
                .rounded_full()
                .accessibility_label(preset.label)
                .on_press(move || {
                    // Persist the choice; the shell subscribes the
                    // preference and applies the preset (apply and
                    // restore share that one path).
                    failed.set(!store.set_preference(PREF_ACCENT, key));
                })
                .selected(selected)
                .bg(preset.marker_color())
                // The ring is a binding, not a frozen state style: it
                // follows both the selection and a live accent switch.
                .border_2()
                .border_color(move || {
                    if selected() {
                        tokens::accent_ink()
                    } else {
                        theme().background.opacity(0.0)
                    }
                })
                .hover(|s| s.opacity(0.85))
                .active(|s| s.scale(0.94))
                .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
                .transition_colors()
                .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
                .child(when(selected, move || {
                    text("✓")
                        .text_size(px(13.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(preset.on_marker_color())
                }))
        }))
}

fn theme_option(
    store: Store,
    failed: Signal<bool>,
    appearance: Appearance,
    label: &'static str,
) -> ButtonElement {
    let dark = appearance == Appearance::Dark;
    let follow_system = appearance == Appearance::System;
    button()
        .accessibility_label(label)
        .on_press(move || failed.set(!store.set_appearance(appearance)))
        .selected(move || store.appearance() == appearance)
        .selected_style(|s| s.border_color(tokens::accent_ink))
        .flex_1()
        .min_w(px(0.0))
        .p_2()
        .gap_2()
        .items_stretch()
        .rounded(tokens::radius_card())
        .bg(tokens::panel_raised)
        .border_1()
        .border_color(tokens::border_subtle)
        .hover(|s| s.bg(tokens::panel_subtle))
        .active(|s| s.bg(tokens::control_hover))
        .transition_colors()
        .duration(tokens::ms(tokens::MOTION_INSTANT_MS))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(tokens::accent_ink))
        .child(
            view()
                .h(px(76.0))
                .flex_row()
                .overflow_hidden()
                .rounded(tokens::radius_row())
                .child(theme_preview(dark).flex_1())
                .child(follow_system.then(|| theme_preview(true).flex_1())),
        )
        .child(text(label).text_sm().text_color(tokens::fg_primary))
}

fn theme_preview(dark: bool) -> View {
    let sample =
        move |color: nsh_core::element::AdaptiveColor| if dark { color.dark } else { color.light };
    view()
        .flex_row()
        .bg(move || sample(tokens::shell_bg()))
        .child(
            view()
                .w(percentage(0.30))
                .bg(move || sample(tokens::shell_surface()))
                .border_r_1()
                .border_color(move || sample(tokens::shell_border())),
        )
        .child(
            view()
                .flex_1()
                .m_2()
                .rounded(tokens::radius_row())
                .bg(move || sample(tokens::panel_bg()))
                .border_1()
                .border_color(move || sample(tokens::border_subtle())),
        )
}
