# Architecture

SC Ore Scanner is a two-process desktop app: a **Python backend** that watches the
screen and resolves ore, and a **Tauri + React overlay** that displays it. They
talk over a local WebSocket.

```
┌──────────────────────────── Backend (Python / FastAPI) ────────────────────────────┐
│                                                                                      │
│  ScreenCapture ──► OCREngine ──► RSResolver ──► ConnectionManager ──► WebSocket ─────┼──┐
│   (mss grab,       (preprocess   (RS number      (aggregate, build                   │  │
│    scan gating)     + RapidOCR)   -> ore math)    ScanResult)                        │  │
│                                                                                      │  │
└──────────────────────────────────────────────────────────────────────────────────┘  │
                                                                                         │ ws://127.0.0.1:8765/ws
┌──────────────────────────── Frontend (Tauri v2 / React) ───────────────────────────┐  │
│                                                                                      │  │
│  useWebSocket ──► useOreStore (zustand) ──► Overlay ──► OreCard (name, qty, tier) ◄──┼──┘
│   (reconnect)      (ores, connected,                                                 │
│                     scannerActive)                                                   │
└──────────────────────────────────────────────────────────────────────────────────┘
```

## Backend modules (`backend/src/`)

| Module | Responsibility |
|---|---|
| `config/settings.py` | Pydantic settings (scan region, OCR/signature/server config). Loads/saves `src/config/settings.json`; env override via `SC_SCANNER_*`. |
| `capture/capture.py` | `ScreenCapture` — grabs the configured region with `mss`, and "scan-state gating": only returns a frame when the scanner HUD looks active (bright pixels at sample points). |
| `ocr/ocr_engine.py` | `OCREngine` — preprocesses the region (see [ocr-pipeline.md](ocr-pipeline.md)), runs RapidOCR (ONNX), strips the comma to extract the 3–6 digit number, and debounces (N consecutive frames). |
| `resolver/resolver.py` | `RSResolver` — turns a detected RS number into ore matches via division (`detected = base_rs × quantity`), with fuzzy tolerance and OCR-error correction. |
| `server/app.py` | FastAPI app: the scanning loop, the `/ws` stream, and REST endpoints for config/control. |

## Data flow

1. The scanning loop (`server/app.py:scanning_loop`) runs every `scan_interval` seconds.
2. `ScreenCapture.capture_region()` returns the cropped region (or `None` if gated/unconfigured).
3. `OCREngine.detect_numbers()` preprocesses and OCRs the frame → candidate numbers.
4. `OCREngine.get_confirmed_numbers()` returns numbers seen in N consecutive frames (debounce).
5. `RSResolver.resolve()` maps each confirmed number to ore matches; `aggregate_detections()` keeps the best per ore.
6. The result is broadcast as a `ScanResult` to all WebSocket clients.
7. The overlay's `useWebSocket` hook feeds `useOreStore`, and `Overlay` renders one `OreCard` per ore, sorted by tier then quantity.

## RS resolution

Each ore has a base radar signature (`base_rs`). A scanned node shows
`base_rs × quantity`, so the resolver divides the detected number by every known
signature and accepts whole-number quotients within `[min_quantity, max_quantity]`,
allowing a small error margin. It also attempts OCR-error correction (digit
add/remove, quantity+signature split) for 5–6 digit reads — this is what lets a
comma-mangled `105620` still resolve to `10620 = 3 × Beryl`.

See [`backend/data/signatures.json`](../backend/data/signatures.json) for the 27-ore database.
