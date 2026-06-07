# Architecture

SC Ore Scanner is a **single Rust app** (Tauri v2). A background thread captures the
screen, OCRs the mining scanner's RS number, and resolves it to ore; results are
pushed to the React overlay via a Tauri event. No separate backend, no WebSocket.

```
┌─────────────────────────── one process (Tauri app) ───────────────────────────┐
│                                                                                 │
│  scan thread (frontend/src-tauri/src/scan.rs)                                   │
│    xcap capture ─► preprocess ─► ocrs OCR ─► extract numbers ─► debounce ─►     │
│      (primary      (crop/upscale  (lines)     (per-token,        (3 frames)     │
│       monitor)      /CLAHE)                     RS range)                        │
│                                                   │                             │
│                                       resolver ─► aggregate ─► emit("scan-result")
│                                       (RS -> ore)                  │             │
│                                                                    ▼            │
│  React overlay (frontend/src)                                                   │
│    useScanEvents ─► useOreStore (zustand) ─► Overlay ─► OreCard (name, qty,     │
│      (Tauri event)   (ores, connected,                   tier, aUEC/SCU)        │
│                       configured)                                               │
└────────────────────────────────────────────────────────────────────────────────┘
```

## The `core` crate (`core/src/`) — detection, no UI

| Module | Responsibility |
|---|---|
| `config.rs` | `Config` (scan region, interval, debounce frames, CLAHE) loaded/saved as JSON in the app config dir. |
| `preprocess.rs` | Crop to the scan region, upscale ×4 (Lanczos), optional grayscale + CLAHE. |
| `ocr.rs` | `Ocr` — the `ocrs` engine; detection/recognition `.rten` models embedded at build time (`build.rs`). |
| `pipeline.rs` | `recognize_rs_numbers` / `recognize_rs_numbers_from_processed` (OCR → split each line into number tokens via the internal `extract_numbers` helper → keep 3–6 digit numbers in RS range), `resolve_and_aggregate`, and the one-shot `detect_ores` (preprocess + recognize + resolve) used by the `validate` example and the e2e test. |
| `debounce.rs` | `Debouncer` — confirm a number only after it appears in N consecutive frames. |
| `resolver.rs` | `Resolver` — RS number → ore matches via division (`detected = base_rs × quantity`), with fuzzy tolerance and OCR-error correction. |
| `signatures.rs` | `OreSignature` (+ per-body `Location`); loads the embedded `core/data/mineables.json` — generated from the curated `signatures.json` + Wiki API by `scripts/fetch_mineables.py`. |
| `mineables.rs` | `mineables::load` — sources the dataset at startup: live Pages feed → on-disk cache → embedded copy (mirrors `prices.rs`). |
| `prices.rs` | `PriceCache` — fetches the UEX price feed (hourly). |

## The Tauri shell (`frontend/src-tauri/src/`)

- `scan.rs` — the background scan loop: capture → preprocess →
  `recognize_rs_numbers_from_processed` → debounce → `resolve_and_aggregate` →
  `emit("scan-result")`. At startup it loads the mineables dataset (feed → cache →
  embedded) to build the resolver, and fetches prices.
- `main.rs` — windows, first-run overlay placement, the calibration command
  (`open_calibration`, **async** — see gotchas), `save_scan_region`, `get_config`,
  `set_config`, `quit`, logging.

## IPC reference

The React overlay talks to the Rust shell over Tauri **commands** and one **event**.

| Command (`#[tauri::command]`) | Args | Purpose |
|---|---|---|
| `open_calibration` | — | Open the full-screen calibration overlay (**async** — see gotchas). |
| `save_scan_region` | `x, y, w, h` | Persist the calibrated region to `config.json` (rejects regions < 8 px). |
| `get_config` | — | Return the current `Config` for the settings UI. |
| `set_config` | `update` | Update the tunable subset (clamped, below) + the mining location; the scan loop hot-reloads it live. |
| `get_mining_locations` | — | Distinct mining bodies `{system, body}` (from the dataset) for the settings location picker. |
| `quit` | — | Exit the app. |

`set_config` clamps each tunable to the same ranges `Config::load` enforces (so a
hand-edited `config.json` is sanitized too): `scan_interval_secs` **0.3–5.0**,
`min_consecutive_frames` **1–6**, `upscale` **1–6**, `clahe_clip_limit` **0.0–8.0**;
`mining_location` is a free-form body name used to rank/filter ambiguous candidates (not clamped).

**Event `scan-result`** — emitted every cycle, payload `ScanResult`:

- `ores`: map of ore name → `OreOut` `{ name, quantity, tier, tier_value, volatile,
  confidence, detected_rs, unit_price?, alternatives[], candidates[], group_label? }`.
  For a signature-degenerate reading (≥2 `candidates`, e.g. FPS gems all = 3000), each
  candidate carries `{ name, quantity, tier, unit_price?, probability? }`; the overlay
  shows the set ranked by per-location spawn `probability` when a `mining_location` is set.
- `scanner_active`: whether the OCR loop is running
- `configured`: `false` until a scan region is calibrated (overlay prompts "Set region")

`OreOut`/`ScanResult` (`scan.rs`) mirror `OreData`/`ScanResult` in
`src/store/useOreStore.ts` — keep the two in sync.

## Data flow

1. The scan thread loops every `scan_interval_secs` (default 0.75s).
2. `resolve_primary_monitor()` picks the primary display (resolved once and cached
   across cycles, re-resolved on a capture failure); `capture_region()` grabs the
   calibrated region.
3. The frame is cropped to `scan_region` and preprocessed.
4. `ocrs` OCRs the crop; `recognize_rs_numbers_from_processed` splits each line into
   number tokens (via the internal `extract_numbers` helper, so the RS value isn't
   merged with the distance marker) and keeps 3–6 digit numbers in `valid_rs_min..max`.
5. The `Debouncer` confirms numbers seen in `min_consecutive_frames` consecutive frames.
6. `resolve_and_aggregate` maps each confirmed number to its best ore and keeps the
   best per ore.
7. A `ScanResult` (ores + `scanner_active` + `configured`) is emitted as `scan-result`.
8. `useScanEvents` feeds `useOreStore`; `Overlay` renders one `OreCard` per ore,
   sorted by tier then quantity, with the UEX price per SCU.

## RS resolution

Each ore has a base radar signature (`base_rs`). A scanned node shows
`base_rs × quantity`, so the resolver divides the detected number by every known
signature and accepts whole-number quotients within `[min_quantity, max_quantity]`,
allowing a small error margin. It also attempts OCR-error correction (digit
add/remove, quantity+signature split) for longer reads — this is what lets a
comma-mangled read still resolve correctly.

See [`core/data/signatures.json`](../core/data/signatures.json) for the signature database.

## Why v2 dropped Python

v1 was a Python/FastAPI backend (mss + RapidOCR) talking to the overlay over a
WebSocket. v2 folds detection into the Rust app (`xcap` + `ocrs`): one process, one
binary, no install, no IPC. The detection pipeline is a faithful port — see
[ocr-pipeline.md](ocr-pipeline.md).
