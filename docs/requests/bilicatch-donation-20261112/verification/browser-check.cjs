const fs = require("node:fs");
const path = require("node:path");
const { chromium } = require("C:/Users/yangjianlin/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright");
const sharp = require("C:/Users/yangjianlin/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/sharp");

const evidenceDirectory = path.resolve(__dirname, "evidence");
const scenarios = [
  { name: "light-zh-1280x900", width: 1280, height: 900, query: "demo=1&theme=light" },
  { name: "dark-zh-800x900", width: 800, height: 900, query: "demo=1&theme=dark" },
  { name: "light-en-800x900", width: 800, height: 900, query: "demo=1&theme=light&locale=en-US" },
];

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

async function inspectScreenshot(filePath) {
  const { channels, width, height } = await sharp(filePath).stats();
  return {
    width,
    height,
    standardDeviations: channels.map((channel) => Number(channel.stdev.toFixed(2))),
    nonblank: channels.some((channel) => channel.stdev > 5),
  };
}

async function inspectDonation(page) {
  const trigger = page.getByTestId("open-donation");
  await trigger.scrollIntoViewIfNeeded();
  await trigger.click();
  const dialog = page.locator("[role='dialog']");
  await dialog.waitFor({ state: "visible" });
  await page.waitForTimeout(250);

  await page.waitForFunction(() => [...document.querySelectorAll("[role='dialog'] img")]
    .every((image) => image.complete && image.naturalWidth > 0 && image.naturalHeight > 0));

  const metrics = await page.evaluate(() => {
    const dialogElement = document.querySelector("[role='dialog']");
    const images = [...document.querySelectorAll("[role='dialog'] img")];
    const dialogBox = dialogElement?.getBoundingClientRect();
    return {
      dialogCount: document.querySelectorAll("[role='dialog']").length,
      horizontalOverflow: document.documentElement.scrollWidth > window.innerWidth,
      dialogWithinViewport: Boolean(dialogBox && dialogBox.left >= 0 && dialogBox.right <= window.innerWidth),
      titleVisible: (() => {
        const titleBox = document.querySelector("#donation-dialog-title")?.getBoundingClientRect();
        return Boolean(titleBox && titleBox.top >= 0 && titleBox.bottom <= window.innerHeight);
      })(),
      scrollContainers: [...document.querySelectorAll(".n-scrollbar-container, .n-modal-scroll-content")].map((element) => ({
        className: element.className,
        scrollTop: element.scrollTop,
        clientHeight: element.clientHeight,
        scrollHeight: element.scrollHeight,
      })),
      imageDetails: images.map((image) => {
        const box = image.getBoundingClientRect();
        const naturalRatio = image.naturalWidth / image.naturalHeight;
        const renderedRatio = box.width / box.height;
        return {
          alt: image.alt,
          naturalWidth: image.naturalWidth,
          naturalHeight: image.naturalHeight,
          renderedWidth: Number(box.width.toFixed(2)),
          renderedHeight: Number(box.height.toFixed(2)),
          ratioDelta: Number(Math.abs(naturalRatio - renderedRatio).toFixed(4)),
          horizontallyContained: Boolean(dialogBox && box.left >= dialogBox.left && box.right <= dialogBox.right),
        };
      }),
    };
  });

  assert(metrics.dialogCount === 1, "expected exactly one donation dialog");
  assert(!metrics.horizontalOverflow, "page has horizontal overflow");
  assert(metrics.dialogWithinViewport, "dialog exceeds horizontal viewport bounds");
  assert(metrics.titleVisible, "dialog title is not visible when opened");
  assert(metrics.imageDetails.length === 2, "expected two QR images");
  assert(metrics.imageDetails.every((image) => image.horizontallyContained), "QR image is horizontally clipped");
  assert(metrics.imageDetails.every((image) => image.ratioDelta < 0.01), "QR image aspect ratio indicates cropping");

  await page.waitForFunction(() => document.activeElement?.getAttribute("data-testid") === "close-donation");
  const initialFocus = await page.evaluate(() => document.activeElement?.getAttribute("data-testid"));
  await page.keyboard.press("Tab");
  const tabFocusInsideModal = await page.evaluate(() => Boolean(document.activeElement?.closest(".n-modal-body-wrapper")));
  assert(tabFocusInsideModal, "Tab focus escaped the modal");

  await page.keyboard.press("Escape");
  await dialog.waitFor({ state: "hidden" });
  const escapeRestoredFocus = await trigger.evaluate((element) => document.activeElement === element);
  assert(escapeRestoredFocus, "Escape did not restore focus to the donation trigger");
  await page.waitForTimeout(250);

  await trigger.click();
  await dialog.waitFor({ state: "visible" });
  await page.mouse.click(5, 5);
  await dialog.waitFor({ state: "hidden" });
  const maskRestoredFocus = await trigger.evaluate((element) => document.activeElement === element);
  assert(maskRestoredFocus, "mask close did not restore focus to the donation trigger");
  await page.waitForTimeout(250);

  await trigger.click();
  await dialog.waitFor({ state: "visible" });
  await page.getByTestId("close-donation").click();
  await dialog.waitFor({ state: "hidden" });
  const buttonRestoredFocus = await trigger.evaluate((element) => document.activeElement === element);
  assert(buttonRestoredFocus, "close button did not restore focus to the donation trigger");
  await page.waitForTimeout(250);

  await trigger.click();
  await dialog.waitFor({ state: "visible" });
  await page.waitForTimeout(250);
  return { ...metrics, initialFocus, tabFocusInsideModal, escapeRestoredFocus, maskRestoredFocus, buttonRestoredFocus };
}

