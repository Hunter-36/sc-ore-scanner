# Contributing to SC Ore Scanner

Contributions are **openly welcome** — bug reports, feedback, feature ideas, and
pull requests are all appreciated. This is a community tool and it gets better
when miners pitch in. o7

If you're unsure about something, open an issue first and ask — no question is too
small.

## Ways to help

- 🐛 **Bug reports** — open an issue with what happened, what you expected, your
  Windows version, and (super helpful) a **screenshot of your mining HUD**.
- 💡 **Feature ideas / feedback** — open an issue describing the use case.
- 🖼️ **OCR captures** — screenshots at different resolutions/HUD scales make the
  detection more robust (see [Adding test captures](#adding-test-captures)).
- 🔧 **Code** — fixes and features via pull request (see below).

## Dev setup

This project uses **[uv](https://github.com/astral-sh/uv)** for Python and
**[pnpm](https://pnpm.io/)** for the frontend.

```bash
# Backend
cd backend
uv venv
uv pip install -r requirements-dev.txt    # core + test tooling
uv pip install -r requirements-ml.txt     # OCR engine (for e2e)

# Frontend
cd ../frontend
pnpm install
```

See [`CLAUDE.md`](CLAUDE.md) and [`docs/`](docs/) (architecture, OCR pipeline,
testing, CI/CD) for how everything fits together.

## Before you open a PR

All of these must pass — CI enforces them:

**Backend** (from `backend/`):
```bash
ruff check .
pytest tests/unit          # fast, no OCR deps
pytest tests/e2e           # OCR pipeline (needs requirements-ml.txt)
```

**Frontend** (from `frontend/`):
```bash
pnpm typecheck
pnpm test                  # vitest
pnpm test:e2e              # Playwright (pnpm exec playwright install chromium first)
```

## Standards

- **Python:** type hints + docstrings, match the existing module style. Keep `ruff`
  clean. Pydantic models for data shapes.
- **Frontend:** functional React components, TypeScript, keep `tsc` happy.
- **Keep the dependency split** (`requirements-core` / `-ml` / `-dev`) — it's what
  keeps unit-test CI fast.
- **Don't reintroduce heavyweight ML deps.** OCR is intentionally RapidOCR (ONNX,
  no PyTorch) so the app stays ~150 MB to install. The OCR preprocessing is
  contrast-based on purpose — see [`docs/ocr-pipeline.md`](docs/ocr-pipeline.md)
  before changing it.
- **Commits:** clear, imperative messages (a `type: summary` prefix like
  `fix:` / `feat:` / `docs:` is nice but not required).

## Adding ore signatures

Ore data lives in [`backend/data/signatures.json`](backend/data/signatures.json).
Add an entry with the ore's `base_rs` (single-node radar signature) and tier info,
then add an assertion to `backend/tests/unit/test_resolver.py` so it stays correct.

## Adding test captures

The end-to-end suite is driven by
[`backend/tests/e2e/manifest.json`](backend/tests/e2e/manifest.json). To add a
case:

1. Drop a screenshot (or short video) into `backend/tests/test_images/`.
2. Add a manifest entry with its `scan_region` and `expected_top` ore.
3. Run `pytest tests/e2e` — each capture is run **10×** and must produce the
   expected ore as the top match in **≥90%** of runs.

Captures at resolutions/HUD scales other than the ones already covered are
especially valuable, since detection has only been validated at one resolution.

## Pull request flow

1. Fork and branch off `master`.
2. Make your change; add/adjust tests.
3. Make sure the checks above pass.
4. Open the PR with a short description of what and why. Screenshots/GIFs welcome
   for overlay changes.

Thanks for contributing! 🚀
