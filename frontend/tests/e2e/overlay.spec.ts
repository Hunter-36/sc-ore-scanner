import { test, expect } from '@playwright/test';

/**
 * v2: the overlay receives detections via Tauri events from the in-process Rust
 * core (no WebSocket). In a plain browser there's no Tauri runtime, so there are
 * no events and the overlay shows its "starting" state. This is a render smoke
 * test; the data path (updateFromScan) is covered by the vitest store tests and
 * by running the real app.
 */
test('overlay renders and shows the starting state without a backend', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.title')).toHaveText('SC ORE SCANNER');
  await expect(page.locator('.status-text')).toHaveText('OFFLINE');
  await expect(page.locator('.message')).toContainText('Starting scanner');
});
