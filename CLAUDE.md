# CLAUDE.md

Guidance for working in this repository with Claude Code.

## What this is

SC Ore Scanner: a real-time Star Citizen mining overlay, shipped as a **single
self-contained Rust app** (Tauri v2). It captures the screen in-process, OCRs the
mining scanner's **RS** (Radar Signature) number with the pure-Rust `ocrs` engine,
resolves it to an ore type + quantity, and shows it in a React overlay. The scan
loop runs on a background thread and pushes results to the UI via a **Tauri event**
(`scan-result`) — there is no Python backend and no WebSocket.

Read [`docs/architecture.md`](docs/architecture.md) for the full picture.

> v1 was a Python/FastAPI backend + Tauri frontend over a WebSocket. v2.0.0 was an
> all-Rust rewrite (single binary). If you find references to Python/WebSocket
> anywhere, they're stale — fix them.

## Layout

- **`core/`** — `scanner-core`, the detection library (no UI). Modules: `config`,
  `preprocess` (crop/upscale/CLAHE), `ocr` (ocrs, models embedded via `build.rs`),
  `pipeline` (OCR → number extraction → resolve/aggregate), `debounce`, `resolver`
  (RS → ore, division match + OCR-error correction), `signatures` (ore data +
  per-location spawn data; embeds the generated `core/data/mineables.json`),
  `mineables` (fetches that dataset from the Pages feed at startup; cache + embedded
  fallback), `prices` (UEX feed). Tests in `core/tests/`, fixtures in `core/tests/fixtures/`.
- **`frontend/`** — Tauri v2 app. `src/` is the React overlay; `src-tauri/src/`
  is the Rust shell: `scan.rs` (capture → detect → emit loop) and `main.rs`
  (windows, calibration, quit, logging). Depends on `scanner-core` by path.
- **Ore data:** `core/data/signatures.json` is the **hand-curated source** (base_rs,
  tier, volatile per ore). `scripts/fetch_mineables.py` enriches it with per-location
  spawn data from the Star Citizen Wiki API (via the `api_slug` map in
  `mineables-curation.json`) → `core/data/mineables.json` (the embedded/fetched dataset).
  Edit signatures.json then regenerate: `uv run scripts/fetch_mineables.py`.
- **`scripts/`** (Python, run with `uv`): `fetch_prices.py` (UEX prices) and
  `fetch_mineables.py` (Wiki-API ore dataset) — CI data jobs (see `feeds.yml`) that
  publish to GitHub Pages. Not part of the app binary.

## Toolchain

