# CI/CD

Three GitHub Actions workflows live in [`.github/workflows/`](../.github/workflows/).

## CI — `ci.yml`

Fast checks on every push and pull request.

| Job | Runner | Steps |
|---|---|---|
| `backend-unit` | ubuntu | apt `libgl1`/`libglib2.0-0`/`xvfb`; `uv pip install -r requirements-dev.txt`; `ruff check`; `xvfb-run pytest tests/unit` |
| `frontend-unit` | ubuntu | pnpm install (frozen); `pnpm typecheck`; `pnpm test` (vitest) |
| `rust-check` | ubuntu | Tauri system libs; `cargo check` on `frontend/src-tauri` (with `rust-cache`) |

No ML stack is installed here, so it stays quick.

## E2E — `e2e.yml`

Heavier validation on push / PR (and `workflow_dispatch`).

| Job | Runner | Steps |
|---|---|---|
| `backend-ocr-e2e` | ubuntu | full deps incl. **CPU torch** (`--extra-index-url .../whl/cpu`); EasyOCR models cached at `~/.EasyOCR`; `xvfb-run pytest tests/e2e` |
| `frontend-display-e2e` | ubuntu | pnpm install; `playwright install --with-deps chromium`; `pnpm test:e2e`; uploads `playwright-report` artifact |

## Release — `release.yml`

Builds the installer and publishes a GitHub Release.

- **Trigger:** pushing a tag matching `v*` (or manual dispatch).
- **Runner:** `windows-latest`.
- Uses [`tauri-apps/tauri-action`](https://github.com/tauri-apps/tauri-action) to run
  `pnpm build` + bundle, then create a **draft** Release named `SC Ore Scanner <tag>`
  with the `.msi`/`.exe` attached.
- Needs `contents: write` (granted in the workflow); uses the default `GITHUB_TOKEN`.

Cut a release:
```bash
git tag v1.0.1
git push origin v1.0.1
```
Then review and publish the draft Release on GitHub.

## Notes & gotchas

- **xvfb:** backend tests open `mss` via the FastAPI lifespan, which needs an X
  display on Linux — hence `xvfb-run`.
- **pnpm version:** `packageManager` in `frontend/package.json` pins pnpm; CI uses
  `pnpm/action-setup` which respects it. Locally, avoid mixing a corepack-shimmed
  pnpm of a different major version.
- **EasyOCR cache:** first e2e run downloads detection/recognition models (~64 MB);
  the cache key is static so later runs reuse them.
- **Icons:** `frontend/src-tauri/icons/` is committed (the build requires it).
  Regenerate from a 1024² source with `pnpm tauri icon path/to/source.png`.
