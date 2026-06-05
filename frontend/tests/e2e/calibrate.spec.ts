import { test, expect, Page } from '@playwright/test';

/**
 * Calibration overlay (a port of the Python calibrate.py): full-screen drag to
 * select a region. Reached in a browser via the ?calibrate fallback in App.tsx.
 * No Tauri runtime here, so save/close on release is a no-op — these tests cover
 * the panel and the drag-to-draw behaviour. devicePixelRatio is 1 headless, so
 * client coords == box coords.
 */

const VIEW = { width: 1280, height: 720 };
// A region safely below the instruction panel (panel sits top-center, ~y < 180).
const BOX = { x1: 200, y1: 300, x2: 520, y2: 520 };
const EXPECT = { x: 200, y: 300, w: 320, h: 220 };
const TOL = 6;

async function near(page: Page, exp: { x: number; y: number; w: number; h: number }) {
  const bb = await page.locator('.calibrate-box').boundingBox();
  expect(bb, 'box should have a bounding box').not.toBeNull();
  expect(Math.abs(bb!.x - exp.x)).toBeLessThanOrEqual(TOL);
  expect(Math.abs(bb!.y - exp.y)).toBeLessThanOrEqual(TOL);
  expect(Math.abs(bb!.width - exp.w)).toBeLessThanOrEqual(TOL);
  expect(Math.abs(bb!.height - exp.h)).toBeLessThanOrEqual(TOL);
}

async function drawBox(page: Page) {
  await page.mouse.move(BOX.x1, BOX.y1);
  await page.mouse.down();
  await page.mouse.move(BOX.x2, BOX.y2, { steps: 8 });
  // Note: we don't release here — release triggers save+close (a no-op in the
  // browser) but leaves the box on screen; tests assert mid- and post-release.
}

test.beforeEach(async ({ page }) => {
  await page.setViewportSize(VIEW);
  await page.goto('/?calibrate');
  await expect(page.locator('.calibrate')).toBeVisible();
});

test('shows the panel with instructions and cancel', async ({ page }) => {
  await expect(page.locator('.calibrate-title')).toHaveText('Set scan region');
  await expect(page.locator('.calibrate-steps')).toContainText('RS');
  await expect(page.getByRole('button', { name: 'Cancel (Esc)' })).toBeVisible();
  await expect(page.locator('.calibrate-box')).toHaveCount(0);
});

test('dragging draws a selection box of the expected geometry', async ({ page }) => {
  await drawBox(page);
  await expect(page.locator('.calibrate-box')).toBeVisible();
  await near(page, EXPECT);
  await page.mouse.up();
});

test('a tiny drag does not leave a box', async ({ page }) => {
  await page.mouse.move(300, 300);
  await page.mouse.down();
  await page.mouse.move(305, 304, { steps: 3 });
  await page.mouse.up();
  await expect(page.locator('.calibrate-box')).toHaveCount(0);
});

test('pressing on the panel does not start a drag', async ({ page }) => {
  const title = (await page.locator('.calibrate-title').boundingBox())!;
  await page.mouse.move(title.x + 10, title.y + 5);
  await page.mouse.down();
  await page.mouse.move(title.x + 200, title.y + 220, { steps: 6 });
  await page.mouse.up();
  await expect(page.locator('.calibrate-box')).toHaveCount(0);
});
