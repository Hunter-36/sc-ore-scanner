import { test, expect, Page } from '@playwright/test';

/**
 * Calibration UI (the second "calibrate" window). It loads the same bundle; in a
 * browser we reach it via the ?calibrate fallback (see App.tsx). No Tauri runtime
 * here, so Confirm's save/close is a no-op — these tests cover the drag/move/
 * resize/redraw interactions and the box geometry.
 */

const VIEW = { width: 1280, height: 720 };

// A region safely below the instruction panel (panel sits top-center, ~y<170).
const BOX = { x1: 150, y1: 300, x2: 500, y2: 520 };
const EXPECT = { x: 150, y: 300, w: 350, h: 220 };
const TOL = 8; // px tolerance (2px border + rounding)

async function near(page: Page, selector: string, exp: { x: number; y: number; w: number; h: number }) {
  const bb = await page.locator(selector).boundingBox();
  expect(bb, `${selector} should have a bounding box`).not.toBeNull();
  expect(Math.abs(bb!.x - exp.x)).toBeLessThanOrEqual(TOL);
  expect(Math.abs(bb!.y - exp.y)).toBeLessThanOrEqual(TOL);
  expect(Math.abs(bb!.width - exp.w)).toBeLessThanOrEqual(TOL);
  expect(Math.abs(bb!.height - exp.h)).toBeLessThanOrEqual(TOL);
}

async function drawBox(page: Page) {
  await page.mouse.move(BOX.x1, BOX.y1);
  await page.mouse.down();
  await page.mouse.move(BOX.x2, BOX.y2, { steps: 8 });
  await page.mouse.up();
}

test.beforeEach(async ({ page }) => {
  await page.setViewportSize(VIEW);
  await page.goto('/?calibrate');
  // The ?calibrate view mounts behind an async effect — wait for it before
  // dispatching mouse events, or the drag lands on the not-yet-replaced overlay.
  await expect(page.locator('.calibrate-panel')).toBeVisible();
});

test('shows the calibration panel with instructions and cancel', async ({ page }) => {
  await expect(page.locator('.calibrate-title')).toHaveText('Set scan region');
  await expect(page.locator('.calibrate-steps')).toContainText('RS');
  await expect(page.getByRole('button', { name: 'Cancel (Esc)' })).toBeVisible();
  // No box yet -> no Confirm/Redraw.
  await expect(page.getByRole('button', { name: 'Confirm (Enter)' })).toHaveCount(0);
  await expect(page.locator('.calibrate-box')).toHaveCount(0);
});

test('dragging draws a box of the expected size, revealing Confirm/Redraw', async ({ page }) => {
  await drawBox(page);
  await expect(page.locator('.calibrate-box')).toBeVisible();
  await near(page, '.calibrate-box', EXPECT);
  await expect(page.locator('.calibrate-size')).toHaveText('350 × 220');
  await expect(page.getByRole('button', { name: 'Confirm (Enter)' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Redraw' })).toBeVisible();
});

test('the box can be moved by dragging its body', async ({ page }) => {
  await drawBox(page);
  // Drag from the box centre by (+60, +40).
  const cx = EXPECT.x + EXPECT.w / 2;
  const cy = EXPECT.y + EXPECT.h / 2;
  await page.mouse.move(cx, cy);
  await page.mouse.down();
  await page.mouse.move(cx + 60, cy + 40, { steps: 8 });
  await page.mouse.up();
  await near(page, '.calibrate-box', { x: EXPECT.x + 60, y: EXPECT.y + 40, w: EXPECT.w, h: EXPECT.h });
});

test('the box can be resized via the SE handle', async ({ page }) => {
  await drawBox(page);
  const handle = await page.locator('.handle-se').boundingBox();
  expect(handle).not.toBeNull();
  const hx = handle!.x + handle!.width / 2;
  const hy = handle!.y + handle!.height / 2;
  await page.mouse.move(hx, hy);
  await page.mouse.down();
  await page.mouse.move(hx + 40, hy + 30, { steps: 8 });
  await page.mouse.up();
  await near(page, '.calibrate-box', { x: EXPECT.x, y: EXPECT.y, w: EXPECT.w + 40, h: EXPECT.h + 30 });
});

test('Redraw clears the box', async ({ page }) => {
  await drawBox(page);
  await expect(page.locator('.calibrate-box')).toBeVisible();
  await page.getByRole('button', { name: 'Redraw' }).click();
  await expect(page.locator('.calibrate-box')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Confirm (Enter)' })).toHaveCount(0);
});

test('clicking the instruction panel does not start a drag', async ({ page }) => {
  const title = await page.locator('.calibrate-title').boundingBox();
  expect(title).not.toBeNull();
  await page.mouse.move(title!.x + 10, title!.y + 5);
  await page.mouse.down();
  await page.mouse.move(title!.x + 200, title!.y + 200, { steps: 6 });
  await page.mouse.up();
  await expect(page.locator('.calibrate-box')).toHaveCount(0);
});
