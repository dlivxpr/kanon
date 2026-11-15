# Woday 官方资料调研

> 调研对象：`https://www.woday.app/` 及其直接链接的官方产品页、帮助中心、更新日志、隐私政策与官方截图素材。
> 调研日期：2026-11-15。站点响应头 `last-modified: Fri, 13 Nov 2026 12:47:06 GMT`，HTML 注释 `Published Nov 13, 2026, 12:46 PM UTC`。
> 用途：为 HTML 高保真原型提供产品、信息架构、文案与视觉依据。§1–11 为官网子任务调研，§12 为主 agent 补充的 `docs/temp/` 视频证据；未运行或反编译 Woday 应用本身。

## 证据等级

| 标记 | 含义 | 典型来源 |
|---|---|---|
| **A** | 官方产品页 / 帮助中心原文 | `woday.app`、`woday.app/docs/*` |
| **B** | 官方更新日志（版本相关，可能领先或滞后于文档） | `woday.app/updates/*` |
| **C** | 官方营销截图（第三方无法验证其是否等同当前版本） | `framerusercontent.com/images/*` |
| **D** | 本文推断 / 综合判断，非官方原文 | 明确标注 |

---

## 0. 结论摘要

1. **定位**：Woday 是 macOS 上的“日历 + 任务 + 笔记 + 提醒”四合一日程规划器，官方一句话是 “Tasks, Notes & Calendar. One beautiful Mac app.”，主标题是 “Your day, beautifully connected.”。来源：<https://www.woday.app/>
2. **核心差异化**：本地优先、无账号、无分析/遥测、一次性买断（限时 $19，原价 $39）、原生 macOS、可选 iCloud 私有同步、可选自带 API key 的 AI 助手。来源：<https://www.woday.app/>、<https://www.woday.app/privacy>、<https://www.woday.app/docs/icloud-sync>
3. **信息架构**：应用侧边栏固定为 `Today / Upcoming / Calendar / Notes / Tasks / Events`，其下是 `Favorites`、`Recent`、`Folders`、`Tags`；右侧常驻 `Inbox` 面板；顶部居中搜索框 `Search, or create anything...`。来源（官方截图）：<https://framerusercontent.com/images/1ehn5X8dKtbPfjrvYt7rqPMik.webp?width=2262&height=1484>
4. **组织模型**：`Folder` 可同时容纳笔记、任务、事件（v1.3）；`Tag` 有独立页面、图标与颜色；任务可关联事件、笔记、子任务、提醒、习惯 streak；笔记是 block 编辑器并可用 `@` 提及任务/事件/笔记。来源：<https://www.woday.app/updates/woday-1-3>、<https://www.woday.app/docs/writing-notes>、<https://www.woday.app/docs/tasks-reminders-habits>
5. **视觉语言**：营销站是“黑底 + 白色大圆角画布 + 超大紧排粗体 + 红色毛绒 mascot”；应用内是“近黑窗口外壳 + 暖白面板 + 浅灰行卡 + 红色时间强调 + 柔和彩色标签”。官方 FAQ 自己概括为 “clean black-and-white card design”。来源：<https://www.woday.app/>（FAQ `What is Woday?`）
6. **对原型最重要的一点**：Woday 的“项目管理”不是独立模块，而是由 **Folders（可含任务/事件/笔记）+ Tags（可开页面）+ 多视图（list / board / calendar）+ Inbox 捕获** 组合出来的。原型若做“笔记 + 项目 + 待办”，应优先复刻这套组织方式，而不是另造项目实体。依据：<https://www.woday.app/updates/woday-1-3>、<https://www.woday.app/updates/woday-1-2>、官方截图。

---

## 1. 来源清单与新鲜度

| 来源 | URL | 类型 | 本次用途 | 新鲜度 / 备注 |
|---|---|---|---|---|
| 官网首页 | <https://www.woday.app/> | A/C | 定位、卖点、完整文案、FAQ、定价、视觉 | Framer 构建；2026-11-13 发布 |
| 帮助中心索引 | <https://www.woday.app/docs> | A | 文档 IA、8 篇指南入口 | 无单独日期 |
| Getting started | <https://www.woday.app/docs/getting-started> | A | 首次使用流程、本地存储、无账号 | 无单独日期 |
| System requirements | <https://www.woday.app/docs/system-requirements> | A | macOS 26+、安装、自动更新 | 无单独日期 |
| Tasks, reminders and habits | <https://www.woday.app/docs/tasks-reminders-habits> | A | 任务/提醒/习惯 | 无单独日期 |
| Writing notes | <https://www.woday.app/docs/writing-notes> | A | block 编辑器、`@` 提及、AI 改写 | 无单独日期 |
| Calendar and reminders | <https://www.woday.app/docs/calendar-and-reminders> | A | Apple Calendar/Reminders、Google/Outlook 路径 | 无单独日期 |
| iCloud sync | <https://www.woday.app/docs/icloud-sync> | A | CloudKit、私有库、开关路径 | 无单独日期 |
| License | <https://www.woday.app/docs/license> | A | 授权、2 台 Mac、买断 | 无单独日期 |
| Getting help | <https://www.woday.app/docs/getting-help> | A | 支持邮箱、退款、反馈 | 无单独日期 |
| 更新日志列表 | <https://www.woday.app/updates> | B | v1.0.0 → v1.3 时间线 | 2026-11-11 最新条目 |
| v1.0.0 | <https://www.woday.app/updates/woday-1-0> | B | 首版范围 | 2026-09-01 |
| v1.1 | <https://www.woday.app/updates/woday-1-1> | B | AI、MCP、Todoist/Linear、导入 | 2026-11-06 |
| v1.2 | <https://www.woday.app/updates/woday-1-2> | B | 日期选择器、`#`/`@` token | 2026-11-07 |
| v1.2.2 | <https://www.woday.app/updates/woday-1-2-2> | B | Apple Notes 同步、Gemini/Grok | 2026-11-09 |
| v1.3 | <https://www.woday.app/updates/woday-1-3> | B | Folders/Tags、launcher、菜单栏、附件、双向 Reminders | 2026-11-11 |
| 隐私政策 | <https://www.woday.app/privacy> | A | 本地优先、CloudKit、Google API、AI、Polar、Sparkle | 2026-11-05 更新 |
| 使用条款 | <https://www.woday.app/terms> | A | 授权与使用边界 | 2026-11-02 更新 |
| Roadmap | <https://www.woday.app/roadmap> | A | 仅反馈入口，无排期 | 无日期 |
| Contact | <https://www.woday.app/contact> | A | 联系页 | 静态 HTML 未呈现表单正文 |
| 客户门户 | <https://polar.sh/woday/portal/> | A（外部官方） | 授权管理 | Polar.sh |
| 结账 | <https://buy.polar.sh/polar_cl_VHWbLrz3Y37Dy4iR0FFE9xHL0i4yeyfxJONtJ0boouD> | A（外部官方） | 购买 | Polar.sh / Stripe |
| 官方截图（Today + Inbox） | <https://framerusercontent.com/images/1ehn5X8dKtbPfjrvYt7rqPMik.webp?width=2262&height=1484> | C | 主界面布局与组件 | 营销素材，画面日期 2026-10-29 |
| 官方截图（Upcoming 列表） | <https://framerusercontent.com/images/ScV3adeYFJAsJeHMrSLAx2YfpU.webp?width=2262&height=1484> | C | 分组列表 | 同上 |
| 官方截图（Calendar 3 天） | <https://framerusercontent.com/images/Z9ruOPUEvZGUKpglFKqL1aaBdR0.webp?width=2262&height=1484> | C | 时间网格 | 同上 |
| 官方截图（Tasks + 详情） | <https://framerusercontent.com/images/kTSServawTKLwIVjsfagEwBixE.webp?width=2262&height=1484> | C | 任务列表与右侧详情 | 同上 |
| 官方截图（Note 详情） | <https://framerusercontent.com/images/HvzCAtJrDe7J90cFVmY96mRQDlo.webp?width=2262&height=1484> | C | 笔记封面/正文/任务/事件 | 同上 |
| 官方截图（Upcoming 看板） | <https://framerusercontent.com/images/IgxmC6pWAFGON3rqTDEmpqqyg2s.webp?width=2262&height=1484> | C | 按天分列看板 | 同上 |
| Mascot（派对） | <https://framerusercontent.com/images/yE97lEjNs3E0GTaqpBNDkfATiA.webp?width=1254&height=1254> | C | 品牌形象 | 1254×1254 |
| Mascot（基础） | <https://framerusercontent.com/images/SW9brpLtiEEvQRGEQawRhg5Zsw0.webp?width=1254&height=1254> | C | 品牌形象 | 1254×1254 |
| App 图标 | <https://framerusercontent.com/images/nPT05p7fGZHvLAjGddpyWcMw.webp?width=768&height=768> | C | 品牌标识 | 768×768 |

**未采用**：`docs/temp/` 下全部视频（按任务要求排除）；第三方评测、社交媒体、Mac App Store 页面（未发现官方链接）。

---

## 2. 产品定位

### 2.1 官方定义

- 浏览器标题：`Woday — All-in-One Calendar, Tasks & Notes App for Mac`。来源：<https://www.woday.app/>
- Meta description：`Meet Woday, a beautifully designed daily planner for Mac. Bring your calendar, tasks, reminders, and notes together in one place. Plan your day, your way.` 来源：<https://www.woday.app/>
- Hero eyebrow：`Tasks, Notes & Calendar. One beautiful Mac app.` 来源：<https://www.woday.app/>
- Hero H1：`Your day, beautifully connected.` 来源：<https://www.woday.app/>
- Hero 正文：`Meet Woday, your all-in-one planner for Mac. Bring your calendar, tasks, reminders, and notes together in one beautifully simple workspace. Plan your days, organize your life, and keep everything connected.` 来源：<https://www.woday.app/>
- FAQ 自述：`Woday is one Mac app for your tasks, calendar, and notes, so your to-dos, events, and writing live in one place instead of three. Built natively for macOS with a clean black-and-white card design.` 来源：<https://www.woday.app/>（FAQ 第 1 条）
- 隐私政策自述：`Woday is a life planner that combines to-dos, notes and a calendar in one macOS app.` 来源：<https://www.woday.app/privacy>

### 2.2 价值主张（官方文案归纳）

