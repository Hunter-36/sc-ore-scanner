import { test, expect } from '@playwright/test';
import { WebSocketServer, WebSocket } from 'ws';

/**
 * The overlay connects to the backend at ws://127.0.0.1:8765/ws. We stand up a
 * mock server on that port that pushes a canned scan result, then assert the
 * overlay renders the detected ore (name + quantity + tier + status).
 */

const SCAN_RESULT = {
  ores: {
    beryl: {
      name: 'Beryl',
      quantity: 3,
      tier: 'A',
      tier_value: 3,
      volatile: false,
      confidence: 1.0,
      detected_rs: 10620,
      unit_price: 19745,
      value: 59235,
    },
    quantanium: {
      name: 'Quantainium',
      quantity: 1,
      tier: 'S',
      tier_value: 4,
      volatile: true,
      confidence: 0.95,
      detected_rs: 3170,
    },
  },
  scanner_active: true,
  timestamp: 1700000000,
  session: { distinct_ores: 2, total_detections: 5 },
};

let wss: WebSocketServer;

test.beforeAll(() => {
  wss = new WebSocketServer({ port: 8765, path: '/ws' });
  wss.on('connection', (socket: WebSocket) => {
    socket.send(JSON.stringify(SCAN_RESULT));
  });
});

test.afterAll(() => {
  wss.close();
});

test('overlay shows connected status and renders detected ores', async ({ page }) => {
  await page.goto('/');

  // WebSocket connected + scanner active -> "SCANNING".
  await expect(page.locator('.status-text')).toHaveText('SCANNING');

  // Beryl x3 card.
  const beryl = page.locator('.ore-card', { hasText: 'Beryl' });
  await expect(beryl.locator('.ore-name')).toHaveText('Beryl');
  await expect(beryl.locator('.ore-quantity')).toContainText('3x');
  await expect(beryl.locator('.ore-tier')).toHaveText('A');
  // Estimated value (UEX) renders.
  await expect(beryl.locator('.ore-value')).toContainText('aUEC');

  // Highest-tier ore (Quantainium, S) is rendered and sorts first.
  await expect(page.locator('.ore-name').first()).toHaveText('Quantainium');
  await expect(page.locator('.ore-card', { hasText: 'Quantainium' }).locator('.ore-quantity'))
    .toContainText('1x');

  // Session footer reflects the broadcast session summary.
  await expect(page.locator('.session-footer')).toContainText('5 detections');
  await expect(page.locator('.session-footer')).toContainText('2 types');
});

test('overlay shows offline message when backend is unreachable', async ({ page }) => {
  // Close the mock server so the socket can't connect.
  await new Promise<void>((resolve) => wss.close(() => resolve()));

  await page.goto('/');
  await expect(page.locator('.status-text')).toHaveText('OFFLINE');
  await expect(page.locator('.message')).toContainText('Starting scanner');

  // Re-open for any subsequent runs/retries.
  wss = new WebSocketServer({ port: 8765, path: '/ws' });
  wss.on('connection', (socket: WebSocket) => socket.send(JSON.stringify(SCAN_RESULT)));
});
