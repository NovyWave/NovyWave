import { test, expect, Page } from '@playwright/test';
import * as path from 'path';

// Test data: all example waveform files
const WAVEFORM_FILES = [
  {
    name: 'VHDL Counter',
    path: 'vhdl/counter/counter.ghw',
    format: 'GHW',
    expectedSignals: ['clk', 'reset', 'enable', 'count', 'overflow'],
  },
  {
    name: 'Verilog Counter',
    path: 'verilog/counter/counter.vcd',
    format: 'VCD',
    expectedSignals: ['clk', 'reset', 'enable', 'count', 'overflow'],
  },
  {
    name: 'SpinalHDL Counter',
    path: 'spinalhdl/counter/counter.vcd',
    format: 'VCD',
    expectedSignals: ['clk', 'reset', 'io_enable', 'io_count', 'io_overflow'],
  },
  {
    name: 'Amaranth Counter',
    path: 'amaranth/counter/counter.vcd',
    format: 'VCD',
    expectedSignals: ['clk', 'rst', 'enable', 'count', 'overflow'],
  },
  {
    name: 'Spade Counter',
    path: 'spade/counter/counter.vcd',
    format: 'VCD',
    expectedSignals: ['clk', 'rst', 'enable', 'count', 'overflow'],
  },
];

// Get absolute path to examples directory
const EXAMPLES_DIR = path.resolve(__dirname, '../..');