| 支柱 | 官方证据 | 来源 |
|---|---|---|
| 一处集中 | “one place instead of three”、`Bring your calendar, tasks, reminders, and notes together` | <https://www.woday.app/> |
| 原生 Mac | `Built natively for macOS`、`Native macOS app`、`requires macOS 26 (Tahoe) or later` | <https://www.woday.app/>、<https://www.woday.app/docs/system-requirements> |
| 本地与隐私 | `Your data stays private, synced through your own iCloud.`；无账号、无分析、无崩溃上报 | <https://www.woday.app/>、<https://www.woday.app/privacy> |
| 买断 | `One-time payment. No subscription.`；`$19`（原价 `$39`），终身访问与更新 | <https://www.woday.app/> |
| 轻量 AI | 设备端 Apple Intelligence 改写/摘要；可选自带 key 的聊天助手 | <https://www.woday.app/privacy>、<https://www.woday.app/updates/woday-1-1> |
| 可连接 | Apple Calendar/Reminders、Google/Outlook（经 macOS）、Todoist、Linear、Apple Notes、MCP | <https://www.woday.app/docs/calendar-and-reminders>、<https://www.woday.app/updates/woday-1-1>、<https://www.woday.app/updates/woday-1-2-2> |

### 2.3 目标用户（推断，标记 D）

依据产品文案中的 “plan your days / manage projects / keep track of tasks / capture your ideas” 与三栏工作区，可推断主力用户是：

- 已经在 Mac 上工作、希望把日历、待办、笔记从 2–3 个工具合并到一个窗口的知识工作者；
- 对订阅制、账号、云端同步敏感，偏好一次买断与本地数据的独立用户 / 苹果生态用户；
- 需要“文件夹 + 标签 + 多视图”管理中小型项目，但不想要 Notion 级复杂度的用户。

以上为**推断**，官网未给出用户画像、访谈或使用数据。来源基础：<https://www.woday.app/>。

### 2.4 竞争锚点

- 对 Notion 的锚定：笔记特性标签直接写 `Notion style notes`、`Blocks`。来源：<https://www.woday.app/>
- 对任务工具的锚定：`switch between list, Kanban board, and calendar views`、`Inbox`、`Upcoming`。来源：<https://www.woday.app/>
- 对“三件套”的锚定：`your to-dos, events, and writing live in one place instead of three`。来源：<https://www.woday.app/>（FAQ）
- 对订阅制的锚定：`One-time payment. No subscription.`。来源：<https://www.woday.app/>

### 2.5 不做什么（官方边界）

- 不做账号体系：`There is no sign-up, no login, no server of ours holding your content.` 来源：<https://www.woday.app/privacy>
- 不做分析/遥测/崩溃上报：`We do not collect analytics, telemetry, usage statistics or crash reports.` 来源：<https://www.woday.app/privacy>
- 不在 Mac App Store 销售：`Woday is sold outside the Mac App Store.` 来源：<https://www.woday.app/privacy>
- 暂无 iOS：`An iOS app is on the roadmap.`、`iOS app coming soon`。来源：<https://www.woday.app/docs/system-requirements>、<https://www.woday.app/>
- 不做团队协作/共享编辑：官方资料未出现相关能力（**不能据此断定永远没有，只表示本次来源未覆盖**）。

---

## 3. 信息架构

### 3.1 官网 IA

顶部导航（从左到右）：

`Woday`（logo）｜`Features`｜`FAQ`｜`Updates`｜`Pricing`｜`Download`（白色胶囊 + Apple 图标）

页脚三组：

- `Menu`：`Home`、`Features`、`FAQ`、`Pricing`、`Docs`、`Updates`
- `Navigation`：`Contact`、`Help & docs`、`Share feedback`、`Roadmap`、`Privacy policy`、`Terms of service`、`Customer portal`
- `More products`：`Yee.Site`、`Macapp.Supply`、`Supaste.com`、`Dock.Cool`、`Runey.app`、`Revone.app`、`Icoon.co`、`Selected.site`、`Supaframe.io`、`Frameblox.com`

补充：`Docs` 不在顶部主导航，只在页脚；`/roadmap` 页只有一句 `Help shape Woday` 与 `Send feedback`，没有公开排期表。来源：<https://www.woday.app/>、<https://www.woday.app/roadmap>

站点地图（可直接访问的官方页面）：

```
/
├── /docs
│   ├── /docs/getting-started
│   ├── /docs/system-requirements
│   ├── /docs/tasks-reminders-habits
│   ├── /docs/writing-notes
│   ├── /docs/calendar-and-reminders
│   ├── /docs/icloud-sync
│   ├── /docs/license
│   └── /docs/getting-help
├── /updates
│   ├── /updates/woday-1-0
│   ├── /updates/woday-1-1
│   ├── /updates/woday-1-2
│   ├── /updates/woday-1-2-2
│   └── /updates/woday-1-3
├── /roadmap
├── /contact
├── /privacy
└── /terms
```

来源：<https://www.woday.app/>、<https://www.woday.app/docs>、<https://www.woday.app/updates>

### 3.2 应用 IA（依据官方截图 + 文案）

**窗口级结构：三栏。**

| 区域 | 内容 | 证据 |
|---|---|---|
| 顶部工具栏 | macOS 红黄绿交通灯；侧栏开关；新建（方形+铅笔）；后退/前进；居中搜索 `Search, or create anything...`，右侧 `⌘K` 键帽 | 官方截图 |
| 左侧栏 | 一级导航：`Today`、`Upcoming`、`Calendar`、`Notes`、`Tasks`、`Events`；收藏：`Favorites`；最近：`Recent`；`Folders`（`Work`、`Personal`）；`Tags`（`Health`、`Ideas`、`Work`）；每组可折叠，`Folders`/`Tags` 右侧有 `+`；底部 `...` | 官方截图 |
| 中央主面板 | 当前视图（Today / Upcoming / Calendar / Tasks / Note 详情），白色大圆角面板 | 官方截图 |
| 右侧 Inbox | 常驻捕获区：`Inbox` 标题、`All` 筛选、列表/分栏切换、`New task` 行、分组 `Overdue` / `Inbox` / `Today`；笔记条目带 emoji 图标 | 官方截图 |

官方截图：<https://framerusercontent.com/images/1ehn5X8dKtbPfjrvYt7rqPMik.webp?width=2262&height=1484>

**应用内设置分区（文档中出现的路径）：**

- `Settings ▸ Notifications`：控制每种通知。来源：<https://www.woday.app/docs/tasks-reminders-habits>
- `Settings ▸ Sync`：开关 iCloud 同步。来源：<https://www.woday.app/docs/icloud-sync>
- `Settings ▸ License`：激活/停用授权。来源：<https://www.woday.app/docs/license>
- `Settings ▸ Connections ▸ Google Calendar`：连接/移除 Google 日历。来源：<https://www.woday.app/privacy>
- AI 助手与 API key：`Settings` 内开启/关闭并增删 key，具体子页名称官方未写明。来源：<https://www.woday.app/privacy>

**未在官方截图/文档中出现**：完整 Settings 窗口、Notes 列表页、Events 列表页、Quick Launcher 浮层、菜单栏面板、AI 聊天界面。原型中若需要，只能依据更新日志的文字描述做**合理重建**，并在标注中写明。

### 3.3 领域对象模型

| 对象 | 关键字段/行为 | 证据 |
|---|---|---|
| Task | 标题、备注/富文本、due date、`During date`（含义未解释）、priority、status、subtasks、tags、folder、reminder、recurrence/habit、streak、attachments、关联 event/note、完成历史 | <https://www.woday.app/docs/tasks-reminders-habits>、<https://www.woday.app/>、<https://www.woday.app/privacy> |
| Habit | 由任意 task 转为 recurring；按计划日重现；记录 streak；在 Today/Upcoming/Calendar 显示 flame 标记 | <https://www.woday.app/docs/tasks-reminders-habits> |
| Note | block 内容（heading、list、to-do、quote、divider、image）、cover、icon、folder、tags、`@` mentions、attachments、AI 改写/摘要 | <https://www.woday.app/docs/writing-notes>、<https://www.woday.app/updates/woday-1-3> |
| Event | 日期时间、来源日历、关联 tasks、关联 notes、folder、会议链接 | <https://www.woday.app/>、<https://www.woday.app/docs/writing-notes> |
| Folder | 可同时容纳 notes / tasks / events；有图标与名称；作为 badge 出现在条目上，可点击跳转；有独立页面，可卡片/列表展示并按类型、日期、tag 过滤分组 | <https://www.woday.app/updates/woday-1-3> |
| Tag | 名称、自定义图标、颜色；可点击进入 tag 页面；可作为筛选维度 | <https://www.woday.app/updates/woday-1-3> |
| Inbox | 未安排条目的默认捕获区；任务视图内也作为分组出现 | 官方截图、<https://www.woday.app/> |
| Attachment | 任意文件附到 note/task/event；Quick Look 预览；iCloud 同步；单文件上限 100 MB | <https://www.woday.app/updates/woday-1-3> |
| License | 邮箱收到的 key；一 license 一人最多 2 台 Mac；可停用旧设备；终身更新 | <https://www.woday.app/docs/license> |
| Connection | Apple Calendar、Apple Reminders、Google Calendar、Outlook（经 macOS）、Todoist、Linear、Apple Notes | 见 §4.8 / §4.9 |

**术语冲突（重要）**：官方同时使用 `project`、`list`、`folder` 三个词。v1.2 说 `Type # to insert a project or list`；v1.3 说 `Folders now hold tasks and events too, not just notes`。三者是否为同一实体的不同称呼，官方文档未说明。来源：<https://www.woday.app/updates/woday-1-2>、<https://www.woday.app/updates/woday-1-3>

### 3.4 视图矩阵

| 视图 | 已确认形态 | 证据 |
|---|---|---|
| `Today` | 标题 + 日期 `2026. October 29.` + 一周日期条 + 问候统计卡 + 当日任务列表 | 官方截图 |
| `Upcoming` | 两种形态：按天分组的列表（`Next 30 days`）；按天分列的看板（`Oct 29 – Nov 2`） | 官方截图 |
| `Calendar` | 3 天时间网格；`3 days` 下拉；`Today` 按钮；前后箭头；时间轴 8:00–18:00 | 官方截图 |
| `Tasks` | 分组列表（`Inbox` / `Overdue` / `Tomorrow` / `Sat, Oct 31`，每组显示任务数）+ 右侧任务详情面板；顶部 `Due date` 排序下拉、列表/看板/日历切换 | 官方截图 |
| `Notes` | 官方截图只展示单篇笔记详情，未展示库列表 | 官方截图 |
| `Events` | 仅在侧边栏出现，未见页面截图/文档说明 | 官方截图 |
| 视图切换 | Tasks 顶部有三个图标：列表、看板、日历；Upcoming 顶部有列表与看板 | 官方截图 |

