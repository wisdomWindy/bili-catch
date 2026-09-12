const fs = require("node:fs");
const path = require("node:path");
const { chromium } = require("C:/Users/yangjianlin/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright");

const root = path.resolve(
  __dirname,
  "../docs/requests/bilicatch-v1-desktop/module-runs/audio-download/verification/screenshots",
);
fs.mkdirSync(root, { recursive: true });

const cases = [
  ["audio-anonymous-1280-light", 1280, 800, "anonymous", "light", "zh-CN"],
  ["audio-authenticated-1280-light", 1280, 800, "authenticated", "light", "zh-CN"],
  ["audio-authenticated-800-dark-en", 800, 700, "authenticated", "dark", "en-US"],
];

(async () => {
  const browser = await chromium.launch({
    headless: true,
    executablePath: "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  });
  const results = [];
  for (const [name, width, height, auth, theme, locale] of cases) {
    const page = await browser.newPage({ viewport: { width, height } });
    await page.goto(
      `http://127.0.0.1:1422/#/download?demo=1&autoparse=1&auth=${auth}&theme=${theme}&locale=${locale}`,
    );
    await page.locator(".options-section").waitFor();
    await page.locator(".mode-selector label").nth(2).click();
    const format = page.locator("[data-testid='audio-format']");
    if (auth === "authenticated") await format.selectOption("flac");
    await page.waitForTimeout(100);
    const metrics = await page.evaluate(() => {
      const formatSelect = document.querySelector("[data-testid='audio-format']");
      const profileSelect = document.querySelector("[data-testid='audio-profile']");
      const controls = [...document.querySelectorAll(".mode-selector label, .option-grid select, .action-buttons button")]
        .filter((element) => element.getClientRects().length > 0);
      const regions = [...document.querySelectorAll(".video-summary, .configuration-grid, .action-bar")];
      return {
        viewportWidth: window.innerWidth,
        documentWidth: document.documentElement.scrollWidth,
        formatValue: formatSelect?.value,
        profileValue: profileSelect?.value,
        profileDisabled: profileSelect?.disabled,
        formatOptions: formatSelect instanceof HTMLSelectElement
          ? [...formatSelect.options].map((option) => ({ value: option.value, disabled: option.disabled, text: option.textContent.trim() }))
          : [],
        clippedRegions: regions.filter((element) => {
          const bounds = element.getBoundingClientRect();
          return bounds.left < -1 || bounds.right > window.innerWidth + 1;
        }).length,
        undersizedControls: controls.filter((element) => {
          const bounds = element.getBoundingClientRect();
          return bounds.width < 36 || bounds.height < 36;
        }).length,
      };
    });
    await page.screenshot({ path: path.join(root, `${name}.png`), fullPage: true });
    results.push({ name, auth, ...metrics });
    await page.close();
  }
  await browser.close();
  console.log(JSON.stringify(results, null, 2));
  const failed = results.some((result) => {
    const flac = result.formatOptions.find((option) => option.value === "flac");
    return result.documentWidth > result.viewportWidth
      || result.clippedRegions > 0
      || result.undersizedControls > 0
      || !flac
      || flac.disabled !== (result.auth === "anonymous")
      || (result.auth === "authenticated" && (result.formatValue !== "flac" || result.profileValue !== "lossless" || !result.profileDisabled));
  });
  if (failed) process.exitCode = 1;
})();
