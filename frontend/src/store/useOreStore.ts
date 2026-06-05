import { create } from 'zustand';

// IPC contract — mirrors the Rust structs emitted on the "scan-result" Tauri
// event (frontend/src-tauri/src/scan.rs: OreOut / ScanResult). Keep in sync.

export interface OreData {
  name: string;
  quantity: number;
  tier: string;
  tier_value: number;
  volatile: boolean;
  confidence: number;
  detected_rs: number;
  unit_price?: number | null;  // aUEC per SCU (UEX Corp), if known
  alternatives?: string[];     // equally-likely readings of an ambiguous RS, e.g. ["5x Aslarite"]
}

export interface ScanResult {
  ores: Record<string, OreData>;
  scanner_active: boolean;
  configured?: boolean;  // false until a scan region is calibrated
}

interface OreStore {
  ores: Record<string, OreData>;
  scannerActive: boolean;
  configured: boolean;
  connected: boolean;

  setConnected: (connected: boolean) => void;
  updateFromScan: (result: ScanResult) => void;
}

export const useOreStore = create<OreStore>((set) => ({
  ores: {},
  scannerActive: false,
  configured: false,
  connected: false,

  setConnected: (connected) => set({ connected }),

  updateFromScan: (result) =>
    set({
      ores: result.ores,
      scannerActive: result.scanner_active,
      configured: result.configured ?? true,
    }),
}));