> 不能确认每个视图都支持全部三种模式；上表只记录截图中可见的控件。

---

## 4. 核心功能与操作流

### 4.1 首次启动与五分钟上手

官方给出的首跑路径（A）：

1. 打开应用，无需注册账号；内容存储在本地 Mac。来源：<https://www.woday.app/docs/getting-started>
2. 添加一个任务并给它 due date。
3. 写一条快速笔记，并在笔记里 mention 这个任务。
4. 打开日历查看当天。
5. 如果有多台 Mac，在 Settings 打开 iCloud sync。

原文：`Add a task and give it a due date` / `Write a quick note and mention that task inside it` / `Open the calendar to see your day at a glance` / `Turn on iCloud sync in Settings if you use more than one Mac`。来源：<https://www.woday.app/docs/getting-started>

### 4.2 捕获：Inbox、搜索与 Quick Launcher

- **Inbox 常驻右栏**：顶部一行 `New task` 带空心圆，可直接输入；条目按 `Overdue`、`Inbox`、`Today` 分组。证据：官方截图。
- **顶部搜索即创建**：placeholder 为 `Search, or create anything...`，右侧 `⌘K`。证据：官方截图。
- **Quick Launcher（v1.3）**：快捷键 `⌥Space`；`Search everything, create tasks, events and notes`；输入 `>` 进入命令，`Tab` 打开 quick actions；输入 `#` 插入 tag、`&` 插入 folder、`@` 插入日期。来源：<https://www.woday.app/updates/woday-1-3>
- **菜单栏面板（v1.3）**：`⌃⌥Space`；三种样式 `classic dropdown`、可拖动的 `floating panel`、贴左/右屏幕边缘的 `docked full height`；面板含 `Today`、`Calendar`、`Tasks`、`Events`、`Notes` 五个 tab；可选“鼠标触边出现”。来源：<https://www.woday.app/updates/woday-1-3>

### 4.3 任务

官方流程（A）：

1. 新建任务。
2. 设置 due date、priority 或 subtasks。
3. 用 tags 与 folders 归类。
4. 可选：设置 reminder；Woday 在到期时发 **本地通知**，通知类型在 `Settings ▸ Notifications` 控制。
5. 可选：把任意任务变成 recurring habit；它在每个计划日重现，记录 streak，并在 `Today`、`Upcoming`、`calendar` 显示 flame 标记。

来源：<https://www.woday.app/docs/tasks-reminders-habits>

补充（B）：

- 任务可关联 events 或 notes；可加 rich notes；视图可在 list / Kanban board / calendar 间切换；`Inbox` 与 `Upcoming` 用于收拢未安排与近期任务。来源：<https://www.woday.app/>
- v1.3 起可把任务安排到“week / month / Someday”，而不是固定到某一天；它会待在自己的分组里。来源：<https://www.woday.app/updates/woday-1-3>
- 任务详情面板可见：完成圆点、标题、`5 in a row` 火焰徽章、`Daily · 12/100`、`Today, 7:30`、`Health` 标签、`+` 添加标签、`Write a subtask or a note...` 输入位。证据：官方截图。

### 4.4 笔记

官方流程（A）：

1. 新建笔记，使用 **block-based editor**：可插入 headings、lists、to-dos、quotes、dividers、images。
2. 设置 cover image 与 icon，便于在库中识别。
3. 输入 `@` mention 一个 task、event 或另一篇 note；mention 会变成链接。
4. 在支持 Apple Intelligence 的 Mac 上，可在编辑器内 rewrite / summarize；处理在设备端完成。

来源：<https://www.woday.app/docs/writing-notes>

补充（B）：

- v1.3：`Type / in the middle of a line in notes, not just on empty lines.`（slash 菜单可在行中触发）。来源：<https://www.woday.app/updates/woday-1-3>
- v1.3：可把任意文件附到 notes/tasks/events，带 Quick Look 预览，iCloud 同步，单文件上限 100 MB。来源：<https://www.woday.app/updates/woday-1-3>
- v1.2.2：Apple Notes 可经 iCloud 选择性同步到 Woday，转为原生 block（含 lists、checklists、tables、images），支持双向编辑。来源：<https://www.woday.app/updates/woday-1-2-2>

笔记详情截图可见结构（C）：

`封面图` → `emoji 图标 + 标题` → 元信息胶囊 `Created Oct 21, 2026` / `Edited now` / `Lisbon holiday`（folder）/ `Ideas`（tag）/ `+` → 分节标题 `Before we fly` → checklist 任务（含 due date、`5d left`）→ 分隔线 → `Flight` → 事件条目（红点 + `Lisbon holiday` + `Wed, Nov 4 6:40`）→ `Itinerary`。封面右上角有 5 个悬浮圆形按钮。

来源：<https://framerusercontent.com/images/HvzCAtJrDe7J90cFVmY96mRQDlo.webp?width=2262&height=1484>

### 4.5 日历与提醒

官方流程（A）：

1. 第一次打开 calendar，Woday 请求读取 Apple Calendar 与 Reminders 的权限；可随时在 `System Settings ▸ Privacy & Security` 更改。
2. Google / Outlook 日历只要已加入 macOS，就会自动出现；加入路径 `System Settings ▸ Internet Accounts`。
3. 在 Woday 勾选一个 Apple Reminder，Reminders app 中也会完成。

来源：<https://www.woday.app/docs/calendar-and-reminders>

补充：

- 日历视图支持 weekly、monthly、2/3/4 days；可把 tasks 加到 events、链接 notes、用 folders/tags 组织、同步 Apple Calendar、直接从 Woday join meetings。来源：<https://www.woday.app/>
- v1.3 起 Apple Reminders 为双向：可在 Woday 编辑、完成、组织，改动同步回 Reminders。来源：<https://www.woday.app/updates/woday-1-3>
- Google Calendar 另有**应用内直连**路径：`Settings ▸ Connections ▸ Google Calendar`，OAuth 授权；读取所选日历约“过去两周 + 未来三个月”，可创建/更新/删除仅由 Woday 创建的事件；token 存 macOS Keychain。来源：<https://www.woday.app/privacy>

### 4.6 文件夹、标签与“项目”

v1.3 原文（B）：

- `Folders now hold tasks and events too, not just notes.`
- `Folder pages show everything as cards or a list, with filters and grouping by type, date or tag.`
- `You'll also find new Folders and Tags pages in the sidebar menu.`
- `Click a tag's name to open its own page. You can also give any tag a custom icon and colour.`
- `Items filed into a folder show its icon and name everywhere they appear. Click the badge to jump straight to the folder.`
- `Sidebar, your way: Show, hide and reorder sections, now including Tasks and Events. Long sections get tidy "Show all" caps.`

来源：<https://www.woday.app/updates/woday-1-3>

**对原型的含义（D）**：如果目标是“笔记 + 项目管理 + 待办”，Woday 给出的映射是：

- 项目 ≈ Folder（可放 notes/tasks/events，独立页面，卡片/列表，过滤分组，badge 跳转）
- 分类 ≈ Tag（独立页面、颜色、图标）
- 任务 ≈ Task（可挂 folder/tag/event/note/subtask/habit）
- 收集箱 ≈ Inbox（右栏常驻）
- 多视图 ≈ list / board / calendar

### 4.7 搜索与命令

- 顶部搜索：`Search, or create anything...`，`⌘K`。证据：官方截图。
- Quick Launcher：`⌥Space`；`>` 命令、`Tab` quick actions、`#` tag、`&` folder、`@` date。来源：<https://www.woday.app/updates/woday-1-3>
- v1.2 另有行内 token：`#` 插入 project/list，`@` 插入 tag 或 person，从下拉选择后 token 落在当前行。来源：<https://www.woday.app/updates/woday-1-2>

### 4.8 同步与授权

- 路径：`Settings ▸ Sync` 打开 iCloud sync；同一 Apple Account 的其他 Mac 也打开后同步。
- 数据存于用户私有 iCloud 数据库（Apple CloudKit），官方无权访问；关闭同步则只在本机。
- 来源：<https://www.woday.app/docs/icloud-sync>
- 隐私政策补充：本地数据库、无服务器存内容、无分析；删除项先进入 Woday 自己的 Trash；授权 key 与设备名在激活/停用/周期校验时发给 Polar。来源：<https://www.woday.app/privacy>

### 4.9 连接、导入与 AI

| 能力 | 细节 | 来源 |
|---|---|---|
| Todoist | 双向同步 | <https://www.woday.app/updates/woday-1-1> |
| Linear | 连接 | <https://www.woday.app/updates/woday-1-1> |
| Apple Reminders | 作为真正的双向任务导入/编辑 | <https://www.woday.app/updates/woday-1-1>、<https://www.woday.app/updates/woday-1-3> |
| Apple Notes | 选择文件夹同步，转原生 block，双向编辑 | <https://www.woday.app/updates/woday-1-2-2> |
| 导入 | CSV、Excel、Markdown、PDF | <https://www.woday.app/updates/woday-1-1> |
| MCP server | 本地运行，可接任意 MCP client | <https://www.woday.app/updates/woday-1-1> |
| AI 聊天助手 | 自带 Claude / ChatGPT key；可查询、创建、编辑任务/笔记/日历；默认关闭，仅发送请求时运行 | <https://www.woday.app/updates/woday-1-1>、<https://www.woday.app/privacy> |
| AI provider 扩展 | Gemini、Grok 加入，自带 key；`Auto AI` 按消息选择模型 | <https://www.woday.app/updates/woday-1-2-2> |
| 设备端 AI | Apple Intelligence 在笔记内 rewrite / summarize，内容不出设备 | <https://www.woday.app/docs/writing-notes>、<https://www.woday.app/privacy> |

**注意**：这是两套 AI 能力——(1) 设备端 Apple Intelligence 的改写/摘要；(2) 可选、自带 API key 的聊天助手 + MCP。FAQ 只描述了 (1)，更新日志描述 (2)，两者并不冲突，但 FAQ 未覆盖新版本。（见 §10）

---

## 5. 页面文案（逐字）

> 以下保留英文原文、标点与官方拼写错误（如 `Qucik search`）。中文仅用于小标题说明。

### 5.1 首页

**导航**

```
Woday
Features
FAQ
Updates
Pricing
Download
```

**Hero**

