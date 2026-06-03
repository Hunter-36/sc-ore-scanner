import { describe, it, expect, beforeEach } from 'vitest';
import { useOreStore, ScanResult } from './useOreStore';

const initialState = {
  ores: {},
  scannerActive: false,
  connected: false,
  lastUpdate: 0,
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
  timestamp: 123,
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

  it('updateFromScan applies ores, scanner state, and timestamp', () => {
    useOreStore.getState().updateFromScan(sampleScan);
    const s = useOreStore.getState();
    expect(Object.keys(s.ores)).toEqual(['beryl']);
    expect(s.ores.beryl.quantity).toBe(3);
    expect(s.ores.beryl.name).toBe('Beryl');
    expect(s.scannerActive).toBe(true);
    expect(s.lastUpdate).toBe(123);
  });

  it('clear wipes ores and deactivates the scanner', () => {
    useOreStore.getState().updateFromScan(sampleScan);
    useOreStore.getState().clear();
    const s = useOreStore.getState();
    expect(s.ores).toEqual({});
    expect(s.scannerActive).toBe(false);
  });

  it('setConnected toggles the connection flag', () => {
    useOreStore.getState().setConnected(true);
    expect(useOreStore.getState().connected).toBe(true);
    useOreStore.getState().setConnected(false);
    expect(useOreStore.getState().connected).toBe(false);
  });
});
