import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './scripts',
  timeout: 30000,
  fullyParallel: false,
  workers: 1,
  globalSetup: './scripts/global-setup.ts',
  use: {
    baseURL: 'http://127.0.0.1:1420',
    viewport: { width: 1600, height: 1000 },
    deviceScaleFactor: 1,
    headless: true,
  },
});