```
Tasks, Notes & Calendar. One beautiful Mac app.
Your day, beautifully connected.
Meet Woday, your all-in-one planner for Mac. Bring your calendar, tasks, reminders, and notes together in one beautifully simple workspace. Plan your days, organize your life, and keep everything connected.
★★★★★  Used by 200+ mac users
Get Woday for Mac
One-time payment    Local and privacy    macOS Tahoe 26.0 or later
```

**价值条**

```
Woday connects your events, notes, tasks, and reminders in one clear, organized place.
Calendar    Notes    Reminders
Your data stays private, synced through your own iCloud.
```

**功能索引**

```
Everything you need, together.
One app. Endless possibilities.
Your calendar, tasks, notes, and reminders finally work together. Everything you need to plan your days, stay organized, and get things done, all in one place.

Reminder
Plan your day
Connections
Inbox
Repeat as habit
```

**居中宣言**

```
Make every day feel a little more organized. Plan your schedule, manage projects, keep track of tasks, and capture your ideas, all in one beautifully designed Mac app that brings everything together.
```

**Calendar & Events**

```
More than just a calendar.
Your events are connected to everything that matters. Add tasks to events, link notes, organize with folders and tags, and sync with Apple Calendar. Join meetings right from Woday without missing a beat.

Weekly, monthly view
2, 3, 4 days view
Everything in your calendar
Join meetings
```

**Tasks & Habits**

```
To-dos that do so much more.
Break big ideas into subtasks, set reminders, add rich notes, and connect tasks to events or other notes. Turn recurring tasks into habits, organize with priorities, statuses, and tags, and switch between list, Kanban board, and calendar views. Keep track of everything with Inbox and Upcoming.

List view
Board view
Habits
Set reminder
Subtasks
Due date
During date
Priority
```

**Notes & Documents**

```
A place for every thought.
Capture ideas in a beautiful, intuitive block-based editor. Create tasks, subtasks, events, and even other notes directly inside your pages. Keep everything organized with folders, tags, custom icons, and beautiful covers.

Notion style notes
Blocks
Beautiful notes
AI summary
AI rewriter
Add events, tasks
Tags, folders
```

**Quick Launcher**

```
A shortcut to everything.
Search anything, create tasks, schedule events, or capture notes in seconds. Just press a keyboard shortcut, type, and you're done. No interruptions, no switching windows.

Quick access
Qucik search
Create tasks, events, notes
```

**定价**

```
One price. Lifetime access.
One-time payment. No subscription.
Get lifetime access to Woday on your Mac.
Try it risk-free. If Woday doesn't fit your workflow, email us within 14 days and we'll refund your purchase.

Woday app for macOS
$19  $39
Limited offer for early users 🥳
14-day money-back guarantee
Lifetime access & updates
All features unlocked from day one
Native macOS app
iOS app coming soon
2 device licenses
Get Woday for Mac

Secure checkout by Polar.sh, powered by Stripe.
Prices are in USD, excluding VAT and may vary by location.
```

**收尾**

```
Make room for a better day.
One beautiful place for everything on your mind and everything on your schedule. Meet your new favorite Mac app.
```

**FAQ 区**

```
Frequently Asked Questions
Meet Woday, your all-in-one planner for Mac. Bring your schedule, tasks, and notes together and make every day feel effortless.
Have a Question?
```

**Featured**

```
Featured on
```

**页脚**

```
Woday
Your day, beautifully connected.
Tasks, Notes & Calendar. One beautiful Mac app.
Download Woday macOS app
© 2026 Woday.app - All rights reserved
Built with 💙 by Solt Wagner
```

来源：<https://www.woday.app/>（页面文本与页面 bundle 双重核对）

### 5.2 FAQ 全部问答（官方原文）

1. **What is Woday?**
   `Woday is one Mac app for your tasks, calendar, and notes, so your to-dos, events, and writing live in one place instead of three. Built natively for macOS with a clean black-and-white card design.`

2. **Which macOS version do I need? Is there an iPhone app?**
   `Woday requires macOS 26 (Tahoe) or later. An iOS app is on the roadmap.`

3. **Where is my data stored? Is it private?**
   `Your data lives locally on your Mac. No account to create, and your tasks, events, and notes never touch our servers. Optional iCloud sync uses your own private iCloud account, which we can't access.`

4. **Does it sync across my Macs?**
   `Yes. Turn on iCloud sync in Settings and everything stays in sync across your Macs.`

5. **Does it work with Apple Calendar, Reminders, Google, or Outlook?**
   `Yes. Woday has two-way sync with Apple Calendar and Reminders. Google and Outlook calendars added to macOS (System Settings › Internet Accounts) show up too.`

6. **What can the AI do, and is it private?**
   `Woday uses on-device Apple Intelligence to rewrite and summarize text in your notes. Nothing leaves your Mac. On Macs without Apple Intelligence, everything else works the same.`

7. **How much does it cost?**
   `Woday is a one-time purchase, not a subscription. For a limited time it's $19, which gets you lifetime access and lifetime updates. The price will go up after the launch period.`

8. **How many Macs can I use it on?**
   `One license covers one person on up to 2 Macs. Updates arrive right in the app at no extra cost, and your license works forever.`

9. **Is there a free trial?**
   `Not during the beta, but every purchase has a 14-day money-back guarantee. A free trial arrives with 1.0.`

10. **How do I get help?**
    `Email soltwagner@gmail.com and you'll hear back from the developer directly.`

来源：<https://www.woday.app/>（展开 FAQ 后读取）；第 3–10 条同时存在于首页 JS bundle，答案与展开态一致。

### 5.3 帮助中心文案

| 页面 | 标题 / 关键句 |
|---|---|
| `/docs` | `Woday Docs`；`Short, practical guides for planning your day with Woday, from your first task to iCloud sync and your license.`；`Can't find what you're looking for? Email soltwagner@gmail.com and you'll hear back from the developer directly.` |
| Getting started | `Getting started with Woday`；`A quick tour of Woday and how tasks, notes and your calendar fit together in one Mac app.` |
| System requirements | `System requirements and installation`；`Woday requires macOS 26 (Tahoe) or later. It's built natively for macOS with Apple's latest frameworks, so it feels fast and at home on your Mac.` |
| Tasks | `Tasks, reminders and habits`；`Set a reminder on any task and Woday will send a local notification on your Mac when it's due.` |
| Notes | `Writing notes`；`Type @ to mention a task, an event or another note. Mentions become links, so your writing stays connected to the work it describes.` |
| Calendar | `Connecting Apple Calendar and Reminders`；`Google and Outlook calendars show up too, as long as they're added to macOS.` |
| iCloud | `Syncing with iCloud`；`Synced data is stored in your private iCloud database through Apple's CloudKit. We have no access to it.` |
| License | `Activating and managing your license`；`One license covers one person on up to 2 Macs.` |
| Help | `Getting help`；`Every purchase comes with a 14-day money-back guarantee.` |

来源：<https://www.woday.app/docs>、<https://www.woday.app/docs/getting-started>、<https://www.woday.app/docs/system-requirements>、<https://www.woday.app/docs/tasks-reminders-habits>、<https://www.woday.app/docs/writing-notes>、<https://www.woday.app/docs/calendar-and-reminders>、<https://www.woday.app/docs/icloud-sync>、<https://www.woday.app/docs/license>、<https://www.woday.app/docs/getting-help>

### 5.4 应用内微文案（从官方截图转录，C）

```
Search, or create anything...
Today
Upcoming
Calendar
Notes
Tasks
Events
Favorites
Recent
Folders
Tags
Welcome to Woday
Work
Personal
Health
Ideas
Inbox
All
New task
Overdue
Today
Tomorrow
2026. October 29.
Good evening, you have
2 events
1 daily habit
and
2 important
Due date
5 in a row
Daily · 12/100
Today, 7:30
Write a subtask or a note...
Created Oct 21, 2026
Edited now
Lisbon holiday
Before we fly
Check in online
Print the hotel reservation
Pack camera + charger
Buy travel insurance
5d left
Flight
Wed, Nov 4 6:40
Itinerary
Next 30 days
Oct 29 – 31
3 days
18h left
17h
2d left
3d left
```

来源：<https://framerusercontent.com/images/1ehn5X8dKtbPfjrvYt7rqPMik.webp?width=2262&height=1484>、<https://framerusercontent.com/images/ScV3adeYFJAsJeHMrSLAx2YfpU.webp?width=2262&height=1484>、<https://framerusercontent.com/images/Z9ruOPUEvZGUKpglFKqL1aaBdR0.webp?width=2262&height=1484>、<https://framerusercontent.com/images/kTSServawTKLwIVjsfagEwBixE.webp?width=2262&height=1484>、<https://framerusercontent.com/images/HvzCAtJrDe7J90cFVmY96mRQDlo.webp?width=2262&height=1484>、<https://framerusercontent.com/images/IgxmC6pWAFGON3rqTDEmpqqyg2s.webp?width=2262&height=1484>

---

## 6. 视觉语言

### 6.1 营销站

#### 色彩

| Token | 值 | 用途 | 证据 |
|---|---|---|---|
| 黑 | `#000000` | 页面底色、顶栏、正文大字 | CSS token `--token-46f50c96…`；首页 |
| 白 | `#FFFFFF` | 白色画布、黑底上的文字 | CSS token `--token-3057ff86…` |
| 浅灰面 | `#F5F5F5` | 特性胶囊、次级面 | CSS token `--token-a14268e4…` |
| 浅灰面 2 | `#F7F7F7` | 次级浅面 | CSS token `--token-bbe2f5a4…` |
| 中灰 | `#757575` | 辅助文字 | CSS token `--token-6fbbe837…` |
| 正文灰 | `#666666` | 白底正文 | 样式预设 `1ti0tld` |
| 链接蓝 | `rgb(0, 153, 255)` | 链接、强调 | 页面 `--framer-link-text-color` |
| 品牌红 | 约 `#FA4A31` | logo、mascot、选中日期、时间/事件点 | 官方截图取样（C） |
| 暖白 | 约 `#FCFBFA` | 应用主面板 | 官方截图取样（C） |
| 外壳黑 | 约 `#0B0B0D` | 应用窗口/侧栏 | 官方截图取样（C） |

> 应用内色值是**对官方营销截图做像素取样**得到的近似值，不是官方设计 token；营销站色值来自站点 CSS。

#### 字体与字号

