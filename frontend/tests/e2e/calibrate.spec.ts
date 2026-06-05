import { test, expect, Page } from '@playwright/test';

/**
 * Calibration UI. It loads the same bundle (reached here via the ?calibrate
 * fallback in App.tsx). There's no Tauri runtime in a browser, so get_capture
 * fails and the component uses its dev-only mock screenshot — enough to exercise
 * the draw / move / resize / redraw interactions on the stage. Confirm's
 * save/close is a no-op here. Coordinates are checked as deltas so they're
 * independent of the capture->display scale.
 */

const VIEW = { width: 1280, height: 720 };
const TOL = 7;

async function stageBox(page: Page) {
  const b = await page.locator('.calibrate-stage').boundingBox();
  expect(b, 'stage should have a bounding box').not.toBeNull();
  return b!;
}

async function drawBox(page: Page) {
  const s = await stageBox(page);
  await page.mouse.move(s.x + s.width * 0.2, s.y + s.height * 0.25);
  await page.mouse.down();
  await page.mouse.move(s.x + s.width * 0.6, s.y + s.height * 0.7, { steps: 8 });
  await page.mouse.up();
}

test.beforeEach(async ({ page }) => {
  await page.setViewportSize(VIEW);
  await page.goto('/?calibrate');
  // Stage renders once the (mock) screenshot has loaded and been sized.
  await expect(page.locator('.calibrate-stage')).toBeVisible();
});

test('shows the panel with instructions and cancel, no box yet', async ({ page }) => {
  await expect(page.locator('.calibrate-title')).toHaveText('Set scan region');
  await expect(page.locator('.calibrate-steps')).toContainText('RS');
  await expect(page.getByRole('button', { name: 'Cancel (Esc)' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Confirm (Enter)' })).toHaveCount(0);
  await expect(page.locator('.calibrate-box')).toHaveCount(0);
});

test('dragging draws a box and reveals Confirm/Redraw + size readout', async ({ page }) => {
  await drawBox(page);
  await expect(page.locator('.calibrate-box')).toBeVisible();
  await expect(page.locator('.calibrate-size')).toContainText('px');
  await expect(page.getByRole('button', { name: 'Confirm (Enter)' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Redraw' })).toBeVisible();
});

test('the box can be moved by dragging its body', async ({ page }) => {
  await drawBox(page);
  const before = (await page.locator('.calibrate-box').boundingBox())!;
  const cx = before.x + before.width / 2;
  const cy = before.y + before.height / 2;
  await page.mouse.move(cx, cy);
  await page.mouse.down();
  await page.mouse.move(cx + 50, cy + 30, { steps: 8 });
  await page.mouse.up();
  const after = (await page.locator('.calibrate-box').boundingBox())!;
  expect(Math.abs(after.x - before.x - 50)).toBeLessThanOrEqual(TOL);
  expect(Math.abs(after.y - before.y - 30)).toBeLessThanOrEqual(TOL);
  expect(Math.abs(after.width - before.width)).toBeLessThanOrEqual(3);
});

test('the box can be resized via the SE handle', async ({ page }) => {
  await drawBox(page);
  const before = (await page.locator('.calibrate-box').boundingBox())!;
  const handle = (await page.locator('.handle-se').boundingBox())!;
  await page.mouse.move(handle.x + handle.width / 2, handle.y + handle.height / 2);
  await page.mouse.down();
  await page.mouse.move(handle.x + 40, handle.y + 25, { steps: 8 });
  await page.mouse.up();
  const after = (await page.locator('.calibrate-box').boundingBox())!;
  expect(Math.abs(after.width - before.width - 40)).toBeLessThanOrEqual(TOL);
  expect(Math.abs(after.height - before.height - 25)).toBeLessThanOrEqual(TOL);
});

test('Redraw clears the box', async ({ page }) => {
  await drawBox(page);
  await expect(page.locator('.calibrate-box')).toBeVisible();
  await page.getByRole('button', { name: 'Redraw' }).click();
  await expect(page.locator('.calibrate-box')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Confirm (Enter)' })).toHaveCount(0);
});

test('pressing on the panel does not start a drag', async ({ page }) => {
  const title = (await page.locator('.calibrate-title').boundingBox())!;
  await page.mouse.move(title.x + 10, title.y + 5);
  await page.mouse.down();
  await page.mouse.move(title.x + 200, title.y + 200, { steps: 6 });
  await page.mouse.up();
  await expect(page.locator('.calibrate-box')).toHaveCount(0);
});
