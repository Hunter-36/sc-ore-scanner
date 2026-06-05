import { test, expect, Page } from '@playwright/test';

/**
 * Overlay display tests. In a plain browser there's no Tauri runtime, so the
 * Rust core never emits "scan-result". Instead we drive the real store +
 * components through the dev-only `mock-scan` bridge (see useScanEvents.ts),
 * which exercises the full render path: events -> store -> sorting -> OreCard.
 */

type Ore = {
  name: string;
  quantity: number;
  tier: string;
  tier_value: number;
  volatile: boolean;
  confidence: number;
  detected_rs: number;
  unit_price?: number | null;
};

type ScanResult = {
  ores: Record<string, Ore>;
  scanner_active: boolean;
  configured?: boolean;
  timestamp: number;
  session?: { distinct_ores: number; total_detections: number };
};

async function emitScan(page: Page, result: ScanResult) {
  await page.evaluate(
    (r) => window.dispatchEvent(new CustomEvent('mock-scan', { detail: r })),
    result,
  );
}

const QUANTANIUM: Ore = {
  name: 'Quantanium',
  quantity: 2,
  tier: 'S',
  tier_value: 4,
  volatile: true,
  confidence: 0.95,
  detected_rs: 1234,
  unit_price: 88,
};

const BERYL: Ore = {
  name: 'Beryl',
  quantity: 3,
  tier: 'B',
  tier_value: 2,
  volatile: false,
  confidence: 0.72,
  detected_rs: 7080,
  unit_price: 40,
};

test.beforeEach(async ({ page }) => {
  await page.goto('/');
});

test('renders the starting/offline state without a backend', async ({ page }) => {
  await expect(page.locator('.title')).toHaveText('SC ORE SCANNER');
  await expect(page.locator('.status-text')).toHaveText('OFFLINE');
  await expect(page.locator('.status-dot')).toHaveClass(/disconnected/);
  await expect(page.locator('.message')).toContainText('Starting scanner');
});

test('header exposes the calibrate and close buttons', async ({ page }) => {
  const calibrate = page.getByRole('button', { name: 'Set scan region' });
  await expect(calibrate).toBeVisible();
  await expect(calibrate).toContainText('Set region');
  await expect(page.getByRole('button', { name: 'Close overlay' })).toBeVisible();
});

test('prompts to set a scan region when none is configured', async ({ page }) => {
  await emitScan(page, { ores: {}, scanner_active: false, configured: false, timestamp: 1 });
  await expect(page.locator('.message')).toContainText('Set your scan region');
  await expect(page.locator('.message')).toContainText('Set region');
});

test('clicking calibrate does not crash the overlay (no Tauri in browser)', async ({ page }) => {
  await page.getByRole('button', { name: 'Set scan region' }).click();
  await expect(page.locator('.overlay')).toBeVisible();
  await expect(page.locator('.title')).toHaveText('SC ORE SCANNER');
});

test('renders ore cards from a scan, sorted by tier, with prices', async ({ page }) => {
  await emitScan(page, {
    ores: { quantainium: QUANTANIUM, beryl: BERYL },
    scanner_active: true,
    timestamp: 1,
    session: { distinct_ores: 2, total_detections: 5 },
  });

  await expect(page.locator('.status-text')).toHaveText('SCANNING');
  await expect(page.locator('.status-dot')).toHaveClass(/connected/);

  const cards = page.locator('.ore-card');
  await expect(cards).toHaveCount(2);

  // Tier S (tier_value 4) sorts above tier B (tier_value 2).
  await expect(cards.nth(0).locator('.ore-name')).toHaveText('Quantanium');
  await expect(cards.nth(1).locator('.ore-name')).toHaveText('Beryl');

  await expect(cards.nth(0).locator('.ore-quantity')).toContainText('2x');
  await expect(cards.nth(1)).toContainText('≈ 40 aUEC/SCU');

  await expect(page.locator('.session-footer')).toContainText('5 detections');
  await expect(page.locator('.session-footer')).toContainText('2 types');
});

test('shows volatile and low-confidence badges appropriately', async ({ page }) => {
  await emitScan(page, {
    ores: { quantainium: QUANTANIUM, beryl: BERYL },
    scanner_active: true,
    timestamp: 1,
  });

  // Quantanium is volatile (⚠) and high-confidence (no % badge).
  const quant = page.locator('.ore-card', { hasText: 'Quantanium' });
  await expect(quant.locator('.volatile-badge')).toBeVisible();
  await expect(quant.locator('.ore-confidence')).toHaveCount(0);

  // Beryl is non-volatile but 72% confidence -> shows ~72%.
  const beryl = page.locator('.ore-card', { hasText: 'Beryl' });
  await expect(beryl.locator('.volatile-badge')).toHaveCount(0);
  await expect(beryl.locator('.ore-confidence')).toHaveText('~72%');
});

test('shows "no ores" when scanning but nothing detected', async ({ page }) => {
  await emitScan(page, {
    ores: {},
    scanner_active: true,
    timestamp: 1,
    session: { distinct_ores: 0, total_detections: 0 },
  });

  await expect(page.locator('.status-text')).toHaveText('SCANNING');
  await expect(page.locator('.message')).toContainText('No ores detected');
  // No detections yet -> no session footer.
  await expect(page.locator('.session-footer')).toHaveCount(0);
});

test('omits the price line when an ore has no known price', async ({ page }) => {
  await emitScan(page, {
    ores: { beryl: { ...BERYL, unit_price: null } },
    scanner_active: true,
    timestamp: 1,
  });
  const beryl = page.locator('.ore-card', { hasText: 'Beryl' });
  await expect(beryl).toBeVisible();
  await expect(beryl.locator('.ore-value')).toHaveCount(0);
});