- 字体族：`Inter` 与 `Inter Display`，`@font-face` 指向 `framerusercontent.com/assets/*.woff2`。来源：<https://www.woday.app/>（页面 CSS）
- 展示级 H1：`80px`（≤1199px 为 `64px`，≤809px 为 `40px`），`line-height:1.05em`，`letter-spacing:-0.04em`，`font-weight:700`，白色/黑色。来源：样式预设 `99muhf`
- 区块级大标题：`60px`（`44px` / `32px`），`line-height:1.08em`，`letter-spacing:-0.04em`，`700`。来源：样式预设 `z1xzpl`
- 卡片/子标题：`30px`（`24px` / `19px`），`line-height:1.4em`，`letter-spacing:-0.04em`，`700`。来源：样式预设 `k92dzx`
- 正文大号：`20px`，`line-height:1.6em`，`400`，`#666`；另有 `18px` 版本。来源：样式预设 `1ti0tld`
- 小号正文：`16px`，`line-height:1.5em`，`400`，`#666`。来源：样式预设 `1xjdtk9`
- 导航/按钮：`16px`、`600`、`letter-spacing:-0.02em`。来源：样式预设 `xvrsu4`
- FAQ 问题：`18px`、`700`、`letter-spacing:-0.04em`、`line-height:1.4em`、白色。来源：首页 FAQ 内联样式

#### 布局

- 内容最大宽度 `1200px`；桌面左右各 `40px` 内边距（`calc(min(max(100vw, 1px), 1200px) - 80px)`）；平板为 `- 48px`。来源：页面 bundle 的 `overrides`
- 黑色页面外壳 + 白色大圆角内容画布；画布之间有黑色间隔，形成“卡片章节”节奏。证据：官方截图与首页。
- 顶栏 sticky，黑色，左侧 logo + `Woday`，中部导航，右侧白色胶囊 `Download`。证据：官方截图。
- 首屏：白画布内居中排版，eyebrow 16px 灰、H1 两行 80px、正文 20px 灰、五星 + `Used by 200+ mac users`、黑色胶囊 CTA、底部三枚灰度 meta。证据：官方截图。

#### 圆角体系（站点 CSS）

| 值 | 用途 |
|---|---|
| `20px` | FAQ 折叠卡 |
| `30px` | 功能卡 / 应用截图框 |
| `60px` | 大圆角容器（白色画布/窗口） |
| `120px` | 胶囊按钮（如 `Have a Question?`） |
| `14px` | 小卡片/局部容器 |

来源：<https://www.woday.app/>（页面 CSS）；FAQ 20px 同时见内联样式。

#### 组件

- **胶囊按钮**：白底黑字（顶栏 `Download`）、黑底白字（`Get Woday for Mac`）、深灰底白字（`Have a Question?`）；圆角 120px；高度约 48–56px。
- **特性胶囊**：浅灰 `#F5F5F5` 底、黑字、圆角 999px、`16px/600`，可换行成两行。
- **FAQ 折叠卡**：`rgba(255,255,255,0.05)` 底、圆角 20px、白色 18px 粗体问题、左侧 `+`（opacity 0.2），展开后变 `×`，答案灰色。证据：官方截图。
- **应用截图框**：黑色 macOS 窗口，圆角 30px，顶部工具栏，三栏内容；在白色画布中常以“贴边裁切”方式出现。
- **定价卡**：白色圆角约 24–30px，居中，左对齐 check 列表，底部黑色胶囊 CTA；价格 `$19` 80px、原价 `$39` 30px 删除线。
- **奖项/Featured**：灰度徽章与文字，位于 FAQ 下方。

#### 图形与品牌

- Logo：红色圆角方块内的猫脸负空间图形，旁边白色 `Woday` 字标。来源：<https://framerusercontent.com/images/nPT05p7fGZHvLAjGddpyWcMw.webp?width=768&height=768>
- Mascot：红色毛绒、心形/双瓣身体、两只白色椭圆眼；多种姿势（戴墨镜、戴眼镜、派对帽、吹卷哨、惊讶）。来源：<https://framerusercontent.com/images/yE97lEjNs3E0GTaqpBNDkfATiA.webp?width=1254&height=1254>、<https://framerusercontent.com/images/SW9brpLtiEEvQRGEQawRhg5Zsw0.webp?width=1254&height=1254>
- 图像风格：3D 渲染、软阴影、纯白或纯黑背景；应用截图本身是产品 UI，不使用插画。
- 页脚：巨型 mascot 特写从底部裁切露出，形成品牌记忆点。证据：官方截图。

#### 动效

- 元素初始 `opacity:0; transform:translateY(30px)`，进入视口后动画到 `opacity:1; transform:none`；Framer `animateOnce`、`threshold .5`。来源：<https://www.woday.app/>（页面源码与 bundle）
- 导航/按钮 hover 以轻微透明度或底色变化为主；未观察到夸张位移。
- 页面本身的滚动叙事依赖 sticky 与大图切换，但单个组件的动效克制。

### 6.2 应用 UI

> 以下基于官方营销截图（C），不是从应用运行时提取；分辨率 2262×1484。

#### 色彩

| 角色 | 近似值 | 用法 |
|---|---|---|
| 窗口/侧栏底 | `#0B0B0D` | 三栏之外的深色底 |
| 搜索框底 | `#1C1C1C` | 顶部居中搜索 |
| 主面板 | `#FCFBFA` | Today / Tasks / Calendar / Note 面板 |
| 行卡 | `#F8F7F6` | 任务行、事件块、分组卡 |
| 主文字 | `#1A1A1A` | 标题、任务名 |
| 次文字 | `#747674` | 日期、元信息 |
| 弱文字 | `#9A9A9A` | 分组标题、占位 |
| 红强调 | `#FA4A31` | 选中日期、时间、事件点、习惯徽章 |
| 绿 | `#53C26D` | `Health` 标签文字/状态 |
| 浅绿底 | `#E4E9DF` | `Health` 标签底 |
| 黄 | `#F7C430` | 优先级/重要标记、勾选圈 |
| 浅蓝灰 | `#E4E6EE` | `Work` 标签底 |
| 浅紫 | 约 `#EDE9FE` | `Ideas` 标签底（截图取样，值不精确） |

> 除主色外，标签底色为低饱和 pastel；具体 hex 受截图压缩影响，原型可用同色系近似。

#### 字体

- 观感为 macOS 系统字体（SF Pro 系列），不是网站用的 Inter；标题字重约 600–700，正文 400–500。
- 官方未声明应用字体文件，**不能确认**具体字体名称；原型建议用 `-apple-system, BlinkMacSystemFont, "SF Pro Text", "Helvetica Neue", sans-serif`。

#### 布局比例（按 2262×1484 截图估算，D）

- 顶部工具栏：约 `56px` 高；交通灯直径约 `12px`，间距 `8px`。
- 左侧栏：约占窗口宽 `17%`；一级项高约 `34px`，选中项为深灰胶囊（`#2A2A2C` 左右），图标 `18px`，文字 `13–14px`。
- 中央面板：约占 `43–45%`；内边距约 `24px`；面板圆角约 `20px`。
- 右侧 Inbox：约占 `23–24%`；与中央面板同高，独立白色圆角面板。
- 面板间距约 `16–20px`；整体留黑边约 `16px`。
- 列表行高约 `44–48px`；行卡圆角约 `10–12px`；行间距约 `8px`。

#### 组件

- **侧栏项**：图标 + 标签；选中态深灰胶囊；分组标题 `Favorites`、`Recent`、`Folders`、`Tags` 为 11–12px 灰字，带折叠 chevron；`Folders`/`Tags` 右侧 `+`。
- **搜索框**：黑色胶囊，左放大镜，placeholder 灰，右侧 `⌘K` 键帽。
- **面板头**：大标题（Today / Upcoming / Calendar / Tasks / Inbox）约 30–34px；副标题 12–13px 灰；右侧控件为小胶囊（`Today`、`3 days`、`Due date`、`All`）与图标 segmented control。
- **日期条**：7 列，星期缩写 + 日期数字；选中日为红色数字 + 红色星期 + 下方短横线。
- **问候卡**：浅灰圆角卡，文案 `Good evening, you have` + 三个胶囊（红/灰/黄），用于当日概览。
- **任务行**：左侧圆形 checkbox（空心彩色描边 / 完成时黑底白勾），标题 15px；第二行元信息（日期、时间、标签胶囊）；完成后标题灰色删除线。
- **标签胶囊**：11px，pastel 底 + 深色同色系文字，圆角 999px。
- **事件条目**：红色圆点 + 标题 + 时间；在笔记中作为嵌入块出现。
- **笔记封面**：整宽图片，圆角约 12px；右上角悬浮圆形工具按钮（换封面、图标、设置、更多、侧栏）。
- **笔记元信息**：浅灰胶囊 + 图标（`Created …`、`Edited now`、folder、tag），末尾圆形 `+`。
- **任务详情面板**：顶部 checkbox、展开、更多、关闭；标题 24px；红色 `5 in a row` 火焰徽章、灰色 `Daily · 12/100`、时间胶囊、绿色 `Health`、圆形 `+`；底部输入 `Write a subtask or a note...`。
- **看板列**：列头 `Today`/`Friday` + 日期 + 进度环；列内卡片为浅灰圆角块，含标题、时间、标签、`18h left`、文件夹 badge。

#### 图标与图像

- 线性 SF Symbols 风格图标：太阳（Today）、日历（Upcoming/Calendar）、文档（Notes）、勾选方块（Tasks）、日历条（Events）、文件夹、标签、星星（Favorites）。
- 笔记/文件夹使用 emoji 作为彩色标识（📍、🚀、💡、🥳 等），是信息密度中的重要视觉锚点。
- 空状态/引导内容：Inbox 中有 `Welcome to Woday`、`Everything Woday can do` 等示例条目，带 emoji 与 tag。

---

## 7. HTML 高保真还原规格

> 本节是**综合建议（D）**，数值来自 §6 的官方 CSS 与截图测量；不是 Woday 官方设计规范。

### 7.1 Design tokens（建议直接用于原型）

