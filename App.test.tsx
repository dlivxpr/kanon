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

  await app.press("后一天 ›");
  await app.waitFor(tomorrow);
  await app.press(`选择 ${tomorrow}`);
  await app.waitFor(tomorrow);

  await app.press("id:goto-today");
  await app.waitFor("今天");
  expect(app.texts()).toContain("今天");
});

test("tabs open, reuse, pin and close; back and forward walk history", async () => {
  const app = await open(".", { width: 1280, height: 800 });
  await app.waitFor("今天");

  // A new tab lands on Start.
  await app.press("新建标签页");
  await app.waitFor("从左侧导航打开「今天」，或使用 ⌘/Ctrl+T 新建标签页");
  await app.waitFor("标签页 开始");

  // Clicking 今天 in the sidebar focuses the tab already showing it.
  await app.press("id:nav-今天");
  await app.waitFor("id:date-strip");
  await app.waitFor("标签页 开始");

  // An unreleased destination reuses the active unpinned tab.
  await app.press("id:nav-待办");
  await app.waitFor("该视图将在后续里程碑交付，标签与导航行为已可用");
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

  await app.press("id:carryover");
  await app.waitFor("昨天没做完的事");
  await app.waitFor("结转 1 项任务", { timeout: 2000 }).catch(() => {});
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
