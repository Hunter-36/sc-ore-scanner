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
  error?: string | null; // set when the scan loop can't run (e.g. OCR failed to load)
}

// How many consecutive empty scans to keep the last result visible before
// clearing. The scan loop emits every ~0.75s, so 2 ≈ 1.5s of "linger". Pairs
// with the window debouncer: together they stop an ambiguous sig whose OCR read
// jitters (e.g. 14,160) from flickering the card on every dropped frame.
const LINGER_SCANS = 2;

interface OreStore {
  ores: Record<string, OreData>;
  scannerActive: boolean;
  configured: boolean;
  connected: boolean;
  // A fatal scan-loop error (e.g. OCR engine load failure); null when healthy.
  error: string | null;
  // Internal: consecutive empty scans since the last detection (drives linger).
  emptyScans: number;

  setConnected: (connected: boolean) => void;
  updateFromScan: (result: ScanResult) => void;
}

export const useOreStore = create<OreStore>((set) => ({
  ores: {},
  scannerActive: false,
  configured: false,
  connected: false,
  error: null,
  emptyScans: 0,

  setConnected: (connected) => set({ connected }),

  updateFromScan: (result) =>
    set((state) => {
      const flags = {
        scannerActive: result.scanner_active,
        configured: result.configured ?? true,
        error: result.error ?? null,
      };
      const hasOres = Object.keys(result.ores).length > 0;

      // A detection always shows immediately and resets the linger counter.
      if (hasOres) {
        return { ...flags, ores: result.ores, emptyScans: 0 };
      }

      // Empty scan. While actively scanning a calibrated region, hold the last
      // result for a few scans so a transient gap doesn't blank the card. If the
      // scanner isn't active/configured (e.g. no region set), clear at once.
      const lingering =
        flags.scannerActive &&
        flags.configured &&
        Object.keys(state.ores).length > 0 &&
        state.emptyScans + 1 <= LINGER_SCANS;

      return {
        ...flags,
        ores: lingering ? state.ores : {},
        emptyScans: state.emptyScans + 1,
      };
    }),
}));
