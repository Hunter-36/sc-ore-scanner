<!-- Link the issue this addresses, e.g. "Closes #12" -->
## Summary

Closes #

<!-- What changed and why? -->

## ⚠️ Version bump (do this BEFORE merging a release-worthy change)

Merging to `master` **automatically tags and publishes a release when the version
changes**. If this PR should ship a release, bump the version to the same value in
**all** of these places (CI fails if the frontend three disagree):

- [ ] `frontend/package.json`
- [ ] `frontend/src-tauri/tauri.conf.json`
- [ ] `frontend/src-tauri/Cargo.toml`
- [ ] `backend/main.py` (the `Version:` log line)
- [ ] `backend/src/server/app.py` (`FastAPI(version=...)`)

SemVer: **patch** = fix, **minor** = feature, **major** = breaking.
Docs-only / chore PRs can leave the version unchanged — no release will be cut.

## Checklist

- [ ] Backend tests pass (`pytest` in `backend/`)
- [ ] Frontend checks pass (`pnpm typecheck && pnpm test` in `frontend/`)
- [ ] Linked the issue above
