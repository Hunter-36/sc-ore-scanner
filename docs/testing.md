# Testing

Three layers: Rust unit tests, a Rust OCR-accuracy e2e over real captures, and
frontend (vitest + Playwright).

## Core (Rust) — `core/`

```bash
cargo test                              # all of the below
cargo run --example validate --release  # human-readable accuracy check on the fixtures
cargo fmt --check && cargo clippy -- -D warnings
```

- **Unit** — `tests/resolver.rs` + `#[cfg(test)]` modules:
  - `tests/resolver.rs`: division matching, sorting, confidence scaling, OCR-error
    correction, aggregation, quantity range, signature count.
  - `pipeline::tests`: `extract_numbers` (e.g. `"0 7,080 18.8km"` → `["0","7080","18","8"]`).
  - `debounce::tests`: confirm-after-N, streak reset on a gap, transient misread never confirms.
- **OCR accuracy e2e** — `tests/e2e.rs`: crops each capture in `tests/fixtures/` to the
  scan region, runs the **real embedded OCR + resolver**, and asserts the expected top
  ore (Beryl×3, Beryl×2, none). ocrs is deterministic, so one run per fixture suffices.
  This is the Rust port of v1's manifest-driven Python e2e.

**Adding an accuracy case:** drop a PNG in `core/tests/fixtures/` and add an assertion
in `tests/e2e.rs`. When you change OCR/resolver/preprocess behaviour, run `cargo test`
and extend the fixtures if accuracy shifts.

> The e2e needs the ocrs models, which `build.rs` downloads at build time — the first
> `cargo test` needs network.

## Frontend — `frontend/`

```bash
pnpm test          # vitest — store logic (useOreStore)
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
a Tauri `cargo check`, and the version-consistency check. `e2e.yml` runs Playwright.
See [ci-cd.md](ci-cd.md).
