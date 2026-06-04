# CI/CD

Three GitHub Actions workflows live in [`.github/workflows/`](../.github/workflows/).

## CI — `ci.yml`

Fast checks on every push and pull request.

| Job | Runner | Steps |
|---|---|---|
| `backend-unit` | ubuntu | apt `libgl1`/`libglib2.0-0`/`xvfb`; `uv pip install -r requirements-dev.txt`; `ruff check`; `xvfb-run pytest tests/unit` |
| `frontend-unit` | ubuntu | pnpm install (frozen); `pnpm typecheck`; `pnpm test` (vitest) |
| `rust-check` | ubuntu | Tauri system libs; `cargo check` on `frontend/src-tauri` (with `rust-cache`) |
| `versions` | ubuntu | Fails if `package.json` / `tauri.conf.json` / `Cargo.toml` versions disagree |

No ML stack is installed here, so it stays quick.

## E2E — `e2e.yml`

Heavier validation on push / PR (and `workflow_dispatch`).

| Job | Runner | Steps |
|---|---|---|
| `backend-ocr-e2e` | ubuntu | full deps incl. **RapidOCR** (ONNX, no PyTorch); `xvfb-run pytest tests/e2e` |
| `frontend-display-e2e` | ubuntu | pnpm install; `playwright install --with-deps chromium`; `pnpm test:e2e`; uploads `playwright-report` artifact |

## Release — `release.yml`

Builds the portable Windows zip and publishes a GitHub Release **automatically on
merge to `master`**.

- **Trigger:** push to `master`/`main` (also a manual `v*` tag, or "Run workflow").
- **`check` job (ubuntu):** computes the tag from `frontend/package.json`'s version
  (`v<version>`). If a release for that tag already **exists**, it sets
  `should_release=false` and the build is skipped — so docs/chore merges that don't
  change the version are a no-op.
- **`build-windows` job (windows, only if releasing):** `pnpm tauri build --no-bundle`,
  assemble `sc-ore-scanner-<tag>-windows.zip` (overlay exe + backend source + scripts),
  then [`softprops/action-gh-release`](https://github.com/softprops/action-gh-release)
  creates the tag at the merge commit and publishes the release (latest) with the zip.
- Needs `contents: write` (granted in the workflow); uses the default `GITHUB_TOKEN`.

**To ship a release:** bump the version (see below) in your PR. When it merges, the
matching `vX.Y.Z` release is built and published automatically.

## Versioning & commits

- **[Semantic Versioning](https://semver.org/):** `fix`/`docs`/`chore`/`refactor`/security
  → **patch**; backward-compatible `feat` → **minor**; breaking change → **major**.
- **[Conventional Commits](https://www.conventionalcommits.org/):** `type(scope): summary`
  (`feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `ci`, `perf`); `!` /
  `BREAKING CHANGE:` for breaking.
- **Bump the version in all five places** (the CI `versions` job in `ci.yml` fails if
  the first three disagree): `frontend/package.json`, `frontend/src-tauri/tauri.conf.json`,
  `frontend/src-tauri/Cargo.toml`, `backend/main.py` (`Version:` log), and
  `backend/src/server/app.py` (`FastAPI(version=...)`). The PR template lists these.

## Notes & gotchas

- **xvfb:** backend tests open `mss` via the FastAPI lifespan, which needs an X
  display on Linux — hence `xvfb-run`.
- **pnpm version:** `packageManager` in `frontend/package.json` pins pnpm; CI uses
  `pnpm/action-setup` which respects it. Locally, avoid mixing a corepack-shimmed
  pnpm of a different major version.
- **OCR models:** RapidOCR ships its ONNX models inside the wheel, so there's no
  runtime model download to cache and no PyTorch — the e2e job is light and fast.
- **Icons:** `frontend/src-tauri/icons/` is committed (the build requires it).
  Regenerate from a 1024² source with `pnpm tauri icon path/to/source.png`.
