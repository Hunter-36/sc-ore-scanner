# OCR Pipeline & Tuning

How a captured scan region becomes a detected RS number, and the reasoning behind the
preprocessing — useful when adapting to new resolutions or debugging missed detections.

## The target

The mining scanner HUD shows the RS value as **bright teal digits with a thousands
comma and a location-pin glyph**, on a dark, particle-flecked background, e.g.
`📍 10,620`. Often the **distance marker** sits right next to it (`7,080 18.8km`).

Things that can trip up OCR:
- **The comma** — could be read as a digit.
- **The location-pin glyph** — could read as a leading digit.
- **The distance marker on the same line** — `ocrs` returns a whole line, so the RS
  value and distance arrive merged (`"0 7,080 18.8km"`).
- **Floating particles** — bright specks over a large region.

## The engine: ocrs (pure Rust)

Detection/recognition is done by [`ocrs`](https://github.com/robertknight/ocrs)
(rten backend). It's the engine because it:
- is **pure Rust** — no ONNX runtime, no PyTorch, no Python; the `.rten` models
  (~12 MB) are **embedded into the binary** at build time (`core/build.rs`);
- reads the bright HUD digits well **without heavy preprocessing**.

## The pipeline (`preprocess_for_ocr` → `recognize_rs_numbers` → debounce → resolve)

1. **Crop** to the calibrated scan region.
2. **Upscale** ×`upscale` (default 4, Lanczos) so the small text is large enough.
3. *(Optional)* **grayscale + CLAHE** contrast — see below; off by default.
4. **ocrs** OCRs the crop, returning text lines.
5. **Per-number extraction** (the internal `extract_numbers` helper): split each line into number
   tokens — commas are kept as thousands separators, but spaces/periods/letters end a
   token. So `"0 7,080 18.8km"` → `["0","7080","18","8"]`. Keep tokens that are 3–6
   digit numbers within `valid_rs_min..max`. (The pin glyph / distance fragments fall
   out by length.)
6. **Debounce** (`Debouncer`): a number must appear in at least `min_consecutive_frames`
   (3) of the last `2 × min_consecutive_frames` (6) frames before it's reported — a
   *window*, not a strict consecutive run, so a sig whose last digit wobbles frame-to-frame
   (e.g. 14,160 vs 14,150) stays confirmed through the jitter. Filters transient misreads.
7. **Resolve**: confirmed numbers go to the `Resolver` (division + OCR-error correction)
   → ore matches → aggregate best-per-ore.

## Two deliberate deviations from v1 (RapidOCR)

- **Per-number extraction instead of whole-line stripping.** v1's RapidOCR returned each
  text element as a separate box, so it stripped each independently. `ocrs` returns whole
  lines, so we split lines into number tokens to stay equivalent. Don't revert to
  stripping all non-digits from a line — that merged the RS value and distance into an
  8-digit blob that was rejected.
- **Debouncing instead of a confidence gate.** v1 dropped low-confidence reads (RapidOCR
  gave a score). `ocrs` exposes no per-line confidence (only chars + rects), so the
  windowed debounce carries that role — a transient misread can't reach a majority of the
  recent frames, so it never confirms.

## CLAHE is off by default

CLAHE (contrast-limited adaptive histogram equalization, matching cv2) is implemented in
`preprocess::clahe`, but at v1's `clip=2.0` it **regressed** ocrs detection (a passing
fixture stopped detecting) — ocrs reads the raw upscaled text better. So
`clahe_clip_limit` defaults to `0` (off) and is opt-in via config for dark/low-contrast
frames. **Contrast-based, not threshold-based** — aggressive thresholding destroyed digit
strokes in v1; don't reintroduce it.

## Relevant config

| Field | Default | Purpose |
|---|---|---|
| `upscale` | 4 | Upscale multiplier before OCR |
| `scan_interval_secs` | 0.75 | Seconds between scans |
| `min_consecutive_frames` | 3 | Debounce: frames required to confirm a number |
| `clahe_clip_limit` | 0.0 | CLAHE contrast limit (0 = off) |
| `clahe_grid` | [8, 8] | CLAHE tile grid |

## Resolution / region caveat

The pipeline crops to the calibrated region and applies a fixed upscale, so it adapts
reasonably to different region sizes, but it's been **validated against one capture
resolution** (the fixtures in `core/tests/fixtures/`). When adapting to a new resolution
or HUD scale, add captures there and a `tests/e2e.rs` assertion, and confirm the test
stays green.
