const fs = require("node:fs");
const path = require("node:path");
const { chromium } = require("C:/Users/yangjianlin/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright");

const root = path.resolve(
  __dirname,
  "../docs/requests/bilicatch-v1-desktop/module-runs/authentication/verification/screenshots",
);
fs.mkdirSync(root, { recursive: true });

const cases = [
  ["auth-waiting-1280-light", 1280, 800, "waiting_scan", "light", false],
  ["auth-waiting-900-dark", 900, 700, "waiting_scan", "dark", false],
  ["auth-confirm-800-light", 800, 650, "waiting_confirm", "light", false],
  ["auth-requesting-900-light", 900, 700, "requesting", "light", false],
  ["auth-expired-900-light", 900, 700, "expired", "light", false],
  ["auth-error-900-dark", 900, 700, "error", "dark", false],
  ["auth-authenticated-900-light", 900, 700, "authenticated", "light", true],
];

(async () => {
  const browser = await chromium.launch({
    headless: true,
    executablePath: "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  });
  const results = [];
  for (const [name, width, height, status, theme, hold] of cases) {
    const page = await browser.newPage({ viewport: { width, height } });
    const holdQuery = hold ? "&hold=1" : "";
    await page.goto(
      `http://127.0.0.1:1422/#/login?demo=1&auth=${status}&theme=${theme}${holdQuery}`,
    );
    await page.locator("[data-testid='page-heading']").waitFor();
    if (status === "waiting_scan" || status === "waiting_confirm") {
      await page.locator("canvas").waitFor();
      await page.waitForFunction(() => {
        const canvas = document.querySelector("canvas");
        if (!(canvas instanceof HTMLCanvasElement)) return false;
        const pixels = canvas.getContext("2d")?.getImageData(0, 0, canvas.width, canvas.height).data;
        if (!pixels) return false;
        let nonWhite = 0;
        for (let index = 0; index < pixels.length; index += 4) {
          if (pixels[index] < 245 || pixels[index + 1] < 245 || pixels[index + 2] < 245) nonWhite += 1;
        }
        return nonWhite > 1000;
      });
    }
    await page.waitForTimeout(100);
    const metrics = await page.evaluate((currentStatus) => {
      const rect = (selector) => document.querySelector(selector)?.getBoundingClientRect().toJSON();
      const intro = rect(".auth-intro");
      const tool = rect(".auth-tool");
      const visibleButtons = [...document.querySelectorAll("button")]
        .filter((button) => button.getClientRects().length > 0);
      const canvas = document.querySelector("canvas");
      let canvasNonWhite = null;
      if (canvas instanceof HTMLCanvasElement) {
        const pixels = canvas.getContext("2d")?.getImageData(0, 0, canvas.width, canvas.height).data;
        if (pixels) {
          canvasNonWhite = 0;
          for (let index = 0; index < pixels.length; index += 4) {
            if (pixels[index] < 245 || pixels[index + 1] < 245 || pixels[index + 2] < 245) {
              canvasNonWhite += 1;
            }
          }
        }
      }
      return {
        status: currentStatus,
        viewportWidth: window.innerWidth,
        documentWidth: document.documentElement.scrollWidth,
        clippedRegions: [intro, tool].filter(
          (region) => region && (region.left < 0 || region.right > window.innerWidth + 1),
        ).length,
        overlappingRegions: intro && tool
          ? (Math.min(intro.right, tool.right) - Math.max(intro.left, tool.left) > 1
            && Math.min(intro.bottom, tool.bottom) - Math.max(intro.top, tool.top) > 1 ? 1 : 0)
          : 0,
        undersizedButtons: visibleButtons.filter((button) => {
          const bounds = button.getBoundingClientRect();
          return bounds.width < 36 || bounds.height < 36;
        }).length,
        canvasSize: canvas instanceof HTMLCanvasElement ? [canvas.width, canvas.height] : null,
        canvasNonWhite,
        qrBackground: canvas
          ? getComputedStyle(canvas.closest(".qr-stage")).backgroundColor
          : null,
        qrTextLeaked: document.body.innerText.includes("passport.bilibili.com"),
      };
    }, status);
    await page.screenshot({ path: path.join(root, `${name}.png`), fullPage: true });
    results.push({ name, ...metrics });
    await page.close();
  }
  await browser.close();
  console.log(JSON.stringify(results, null, 2));
  const failed = results.some((result) => {
    const expectsCanvas = result.status === "waiting_scan" || result.status === "waiting_confirm";
    return result.documentWidth > result.viewportWidth
      || result.clippedRegions > 0
      || result.overlappingRegions > 0
      || result.undersizedButtons > 0
      || result.qrTextLeaked
      || (expectsCanvas && (
        result.canvasSize?.[0] !== 224
        || result.canvasSize?.[1] !== 224
        || result.canvasNonWhite < 1000
        || result.qrBackground !== "rgb(255, 255, 255)"
      ));
  });
  if (failed) process.exitCode = 1;
})();
