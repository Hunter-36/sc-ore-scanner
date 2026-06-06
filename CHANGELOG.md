# Changelog

All notable changes to SC Ore Scanner. This project follows
[Semantic Versioning](https://semver.org/) and
[Conventional Commits](https://www.conventionalcommits.org/). Each GitHub Release
also has an auto-generated, per-PR "What's Changed" list; this file is the
curated, human-readable summary.

## [2.2.3] - 2026-06-06

### Fixed
- Debounce state is now correct across recalibration and error frames: the scan
  loop resets the debouncer when the scan region changes (so an ore from the old
  box can't falsely confirm in the new one), and a capture/OCR error now counts
  as a missed frame (breaking the confirm streak) instead of being skipped. (#63)

## [2.2.2] - 2026-06-06

### Fixed
- The background scan loop now survives a panic in any single cycle: the
  per-frame work (capture / OCR / resolve) runs under `catch_unwind`, so one bad
  frame is logged and the loop continues instead of silently ending detection
  for the session. The debouncer is reset after a caught panic. (#72)

## [2.2.1] - 2026-06-06

### Fixed
- `Config::load` now clamps every tunable to the same ranges the settings UI
  enforces, so a hand-edited or migrated `config.json` can't break detection.
  Notably, a `min_consecutive_frames` above the debouncer's history window would
  previously make detection silently never confirm. (#66)

## [2.2.0] - 2026-06-05

### Added
- In-app **settings page** (gear icon): tune scan interval, confirm frames, upscale,
  and contrast live — with **Responsive / Balanced / Low-impact** presets and a
  "≈ Xs to confirm" readout. Changes apply without a restart. (#49, #47)

### Changed
- Config writes are atomic (temp + rename, unique temp per write) so live-tuning
  can't make the scan loop momentarily drop the scan region.
- The debouncer updates its frame count in place, so changing "confirm frames"
  while watching a rock doesn't drop the currently-shown ore.

## [2.1.1] - 2026-06-05

### Changed
- Screen capture now converts only the calibrated scan region to RGB each cycle
  instead of the whole monitor frame — much less CPU on high-resolution displays. (#38)

### Security
- `save_scan_region` rejects regions smaller than 8 px so a bad value can't
  silently break detection. (#41)

## [2.1.0] - 2026-06-05

### Added
- Ambiguous radar signatures now show every equally-likely reading
  (e.g. "6× Savrilium / 5× Aslarite") instead of silently committing to one. (#21)

## [2.0.0] - 2026-06-05

### Changed
- **BREAKING:** rewrote the app as a single self-contained Rust binary (Tauri) —
  dropped the Python/FastAPI backend and the WebSocket. Detection runs in-process
  with `xcap` screen capture and the embedded pure-Rust `ocrs` OCR engine. Ships
  as one exe (no Python, no install). (#24)
- In-app calibration (drag-to-select) replaces the Python `calibrate.py`.

## 1.4.3 and earlier

v1 was a Python/FastAPI backend + Tauri overlay over a local WebSocket. See the
[GitHub Releases](https://github.com/Hunter-36/sc-ore-scanner/releases) for the
full v1 history.

[2.2.0]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.2.0
[2.1.1]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.1.1
[2.1.0]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.1.0
[2.0.0]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.0.0
