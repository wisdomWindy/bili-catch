const path = require("node:path");
const { chromium } = require("C:/Users/yangjianlin/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright");

const root = path.resolve(__dirname, "../docs/requests/bilicatch-v1-desktop/module-runs/task-management/verification/screenshots");
const cases = [
  ["tasks-1280", 1280, 800, "#/tasks?demo=1"],
  ["tasks-900", 900, 700, "#/tasks?demo=1"],
  ["tasks-800", 800, 650, "#/tasks?demo=1"],
  ["tasks-empty", 900, 700, "#/tasks?demo=1&empty=1"],
  ["tasks-error", 900, 700, "#/tasks?demo=1&error=1"],
];

(async () => {
  const browser = await chromium.launch({
    headless: true,
    executablePath: "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  });
  const results = [];
  for (const [name, width, height, hash] of cases) {
    const page = await browser.newPage({ viewport: { width, height } });
    await page.goto(`http://127.0.0.1:1420/${hash}`);
    await page.locator("[data-testid='page-heading']").last().waitFor();
    await page.waitForTimeout(150);
    const metrics = await page.evaluate(() => {
      const buttons = [...document.querySelectorAll(".task-row__actions .icon-button")];
      return {
        viewportWidth: window.innerWidth,
        documentWidth: document.documentElement.scrollWidth,
        overflowing: [...document.querySelectorAll(".task-row, .task-toolbar, .tasks-heading")]
          .filter((element) => element.getBoundingClientRect().right > window.innerWidth + 1).length,
        undersizedButtons: buttons.filter((button) => {
          const rect = button.getBoundingClientRect();
          return rect.width < 36 || rect.height < 36;
        }).length,
        clippedButtons: buttons.filter((button) => {
          const rect = button.getBoundingClientRect();
          return rect.left < 0 || rect.right > window.innerWidth;
        }).length,
      };
    });
    await page.screenshot({ path: path.join(root, `${name}.png`), fullPage: true });
    results.push({ name, ...metrics });
    await page.close();
  }
  await browser.close();
  console.log(JSON.stringify(results, null, 2));
  if (results.some((result) => result.documentWidth > result.viewportWidth || result.overflowing || result.undersizedButtons || result.clippedButtons)) process.exitCode = 1;
})();
