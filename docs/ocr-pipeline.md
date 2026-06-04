# OCR Pipeline & Tuning

This documents how a captured scan region becomes a detected RS number, and the
reasoning behind the preprocessing — useful when adapting to new resolutions or
debugging missed detections.

## The target

The mining scanner HUD shows the RS value as **bright teal digits with a
thousands comma and a location-pin glyph**, on a dark, particle-flecked
background, e.g. `📍 10,620`.

Three things actively hurt OCR:

- **The comma** — read as a digit (`10,620` → `105620` / `10820`).
- **The location-pin glyph** — read as a leading digit (`2`, `9`, …).
- **Floating particles** — bright specks that become phantom digits, especially
  if you run OCR over a large/full-screen area.

## The pipeline (`OCREngine.preprocess_image`)

1. **Upscale** by `upscale_factor` (default 4) with **cv2 LANCZOS4**.
   The interpolator matters: PIL's resampler flipped a borderline `6`→`8` on the
   real captures; cv2 LANCZOS4 preserved it.
2. **Grayscale.**
3. **CLAHE** (contrast-limited adaptive histogram equalization) + **min-max
   normalize** — boosts the teal digits against the dark background. This is the
   key step: contrast, not hard thresholding.
4. **Component mask** (`_mask_digit_components`): Otsu-threshold to find bright
   blobs, then keep only components that are **tall** (≥ `min_component_height_frac`
   of the tallest blob) and **large enough** (≥ `min_component_area`). The short
   comma and tiny particles fall below the cutoffs and are removed. The mask is
   applied back to the *contrast-enhanced grayscale*, so digit strokes keep their
   anti-aliased shape (binarizing here destroys thin strokes).

The surviving pin glyph reads as a lone digit, which the `\d{3,6}` filter in
`detect_numbers()` discards. The comma-corrected reads are recovered by the
resolver's OCR-error correction.

## Why not the old pipeline?

The original `preprocess_image` used adaptive thresholding + morphological
opening + small-component removal. On real captures this **destroyed the thin
digit strokes** — EasyOCR read raw frames perfectly (`10,620` @ 1.0) but returned
garbage after preprocessing. The end-to-end suite caught this; the fix above
restored detection. See `git log` for the change.

## Relevant config (`OCRConfig`)

| Field | Default | Purpose |
|---|---|---|
| `upscale_factor` | 4 | Upscale multiplier before OCR |
| `clahe_clip_limit` | 2.0 | CLAHE contrast limit |
| `clahe_grid_size` | (8, 8) | CLAHE tile grid |
| `min_component_area` | 40 | Drop blobs smaller than this (px) |
| `min_component_height_frac` | 0.6 | Keep blobs ≥ this fraction of the tallest blob's height |
| `confidence_threshold` | 0.5 | Min EasyOCR confidence to accept |
| `min_consecutive_frames` | 3 | Debounce: frames required to confirm a number |

## Resolution / region caveat

The thresholds are **relative** (fractions of blob height, an upscale factor),
not absolute pixels, so the pipeline adapts to different calibrated scan-region
sizes. It has, however, only been **validated against one capture resolution**
(the fixtures in `backend/tests/test_images/`). When adapting to a new resolution
or HUD scale, add captures at that resolution to the e2e manifest and confirm the
consistency test stays green — that's the safety net.
