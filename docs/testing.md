# Testing

Three layers: Rust unit tests, a Rust OCR-accuracy e2e over real captures, and
frontend (vitest + Playwright).

## Core (Rust) — `core/`

```bash
cargo test                              # all of the below
cargo run --example validate --release  # human-readable accuracy check on the fixtures
cargo fmt --check && cargo clippy -- -D warnings
```

- **Unit** — `core/tests/*.rs` integration tests + `#[cfg(test)]` modules:
  - `tests/resolver.rs`: division matching, sorting, confidence scaling, OCR-error
    correction, aggregation, quantity range, signature count.
  - `tests/config.rs`: config load → mutate → save → load round-trip, plus clamping
    and NaN/degenerate-region normalization (the `save_scan_region` path).
  - `tests/prices.rs`: `PriceCache` feed fetch — timeout and response-size bounds, and
    keeping the last-good cache on failure (hermetic, via a local TCP server).
  - `pipeline::tests`: `extract_numbers` (e.g. `"0 7,080 18.8km"` → `["0","7080","18","8"]`).
  - `debounce::tests`: windowed confirm (≥N of last 2N), tolerates intermittent misses,
    a single dropped frame no longer blanks, transient misread never confirms, reset.
  - `signatures::tests`: the embedded `mineables.json` parses and is non-empty.
- **OCR accuracy e2e** — `tests/e2e.rs`: crops each capture in `tests/fixtures/` to the
  scan region, runs the **real embedded OCR + resolver**, and asserts the expected result —
  unambiguous reads (Beryl×3, Beryl×2, none) and ambiguous signature *sets* (14,160 →
  Beryl-or-S-Type; 19,200 → Savrilium-or-Aslarite). ocrs is deterministic, so one run per
  fixture suffices. This is the Rust port of v1's manifest-driven Python e2e.

**Adding an accuracy case:** drop a PNG in `core/tests/fixtures/` and add an assertion
in `tests/e2e.rs`. When you change OCR/resolver/preprocess behaviour, run `cargo test`
and extend the fixtures if accuracy shifts.

> The e2e needs the ocrs models, which `build.rs` downloads at build time — the first
> `cargo test` needs network.

The Tauri shell crate (`frontend/src-tauri`) also has a few `#[cfg(test)]` units —
log-line redaction (`RedactingWriter`), the scan loop's panic guard (`run_guarded`), and
log rotation — run with `cargo test` from `frontend/src-tauri` (under `vcvars64` on Windows).

## Frontend — `frontend/`

```bash
pnpm test          # vitest — store logic (useOreStore)
pnpm test:watch    # vitest in watch mode (local TDD loop)
pnpm typecheck     # tsc --noEmit
pnpm test:e2e      # Playwright — overlay display + calibration UI
```

There's no Tauri runtime in a browser, so the Playwright tests use two dev-only seams:
- **`mock-scan` event** (in `useScanEvents`, `import.meta.env.DEV`-gated, stripped from
  prod): tests dispatch `CustomEvent('mock-scan', { detail: <ScanResult> })` to drive
  the real store + components (ore cards, sorting, prices, badges, the "set region" prompt).
- **`?calibrate` route** (in `App.tsx`): reaches the calibration overlay in a browser to
  test the drag-to-select box.

## CI

`ci.yml` runs the core Rust tests (incl. the OCR e2e), the frontend typecheck + vitest,
a Tauri `cargo check`, an advisory dependency `audit`, and the version-consistency
check. `e2e.yml` runs Playwright. See [ci-cd.md](ci-cd.md).
