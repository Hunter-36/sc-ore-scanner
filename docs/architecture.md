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
| `pipeline.rs` | `recognize_rs_numbers` (OCR → split each line into number tokens → keep 3–6 digit numbers in RS range) and `resolve_and_aggregate`. |
| `debounce.rs` | `Debouncer` — confirm a number only after it appears in N consecutive frames. |
| `resolver.rs` | `Resolver` — RS number → ore matches via division (`detected = base_rs × quantity`), with fuzzy tolerance and OCR-error correction. |
| `signatures.rs` | Loads the embedded `core/data/signatures.json`. |
| `prices.rs` | `PriceCache` — fetches the UEX price feed (hourly). |

## The Tauri shell (`frontend/src-tauri/src/`)

- `scan.rs` — the background scan loop: capture → preprocess → `recognize_rs_numbers`
  → debounce → `resolve_and_aggregate` → `emit("scan-result")`. Also fetches prices.
- `main.rs` — windows, first-run overlay placement, the calibration command
  (`open_calibration`, **async** — see gotchas), `save_scan_region`, `get_config`,
  `set_config`, `quit`, logging.

## Data flow

1. The scan thread loops every `scan_interval_secs` (default 0.75s).
2. `capture_primary()` grabs the primary monitor (the one calibration targets).
3. The frame is cropped to `scan_region` and preprocessed.
4. `ocrs` OCRs the crop; `pipeline::extract_numbers` splits each line into number
   tokens (so the RS value isn't merged with the distance marker), filtered to 3–6
   digit numbers in `valid_rs_min..max`.
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