async function run() {
  fs.mkdirSync(evidenceDirectory, { recursive: true });
  const browser = await chromium.launch({
    executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe",
    headless: true,
  });
  const results = [];

  try {
    for (const scenario of scenarios) {
      const page = await browser.newPage({ viewport: { width: scenario.width, height: scenario.height } });
      const consoleErrors = [];
      page.on("console", (message) => {
        if (message.type() === "error") consoleErrors.push(message.text());
      });
      await page.goto(`http://127.0.0.1:5173/#/settings?${scenario.query}`, { waitUntil: "networkidle" });
      await page.getByTestId("page-heading").waitFor();
      const donation = await inspectDonation(page);
      const screenshotPath = path.join(evidenceDirectory, `${scenario.name}.png`);
      await page.screenshot({ path: screenshotPath });
      const screenshot = await inspectScreenshot(screenshotPath);
      assert(screenshot.nonblank, `${scenario.name} screenshot is blank`);
      assert(consoleErrors.length === 0, `${scenario.name} emitted console errors: ${consoleErrors.join(" | ")}`);
      results.push({ scenario, donation, screenshot, consoleErrors });
      await page.close();
    }

    const updatePage = await browser.newPage({ viewport: { width: 800, height: 900 } });
    await updatePage.goto("http://127.0.0.1:5173/#/settings?demo=1&theme=light&update=available", { waitUntil: "networkidle" });
    const updateDialog = updatePage.locator("[role='alertdialog']");
    await updateDialog.waitFor({ state: "visible" });
    const updateRegression = await updateDialog.evaluate((element) => ({
      className: element.className,
      title: element.querySelector("h3")?.textContent,
      withinViewport: (() => {
        const box = element.getBoundingClientRect();
        return box.left >= 0 && box.right <= window.innerWidth && box.top >= 0 && box.bottom <= window.innerHeight;
      })(),
    }));
    assert(updateRegression.className.includes("settings-dialog"), "update dialog does not use shared settings dialog class");
    assert(updateRegression.withinViewport, "update dialog exceeds viewport bounds");
    await updatePage.screenshot({ path: path.join(evidenceDirectory, "update-dialog-regression.png") });
    await updatePage.close();
    results.push({ updateRegression });
  } finally {
    await browser.close();
  }

  fs.writeFileSync(path.join(evidenceDirectory, "browser-results.json"), `${JSON.stringify(results, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(results, null, 2)}\n`);
}

run().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
