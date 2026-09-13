const fs = require("node:fs");
const path = require("node:path");
const { chromium } = require("C:/Users/yangjianlin/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright");

const root = path.resolve(__dirname, "../docs/requests/bilicatch-task-actions-layout-fix-20260913/verification/evidence");
const actionControlSelector = ".task-row__actions .icon-button, .task-open-menu summary";
const cases = [
  ["tasks-1280", 1280, 800, "#/tasks?demo=1", true],
  ["tasks-1100", 1100, 700, "#/tasks?demo=1", true],
  ["tasks-900", 900, 700, "#/tasks?demo=1", true],
  ["tasks-851", 851, 700, "#/tasks?demo=1", true],
  ["tasks-800", 800, 650, "#/tasks?demo=1", true],
  ["tasks-empty", 900, 700, "#/tasks?demo=1&empty=1", false],
  ["tasks-error", 900, 700, "#/tasks?demo=1&error=1", false],
];

(async () => {
  fs.mkdirSync(root, { recursive: true });
  const browser = await chromium.launch({
    headless: true,
    executablePath: "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  });
  const results = [];
  for (const [name, width, height, hash, expectsActions] of cases) {
    const page = await browser.newPage({ viewport: { width, height } });
    await page.goto(`http://127.0.0.1:1420/${hash}`);
    await page.locator("[data-testid='page-heading']").last().waitFor();
    await page.locator(expectsActions ? actionControlSelector : ".empty-state").first().waitFor();
    const metrics = await page.evaluate((selector) => {
      const taskList = document.querySelector(".task-list");
      const listBounds = taskList?.getBoundingClientRect();
      const rows = [...document.querySelectorAll(".task-row")];
      const buttons = [...document.querySelectorAll(selector)];
      return {
        viewportWidth: window.innerWidth,
        documentWidth: document.documentElement.scrollWidth,
        listPresent: Boolean(taskList),
        rowCount: rows.length,
        buttonCount: buttons.length,
        overflowing: [...document.querySelectorAll(".task-row, .task-toolbar, .tasks-heading")]
          .filter((element) => element.getBoundingClientRect().right > window.innerWidth + 1).length,
        overflowingRows: rows.filter((row) => row.scrollWidth > row.clientWidth + 1).length,
        buttonsOutsideList: listBounds
          ? buttons.filter((button) => {
            const bounds = button.getBoundingClientRect();
            return bounds.left < listBounds.left - 1 || bounds.right > listBounds.right + 1;
          }).length
          : 0,
        undersizedButtons: buttons.filter((button) => {
          const rect = button.getBoundingClientRect();
          return rect.width < 36 || rect.height < 36;
        }).length,
        clippedButtons: buttons.filter((button) => {
          const rect = button.getBoundingClientRect();
          return rect.left < 0 || rect.right > window.innerWidth;
        }).length,
      };
    }, actionControlSelector);
    await page.screenshot({ path: path.join(root, `${name}.png`), fullPage: true });
    results.push({ name, expectsActions, ...metrics });
    await page.close();
  }
  await browser.close();
  console.log(JSON.stringify(results, null, 2));
  if (results.some((result) =>
    (result.expectsActions && (!result.listPresent || !result.rowCount || !result.buttonCount))
    || result.documentWidth > result.viewportWidth
    || result.overflowing
    || result.overflowingRows
    || result.buttonsOutsideList
    || result.undersizedButtons
    || result.clippedButtons
  )) process.exitCode = 1;
})();
