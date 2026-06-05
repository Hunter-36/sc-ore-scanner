import { create } from 'zustand';

export interface OreData {
  name: string;
  quantity: number;
  tier: string;
  tier_value: number;
  volatile: boolean;
  confidence: number;
  detected_rs: number;
  unit_price?: number | null;  // aUEC per SCU (UEX Corp), if available
  value?: number | null;       // unit_price * quantity
}

export interface SessionSummary {
  distinct_ores: number;
  total_detections: number;
}

export interface ScanResult {
  ores: Record<string, OreData>;
  scanner_active: boolean;
  configured?: boolean;  // false until a scan region is calibrated
  timestamp: number;
  session?: SessionSummary;
}

const EMPTY_SESSION: SessionSummary = { distinct_ores: 0, total_detections: 0 };

interface OreStore {
  ores: Record<string, OreData>;
  scannerActive: boolean;
  configured: boolean;
  connected: boolean;
  lastUpdate: number;
  session: SessionSummary;

  setOres: (ores: Record<string, OreData>) => void;
  setScannerActive: (active: boolean) => void;
  setConnected: (connected: boolean) => void;
  updateFromScan: (result: ScanResult) => void;
  clear: () => void;
}

export const useOreStore = create<OreStore>((set) => ({
  ores: {},
  scannerActive: false,
  configured: false,
  connected: false,
  lastUpdate: 0,
  session: EMPTY_SESSION,

  setOres: (ores) => set({ ores }),

  setScannerActive: (active) => set({ scannerActive: active }),

  setConnected: (connected) => set({ connected }),

  updateFromScan: (result) => set({
    ores: result.ores,
    scannerActive: result.scanner_active,
    configured: result.configured ?? true,
    lastUpdate: result.timestamp,
    session: result.session ?? EMPTY_SESSION
  }),

  clear: () => set({ ores: {}, scannerActive: false })
}));
