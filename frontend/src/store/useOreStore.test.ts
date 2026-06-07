import { describe, it, expect, beforeEach } from 'vitest';
import { useOreStore, ScanResult } from './useOreStore';

const initialState = {
  ores: {},
  scannerActive: false,
  configured: false,
  connected: false,
};

const sampleScan: ScanResult = {
  ores: {
    beryl: {
      name: 'Beryl',
      quantity: 3,
      tier: 'A',
      tier_value: 3,
      volatile: false,
      confidence: 1,
      detected_rs: 10620,
    },
  },
  scanner_active: true,
};

describe('useOreStore', () => {
  beforeEach(() => {
    useOreStore.setState(initialState);
  });

  it('starts empty and disconnected', () => {
    const s = useOreStore.getState();
    expect(s.ores).toEqual({});
    expect(s.scannerActive).toBe(false);
    expect(s.connected).toBe(false);
  });

  it('updateFromScan applies ores and scanner state', () => {
    useOreStore.getState().updateFromScan(sampleScan);
    const s = useOreStore.getState();
    expect(Object.keys(s.ores)).toEqual(['beryl']);
    expect(s.ores.beryl.quantity).toBe(3);
    expect(s.ores.beryl.name).toBe('Beryl');
    expect(s.scannerActive).toBe(true);
    expect(s.configured).toBe(true);
  });

  it('setConnected toggles the connection flag', () => {
    useOreStore.getState().setConnected(true);
    expect(useOreStore.getState().connected).toBe(true);
    useOreStore.getState().setConnected(false);
    expect(useOreStore.getState().connected).toBe(false);
  });

  it('preserves candidates + group_label for a degenerate (ambiguous) reading', () => {
    const scan: ScanResult = {
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
    };
    useOreStore.getState().updateFromScan(scan);
    const ore = useOreStore.getState().ores.hadanite;
    expect(ore.candidates).toHaveLength(3);
    expect(ore.group_label).toBe('Gem');
    expect(ore.candidates?.[0].probability).toBe(58.6);
  });
});