- **Rust stable** for everything. On **this Windows machine, run cargo under
  `vcvars64`** (the MSVC linker isn't otherwise on PATH) — see the vcvars memory.
- **Node: use `pnpm`** (not npm). Committed `pnpm-lock.yaml`.
- **Python: use `uv`** — only needed for `scripts/fetch_prices.py`. **Never invoke
  bare `python`/`python3`/`py`** anywhere — not even a throwaway `python --version`
  or a quick `python3 -c` calc. There's no Python on PATH on this Windows machine,
  so bare invocations hit the App Execution Alias and pop up the Microsoft Store
  "Python install manager". Always go through uv: `uv run python -c "..."`,
  `uv run python scripts/fetch_prices.py`. (Applies to subagents/workflows too.)

## Common commands

Frontend / app (from `frontend/`):
```bash
pnpm install
pnpm tauri dev          # run the overlay app (Rust + React)
pnpm tauri build        # release exe in src-tauri/target/release (installers under bundle/{nsis,msi})
pnpm typecheck
pnpm test               # vitest (store)
pnpm test:e2e           # Playwright overlay display tests
```

Core detection (from `core/`):
```bash
cargo test                              # resolver/debounce/extraction unit + OCR accuracy e2e
cargo run --example validate --release  # accuracy check over the capture fixtures
cargo fmt --check && cargo clippy -- -D warnings
```

(Local Windows builds: prefix with the vcvars64 shell — see the memory.)

## Detection pipeline (faithful to v1)

capture primary monitor → crop to `scan_region` → upscale ×4 (Lanczos) →
*(CLAHE contrast, opt-in)* → `ocrs` OCR → extract each number token from each line
→ keep 3–6 digit numbers in `valid_rs_min..max` → **debounce: confirm a number seen in
≥`min_consecutive_frames` of the last `2×` frames (default 3 of 6 — a window, not a
strict run)** → resolver (division match + OCR-error correction)
→ aggregate best-per-ore → emit `scan-result`.

## Testing model

- **Core unit** (`core/src/**` `#[cfg(test)]` + `core/tests/resolver.rs`): resolver,
  debounce, number extraction.
- **Core e2e** (`core/tests/e2e.rs`): crops real captures in `core/tests/fixtures/`
  to the scan region, runs the **real embedded OCR + resolver**, asserts the
  expected top ore. Add cases by dropping a PNG in `fixtures/` + an assertion.
- **Frontend**: vitest for the store; Playwright for the overlay (drives the real
  store via a dev-only `mock-scan` event, and the calibration UI via `?calibrate`).

## Gotchas

- **Window creation must be async.** A synchronous Tauri command that builds a
  window deadlocks the main thread (window appears but nothing else works). Any
  command that opens a window must be `async fn`. See the matching memory.
- **ocrs exposes no per-line confidence** (only chars + rects), so v1's confidence
  gate can't be ported; **debouncing** covers it.
- **ocrs returns whole lines**, merging the RS value with the distance marker
  (`"0 7,080 18.8km"`). An internal `extract_numbers` helper splits each line into
  number tokens so the RS value is isolated. Don't revert to whole-line digit-stripping.
- **CLAHE is off by default.** It's implemented (matches cv2) but *regressed* ocrs
  detection at v1's clip=2.0, so `clahe_clip_limit` defaults to 0 (off). ocrs reads
  the raw upscaled text well; CLAHE is tunable for dark frames.
- **OCR preprocessing is contrast-based, not threshold-based** — aggressive
  thresholding destroyed digit strokes in v1. Don't reintroduce it.
- **Models are embedded**: `core/build.rs` downloads the ocrs `.rten` models at
  build time into `OUT_DIR` and `include_bytes!`s them. First build needs network;
  models never enter git.
- **Tauri icons are committed** under `frontend/src-tauri/icons/`.
- **Runtime data** lives in `%APPDATA%\com.scorescanner.app\`: `config.json`
  (scan region + tuning), `mineables.json` (cached dataset feed), `.window-state.json`,
  `logs/scanner.log` (+ `scanner.log.1` once it rolls past 5 MB). `SC_ORE_LOG`
  (trace/debug/info/warn/error/off, default info) sets the log level.

## Conventions

- Match surrounding style: functional React components; small, documented Rust
  modules; `serde` structs for data shapes.
- When changing detection (OCR/resolver/preprocess) behaviour, validate with
  `cargo test` (the e2e accuracy test) and extend the fixtures if accuracy changes.

## Workflow: issues → PR → auto-release

- **Work from a GitHub issue, on a branch, via a PR into `master`** — not straight
  commits to master.
- **Merging to `master` auto-tags and publishes a release** (`release.yml`) when the
  version in `frontend/package.json` has no release yet; a no-op if unchanged. So
  the version bump must land **in the PR**.
- **Conventional Commits** — `type(scope): summary` (`feat`, `fix`, `docs`,
  `refactor`, `test`, `chore`, `ci`, `perf`; `!`/`BREAKING CHANGE:` for breaking).
- **SemVer tracks the shipped binary.** A change that doesn't alter the built app —
  **docs, CI, repo meta, tests-only** — gets **no version bump and no release** (it
  just merges; `release.yml` no-ops). Code changes bump: `fix`/`perf`/security →
  patch; `feat` → minor; breaking → major. CI's `versions` job enforces consistency
  when you do bump.
- **Bump the version in all 3 places** (they must agree): `frontend/package.json`,
  `frontend/src-tauri/tauri.conf.json`, `frontend/src-tauri/Cargo.toml`.
- Tests must pass before merge (`cargo test`, `pnpm typecheck && pnpm test`, Playwright).
  See [`docs/ci-cd.md`](docs/ci-cd.md).
