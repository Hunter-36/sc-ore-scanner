# CI/CD

Four GitHub Actions workflows live in [`.github/workflows/`](../.github/workflows/).

## CI — `ci.yml`

Checks on every push and pull request.

| Job | Runner | Steps |
|---|---|---|
| `core-tests` | ubuntu | `cargo fmt --check`; `cargo clippy -- -D warnings`; `cargo test --release` on `core` (unit + OCR accuracy e2e; `build.rs` fetches the ocrs models) |
| `frontend-unit` | ubuntu | pnpm install (frozen); `pnpm typecheck`; `pnpm test` (vitest) |
| `tauri-check` | ubuntu | Tauri system libs; `cargo check` on `frontend/src-tauri` (with `rust-cache`) |
| `audit` | ubuntu | Advisory `cargo audit` (core + app `Cargo.lock`) and `pnpm audit --prod`; `continue-on-error` — surfaces advisories without blocking merges |
| `versions` | ubuntu | Fails if `package.json` / `tauri.conf.json` / `Cargo.toml` versions disagree |

## E2E — `e2e.yml`

| Job | Runner | Steps |
|---|---|---|
| `frontend-display-e2e` | ubuntu | pnpm install; `playwright install --with-deps chromium`; `pnpm test:e2e`; uploads `playwright-report` artifact |

(The OCR accuracy e2e is in the Rust `core-tests` job above.)

## Prices — `prices.yml`

Hourly cron (`0 * * * *`), also runnable manually via **Run workflow**
(`workflow_dispatch`). It runs `scripts/fetch_prices.py` (the only Python left), which
pulls UEX Corp commodity data from `https://api.uexcorp.uk/2.0/commodities`, filters it
to the ores we detect, and writes `public/prices.json` (plus a small `public/index.html`
price table). The `public/` dir is published to GitHub Pages; the app reads the feed
from `DEFAULT_FEED_URL` (`https://hunter-36.github.io/sc-ore-scanner/prices.json`, in
`core/src/prices.rs`).

> UEX Corp's public API is hosted at `api.uexcorp.uk`; `uexcorp.space` is the project's
> website (the attribution shown in-app and in the README). Both are UEX Corp.

## Release — `release.yml`

Builds the app + installers and publishes a GitHub Release **automatically on merge to
`master`**.

- **Trigger:** push to `master`/`main` (also a manual `v*` tag, or "Run workflow").
- **`check` job (ubuntu):** computes the tag from `frontend/package.json` (`v<version>`).
  If a release for that tag already exists, it skips — so docs/chore merges that don't
  change the version are a no-op.
- **`build-windows` job (windows, only if releasing):** `pnpm tauri build` (produces the
  exe + NSIS + MSI), then [`softprops/action-gh-release`](https://github.com/softprops/action-gh-release)
  tags the merge commit and publishes the release with three assets: the portable zip,
  the NSIS `-setup.exe`, and the `.msi`. The build **fails rather than publishing a
  partial release** if any of those three artifacts is missing or empty.
- Needs `contents: write` (granted); uses the default `GITHUB_TOKEN`.

**To ship a release:** bump the version (below) in your PR. When it merges, the matching
`vX.Y.Z` release is built and published automatically.

## Versioning & commits

- **[Semantic Versioning](https://semver.org/) tracks the shipped app.** A change that
  doesn't alter the built binary — **docs, CI, repo meta, tests-only** — gets **no
  version bump and no release** (the `check` job no-ops). Code changes bump:
  `fix`/`perf`/security → **patch**; backward-compatible `feat` → **minor**; breaking
  change → **major**.
- **[Conventional Commits](https://www.conventionalcommits.org/):** `type(scope): summary`
  (`feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `ci`, `perf`); `!` /
  `BREAKING CHANGE:` for breaking.
- **Bump the version in all three places** (the CI `versions` job fails if they disagree):
  `frontend/package.json`, `frontend/src-tauri/tauri.conf.json`,
  `frontend/src-tauri/Cargo.toml`.

## Notes & gotchas

- **OCR models:** `core/build.rs` downloads the ocrs `.rten` models at build time and
  embeds them; CI's `core-tests` job therefore needs network. `rust-cache` caches the
  compiled deps (not the models).
- **pnpm version:** `packageManager` in `frontend/package.json` pins pnpm; CI uses
  `pnpm/action-setup`, which respects it.
- **Icons:** `frontend/src-tauri/icons/` is committed (the build requires it).
  Regenerate from a 1024² source with `pnpm tauri icon path/to/source.png`.
- **Local Windows builds** need the MSVC linker — run cargo under `vcvars64`.
