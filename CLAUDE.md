# CLAUDE.md

Guidance for working in this repository with Claude Code.

## What this is

SC Ore Scanner: a real-time Star Citizen mining overlay. A **Python/FastAPI
backend** captures the screen, OCRs the mining scanner's RS (Radar Signature)
number, and resolves it to an ore type + quantity; a **Tauri v2 + React overlay**
displays it. They communicate over a local WebSocket (`ws://127.0.0.1:8765/ws`).

Read [`docs/architecture.md`](docs/architecture.md) for the full picture.

## Toolchain

- **Python: use `uv`** (not pip/venv directly). `uv venv`, `uv pip install -r ...`, `uv run`.
- **Node: use `pnpm`** (not npm). The repo is migrated to pnpm with a committed `pnpm-lock.yaml`.
- Python 3.11+. Node 18+. Rust stable for Tauri.

## Common commands

Backend (from `backend/`):
```bash
uv pip install -r requirements-dev.txt   # unit-test tooling (no ML)
uv pip install -r requirements.txt       # full app (adds easyocr/torch)
pytest tests/unit                         # fast unit tests
pytest tests/e2e                          # OCR pipeline e2e (needs ML stack)
ruff check .
python main.py                            # run backend (needs a calibrated scan region)
python calibrate.py                       # select the scan region
```

Frontend (from `frontend/`):
```bash
pnpm install
pnpm tauri dev      # run the overlay (expects backend on :8765)
pnpm test           # vitest unit tests
pnpm typecheck
pnpm test:e2e       # Playwright overlay display tests
pnpm build          # tsc + vite build (what the release pipeline runs)
```

Launch both at once (Windows): `launch.bat`.

## Dependency layout

`requirements-core.txt` (app, no ML) · `requirements-ml.txt` (easyocr/torch) ·
`requirements-dev.txt` (core + pytest/ruff/httpx) · `requirements.txt` (core + ml).
Keep the split intact — it's what makes unit-test CI fast. Pytest + ruff config
live in `backend/pyproject.toml`.

## Testing model

- **Backend unit** (`tests/unit/`): resolver, config, OCR preprocessing/debounce, FastAPI.
- **Backend e2e** (`tests/e2e/`): manifest-driven — crops real captures to a scan
  region, runs the real OCR+resolver **10×**, requires the expected ore as top match
  in **≥90%** of runs. Add cases via `tests/e2e/manifest.json` + a file in
  `tests/test_images/`.
- **Frontend**: vitest for the store; Playwright for the overlay display (mock WS).

See [`docs/testing.md`](docs/testing.md).

## Gotchas

- **mss needs a display.** `tests/unit/test_server.py` opens the FastAPI lifespan
  which constructs `mss`. On headless Linux run under `xvfb-run`; on Windows/macOS
  it just works. CI handles this.
- **OCR preprocessing is contrast-based, not threshold-based.** The aggressive
  adaptive-threshold approach destroyed digit strokes and broke detection on real
  captures. Don't reintroduce it. See [`docs/ocr-pipeline.md`](docs/ocr-pipeline.md).
- **cv2 LANCZOS4 (not PIL) for upscaling** — PIL flipped a borderline `6`→`8`.
- **Tauri icons are committed** under `frontend/src-tauri/icons/` (the build needs
  them). Regenerate with `pnpm tauri icon <1024² source>.png`.
- **Settings:** runtime config is `backend/src/config/settings.json` (scan region,
  thresholds). Env overrides via `SC_SCANNER_*`. Tests use temp files — never write
  the real settings.json from a test.

## Conventions

- Match the surrounding style: type hints + docstrings in the backend, functional
  React components, Pydantic models for data shapes.
- When changing detection (OCR/resolver) behavior, validate with `pytest tests/e2e`
  and, if it changes accuracy, update/extend the e2e manifest fixtures.
- Don't commit/push unless asked. Branch off `master` for changes when appropriate.
