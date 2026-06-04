# Testing Guide

The project has four test layers: backend unit, backend OCR end-to-end, frontend
unit (vitest), and frontend display end-to-end (Playwright).

## Backend

Run from `backend/`. The dependency split keeps unit tests fast (no OCR engine):

```bash
# Unit tests — pure logic + FastAPI, no ML stack
uv pip install -r requirements-dev.txt
pytest tests/unit

# End-to-end OCR — needs the ML stack + the capture fixtures
uv pip install -r requirements-ml.txt
pytest tests/e2e
```

Lint:
```bash
ruff check .
```

### Unit suite (`tests/unit/`)

| File | Covers |
|---|---|
| `test_resolver.py` | RS→ore division math, OCR-error correction, aggregation, against the real `signatures.json` |
| `test_config.py` | Settings defaults, `ScanRegion` validation, save/load roundtrip, env override |
| `test_ocr.py` | `preprocess_image` output shape, and the debouncing state machine (no OCR engine load) |
| `test_server.py` | FastAPI endpoints via `TestClient` (`/health`, `/signatures`, `/config`, scan control) |

> The server tests open the FastAPI lifespan, which constructs `mss`. On headless
> Linux this needs a virtual display — CI runs them under `xvfb-run`. Locally on
> Windows/macOS it just works.

### End-to-end OCR suite (`tests/e2e/`)

`test_pipeline.py` is **manifest-driven** by `tests/e2e/manifest.json`. For each
fixture it:

1. crops the image to its `scan_region` (mimicking in-game calibration),
2. runs the **real** `OCREngine` + `RSResolver`,
3. repeats `runs` (10) times,
4. asserts the expected ore is the **top** match in **≥ `min_pass_rate` (90%)** of runs.

Negative cases (`expected_top: null`) assert nothing is detected.

**Adding a fixture:** drop an image (or video) into `backend/tests/test_images/`
and add an entry to `manifest.json`:

```json
{
  "file": "my_capture.png",
  "scan_region": [193, 122, 109, 48],
  "expected_top": { "name": "Beryl", "quantity": 3 },
  "note": "what this capture shows"
}
```

`scan_region` is `[x, y, width, height]`; omit it to use `default_scan_region`.
Videos (`.mp4/.mov/...`) are sampled every `stride` frames.

**Manual run** on any file (great for debugging a new capture):
```bash
python -m tests.e2e.pipeline tests/test_images/sc_mining_scan_rs_10620_some_particles.png
python -m tests.e2e.pipeline path/to/clip.mp4 --stride 15
```

If the ML stack isn't installed, the whole e2e package is skipped (so the
unit-only environment stays green).

## Frontend

Run from `frontend/`:

```bash
pnpm test          # vitest — useOreStore state logic
pnpm typecheck     # tsc --noEmit
pnpm test:e2e      # Playwright overlay display tests
```

### Playwright display tests (`tests/e2e/overlay.spec.ts`)

These boot the real Vite dev server and a **mock backend WebSocket** on port 8765,
then assert the overlay renders correctly:

- connected + scanning → status `SCANNING`, ore cards show `Beryl` / `3x` / tier `A`,
  and the highest-tier ore (Quantainium, S) sorts first;
- backend down → status `OFFLINE` with the "Connecting to backend" message.

The browser is installed with `pnpm exec playwright install chromium` (CI uses
`--with-deps`).