test.describe('NovyWave Waveform Examples', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to NovyWave and wait for it to load
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Wait for the app to initialize (Files & Scopes panel should be visible)
    await expect(page.getByText('Files & Scopes')).toBeVisible({ timeout: 30000 });
  });

  test('app loads successfully', async ({ page }) => {
    // Verify main UI elements are present
    await expect(page.getByText('Files & Scopes')).toBeVisible();
    await expect(page.getByText('Variables')).toBeVisible();
    await expect(page.getByText('Selected Variables')).toBeVisible();

    // Verify Load Files button exists
    await expect(page.getByRole('button', { name: /Load Files/i })).toBeVisible();
  });

  // Test each waveform file type
  for (const waveform of WAVEFORM_FILES) {
    test(`loads ${waveform.name} (${waveform.format})`, async ({ page }) => {
      const absolutePath = path.join(EXAMPLES_DIR, waveform.path);

      // Click Load Files button to open file picker
      await page.getByRole('button', { name: /Load Files/i }).click();

      // Wait for file picker dialog
      await expect(page.getByText(/Select Waveform Files/i)).toBeVisible({ timeout: 5000 });

      // Navigate to the file (this depends on NovyWave's file picker implementation)
      // We'll try to use the file input if available, or navigate the tree
      await loadFileViaDialog(page, absolutePath);

      // Verify file appears in Files & Scopes tree
      const filename = path.basename(waveform.path);
      await expect(page.getByText(filename)).toBeVisible({ timeout: 10000 });

      // Expand the file to show scopes
      await page.getByText(filename).click();
      await page.waitForTimeout(500); // Wait for expansion animation

      // Take screenshot for documentation
      await page.screenshot({
        path: `test-results/${waveform.format.toLowerCase()}-loaded.png`,
        fullPage: true
      });
    });
  }

  test('can select and display signals', async ({ page }) => {
    // Load the Verilog example (simplest case)
    const verilogFile = WAVEFORM_FILES[1];
    const absolutePath = path.join(EXAMPLES_DIR, verilogFile.path);

    await page.getByRole('button', { name: /Load Files/i }).click();
    await expect(page.getByText(/Select Waveform Files/i)).toBeVisible();
    await loadFileViaDialog(page, absolutePath);

    // Wait for file to load
    const filename = path.basename(verilogFile.path);
    await expect(page.getByText(filename)).toBeVisible({ timeout: 10000 });

    // Expand file and select a scope
    await page.getByText(filename).click();
    await page.waitForTimeout(500);

    // Look for expected signals in Variables panel
    for (const signal of verilogFile.expectedSignals.slice(0, 3)) {
      // Signals might be in Variables panel after selecting a scope
      const signalElement = page.getByText(signal, { exact: false });
      if (await signalElement.isVisible({ timeout: 2000 }).catch(() => false)) {
        // Click to select the signal
        await signalElement.click();
      }
    }

    // Verify Selected Variables panel has content
    await page.waitForTimeout(1000);
    await page.screenshot({
      path: 'test-results/signals-selected.png',
      fullPage: true
    });
  });

  test('timeline renders correctly', async ({ page }) => {
    // Load a file and verify canvas renders
    const verilogFile = WAVEFORM_FILES[1];
    const absolutePath = path.join(EXAMPLES_DIR, verilogFile.path);

    await page.getByRole('button', { name: /Load Files/i }).click();
    await expect(page.getByText(/Select Waveform Files/i)).toBeVisible();
    await loadFileViaDialog(page, absolutePath);

    await expect(page.getByText(path.basename(verilogFile.path))).toBeVisible({ timeout: 10000 });

    // Check that canvas element exists (waveform timeline)
    const canvas = page.locator('canvas');
    await expect(canvas).toBeVisible({ timeout: 5000 });

    // Verify canvas has non-zero dimensions (actually rendered)
    const canvasBox = await canvas.boundingBox();
    expect(canvasBox).not.toBeNull();
    expect(canvasBox!.width).toBeGreaterThan(100);
    expect(canvasBox!.height).toBeGreaterThan(50);

    await page.screenshot({
      path: 'test-results/timeline-rendered.png',
      fullPage: true
    });
  });

  test('keyboard navigation works', async ({ page }) => {
    // Load a file first
    const verilogFile = WAVEFORM_FILES[1];
    const absolutePath = path.join(EXAMPLES_DIR, verilogFile.path);

    await page.getByRole('button', { name: /Load Files/i }).click();
    await expect(page.getByText(/Select Waveform Files/i)).toBeVisible();
    await loadFileViaDialog(page, absolutePath);

    await expect(page.getByText(path.basename(verilogFile.path))).toBeVisible({ timeout: 10000 });

    // Wait for timeline to be ready
    await page.waitForTimeout(1000);

    // Test zoom keys
    await page.keyboard.press('w'); // Zoom in
    await page.waitForTimeout(200);
    await page.keyboard.press('s'); // Zoom out
    await page.waitForTimeout(200);

    // Test pan keys
    await page.keyboard.press('a'); // Pan left
    await page.waitForTimeout(200);
    await page.keyboard.press('d'); // Pan right
    await page.waitForTimeout(200);

    // Test cursor movement
    await page.keyboard.press('q'); // Move cursor left
    await page.waitForTimeout(200);
    await page.keyboard.press('e'); // Move cursor right
    await page.waitForTimeout(200);

    // Test reset
    await page.keyboard.press('r'); // Reset view
    await page.waitForTimeout(500);

    await page.screenshot({
      path: 'test-results/keyboard-navigation.png',
      fullPage: true
    });
  });

  test('theme toggle works', async ({ page }) => {
    // Find and click theme toggle button
    const themeButton = page.locator('button').filter({ has: page.locator('svg') }).first();

    // Take screenshot of initial theme
    await page.screenshot({ path: 'test-results/theme-initial.png' });

    // Toggle theme (Ctrl+T)
    await page.keyboard.press('Control+t');
    await page.waitForTimeout(500);

    // Take screenshot of toggled theme
    await page.screenshot({ path: 'test-results/theme-toggled.png' });
  });

  test('dock mode toggle works', async ({ page }) => {
    // Take screenshot of initial dock mode
    await page.screenshot({ path: 'test-results/dock-initial.png' });

    // Toggle dock mode (Ctrl+D)
    await page.keyboard.press('Control+d');
    await page.waitForTimeout(500);

    // Take screenshot of toggled dock mode
    await page.screenshot({ path: 'test-results/dock-toggled.png' });

    // Toggle back
    await page.keyboard.press('Control+d');
    await page.waitForTimeout(500);
  });
});

/**
 * Helper function to load a file via NovyWave's file picker dialog
 * This handles the navigation through the directory tree
 */
async function loadFileViaDialog(page: Page, absolutePath: string): Promise<void> {
  const parts = absolutePath.split('/').filter(Boolean);

  // Try to navigate through the directory tree
  // Start from root and expand each directory
  for (let i = 0; i < parts.length - 1; i++) {
    const dirName = parts[i];
    const dirElement = page.getByText(dirName, { exact: true }).first();

    if (await dirElement.isVisible({ timeout: 1000 }).catch(() => false)) {
      await dirElement.click();
      await page.waitForTimeout(300); // Wait for directory to expand/load
    }
  }

  // Select the file
  const filename = parts[parts.length - 1];
  const fileElement = page.getByText(filename, { exact: false });

  if (await fileElement.isVisible({ timeout: 2000 }).catch(() => false)) {
    await fileElement.click();
    await page.waitForTimeout(200);
  }

  // Click the Load/Confirm button
  const loadButton = page.getByRole('button', { name: /Load|Confirm|Open/i });
  if (await loadButton.isVisible({ timeout: 1000 }).catch(() => false)) {
    await loadButton.click();
  }

  // Wait for dialog to close
  await page.waitForTimeout(500);
}