```css
:root {
  /* marketing */
  --ink: #000;
  --paper: #fff;
  --surface: #f5f5f5;
  --surface-2: #f7f7f7;
  --text-muted: #666;
  --text-faint: #757575;
  --link: #0099ff;

  /* app (sampled from official screenshots) */
  --app-shell: #0b0b0d;
  --app-panel: #fcfbfa;
  --app-row: #f8f7f6;
  --app-search: #1c1c1c;
  --app-text: #1a1a1a;
  --app-muted: #747674;
  --app-faint: #9a9a9a;
  --app-accent: #fa4a31;
  --app-green: #53c26d;
  --app-green-soft: #e4e9df;
  --app-yellow: #f7c430;
  --app-blue-soft: #e4e6ee;

  /* type */
  --font-display: "Inter Display", "Inter", sans-serif;
  --font-marketing: "Inter", sans-serif;
  --font-app: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Helvetica Neue", sans-serif;

  /* scale */
  --display-1: clamp(40px, 6vw, 80px);
  --display-2: clamp(32px, 4.5vw, 60px);
  --h-card: 30px;
  --body-lg: 20px;
  --body: 18px;

  /* geometry */
  --max: 1200px;
  --gutter: 40px;
  --r-faq: 20px;
  --r-frame: 30px;
  --r-canvas: 60px;
  --r-pill: 120px;
}
```

排版基线：

```css
.display-1 {
  font: 700 var(--display-1)/1.05 var(--font-display);
  letter-spacing: -0.04em;
}
.display-2 {
  font: 700 var(--display-2)/1.08 var(--font-display);
  letter-spacing: -0.04em;
}
.body-lg {
  font: 400 var(--body-lg)/1.6 var(--font-marketing);
  color: var(--text-muted);
}
```

### 7.2 营销页复刻蓝图

1. **页面底**：`background:#000`；所有章节在 `max-width:1200px` 内，左右 `40px`；移动端 `24px`。
2. **顶栏**：`position:sticky; top:0`；高约 `72px`；左 logo + `Woday`，中间三个链接，右侧白色胶囊 `Download`；黑色底。
3. **Hero 白画布**：外层黑底，内层白底、`border-radius:40–60px`、上下留白约 `96–120px`；内容居中，最大宽约 `900px`。
4. **Hero 排版**：eyebrow 16px `#666`；H1 两行 80px；正文 20px `#666`，最大宽 `760px`；CTA 黑色胶囊；底部三枚 meta 用 `·` 或 `16px` 间距分隔。
5. **价值条**：白底，居中；一句 24px 左右的黑字；三个 app icon（Calendar/Notes/Reminders）横排；底部灰色 iCloud 行。
6. **功能索引**：白底大圆角画布；eyebrow `Everything you need, together.`；H2 `One app. Endless possibilities.`；灰色正文；五个浅灰胶囊；下方嵌入黑色应用窗口大图（宽约 `1100px`，圆角 30px）。
7. **宣言段**：回到黑底，白色居中大字（约 44–52px），行高 1.15，最大宽 `1000px`。
8. **功能区块**：白底大圆角画布；左文右图或右文左图；左侧 eyebrow 18px 灰、H3 60px、正文 20px 灰、胶囊标签组、mascot；右侧应用截图贴边裁切。四个区块依次为 Calendar & Events、Tasks & Habits、Notes & Documents、Quick Launcher。
9. **定价**：黑底；H2 60px 白色两行；说明 20px 灰；白色定价卡（圆角 30px，宽约 `620px`）；`$19` 80px + `$39` 30px 删除线；check 列表；黑色 CTA；底部小字。
10. **FAQ**：黑底；左侧 40% 标题 + 说明 + 深灰胶囊 + mascot；右侧 60% 十条折叠卡（圆角 20px，背景 `rgba(255,255,255,.05)`）。
11. **页脚**：黑底；四列链接；底部巨型 mascot 特写；版权与作者。

### 7.3 应用外壳复刻蓝图

建议画布：`1440 × 900`（或 1512×982 MacBook 逻辑分辨率），按以下网格：

```css
.app {
  width: 1440px;
  height: 900px;
  background: #0b0b0d;
  display: grid;
  grid-template-rows: 56px 1fr;
}
.toolbar { display: flex; align-items: center; gap: 16px; padding: 0 20px; }
.workspace {
  display: grid;
  grid-template-columns: 220px minmax(0, 1fr) 300px;
  gap: 18px;
  padding: 0 18px 18px;
}
.sidebar { color: #d7d7d7; }
.panel { background: #fcfbfa; border-radius: 20px; padding: 24px; }
```

关键比例：

- 侧栏 `220–240px`；主面板 `1fr`；Inbox `290–310px`。
- 面板内边距 `24px`；面板间距 `18px`；行卡圆角 `10–12px`；行高 `44–46px`。
- 顶部搜索框宽 `420–460px`、高 `36px`、圆角 999px、底色 `#1c1c1c`。
- 侧栏一级项高 `34px`，圆角 `10px`；选中底色 `#2a2a2c`。
- 面板标题 `30–32px/600`；副标题 `13px` 灰；控件高 `30–32px`。

### 7.4 五个屏幕的复刻配方

#### A. Today（默认首页）

```
[面板头] Today                          [Today] [‹] [›]
         2026. October 29.
[日期条] 29 30 31 1 2 3 4 5 6 7 8 9
         Thu Fri Sat Sun Mon Tue Wed Thu Fri Sat Sun Mon
[问候卡] Good evening, you have [2 events] [1 daily habit] and [2 important]
[列表]
  ●  Reply to Anna about the kickoff
     Thu, Oct 29  16:00   [Work]
  ○  Pushups ×100
     🔥 Thu, Oct 29 7:30   [Health]
  ◐  Review Mark's pull request
     Thu, Oct 29 17:00   [Work]
  ✓  ~~Make pasta for dinner~~
     Thu, Oct 29
```

要点：日期条选中日红色；问候卡浅灰底；复选框颜色与标签色对应；完成任务删除线 + 灰字。

#### B. Upcoming

- 头部：`Upcoming` + `Next 30 days`；右侧列表/看板/日历三图标。
- 列表：按天分组，组头左 `Today` 红色 + `Oct 29` 灰；行卡浅灰，含标题、时间、标签；`18h left` + 时钟 icon。
- 看板：按天分列，列头 `Today / Friday / Saturday / Sunday` + 日期 + 右上进度环；每列卡片垂直排列。

#### C. Calendar（3 天）

- 头部：`Oct 29 – 31` + `2026`；`3 days` 胶囊；列表图标；`Today` 胶囊；前后箭头；右侧侧栏图标。
- 网格：3 列日期 + 时间轴（8:00–18:00，整点标签）；列头 `Today 29`（红）/`Fri 30`/`Sat 31`，右侧小进度环。
- 事件/任务卡：浅灰圆角块，红点 + 标题 + 时间；任务带 checkbox。

#### D. Tasks + 详情

- 主面板：`Tasks` + 右侧 `Due date` 排序、列表/看板/日历切换。
- 分组：`Inbox`（`2 tasks`）、`Overdue`（`3 tasks`）、`Tomorrow`（`2 tasks`）、`Sat, Oct 31`（`2 tasks`）。
- 行：checkbox、标题、日期/时间、标签；选中行有深色描边（截图里 `Pushups ×100`）。
- 右侧详情：大 checkbox、标题、`5 in a row` 红胶囊、`Daily · 12/100` 灰胶囊、`Today, 7:30`、`Health`、`+`、`Write a subtask or a note...`。

#### E. Note 详情

- 封面图（渐变红/粉），圆角 12px，高约 200px；右上角 5 个悬浮圆形按钮。
- 标题行：emoji + 32–36px 标题。
- 元信息胶囊行。
- 正文：分节标题（20px/700）+ checklist 行（checkbox + 标题 + 日期 + `5d left`）+ 分隔线 + 事件块（红点 + 标题 + 时间）+ 下一节标题。
- 已完成项删除线。

### 7.5 内容 fixture（直接复用官方截图数据）

- 用户/项目：`Work`、`Personal`、`Lisbon holiday`。
- 标签：`Health`（绿）、`Work`（蓝）、`Ideas`（紫）。
- 任务：`Pushups ×100`（Daily · 12/100，5 in a row）、`Reply to Anna about the kickoff`、`Review Mark's pull request`、`Renew car insurance`、`Water the plants`、`Buy groceries for the week`、`Write release notes`、`Prepare launch slides`。
- 事件：`Design review · Anna` 10:00、`Dinner with Pete` 19:00、`Morning run` 9:00、`Team lunch` 12:30、`Launch review` 14:00。
- 笔记：`Weekend in Lisbon`、`Product launch plan`、`App ideas`、`Everything Woday can do`、`Welcome to Woday`。
- 时间文案：`Good evening, you have`、`18h left`、`5d left`、`Next 30 days`、`2026. October 29.`。

### 7.6 Do / Don't

**Do**

- 用“白面板 + 浅灰行卡 + 细线分隔”建立层次，而不是重阴影。
- 用红色只标记时间/日期/事件/习惯等重要信息；标签用低饱和 pastel。
- 保持三栏：导航、主视图、Inbox；Inbox 是产品灵魂之一。
- 让文件夹承担“项目”角色，并给 folder badge 与独立页面。
- 让任务、事件、笔记可以互相嵌入/链接。
- 营销页用超大紧排标题与大面积黑白对比；app UI 用暖白与深色外壳对比。

**Don't**

- 不要把应用内 UI 做成营销页的纯黑白；应用是“深色外壳 + 暖白面板 + 彩色标签”。
- 不要给每个卡片加大阴影或粗边框；官方几乎不用阴影。
- 不要发明 Woday 没有确认的协作、共享、Web 端、移动端界面。
- 不要把 `project`、`list`、`folder` 当成三个已定义的独立对象；官方术语未统一。
- 不要声称 AI 默认开启；官方明确默认关闭。

---

## 8. 版本演进

| 日期 | 版本 | 类型 | 关键内容 | 来源 |
|---|---|---|---|---|
| 2026-09-01 | v1.0.0 | Major | 首个公开版：tasks（due date/reminder/priority）、rich notes（images/links/covers）、Apple Calendar/Reminders 日历、菜单栏快速捕获 | <https://www.woday.app/updates/woday-1-0> |
| 2026-11-06 | v1.1 | Feature | AI assistant（自带 Claude/ChatGPT key）、内置 MCP server、Todoist 双向、Linear、Apple Reminders 双向任务、CSV/Excel/Markdown/PDF 导入、更快同步 | <https://www.woday.app/updates/woday-1-1> |
| 2026-11-07 | v1.2 | Feature | 重做日期时间选择器（自然语言 “tomorrow 3pm”）、单行快速创建、`#` project/list 与 `@` tag/person token | <https://www.woday.app/updates/woday-1-2> |
| 2026-11-09 | v1.2.2 | Feature | Apple Notes 经 iCloud 同步（lists/checklists/tables/images，双向）、Gemini/Grok 自带 key、Auto AI 选模型、全局面板的自定义重复编辑器 | <https://www.woday.app/updates/woday-1-2-2> |
| 2026-11-11 | v1.3 | Feature | Folders 容纳 tasks/events、Folder/Tag 页面、tag 图标与颜色、folder badge、week/month/Someday 排期、`⌥Space` launcher、菜单栏三形态面板、附件（100 MB）、双向 Apple Reminders、行中 `/` 菜单、侧栏自定义、glass themes、内置 feature tour | <https://www.woday.app/updates/woday-1-3> |

