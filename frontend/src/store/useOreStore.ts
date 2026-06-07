import { create } from 'zustand';

// IPC contract — mirrors the Rust structs emitted on the "scan-result" Tauri
// event (frontend/src-tauri/src/scan.rs: OreOut / ScanResult). Keep in sync.

// One possible ore for a reading (a signature-degenerate set has several the RS can't
// disambiguate, e.g. all FPS gems = 3000). Mirrors CandidateOut in scan.rs.
export interface Candidate {
  name: string;
  quantity: number;
  tier: string;
  tier_value: number;
  volatile: boolean;
  unit_price?: number | null;   // aUEC per SCU, if known
  probability?: number | null;  // spawn % at the active mining location, if set + known
}

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
  candidates?: Candidate[];    // primary first; length ≥2 = ambiguous (show the group)
  group_label?: string | null; // e.g. "Gem", "ROC deposit" for a degenerate set
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
