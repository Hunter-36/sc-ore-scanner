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

All Rust, with **[pnpm](https://pnpm.io/)** for the frontend.

```bash
cd frontend
pnpm install
pnpm tauri dev        # run the app (Rust + React)
```

The detection logic is the `core/` crate (no UI) and can be worked on alone:

```bash
cd core
cargo test            # unit + OCR accuracy e2e (build.rs fetches the OCR models)
```

On **Windows, run cargo under `vcvars64`** so the MSVC linker is on PATH.

See [`CLAUDE.md`](CLAUDE.md) and [`docs/`](docs/) (architecture, OCR pipeline,
testing, CI/CD) for how everything fits together.

## Before you open a PR

All of these must pass — CI enforces them:

**Core** (from `core/`):
```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test                 # unit + OCR accuracy e2e
```

**Frontend** (from `frontend/`):
```bash
pnpm typecheck
pnpm test                  # vitest
pnpm test:e2e              # Playwright (pnpm exec playwright install chromium first)
```

## Standards

- **Rust:** small, documented modules; keep `cargo fmt` + `clippy` clean; `serde`
  structs for data shapes.
- **Frontend:** functional React components, TypeScript, keep `tsc` happy.
- **Don't reintroduce Python or heavyweight ML deps.** OCR is intentionally the
  pure-Rust `ocrs` engine (models embedded). The OCR preprocessing is contrast-based,
  and per-number extraction + debouncing are deliberate — see
  [`docs/ocr-pipeline.md`](docs/ocr-pipeline.md) before changing it.
- **Window-creating Tauri commands must be `async`** (a sync one deadlocks the main
  thread).
- **Commits:** use [Conventional Commits](https://www.conventionalcommits.org/) —
  `type(scope): summary`. Common types: `feat`, `fix`, `docs`, `refactor`, `test`,
  `chore`, `ci`, `perf`. Mark breaking changes with `!` (e.g. `feat!:`) or a
  `BREAKING CHANGE:` footer.

## Adding ore signatures

Ore data lives in [`core/data/signatures.json`](core/data/signatures.json).
Add an entry with the ore's `base_rs` (single-node radar signature) and tier info,
then add an assertion in `core/tests/resolver.rs` so it stays correct.

## Adding test captures

The OCR accuracy e2e is [`core/tests/e2e.rs`](core/tests/e2e.rs). To add a case:

1. Drop a screenshot into `core/tests/fixtures/`.
2. Add an assertion in `tests/e2e.rs` (the scan region + expected top ore).
3. Run `cargo test` — it runs the real OCR + resolver over the fixture.

Captures at resolutions/HUD scales other than the ones already covered are
especially valuable, since detection has only been validated at one resolution.

## Versioning & releases

This project uses [Semantic Versioning](https://semver.org/), and **merging to
`master` automatically tags and publishes a release** when the version changes
(it's a no-op if unchanged). So the version bump happens *in the PR*:

- **patch** (`x.y.Z`) — bug fixes, security, docs, refactors (`fix`/`docs`/`chore`/`refactor`)
- **minor** (`x.Y.0`) — new backward-compatible features (`feat`)
- **major** (`X.0.0`) — breaking changes (e.g. a settings format change that breaks
  existing calibration)

When your change should ship, bump the version to the **same value in all three
places** (the CI `versions` job fails if they disagree):

1. `frontend/package.json`
2. `frontend/src-tauri/tauri.conf.json`
3. `frontend/src-tauri/Cargo.toml`

Docs-only / chore PRs can leave the version unchanged — no release is cut.
See [`docs/ci-cd.md`](docs/ci-cd.md) for the release pipeline details.

## Pull request flow

1. Branch off `master` (work from an open issue where possible).
2. Make your change; add/adjust tests.
3. Bump the version (above) if it should release.
4. Make sure the checks pass.
5. Open the PR — the template includes a version-bump checklist. Screenshots/GIFs
   welcome for overlay changes.

Thanks for contributing! 🚀
