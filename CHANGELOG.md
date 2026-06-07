# Changelog

All notable changes to SC Ore Scanner. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project follows
[Semantic Versioning](https://semver.org/) and
[Conventional Commits](https://www.conventionalcommits.org/). Each GitHub Release
also has an auto-generated, per-PR "What's Changed" list; this file is the
curated, human-readable summary.

## [2.2.10] - 2026-06-06

### Fixed
- Mining signature data corrected to SC 4.8 (per MrKraken's chart, cross-checked
  against the Star Citizen Wiki API): FPS hand-mined gems are now a flat **3000**
  signature (Hadanite/Dolivine/Aphorite were stale per-gem values), **Janalite**
  added, and **Felinite** removed (no longer in the game). Added the ground-vehicle
  (ROC/ATLS) deposits **Beradom/Feynmaline/Glacosite** at the flat **4000** signature,
  and retagged the 26 ship ores from `["ship","vehicle"]` to `["ship"]` (ground
  vehicle is a distinct context). RS readings remain ambiguous within FPS (all 3000)
  and at 4000 (ROC deposits vs the I-Type asteroid) — surfaced as alternatives. (#22)

## [2.2.9] - 2026-06-06

### Changed
- New app icon and visual identity — a custom ore-crystal-in-radar-ring mark —
  applied across the app/taskbar icon, the NSIS and MSI installers, the web
  favicon, and the published price-table page. (#95)

## [2.2.8] - 2026-06-06

### Fixed
- Settings: an edit made just before closing the panel via the gear toggle is no
  longer lost — a pending debounced save is now flushed on unmount, not just by
  the Done button. A genuine save failure (distinct from running outside Tauri)
  is now surfaced in the panel instead of silently swallowed. (#70)

## [2.2.7] - 2026-06-06

### Added
- Single-instance lock: launching the app a second time now focuses the existing
  overlay instead of starting a second scanner that would fight over screen
  capture. (#61)

## [2.2.6] - 2026-06-06

### Changed
- The scan loop now resolves the primary monitor once and caches it, instead of
  re-enumerating displays every frame (~0.75s). The cache is dropped on a capture
  failure, so a monitor hot-plug / resolution change is still picked up. (#68)

## [2.2.5] - 2026-06-06

### Fixed
- Log lines written to `scanner.log` are now scrubbed of the user's Windows home
  path and username (e.g. `C:\Users\<name>\…` → `%USERPROFILE%\…`), so the log
  file users attach to bug reports no longer leaks that PII. (#60)

## [2.2.4] - 2026-06-06

### Fixed
- The UEX price-feed fetch now has a connect/read timeout (10s) and a response
  size cap, so a hung or oversized feed can no longer stall the overlay (the
  fetch runs on the scan thread at startup and hourly). Last-good prices are
  still kept on any failure. (#67)

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

## [2.1.2] - 2026-06-05

### Security
- Set a strict Content-Security-Policy for the overlay webview. (#52)

### Changed
- Added resolver/config unit tests, a CI dependency-audit job, and this changelog. (#51)

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

[2.2.10]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.2.10
[2.2.9]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.2.9
[2.2.8]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.2.8
[2.2.7]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.2.7
[2.2.6]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.2.6
[2.2.5]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.2.5
[2.2.4]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.2.4
[2.2.3]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.2.3
[2.2.2]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.2.2
[2.2.1]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.2.1
[2.2.0]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.2.0
[2.1.2]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.1.2
[2.1.1]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.1.1
[2.1.0]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.1.0
[2.0.0]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.0.0
