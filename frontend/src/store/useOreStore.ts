import { create } from 'zustand';

export interface OreData {
  name: string;
  quantity: number;
  tier: string;
  tier_value: number;
  volatile: boolean;
  confidence: number;
  detected_rs: number;
}

export interface ScanResult {
  ores: Record<string, OreData>;
  scanner_active: boolean;
  timestamp: number;
}

interface OreStore {
  ores: Record<string, OreData>;
  scannerActive: boolean;
  connected: boolean;
  lastUpdate: number;

  setOres: (ores: Record<string, OreData>) => void;
  setScannerActive: (active: boolean) => void;
  setConnected: (connected: boolean) => void;
  updateFromScan: (result: ScanResult) => void;
  clear: () => void;
}

export const useOreStore = create<OreStore>((set) => ({
  ores: {},
  scannerActive: false,
  connected: false,
  lastUpdate: 0,

  setOres: (ores) => set({ ores }),

  setScannerActive: (active) => set({ scannerActive: active }),

  setConnected: (connected) => set({ connected }),

  updateFromScan: (result) => set({
    ores: result.ores,
    scannerActive: result.scanner_active,
    lastUpdate: result.timestamp
  }),

  clear: () => set({ ores: {}, scannerActive: false })
}));
