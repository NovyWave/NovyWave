import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  fullyParallel: false, // Run tests sequentially for waveform loading
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1, // Single worker to avoid file loading conflicts
  reporter: [
    ['html', { outputFolder: 'playwright-report' }],
    ['list']
  ],

  use: {
    baseURL: 'http://localhost:8080',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },

  timeout: 60000, // 60 seconds per test (waveform loading can be slow)
  expect: {
    timeout: 10000, // 10 seconds for assertions
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],

  // NovyWave dev server - expects server to be running already
  // Start with: cd ../.. && makers start
  // Or configure webServer to auto-start (commented out below)
  // webServer: {
  //   command: 'cd ../.. && makers start',
  //   url: 'http://localhost:8080',
  //   reuseExistingServer: true,
  //   timeout: 120000,
  // },
});
