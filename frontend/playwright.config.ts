import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright config for the overlay's display-level E2E tests.
 *
 * Boots the real Vite dev server (port 1420, same as Tauri uses) and drives the
 * overlay in a headless browser. The tests stand up a mock backend WebSocket so
 * the overlay renders exactly what it would with a live scan.
 */
export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? 'list' : 'line',
  use: {
    baseURL: 'http://localhost:1420',
    trace: 'on-first-retry',
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
  ],
  webServer: {
    command: 'pnpm dev',
    url: 'http://localhost:1420',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
