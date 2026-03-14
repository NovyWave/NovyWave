#!/usr/bin/env node

const fs = require("fs");
const os = require("os");
const path = require("path");
const { chromium } = require("../examples/e2e-tests/node_modules/playwright");

const APP_URL = process.env.NOVYWAVE_APP_URL || "http://127.0.0.1:8082";
const STARTUP_TIMEOUT_MS = Number(process.env.NOVYWAVE_STARTUP_TIMEOUT_MS || 15000);
const SETTLE_DELAY_MS = Number(process.env.NOVYWAVE_SETTLE_DELAY_MS || 7000);
const WORKSPACE_TEMPLATE_ROOT =
  process.env.NOVYWAVE_WORKSPACE_TEMPLATE_ROOT || "/tmp/novywave_row_resize_bench/many_rows_valid";
const ROW_DIVIDER_SELECTOR =
  process.env.NOVYWAVE_ROW_DIVIDER_SELECTOR || '[data-testid^="selected-row-divider-"]';
const PANEL_DIVIDER_SELECTOR =
  process.env.NOVYWAVE_PANEL_DIVIDER_SELECTOR || '[data-testid="files-panel-main-divider"]';
const DRAG_SEQUENCE = [8, 16, 24, 32, 40, 48, 56, 64, 56, 48, 40, 32, 24, 16, 8, 0];

function prepareWorkspaceCopy() {
  const templateConfigPath = path.join(WORKSPACE_TEMPLATE_ROOT, ".novywave");
  const workspaceRoot = fs.mkdtempSync(path.join(os.tmpdir(), "novywave_row_resize_browser_"));
  fs.copyFileSync(templateConfigPath, path.join(workspaceRoot, ".novywave"));
  return workspaceRoot;
}

async function waitForReady(page) {
  await page.waitForFunction(
    () => {
      const api = window.__novywave_test_api;
      if (!api) {
        return false;
      }
      const selectedCount = api.getSelectedVariables().length;
      const cursorValuesCount = Object.keys(api.getCursorValues()).length;
      const timeline = api.getTimelineState();
      const canvas = document.querySelector("canvas");
      const canvasReady =
        canvas &&
        canvas.width > 1 &&
        canvas.height > 1 &&
        canvas.toDataURL().length > 1000;
      return (
        selectedCount > 0 &&
        cursorValuesCount > 0 &&
        timeline.renderCount > 0 &&
        canvasReady
      );
    },
    { timeout: STARTUP_TIMEOUT_MS }
  );
}

async function collectState(page) {
  return page.evaluate(() => {
    const api = window.__novywave_test_api;
    const timeline = api.getTimelineState();
    const selected = api.getSelectedVariables();
    const cursorValues = api.getCursorValues();
    const canvas = document.querySelector("canvas");

    return {
      selectedCount: selected.length,
      cursorValuesCount: Object.keys(cursorValues).length,
      timeline,
      canvas: canvas
        ? {
            width: canvas.width,
            height: canvas.height,
            clientWidth: canvas.clientWidth,
            clientHeight: canvas.clientHeight,
            dataUrlLength: canvas.toDataURL().length,
          }
        : null,
      firstValues: selected.slice(0, 6).map((item) => ({
        uniqueId: item.uniqueId,
        name: item.name,
        value: cursorValues[item.uniqueId] || null,
      })),
    };
  });
}

async function collectPerf(page) {
  return page.evaluate(() => window.__novywave_test_api.getPerfCounters());
}

async function runDragMeasurement(page, selector) {
  await page.evaluate(() => window.__novywave_test_api.resetPerfCounters());
  await page.waitForTimeout(200);
  const baselinePerf = await collectPerf(page);
  await page.evaluate(() => window.__novywave_test_api.startFrameSampler());

  const divider = page.locator(selector).first();
  const box = await divider.boundingBox();
  if (!box) {
    throw new Error(`Divider not found for selector: ${selector}`);
  }

  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  for (const dy of DRAG_SEQUENCE) {
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2 + dy);
  }
  await page.mouse.up();

  const frames = await page.evaluate(() => window.__novywave_test_api.stopFrameSampler());
  await page.waitForTimeout(300);
  const perf = await collectPerf(page);

  return { baselinePerf, perf, frames };
}

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  const workspaceRoot = prepareWorkspaceCopy();

  try {
    await page.goto(APP_URL, { waitUntil: "domcontentloaded" });
    await page.waitForFunction(() => Boolean(window.__novywave_test_api), {
      timeout: STARTUP_TIMEOUT_MS,
    });
    await page.evaluate((root) => window.__novywave_test_api.selectWorkspace(root), workspaceRoot);
    await page.waitForTimeout(SETTLE_DELAY_MS);
    await waitForReady(page);

    const before = await collectState(page);
    const row = await runDragMeasurement(page, ROW_DIVIDER_SELECTOR);
    const panel = await runDragMeasurement(page, PANEL_DIVIDER_SELECTOR);
    const after = await collectState(page);
    const ratio =
      panel.frames.p95Ms > 0 ? row.frames.p95Ms / panel.frames.p95Ms : null;

    const result = {
      appUrl: APP_URL,
      workspaceRoot,
      before,
      row,
      panel,
      ratio,
      after,
    };

    const hasCanvas = after.canvas && after.canvas.dataUrlLength > 1000;
    const requestClean = row.perf.requestSendCount === 0;
    const saveClean = row.perf.saveSendCount <= 1;
    const renderReady = after.selectedCount > 0 && after.cursorValuesCount > 0;
    const resizeComparable = ratio === null || ratio <= 1.25;

    console.log(JSON.stringify(result, null, 2));

    if (!hasCanvas || !requestClean || !saveClean || !renderReady || !resizeComparable) {
      process.exitCode = 1;
    }
  } finally {
    await browser.close();
    fs.rmSync(workspaceRoot, { recursive: true, force: true });
  }
}

run().catch((error) => {
  console.error(error);
  process.exit(1);
});
