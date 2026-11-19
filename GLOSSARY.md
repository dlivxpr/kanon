# Kanon

Kanon is a personal project-driven workspace that unifies structured knowledge management (Yuque-style books and documents) with daily action execution (Woday-style tasks, habits, events, and inbox capture).
Kanon 是一款个人本地优先的知识库、待办与项目管理三合一桌面笔记软件。

## Language

### Containers & Organization

**Project**:
A goal-oriented, time-bounded container with a lifecycle status (e.g., planning, active, completed, archived) that groups both actionable work (`Task`, `Event`) and structured knowledge (`Document` catalog) to drive a personal objective forward.
_Avoid_: Folder, workspace, team, repo

**Book**:
An open-ended, non-expiring knowledge base organized around a long-term topic or domain, structuring `Document` items into a book-like table of contents without project lifecycle constraints.
_Avoid_: Folder, wiki, notebook, repository

**Catalog**:
The ordered, hierarchical tree of section groups and `Document` nodes inside a `Project` or `Book` that gives knowledge a book-like reading order and structure.
_Avoid_: Folder tree, directory, file list

**Tag**:
A color-coded, cross-cutting label that can be attached to `Task`, `Memo`, `Document`, or `Event` items to aggregate related work across different `Project` and `Book` containers.
_Avoid_: Category, folder, topic

### Capture & Knowledge

**Inbox**:
A persistent capture and triage queue that collects unorganized `Task` items and fleeting `Memo` notes before they are scheduled onto the calendar or filed into a `Project` or `Book`.
_Avoid_: Backlog, scratchpad

**Memo**:
A lightweight, unstructured fleeting note captured in the `Inbox` (similar to Yuque "小记") without requiring an upfront `Project` or `Book` location; can later be promoted into a formal `Document` or converted into `Task` items.
_Avoid_: Quick note, sticky note, snippet

**Document**:
A structured long-form note belonging to either a `Project` or a `Book`, composed of rich blocks and positioned within a hierarchical `Catalog`, with separate reading and editing modes.
_Avoid_: Page, article, post

**Backlink**:
An automatic reverse reference recorded when one `Document`, `Task`, or `Event` mentions another via `@`, surfaced alongside the document outline to reveal knowledge and execution connections.
_Avoid_: Citation, trackback
**知识库（Knowledge Base）**:
按主题长期沉淀结构化文档的书目式容器，包含首页与多级目录树，只容纳文档与目录分组。
_Avoid_: Folder, 文件夹, 笔记本, Space

### Execution & Schedule
**文档（Document）**:
严格归属于唯一知识库的块级富文本内容单元，支持阅读态与编辑态切换，可内嵌表格、代码块、公式、卡片与任务关联。
_Avoid_: Note, 笔记, 文章, 文稿

**Task**:
An actionable to-do item with completion state, optional due date/time, priority, subtasks, and optional assignment to a `Project`, surfaced in global schedule views (`Today`, `Upcoming`, `Tasks`) and optionally embedded in a `Document`.
_Avoid_: Todo, issue, ticket, checklist
**项目（Project）**:
具有明确目标与阶段的工作单元，聚合所属任务与里程碑，并可关联知识库或指定目录分组作为项目文档树。
_Avoid_: Folder, Workspace, 团队

**Checklist**:
A lightweight, document-local checkable line item inside a `Document` or `Memo` that does not appear in global `Task` views unless explicitly promoted into a `Task`.
_Avoid_: Todo, global task
**任务（Task）**:
可独立追踪完成状态的待办事项，可停留在收件箱、排入日期或归属某个项目，并可与文档双向关联。
_Avoid_: Todo 项, Issue, 卡片

**Habit**:
A recurring daily or scheduled practice tracked for consistency and completion streaks across days.
_Avoid_: Routine, check-in
**小记（Quick Note）**:
用于低摩擦捕获碎片想法的轻量记录流，可附加标签、置顶，并可转化为文档或任务。
_Avoid_: 随手记, 闪念, 草稿

**Event**:
A calendar-bound appointment or time block occupying a specific date and time range on the schedule.
_Avoid_: Meeting, reminder
**标签（Tag）**:
跨知识库、项目、任务与小记的横向分类标识，用于多维度检索与聚合。
_Avoid_: 分类, 分组, Folder

**收件箱（Inbox）**:
暂存尚未排期且未分配项目的待办任务默认捕获区。
_Avoid_: 收集箱, 草稿箱, 临时列表

**目录节点（Catalog Node）**:
知识库树状目录中的结构单元，既可以是纯分组节点，也可以是允许继续嵌套子文档的文档节点。
_Avoid_: 文件夹, 菜单项

**里程碑（Milestone）**:
项目中的阶段性目标节点，包含目标日期并聚合关联任务的推进进度。
_Avoid_: 阶段, Sprint, 版本

**文档清单项（Checklist Item）**:
仅存在于单篇文档正文内的普通复选框条目，不自动同步到全局待办；需显式转化为「任务」或嵌入关联任务卡片后才跨视图联动。
_Avoid_: 文档任务, 内联 Todo