**观察（D）**：版本节奏非常快（10 周 5 个版本），文档与 FAQ 明显滞后于更新日志；做原型时应以“最新更新日志 + 截图”为准，但对未截图的新功能保持标注。

---

## 9. 商务、授权与隐私

### 9.1 价格与授权

- 一次性买断：限时 `$19`，原价 `$39`；`Lifetime access & updates`；`All features unlocked from day one`；`2 device licenses`；`iOS app coming soon`。来源：<https://www.woday.app/>
- 授权：购买后邮件收到 key；`Settings ▸ License` 粘贴并 `Activate`；一 license 一人最多 2 台 Mac；换机前在旧机 `Deactivate on this device`；含终身更新。来源：<https://www.woday.app/docs/license>
- 退款：14 天无理由；联系 `soltwagner@gmail.com`。来源：<https://www.woday.app/docs/getting-help>、<https://www.woday.app/>
- 支付：Polar.sh 为 merchant of record，Stripe 处理；USD，不含 VAT，价格可能因地区而异。来源：<https://www.woday.app/>、<https://www.woday.app/privacy>
- 客户门户：<https://polar.sh/woday/portal/>

### 9.2 隐私与数据

- 本地数据库；无账号；无分析/遥测/崩溃上报；删除项进 Woday Trash。来源：<https://www.woday.app/privacy>
- 可选 iCloud：CloudKit 私有库，官方无权访问；关闭则仅本机。来源：<https://www.woday.app/privacy>、<https://www.woday.app/docs/icloud-sync>
- Apple Calendar/Reminders：经 EventKit 读取；除“勾选 Reminder 回写完成”外为只读（隐私政策表述）。来源：<https://www.woday.app/privacy>
- Google Calendar：可选 OAuth；读取日历列表与所选日历事件（约过去 2 周 + 未来 3 月）与基本资料；可创建/更新/删除仅由 Woday 创建的事件；token 存 Keychain；事件仅在内存显示，不写入 Woday 数据库。来源：<https://www.woday.app/privacy>
- AI：默认关闭；仅向所选 provider 官方 API 发送请求内容；provider 包括 Anthropic/OpenAI（隐私政策）与 Gemini/Grok（更新日志）。来源：<https://www.woday.app/privacy>、<https://www.woday.app/updates/woday-1-2-2>
- 授权校验：激活/停用/周期校验时向 Polar 发送 license key 与 Mac 设备名。来源：<https://www.woday.app/privacy>
- 更新：Sparkle + 自有 update feed（Amore 托管）；仅发送版本与可能的 macOS 版本。来源：<https://www.woday.app/privacy>
- 内置图标/封面：从内容服务器下载图片，可能记录 IP；本地缓存。来源：<https://www.woday.app/privacy>
- 第三方：Polar.sh、Amore、Apple（CloudKit/EventKit/UserNotifications）、Google Calendar API、Anthropic/OpenAI；无广告/分析 SDK。来源：<https://www.woday.app/privacy>

### 9.3 分发与平台

- 官网直接下载，拖入 Applications；自动检查更新，安装需用户同意。来源：<https://www.woday.app/docs/system-requirements>
- macOS 26 (Tahoe) 或更高；原生框架。来源：<https://www.woday.app/docs/system-requirements>
- 非 Mac App Store 销售。来源：<https://www.woday.app/privacy>

---

## 10. 矛盾、缺口与调研限制

### 10.1 官方资料内部矛盾 / 滞后

| # | 矛盾/缺口 | 证据 A | 证据 B | 对原型的处理建议 |
|---|---|---|---|---|
| 1 | 免费试用 | FAQ：`Not during the beta… A free trial arrives with 1.0.`（<https://www.woday.app/>） | v1.0.0 已于 2026-09-01 发布；定价区只提 14 天退款，未提试用（<https://www.woday.app/updates/woday-1-0>） | 不呈现“免费试用”；只写 14 天退款 |
| 2 | AI 范围 | FAQ：仅设备端 Apple Intelligence 改写/摘要（<https://www.woday.app/>） | v1.1+ 有自带 key 的聊天助手、MCP；v1.2.2 加 Gemini/Grok（<https://www.woday.app/updates/woday-1-1>、<https://www.woday.app/updates/woday-1-2-2>） | 分成“设备端 AI 改写”与“可选 AI 助手”两个入口；不要混为一个 |
| 3 | 日历双向 | FAQ：`two-way sync with Apple Calendar and Reminders`（<https://www.woday.app/>） | 隐私政策：Calendar/Reminders 读取，唯一回写是勾选 Reminder（<https://www.woday.app/privacy>） | 文案写“Reminders 双向；Calendar 读取 + 可选 Google 写入”，避免声称 Calendar 全双向 |
| 4 | Google 接入路径 | FAQ/文档：Google/Outlook 经 macOS `Internet Accounts` 出现（<https://www.woday.app/docs/calendar-and-reminders>） | 隐私政策：另有 `Settings ▸ Connections ▸ Google Calendar` OAuth 直连（<https://www.woday.app/privacy>） | 原型可同时保留“系统账户自动出现”和“设置内直连”两种入口，并标注 |
| 5 | `project` / `list` / `folder` | v1.2：`#` 插入 project 或 list（<https://www.woday.app/updates/woday-1-2>） | v1.3：Folders 容纳 tasks/events，新增 Folders/Tags 页面（<https://www.woday.app/updates/woday-1-3>） | 原型统一用 Folder 作为项目容器，`#` token 可标为“项目/列表” |
| 6 | iOS 状态 | `iOS app coming soon`（<https://www.woday.app/>） | `An iOS app is on the roadmap.`（<https://www.woday.app/docs/system-requirements>）；无日期 | 只写“coming soon / roadmap”，不给日期 |
| 7 | 应用字体 | 官网用 Inter/Inter Display（页面 CSS） | 截图观感为 SF Pro；官方未声明 app 字体 | 原型 app 部分用系统字体，营销页用 Inter |
| 8 | 用户数与评分 | `Used by 200+ mac users`、五星（<https://www.woday.app/>） | 无第三方来源、无评价链接 | 视为官方营销声明，不作为事实引用 |

### 10.2 无法确认的内容

- Woday 应用的真实技术栈、数据库、代码质量、性能。
- 应用截图是否与 v1.3 当前 UI 完全一致（截图内容日期为 2026-10-29，v1.3 于 2026-11-11 发布）。
- Notes 库、Events 页、Settings、Quick Launcher、菜单栏面板、AI 聊天、MCP 配置的实际界面。
- `During date` 的准确语义（推测为日期区间，但官方未解释）。
- `Favorites`/`Recent` 的排序与容量规则。
- 文件夹是否支持嵌套、排序、颜色；tag 与 folder 是否能多对多。
- 任务状态（status）的具体枚举；优先级的具体档位。
- 提醒/重复规则的全部选项。
- 免费试用是否最终上线。
- 团队/协作/共享能力是否存在。
- 官网 `contact` 页的可见表单（静态 HTML 未渲染；只确认了邮箱）。
- `/roadmap` 的具体排期（页面只有反馈入口）。

### 10.3 调研限制

1. **未安装/运行 Woday**：应用行为来自官方文档、更新日志、营销截图与本地操作视频，无法完整验证交互动画、键盘行为与边缘状态。
2. **素材分工**：官网子任务未分析 `docs/temp/` 视频；主 agent 已补充视频抽帧证据，见 §12。
3. **营销截图非运行时取证**：截图是 Framer 页面内嵌素材，可能经过构图/裁切/状态摆拍；颜色取样受 WebP 压缩影响。
4. **FAQ 为 JS 折叠组件**：答案通过展开页面与页面 bundle 双重读取；静态 HTML 只含问题。
5. **文档无发布日期**：帮助中心文章没有 `last updated`，无法判断是否覆盖 v1.3。
6. **站点单语**：仅英文；没有中文官方资料。
7. **无官方设计规范/品牌手册**：字号、色值、圆角来自站点 CSS 与截图测量；应用内 token 为取样近似。
8. **无用户研究/竞品数据**：目标用户与“项目管理”映射为推断（D）。

---

## 11. 官方 URL 索引

**产品与页面**

- <https://www.woday.app/>
- <https://www.woday.app/docs>
- <https://www.woday.app/updates>
- <https://www.woday.app/roadmap>
- <https://www.woday.app/contact>
- <https://www.woday.app/privacy>
- <https://www.woday.app/terms>

**帮助中心**

- <https://www.woday.app/docs/getting-started>
- <https://www.woday.app/docs/system-requirements>
- <https://www.woday.app/docs/tasks-reminders-habits>
- <https://www.woday.app/docs/writing-notes>
- <https://www.woday.app/docs/calendar-and-reminders>
- <https://www.woday.app/docs/icloud-sync>
- <https://www.woday.app/docs/license>
- <https://www.woday.app/docs/getting-help>

**更新日志**

- <https://www.woday.app/updates/woday-1-0>
- <https://www.woday.app/updates/woday-1-1>
- <https://www.woday.app/updates/woday-1-2>
- <https://www.woday.app/updates/woday-1-2-2>
- <https://www.woday.app/updates/woday-1-3>

**商务**

- <https://buy.polar.sh/polar_cl_VHWbLrz3Y37Dy4iR0FFE9xHL0i4yeyfxJONtJ0boouD>
- <https://polar.sh/woday/portal/>

**官方截图素材**

