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
  alternatives?: string[];
  candidates?: Array<{
    name: string;
    quantity: number;
    tier: string;
    tier_value: number;
    volatile: boolean;
    unit_price?: number | null;
    probability?: number | null;
  }>;
  group_label?: string | null;
};

type ScanResult = {
  ores: Record<string, Ore>;
  scanner_active: boolean;
  configured?: boolean;
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

test('the whole header text area is a window drag region (#108)', async ({ page }) => {
  // Tauri starts a window drag only when the mousedown target itself carries
  // data-tauri-drag-region. elementFromPoint mirrors that hit-test: a press on
  // the title/status text must fall through to .header-drag, not be swallowed
  // by a display-only child.
  for (const selector of ['.title', '.status-text', '.status-dot']) {
    const hitsDragRegion = await page.locator(selector).evaluate((el) => {
      const { left, top, width, height } = el.getBoundingClientRect();
      const hit = document.elementFromPoint(left + width / 2, top + height / 2);
      return hit !== null && hit.hasAttribute('data-tauri-drag-region');
    });
    expect(hitsDragRegion, `a press on ${selector} should land on the drag region`).toBe(true);
  }
});

test('header buttons are not part of the drag region (#108)', async ({ page }) => {
  // The action buttons must keep receiving their own presses — inside a drag
  // region the window drag would swallow the click.
  for (const name of ['Settings', 'Set scan region', 'Close overlay']) {
    const pressHitsButton = await page.getByRole('button', { name }).evaluate((el) => {
      const { left, top, width, height } = el.getBoundingClientRect();
      const hit = document.elementFromPoint(left + width / 2, top + height / 2);
      return el.contains(hit) && hit !== null && !hit.closest('[data-tauri-drag-region]');
    });
    expect(pressHitsButton, `a press on "${name}" should hit the button itself`).toBe(true);
  }
});

test('prompts to set a scan region when none is configured', async ({ page }) => {
  await emitScan(page, { ores: {}, scanner_active: false, configured: false });
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
});

test('shows volatile and low-confidence badges appropriately', async ({ page }) => {
  await emitScan(page, {
    ores: { quantainium: QUANTANIUM, beryl: BERYL },
    scanner_active: true,
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
  });

  await expect(page.locator('.status-text')).toHaveText('SCANNING');
  await expect(page.locator('.message')).toContainText('No ores detected');
});

test('settings: gear opens the panel, a preset selects, Done closes it', async ({ page }) => {
  await page.getByRole('button', { name: 'Settings' }).click();
  // Panel loads (get_config no-ops in the browser -> defaults).
  await expect(page.getByRole('button', { name: 'Balanced' })).toBeVisible();
  await page.getByRole('button', { name: 'Responsive' }).click();
  await expect(page.getByRole('button', { name: 'Responsive' })).toHaveClass(/active/);
  await page.getByRole('button', { name: 'Done' }).click();
  await expect(page.locator('.settings')).toHaveCount(0);
});

test('shows the alternative reading for an ambiguous signature', async ({ page }) => {
  await emitScan(page, {
    ores: {
      savrilium: {
        name: 'Savrilium',
        quantity: 6,
        tier: 'A',
        tier_value: 3,
        volatile: false,
        confidence: 1,
        detected_rs: 19200,
        unit_price: null,
        alternatives: ['5x Aslarite'],
      },
    },
    scanner_active: true,
  });
  const card = page.locator('.ore-card', { hasText: 'Savrilium' });
  await expect(card.locator('.ore-alt')).toContainText('5x Aslarite');
});

test('omits the price line when an ore has no known price', async ({ page }) => {
  await emitScan(page, {
    ores: { beryl: { ...BERYL, unit_price: null } },
    scanner_active: true,
  });
  const beryl = page.locator('.ore-card', { hasText: 'Beryl' });
  await expect(beryl).toBeVisible();
  await expect(beryl.locator('.ore-value')).toHaveCount(0);
});

test('renders a signature-degenerate reading as a grouped per-candidate card', async ({ page }) => {
  await emitScan(page, {
    ores: {
      hadanite: {
        name: 'Hadanite', quantity: 6, tier: 'S', tier_value: 4, volatile: false,
        confidence: 1, detected_rs: 18000, group_label: 'Gem',
        candidates: [
          { name: 'Aphorite', quantity: 6, tier: 'B', tier_value: 2, volatile: false, unit_price: 110000, probability: 58.6 },
          { name: 'Dolivine', quantity: 6, tier: 'A', tier_value: 3, volatile: false, unit_price: 160000, probability: 35.4 },
          { name: 'Hadanite', quantity: 6, tier: 'S', tier_value: 4, volatile: false, unit_price: 600000, probability: 6 },
        ],
      },
    },
    scanner_active: true,
  });

  const group = page.locator('.ore-card-group');
  await expect(group).toHaveCount(1);
  await expect(group.locator('.ore-name')).toContainText('Gem');
  await expect(group.locator('.ore-name')).toContainText('6x');
  await expect(group.locator('.ore-sig')).toContainText('18000');

  // One row per candidate, ranked by spawn probability (Aphorite 58.6% first).
  const rows = group.locator('.candidate');
  await expect(rows).toHaveCount(3);
  await expect(rows.nth(0).locator('.candidate-name')).toContainText('Aphorite');
  await expect(rows.nth(0).locator('.candidate-prob')).toHaveText('59%');
});
