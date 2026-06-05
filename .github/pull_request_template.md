<!-- Link the issue this addresses, e.g. "Closes #12" -->
## Summary

Closes #

<!-- What changed and why? -->

## ⚠️ Version bump (do this BEFORE merging a release-worthy change)

Merging to `master` **automatically tags and publishes a release when the version
changes**. If this PR should ship a release, bump the version to the same value in
**all three** places (CI fails if they disagree):

- [ ] `frontend/package.json`
- [ ] `frontend/src-tauri/tauri.conf.json`
- [ ] `frontend/src-tauri/Cargo.toml`

SemVer: **patch** = fix, **minor** = feature, **major** = breaking.
Docs-only / chore PRs can leave the version unchanged — no release will be cut.

## Checklist

- [ ] Core tests pass (`cargo test` in `core/`)
- [ ] Frontend checks pass (`pnpm typecheck && pnpm test` in `frontend/`)
- [ ] Linked the issue above
