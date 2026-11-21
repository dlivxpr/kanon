// The workbench shell and Today flows, driven headlessly through the
// Rust root: `nsh test` runs these against the real engine.
import { expect, open, test } from "nsh/test";

const QUICK_ADD = "text:添加任务，回车保存到所选日期";

function ymd(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

const yesterday = ymd(new Date(Date.now() - 86400000));
const tomorrow = ymd(new Date(Date.now() + 86400000));

test("today view creates a task and toggles it with the circle", async () => {
  const app = await open(".", { width: 1280, height: 800 });
  await app.waitFor("今天");
  await app.waitFor("id:date-strip");

  await app.type(QUICK_ADD, "买牛奶");
  await app.press("id:quick-add");
  await app.waitFor("买牛奶");
  await app.waitFor("任务详情", { timeout: 2000 });

  // The quick circle toggles strictly done ↔ todo.
  await app.press("完成 买牛奶 >> nth=0");
  await app.waitFor("已完成");
  await app.press("完成 买牛奶 >> nth=0");
  await app.waitFor("任务详情");

  // The overview agrees: the task is back in 今天, not 已完成.
  await app.press("关闭任务详情");
  await app.waitFor("今天");
  expect(app.texts()).toContain("买牛奶");
});

test("date navigation moves between days and back to today", async () => {
  const app = await open(".", { width: 1280, height: 800 });
  await app.waitFor("今天");

  const nextDate = new Date(Date.now() + 86400000);
  const nextTitle = `${nextDate.getMonth() + 1}月${nextDate.getDate()}日`;
  await app.press("label:后一天");
  await app.waitFor(`text:${nextTitle}`);
  await app.press(`label:选择 ${tomorrow}`);
  await app.waitFor(`text:${nextTitle}`);

  await app.press("id:goto-today");
  await app.waitFor("今天");
  expect(app.texts()).toContain("今天");
});

test("tabs open, reuse, pin and close; back and forward walk history", async () => {
  const app = await open(".", { width: 1280, height: 800 });
  await app.waitFor("今天");

  // A new tab lands on Start.
  await app.press("新建标签页");
  await app.waitFor("id:start-heading");
  await app.waitFor("标签页 开始");

  // Clicking 今天 in the sidebar focuses the tab already showing it.
  await app.press("id:nav-今天");
  await app.waitFor("id:date-strip");
  await app.waitFor("标签页 开始");

  // A released destination reuses the active unpinned tab.
  await app.press("id:nav-待办");
  await app.waitFor("暂无任务");
  await app.waitFor("标签页 待办");

  // Back returns to 今天 inside the same tab; forward walks it again.
  await app.press("后退");
  await app.waitFor("id:date-strip");
  await app.press("前进");
  await app.waitFor("标签页 待办");

  // Pinning the active tab forces the next sidebar click into a new tab.
  await app.press("固定标签页 >> nth=0");
  await app.press("id:nav-日历");
  await app.waitFor("标签页 日历");
  await app.press("关闭标签页 >> nth=2");
  await app.waitFor("标签页 待办");
});

test("carryover moves yesterday's unfinished task onto today", async () => {
  const app = await open(".", { width: 1280, height: 800 });
  await app.waitFor("今天");

  await app.type(QUICK_ADD, "昨天没做完的事");
  await app.press("id:quick-add");
  await app.waitFor("昨天没做完的事");
  await app.waitFor("任务详情");

  // Reschedule it to yesterday.
  await app.clear("label:排期日");
  await app.type("label:排期日", yesterday);
  await app.waitFor(`选择 ${yesterday}`); // the strip now centers yesterday
  await app.press("id:goto-today");
  await app.waitFor("id:carryover");
  expect(app.texts()).not.toContain("昨天没做完的事");

  await app.press("展开结转任务");
  await app.waitFor("id:carryover-preview >> label:打开任务 昨天没做完的事");
  await app.press("收起结转任务");
  expect(app.texts()).not.toContain("昨天没做完的事");
  await app.press("展开结转任务");

  await app.press("id:carryover");
  await app.waitFor("id:today-task-list >> label:打开任务 昨天没做完的事");
  expect(app.shows("id:carryover-preview")).toBe(false);
});

test("task detail edits status, priority, subtasks, tags and memo", async () => {
  const app = await open(".", { width: 1280, height: 800 });
  await app.waitFor("今天");

  await app.type(QUICK_ADD, "整理书架");
  await app.press("id:quick-add");
  await app.waitFor("任务详情");

  // Status and priority flow through the row's meta.
  await app.press("进行中");
  await app.waitFor("进行中");
  await app.press("高");
  await app.waitFor("高优先级");

  // Subtasks and tags list live in the detail.
  await app.type("label:新子任务", "把旧书装箱");
  await app.submit("label:新子任务");
  await app.waitFor("把旧书装箱");
  expect(app.value("label:新子任务")).toBe("");
  await app.type("label:新标签", "家务");
  await app.submit("label:新标签");
  await app.waitFor("#家务");

  // Memo and the independent Due date/time commit to the input state
  // and the row meta.
  await app.type("label:备注", "从客厅开始");
  expect(app.value("label:备注")).toBe("从客厅开始");
  await app.clear("label:截止日");
  await app.type("label:截止日", tomorrow);
  await app.type("label:截止时间", "18:00");
  expect(app.value("label:截止日")).toBe(tomorrow);
  expect(app.value("label:截止时间")).toBe("18:00");
  await app.waitFor(`截止 ${tomorrow} 18:00`);

  // Scheduling it for tomorrow moves it off today's list.
  await app.clear("label:排期日");
  await app.type("label:排期日", tomorrow);
  await app.waitUntil(() => !app.texts().includes("整理书架"));
});

test("inbox badge counts strict inbox tasks and triage updates it", async () => {
  const app = await open(".", { width: 1280, height: 800 });
  await app.waitFor("今天");

  // Quick capture in the Task Overview creates a strict Inbox task.
  await app.type("label:速记待办，回车保存", "给朋友回电话");
  await app.submit("label:速记待办，回车保存");
  await app.waitFor("label:收件箱 1 项任务");

  // Triage it: a Scheduled Date moves it out of the Inbox at once.
  await app.press("打开任务 给朋友回电话");
  await app.waitFor("任务详情");
  await app.type("label:排期日", tomorrow);
  await app.waitFor("label:收件箱 0 项任务");
});

test("tasks view groups tasks without duplicates and filters by status", async () => {
  const app = await open(".", { width: 1280, height: 800 });
  await app.waitFor("今天");

  // Two strict Inbox tasks through the overview capture.
  await app.type("label:速记待办，回车保存", "写季度总结");
  await app.submit("label:速记待办，回车保存");
  await app.type("label:速记待办，回车保存", "整理相册");
  await app.submit("label:速记待办，回车保存");
  await app.waitFor("label:收件箱 2 项任务");

  // One task scheduled for today through the Today quick add.
  await app.type(QUICK_ADD, "买牛奶");
  await app.press("id:quick-add");
  await app.waitFor("买牛奶");

  // The Tasks view groups them exclusively; count without the overview panel.
  await app.press("切换上下文面板");
  await app.press("id:nav-待办");
  await app.waitFor("id:tasks-group-收件箱");
  await app.waitFor("id:tasks-group-今天");
  const texts = app.texts();
  for (const title of ["写季度总结", "整理相册", "买牛奶"]) {
    expect(texts.filter((t) => t === title)).toHaveLength(1);
  }

  // A status filter with no matches shows the honest empty state.
  await app.press("label:状态筛选 已完成");
  await app.waitFor("没有符合筛选的任务");
  await app.press("label:状态筛选 全部");
  await app.waitFor("id:tasks-group-收件箱");
});

test("upcoming view lists future tasks with due indicators in both modes", async () => {
  const app = await open(".", { width: 1280, height: 800 });
  await app.waitFor("今天");

  // A task scheduled for tomorrow with a Due Date.
  await app.type(QUICK_ADD, "体检预约");
  await app.press("id:quick-add");
  await app.waitFor("任务详情");
  await app.clear("label:排期日");
  await app.type("label:排期日", tomorrow);
  await app.type("label:截止日", tomorrow);

  // 列表 mode: day sections with the Due indicator inline.
  await app.press("id:nav-即将到来");
  await app.waitFor("id:upcoming-list");
  expect(app.texts()).toContain("体检预约");
  await app.waitFor(`截止 ${tomorrow}`);

  // 看板 mode: one column per day, same rows and Due indicators.
  await app.press("label:视图 看板");
  await app.waitFor("id:upcoming-board");
  expect(app.texts()).toContain("体检预约");
  await app.waitFor(`截止 ${tomorrow}`);
});

test("window chrome follows the chosen light and dark appearances", async () => {
  const app = await open(".", { width: 1280, height: 800, colorScheme: "light", reducedMotion: true });
  expect(app.viewStyle("id:window-topbar")?.backgroundColor).toBe("#eceee7ff");
  await app.waitFor("id:window-topbar >> label:标签页 今天");
  expect(app.count("id:window-topbar >> text:今天")).toBe(0);
  await app.press("id:nav-设置");
  await app.press("label:外观");
  await app.press("label:深色");
  expect(app.viewStyle("id:window-topbar")?.backgroundColor).toBe("#11120fff");
  await app.press("label:跟随系统");
  expect(app.viewStyle("id:window-topbar")?.backgroundColor).toBe("#eceee7ff");
});

test("appearance changes the workbench accent and can hide a sidebar destination", async () => {
  const app = await open(".", { width: 1280, height: 800, colorScheme: "light" });
  await app.waitFor("id:nav-待办");
  await app.press("id:nav-设置");
  await app.press("label:外观");
  await app.press("label:钴蓝");
  await app.press("label:显示待办");
  expect(app.shows("id:nav-待办")).toBe(false);
  await app.press("label:显示待办");
  await app.waitFor("id:nav-待办");
  await app.press("id:nav-待办");
  await app.waitFor("label:新建任务");
  expect(app.viewStyle("label:新建任务")?.backgroundColor).toBe("#3b82f6ff");
  await app.keyboard.press("mod+k");
  await app.waitFor("label:搜索工作区");
  await app.keyboard.press("escape");
  expect(app.shows("label:搜索工作区")).toBe(false);
});

test("command search creates a knowledge base and closes its dialog", async () => {
  const app = await open(".", { width: 1280, height: 800 });
  await app.waitFor("label:全局搜索");
  await app.press("label:全局搜索");
  await app.type("label:搜索工作区", "> 新建知识库");
  await app.waitFor("text:新建知识库");
  expect(app.shows("label:搜索工作区")).toBe(true);
  await app.press("text:新建知识库");
  await app.waitFor("label:名称");
  expect(app.enabled("text:创建")).toBe(false);
  await app.type("label:名称", "命令验收书目");
  await app.submit("label:名称");
  await app.waitFor("label:标签页 命令验收书目");
  expect(app.shows("label:名称")).toBe(false);
});

test("compact workbench opens and closes its navigation and calendar drawers", async () => {
  const app = await open(".", { width: 660, height: 720, platform: "windows" });
  await app.waitFor("label:切换侧栏");
  expect(app.shows("id:nav-待办")).toBe(false);
  expect(app.shows("id:context-pane")).toBe(false);

  await app.press("label:切换侧栏");
  await app.waitFor("id:nav-待办");
  await app.press("id:nav-待办");
  await app.waitFor("label:标签页 待办");
  expect(app.shows("id:nav-待办")).toBe(false);

  await app.press("label:选择面板模式");
  await app.press("text:迷你日历");
  await app.waitFor("id:mini-calendar");
  await app.press("label:切换上下文面板");
  expect(app.shows("id:mini-calendar")).toBe(false);
  await app.press("label:切换上下文面板");
  await app.waitFor("id:mini-calendar");
});

test("quick create opens the intended form for each entity", async () => {
  const app = await open(".", { width: 1280, height: 800 });
  const quick = "label:快速新建";
  await app.waitFor(quick);

  await app.press(quick);
  await app.press("label:新建文档");
  await app.waitFor("归属知识库");
  expect(app.enabled("text:创建")).toBe(false);
  await app.press("text:先创建知识库…");
  await app.type("label:名称", "验收手册");
  await app.press("text:创建");
  await app.waitFor("label:标签页 验收手册");

  await app.press(quick);
  await app.press("label:新建文档");
  await app.type("label:名称", "空白验收文档");
  await app.press("text:创建");
  await app.waitFor("label:标签页 空白验收文档");
  await app.press("完成");
  await app.waitFor("收藏文档");

  await app.press(quick);
  await app.press("label:新建任务");
  await app.type("label:名称", "快速新增待办");
  await app.press("text:创建");
  await app.waitFor("label:收件箱 1 项任务");
  await app.waitFor("任务详情");

  await app.press(quick);
  await app.press("label:新建项目");
  await app.waitFor("id:project-save");
  await app.press(quick);
  await app.press("label:新建小记");
  await app.waitFor("label:小记正文");
});

test("a Start template creates an editable document with its sections", async () => {
  const app = await open(".", { width: 1280, height: 800, platform: "windows" });
  await app.waitFor("id:nav-开始");
  await app.press("id:nav-new-book");
  await app.type("label:名称", "会议资料");
  await app.press("text:创建");

  await app.press("id:nav-开始");
  await app.press("label:模板中心");
  await app.press("text:会议纪要 · 议题 / 决议 / 下一步行动");
  await app.press("text:创建");
  await app.waitFor("label:标签页 会议纪要");
  await app.press("完成");
  await app.waitFor("收藏文档");
  expect(app.shows("正文为空")).toBe(false);
  await app.waitFor("text:下一步行动");
  await app.press("id:nav-待办");
  await app.waitFor("label:标签页 待办");
  await app.press("label:全局搜索");
  await app.type("label:搜索工作区", "下一步行动");
  await app.waitFor("text:会议纪要");
  await app.press("text:会议纪要");
  await app.waitFor("收藏文档");
  await app.keyboard.press("e");
  await app.waitFor("text:完成");
  await app.keyboard.press("mod+e");
  await app.waitFor("label:收藏文档");
});

test("a tagged note remains after extraction and is archived after conversion", async () => {
  const app = await open(".", { width: 1280, height: 800 });
  await app.waitFor("id:nav-new-book");
  await app.press("id:nav-new-book");
  await app.type("label:名称", "小记归档库");
  await app.press("text:创建");
  await app.waitFor("label:标签页 小记归档库");

  await app.press("id:nav-小记");
  await app.type("label:小记正文", "整理走查记录");
  await app.type("label:小记标签", "走查标签");
  await app.press("id:publish-note");
  await app.waitFor("text:#走查标签");
  await app.press("label:小记操作 1");
  await app.press("text:置顶");
  await app.press("label:小记操作 1");
  await app.waitFor("text:取消置顶");
  await app.press("text:提取为任务");
  await app.waitFor("label:收件箱 1 项任务");
  await app.waitFor("label:小记操作 1");

  await app.press("id:nav-标签");
  await app.press("label:查看标签 走查标签");
  await app.waitFor("text:相关内容");
  await app.press("id:nav-小记");
  await app.press("label:小记操作 1");
  await app.press("text:转为知识库文档");
  await app.press("role:dialog >> label:小记归档库");
  await app.waitFor("label:标签页 整理走查记录");
  await app.press("id:nav-小记");
  expect(app.shows("label:小记操作 1")).toBe(false);
  await app.press("label:已归档");
  await app.waitFor("text:整理走查记录");
});

test("calendar quick add can be cancelled and saves to the selected day", async () => {
  const app = await open(".", { width: 1280, height: 800 });
  const day = ymd(new Date());
  const addDay = `label:${day} 添加任务`;
  const composer = `id:calendar-add-${day}`;
  await app.waitFor("id:nav-日历");
  await app.press("id:nav-日历");
  await app.press("label:范围 3 天");
  await app.press(addDay);
  await app.press("label:取消添加");
  expect(app.shows(composer)).toBe(false);

  await app.press(addDay);
  await app.type(`${composer} >> text:添加任务，回车保存`, "日历验收任务");
  await app.press("id:calendar-add-submit");
  await app.waitFor(`id:calendar-day-${day} >> label:打开任务 日历验收任务`);
  await app.waitFor("text:任务详情");
  expect(app.shows(composer)).toBe(false);
  await app.press("label:迷你日历");
  await app.press(`id:mini-day-${day}`);
  await app.waitFor("label:标签页 今天");
  await app.waitFor("text:日历验收任务");
});

test("favorites open live documents and allow removing the star", async () => {
  const app = await open(".", { width: 1280, height: 800 });
  await app.waitFor("id:nav-new-book");
  await app.press("id:nav-new-book");
  await app.type("label:名称", "研究手册");
  await app.press("text:创建");
  await app.press("label:快速新建");
  await app.press("label:新建文档");
  await app.type("label:名称", "收藏验证文档");
  await app.press("text:创建");
  await app.waitFor("label:标签页 收藏验证文档");
  await app.press("text:完成");
  await app.press("label:收藏文档");
  await app.press("id:nav-收藏");
  await app.press("label:收藏 收藏验证文档（研究手册）");
  await app.waitFor("label:取消收藏文档");
  await app.press("id:nav-收藏");
  await app.press("label:取消收藏 收藏验证文档");
  await app.waitFor("text:暂无收藏");
});
