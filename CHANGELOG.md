# Changelog

All notable changes to SC Ore Scanner. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project follows
[Semantic Versioning](https://semver.org/) and
[Conventional Commits](https://www.conventionalcommits.org/). Each GitHub Release
also has an auto-generated, per-PR "What's Changed" list; this file is the
curated, human-readable summary.

## [2.4.3] - 2026-06-27

### Fixed
- Fixed images to look.. not AI generated.
- **Installer and app icons are refreshed — no more white box around the logo.** The full
  app icon set and the NSIS/WiX installer artwork are regenerated from the current brand
  master (`icon-master.png`). The installer dialogs previously framed the logo with a white
  rectangle because the art was composited from a non-transparent source.
- **The overlay's favicon no longer 404s.** `frontend/index.html` referenced a `/favicon.png`
  that didn't exist; it now points at a real, multi-resolution `favicon.ico`.

### Changed
- **One source of truth for web icons.** `frontend/public/favicon.ico` is the single favicon
  used by both the overlay and the published price page. The price feed page (GitHub Pages)
  no longer inlines its icon as a base64 data URI — it ships `favicon.ico` alongside
  `prices.json` and references it relatively, so the page is smaller and the icon is cached.

## [2.4.2] - 2026-06-10

### Fixed
- **Ultrawide users no longer have to lower their FOV for reliable scans.** The OCR
  upscale is now **adaptive**: instead of a fixed ×4, the pipeline scales the cropped
  scan region toward a readable text height (capped at ×8), so the small RS readout that
  ultrawide (21:9 / 32:9) + high FOV produces is enlarged enough for `ocrs` to read.
  Detection on the validated 16:9 captures is unchanged (still computes to ×4). The
  `Upscale` setting is now the **minimum** factor (a floor), not a fixed multiplier. (#110)

### Added
- **Calibration warns when the drawn region is too small to read.** If the box is below
  the height detection can recover even at max upscale (~24 px), the overlay flags it and
  suggests a tighter box or a higher in-game HUD/render scale — instead of silently saving
  a region that won't detect well. (#110)

## [2.4.0] - 2026-06-07

### Added
- **Panics are now logged.** A global panic hook routes any panic (its message +
  source location) to `scanner.log`, so failures are diagnosable in release builds —
  which have no console. This also enriches the scan loop's existing panic guard with
  the actual panic message instead of a generic line.
- **The overlay surfaces a fatal scanner failure instead of hanging.** If the OCR
  engine fails to load, the scan thread now emits an error to the overlay (shown as
  "Scanner unavailable" with the cause) rather than leaving it stuck on "Starting
  scanner…" forever. The `scan-result` event gained an optional `error` field.
- **Configurable log verbosity.** Set `SC_ORE_LOG=debug` (trace/debug/info/warn/
  error/off) to capture detection-level detail — the OCR line dumps — for a bug
  report, without a custom build. Defaults to `info`.

### Changed
- **The log file no longer grows without bound.** `scanner.log` is rolled to
  `scanner.log.1` at startup once it passes 5 MB, capping disk use across months of
  sessions.

### Fixed
- The frontend's `scan-result` event handler is now guarded, so a malformed payload
  logs to the console instead of silently escaping React's error boundary.

## [2.3.2] - 2026-06-07

### Fixed
- **Ambiguous and high-RS cards no longer flicker** on dropped OCR frames. The debouncer
  now confirms a number seen in a majority of recent frames (≥N of the last 2N) instead of
  requiring a strict consecutive run, so a signature whose last digit wobbles frame-to-frame
  (e.g. 14,160 vs 14,150) stays confirmed through the jitter; the overlay also lingers the
  last result through a brief empty gap. (#103)

### Changed
- Price labels read **"/SCU"** to make the unit explicit. Expanded resolver/debounce test
  coverage. (#103)

## [2.3.1] - 2026-06-07

### Fixed
- Settings sliders now reach the full range the backend accepts: scan interval up to
  **5.0s** (was capped at 3.0) and contrast/CLAHE up to **8.0** (was 4.0), so those values
  are reachable from the UI instead of only by hand-editing config.json. (#94)

### Changed
- CI: bumped the GitHub Actions off the deprecated Node-20 runtime (checkout v6,
  setup-node v6, setup-python v6, pnpm/action-setup v6, softprops/action-gh-release v3,
  upload-pages-artifact v5, deploy-pages v5).

## [2.3.0] - 2026-06-07

### Added
- **Per-candidate display for ambiguous signatures + a location picker.** Signature-
  degenerate readings (where the RS can't identify the ore — e.g. all FPS hand-gems share
  3000, all ROC deposits 4000) now show every possibility on one card with each one's
  value, instead of arbitrarily naming one. Set your **mining location** in Settings
  (gear → Location) and the candidates are ranked by their per-body **spawn probability**
  (e.g. on Cellin a 3000 read is 59% Aphorite / 35% Dolivine / 6% Hadanite; gems that
  don't spawn there are dropped). Uses the per-location data shipped in 2.2.11; no
  location set = candidates shown by value. (#22)

## [2.2.11] - 2026-06-06

### Changed
- The ore signature dataset is now **auto-sourced from the Star Citizen Wiki API** and
  enriched with per-location spawn data. `core/data/signatures.json` stays the curated
  source of base_rs/tier/volatile; `scripts/fetch_mineables.py` attaches each ore's harvest
  locations + spawn probability (via the Wiki API) into `core/data/mineables.json`, which
  the app now loads at startup from the published feed (`mineables.json` on GitHub Pages),
  with an on-disk cache and the embedded copy as offline fallbacks — so ore data can refresh
  per game patch without an app update. Detection behaviour is unchanged; the location data
  is surfaced in a later release. The `Prices` workflow is now the combined **`Feeds`**
  workflow (prices hourly, mineables daily). (#22)

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

[2.3.1]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.3.1
[2.3.0]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.3.0
[2.2.11]: https://github.com/Hunter-36/sc-ore-scanner/releases/tag/v2.2.11
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