- <https://framerusercontent.com/images/1ehn5X8dKtbPfjrvYt7rqPMik.webp?width=2262&height=1484>
- <https://framerusercontent.com/images/ScV3adeYFJAsJeHMrSLAx2YfpU.webp?width=2262&height=1484>
- <https://framerusercontent.com/images/Z9ruOPUEvZGUKpglFKqL1aaBdR0.webp?width=2262&height=1484>
- <https://framerusercontent.com/images/kTSServawTKLwIVjsfagEwBixE.webp?width=2262&height=1484>
- <https://framerusercontent.com/images/HvzCAtJrDe7J90cFVmY96mRQDlo.webp?width=2262&height=1484>
- <https://framerusercontent.com/images/IgxmC6pWAFGON3rqTDEmpqqyg2s.webp?width=2262&height=1484>
- <https://framerusercontent.com/images/yE97lEjNs3E0GTaqpBNDkfATiA.webp?width=1254&height=1254>
- <https://framerusercontent.com/images/SW9brpLtiEEvQRGEQawRhg5Zsw0.webp?width=1254&height=1254>
- <https://framerusercontent.com/images/nPT05p7fGZHvLAjGddpyWcMw.webp?width=768&height=768>


---

## 12. 本地视频补充调研（主 agent）

> 本节补充官网子任务之外的本地一手素材。视频通过 `ffprobe` 读取时长与分辨率，并用 `ffmpeg` 抽取关键画面逐帧检查；未运行 Woday 本身。下列文件均位于仓库 `docs/temp/`。

### 12.1 素材清单与采用依据

| 本地来源 | 实测信息 | 关键画面与操作 | 原型处理 |
|---|---|---|---|
| `dScOAWO-2kSsAj-2_x.mp4` | 1920×1080，19.17 秒 | 浅色 Today、右侧 Inbox、菜单栏 Today 面板、Settings/General | 还原黑色外壳、白色三栏、日期条、逾期任务区与设置结构 |
| `hGYem1KlN7e-Dz5G_x.mp4` | 1734×1080，43.37 秒 | 深色 Today、Inbox 新任务、AI Assistant 空状态、日期选择菜单 | 增加深色主题、Inbox 快速捕获、AI 面板与本地任务状态 |
| `J9mn85Or8rH_aVkS_x.mp4` | 1652×1080，76.02 秒 | Notes 列表/编辑器、图标与封面选择、嵌套 checklist、完成状态 | 实现笔记列表、可编辑标题/正文、内嵌任务与完成切换 |
| `mXNZQmZVV3G2IuAC_x.mp4` | 1652×1080，41.87 秒 | Today、Folders/Tags、Work 聚合页、Appearance 与侧栏显示开关 | 实现文件夹/标签聚合、Work 页面、外观设置和模块开关 |
| `dDAF0YM2NRdzFBFQ_x.mp4` | 1920×1080，33.73 秒 | “Your day in one view.”、“More than a calendar.”、红色 mascot、Mac 应用演示 | 仅用于确认品牌语言，不把 HTML 原型改成营销落地页 |
| `bw_OrBHWjw7WLXwy_x.mp4` | 1920×1080，53.53 秒 | 另一款代码编辑器的 “Make it yours” 对话框与文件树 | 与 Woday 产品界面不符，**排除，不作为还原依据** |

### 12.2 从视频直接观察到的界面规则

1. **窗口骨架**：顶部近黑标题栏，左侧近黑导航，中央暖白大圆角面板，右侧独立 Inbox 面板；中央与右侧之间有黑色间隙。来源：`dScOAWO-2kSsAj-2_x.mp4`、`hGYem1KlN7e-Dz5G_x.mp4`。
2. **Today 层级**：`Today` 标题与完整日期 → 横向日期条 → 问候统计条 → 可折叠的昨日遗留区 → 当日任务与事件。红色用于当前日期、事件点和重要状态，黄色用于中优先级。来源：`dScOAWO-2kSsAj-2_x.mp4`。
3. **Inbox 捕获**：顶部为 `New task` 输入位，任务按 `Overdue`、`Inbox`、`Today` 分组；行内显示时间、folder/tag 徽章和优先级圆环。来源：`hGYem1KlN7e-Dz5G_x.mp4`。
4. **笔记编辑器**：emoji 图标与大标题同行，下方是创建/编辑时间、folder、tag 等元信息；正文支持 checklist，子任务通过缩进表达层级，父任务显示完成计数。来源：`J9mn85Or8rH_aVkS_x.mp4`。
5. **组织方式**：Folders 与 Tags 均有独立页面；Work 页把任务、事件和 `Product launch plan` 笔记放在同一聚合视图，支持列表/卡片切换。来源：`mXNZQmZVV3G2IuAC_x.mp4`。
6. **主题与设置**：浅色和深色都保留近黑外壳，主要切换内容面板、文字、边框与徽章颜色；Appearance 页可控制主题、文字和侧栏模块。来源：`hGYem1KlN7e-Dz5G_x.mp4`、`mXNZQmZVV3G2IuAC_x.mp4`。
7. **AI 未配置状态**：右侧面板标题为 `AI Assistant`，中部展示红色 mascot、`Set up the AI assistant` 与 `Open Settings`，说明需要添加 Claude 或 ChatGPT key。来源：`hGYem1KlN7e-Dz5G_x.mp4`。

### 12.3 对 HTML 实现的约束

- 主交付是应用原型，不新增官网 Hero、价格卡或营销页结构。
- 任务新增/完成、逾期迁移、日期切换、笔记编辑、搜索和主题设置采用真实前端状态，不使用静态截图代替。
- 保留视频中的英文界面文案与紧凑信息密度；没有为中文用户自行翻译产品 UI。
- 视频未覆盖的 Notes 库、Events 完整页面等，按同一组件语言做合理补全，不声称与真实应用逐像素一致。
- 不实现 Apple Calendar/Reminders、iCloud、AI 密钥、通知或后端同步；这些能力需要原生应用与服务权限，HTML 只呈现对应入口或空状态。

### 12.4 补充限制

- 抽帧能确认可见布局与部分操作结果，但不能证明完整动画、所有快捷键和异常状态。
- 视频画面包含多个日期与不同演示数据；HTML 使用浏览器当前日期，任务与事件为演示数据。
- 视频中的菜单栏面板属于 macOS 能力，HTML 的窄屏抽屉只是交互近似，不宣称为官方移动版界面。


---

## 13. 第二轮完整性复核（2026-11-16）

> 本轮按用户要求，重新对照 `docs/temp/` 操作视频、官网当前截图与既有 `woday.html`。重点检查左下角菜单、任务详情、视图切换、设置分类与笔记封面，并在浏览器中执行实际操作，而不是仅核对源码。

### 13.1 已确认并补齐的缺口

| 缺口 | 一手依据 | 本轮处理 |
|---|---|---|
| 左下角 `...` 被长侧栏内容挤到滚动区外 | `mXNZQmZVV3G2IuAC_x.mp4` 中菜单固定于左下角 | 改为独立滚动侧栏内容区，菜单固定在底部；窄屏也保留入口 |
| 菜单缺少 `Analytics`、`Send Feedback`，并混入非原版 `Reset demo data` | 同一视频明确显示 `Analytics / Settings / Trash / Send Feedback` | 菜单改为原版四项；`Reset demo data` 移至 `Settings → General` |
| `Analytics`、`Trash`、`Send Feedback` 没有实际目标界面 | 菜单操作流 | 新增本地统计页、Trash 恢复/永久删除、反馈弹窗与邮件链接 |
| 设置分类不完整 | 视频中依次出现 `General / Notifications / Sync / Connections / Import / AI Connections / Appearance / Shortcuts / Updates / License / About` | 补齐 11 个分类；AI 密钥明确不采集、不持久化 |
| 点击任务标题不能打开详情 | 官方 Tasks 截图 `kTSServawTKLwIVjsfagEwBixE.webp` | 新增右侧任务详情，支持标题、日期、时间、优先级、folder/tag、子任务、备注与删除 |
| Tasks / Upcoming 的视图按钮只是装饰 | 官方 Upcoming 列表与看板截图、Tasks 截图 | 实现列表、按日期/状态分列的看板与日历跳转，支持 due date / priority 排序 |
| 笔记缺少封面和图标操作 | 官方 Note 截图 `HvzCAtJrDe7J90cFVmY96mRQDlo.webp`、J9 视频 | 新增封面主题、图标切换、五枚悬浮按钮与显式封面/tag 操作 |
| Events 只能查看，新建按钮只有提示 | 产品操作流 | 新增事件创建表单、列表展示和删除；测试事件可直接清理 |
| AI 面板的 `Open Settings` 只进入 General | hGY 视频与 AI Connections 设置 | 直接打开 `AI Connections`，说明密钥只能在原生 Mac 应用配置 |

### 13.2 浏览器回归证据

- 左下角四项菜单已打开并截图核对；Analytics 与全部设置分类均可访问。
- 临时任务 `Audit temporary task` 已执行：创建 → `High` 优先级 → 新建子任务 → 删除到 Trash → 恢复 → 再次删除 → 永久删除。
- 新建任务的子任务初始化问题已修复；逾期日期错误显示为 `Next week` 已改为 `Overdue`；任务详情不再错误高亮 Inbox 图标。
- Upcoming 看板已按日期显示任务和事件；Tasks 看板已按 `Inbox / Overdue / Today / Tomorrow / Later` 显示。
- 临时事件 `Audit temporary event` 已创建、确认出现在 Events，再删除并确认消失。
- Lisbon 笔记已验证五枚封面悬浮按钮与封面循环；最终恢复原来的暖色封面。
- Send Feedback 弹窗已打开并通过 Cancel 关闭；没有向外部发送任何内容。
- 测试过程中保留了用户后来添加的 `sett` 任务，没有使用 Reset demo data 清除用户状态。

### 13.3 仍然存在的简化与边界

以下是**有意保留的原型边界或视觉近似**，不再列为“遗漏但已完成”：

1. **原生服务不接后端**：Apple Calendar/Reminders、iCloud、Google/Todoist/Linear、AI、许可证验证和自动更新只展示入口或说明，不模拟连接成功。
2. **笔记不是完整 block 编辑器**：支持可编辑正文与内嵌任务，但未实现 `/` 菜单、`@` mention、富文本选区工具、附件和真实图片库；封面使用本地 CSS 渐变近似，不冒充官方照片资产。
3. **文件夹与标签自定义有限**：已有聚合页和筛选，但新建 folder/tag、拖拽组织与完整看板交互未覆盖。
4. **日历不是完整调度器**：支持周/月/三日视图、任务与事件展示、事件创建/删除，但不支持拖拽改期、重复规则和会议链接。
5. **macOS 专属界面只做近似**：菜单栏面板、屏幕边缘唤出、Quick Launcher 的高级 token、原生通知和系统权限不属于浏览器可验证能力。
6. **动画与精确素材**：已确认主要布局、颜色、控件与操作流，不承诺所有动画、所有字体度量、全部 emoji/照片与真实应用逐像素一致。
