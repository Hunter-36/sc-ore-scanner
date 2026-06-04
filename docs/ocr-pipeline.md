# OCR Pipeline & Tuning

This documents how a captured scan region becomes a detected RS number, and the
reasoning behind the preprocessing — useful when adapting to new resolutions or
debugging missed detections.

## The target

The mining scanner HUD shows the RS value as **bright teal digits with a
thousands comma and a location-pin glyph**, on a dark, particle-flecked
background, e.g. `📍 10,620`.

Things that can trip up OCR:

- **The comma** — can be read as a digit (`10,620` → `105620`).
- **The location-pin glyph** — can read as a leading digit.
- **Floating particles** — bright specks, especially over a large area.

## The engine: RapidOCR (ONNX)

Detection/recognition is done by [RapidOCR](https://github.com/RapidAI/RapidOCR)
(`rapidocr-onnxruntime`). It's the OCR engine because it:

- has **no PyTorch dependency** — ~150 MB installed vs ~2 GB for an easyocr/torch
  stack (this is what makes the app realistically distributable; see
  [ci-cd.md](ci-cd.md) and the release notes);
- **ships its models inside the wheel** — no runtime model download;
- reads the **comma and stylized digits natively** and at high confidence
  (≈0.99 on the real captures).

## The pipeline (`OCREngine.preprocess_image` → `detect_numbers`)

1. **Upscale** by `upscale_factor` (default 4) with **cv2 LANCZOS4**.
   The interpolator matters: PIL's resampler flipped a borderline `6`→`8` on the
   real captures; cv2 LANCZOS4 preserved it.
2. **Grayscale.**
3. **CLAHE** (contrast-limited adaptive histogram equalization) — boosts the
   teal digits against the dark background. That's the whole image step: contrast,
   no thresholding or masking.
4. **RapidOCR** reads the region.
5. **Digit extraction** (`detect_numbers`): for each text line above
   `confidence_threshold`, strip everything non-numeric (`re.sub(r"[^0-9]", "", text)`)
   so `10,620` → `10620`, then accept 3–6 digit values within the valid RS range.
   The lone pin glyph reads as a separate 1-char token and is dropped by the
   length filter.
6. **Debounce + resolve:** numbers confirmed across `min_consecutive_frames` go to
   the `RSResolver`, whose division + OCR-error correction maps them to ore.

## History: why not EasyOCR?

v1.0.0 development started on EasyOCR. Two problems drove the switch to RapidOCR:

1. **Size.** EasyOCR pulls PyTorch — ~2 GB installed. Unacceptable for a tool
   handed to other players.
2. **Fragility.** The EasyOCR-era preprocessing used adaptive thresholding +
   morphology + a connected-component mask to strip the comma/pin/particles. That
   destroyed thin digit strokes (the app detected nothing on real captures) and
   needed careful per-glyph tuning. RapidOCR reads the lightly-contrasted image
   directly, so all that masking went away.

The end-to-end suite is what made the swap safe: it proved RapidOCR cleared the
same 10×/90% bar before EasyOCR was removed.

## Relevant config (`OCRConfig`)

| Field | Default | Purpose |
|---|---|---|
| `upscale_factor` | 4 | Upscale multiplier before OCR |
| `clahe_clip_limit` | 2.0 | CLAHE contrast limit |
| `clahe_grid_size` | (8, 8) | CLAHE tile grid |
| `confidence_threshold` | 0.5 | Min OCR confidence to accept a line |
| `min_consecutive_frames` | 3 | Debounce: frames required to confirm a number |

## Resolution / region caveat

The pipeline crops to the calibrated scan region and applies a fixed upscale +
contrast boost, so it adapts reasonably to different region sizes. It has, however,
only been **validated against one capture resolution** (the fixtures in
`backend/tests/test_images/`). When adapting to a new resolution or HUD scale, add
captures at that resolution to the e2e manifest and confirm the consistency test
stays green — that's the safety net.
