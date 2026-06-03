"""Reusable OCR -> resolver pipeline for end-to-end testing and manual checks.

Turns a single frame (PIL image / image file) or a video file into the set of
resolved ore matches, using the exact same OCREngine + RSResolver the live
scanner uses. This is the seam the E2E tests drive, and it doubles as a CLI:

    # one still frame
    python -m tests.e2e.pipeline tests/test_images/sc_mining_scan_rs_10620_some_particles.png

    # a captured video (samples every Nth frame)
    python -m tests.e2e.pipeline path/to/clip.mp4 --stride 15
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Dict, Iterator, Optional

from PIL import Image

# Make `import src...` work no matter where pytest / the CLI is invoked from.
BACKEND_ROOT = Path(__file__).resolve().parents[2]
if str(BACKEND_ROOT) not in sys.path:
    sys.path.insert(0, str(BACKEND_ROOT))

from src.config import Settings  # noqa: E402
from src.ocr import OCREngine  # noqa: E402
from src.resolver import OreMatch, RSResolver  # noqa: E402

IMAGE_SUFFIXES = {".png", ".jpg", ".jpeg", ".bmp", ".webp"}
VIDEO_SUFFIXES = {".mp4", ".mov", ".mkv", ".avi", ".webm"}


def detect_ores_in_frame(
    img: Image.Image, ocr: OCREngine, resolver: RSResolver
) -> Dict[str, OreMatch]:
    """Run a single frame through OCR + resolution.

    Returns a mapping of ore_id -> best OreMatch for that frame. Uses the raw
    per-frame OCR detections (not the debounced/confirmed set), so it reflects
    exactly what a single image contains.
    """
    detections = ocr.detect_numbers(img)
    matches = []
    for det in detections:
        matches.extend(resolver.resolve(det.number, ocr_confidence=det.confidence))
    return resolver.aggregate_detections(matches)


def crop_to_region(img: Image.Image, region: Optional[list]) -> Image.Image:
    """Crop to a [x, y, width, height] scan region (mimics in-game calibration).

    Returns the image unchanged when region is None/empty.
    """
    if not region:
        return img
    x, y, w, h = region
    return img.crop((x, y, x + w, y + h))


def top_match(aggregated: Dict[str, OreMatch]) -> Optional[OreMatch]:
    """Return the highest-confidence ore match, or None if there were none."""
    if not aggregated:
        return None
    return max(aggregated.values(), key=lambda m: m.confidence)


def iter_video_frames(path: Path, stride: int = 15) -> Iterator[Image.Image]:
    """Yield every `stride`-th frame of a video as an RGB PIL image."""
    import cv2  # local import: only needed when actually decoding video

    cap = cv2.VideoCapture(str(path))
    try:
        idx = 0
        while True:
            ok, frame = cap.read()
            if not ok:
                break
            if idx % stride == 0:
                rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
                yield Image.fromarray(rgb)
            idx += 1
    finally:
        cap.release()


def detect_ores_in_video(
    path: Path, ocr: OCREngine, resolver: RSResolver, stride: int = 15
) -> Dict[str, OreMatch]:
    """Aggregate ore detections across sampled frames of a video.

    Keeps the highest-confidence match per ore across all sampled frames,
    mirroring how the live scanner accumulates results over time.
    """
    aggregated: Dict[str, OreMatch] = {}
    for frame in iter_video_frames(path, stride=stride):
        for ore_id, match in detect_ores_in_frame(frame, ocr, resolver).items():
            if ore_id not in aggregated or match.confidence > aggregated[ore_id].confidence:
                aggregated[ore_id] = match
    return aggregated


def detect_ores_in_file(
    path: Path,
    ocr: OCREngine,
    resolver: RSResolver,
    stride: int = 15,
) -> Dict[str, OreMatch]:
    """Dispatch on file type: still image vs video."""
    suffix = path.suffix.lower()
    if suffix in VIDEO_SUFFIXES:
        return detect_ores_in_video(path, ocr, resolver, stride=stride)
    if suffix in IMAGE_SUFFIXES:
        with Image.open(path) as img:
            return detect_ores_in_frame(img.convert("RGB"), ocr, resolver)
    raise ValueError(f"Unsupported fixture type: {path.suffix} ({path})")


def build_engines(settings: Optional[Settings] = None) -> tuple[OCREngine, RSResolver]:
    """Construct an initialized OCR engine and resolver from settings."""
    settings = settings or Settings()
    ocr = OCREngine(settings)
    ocr.initialize()
    resolver = RSResolver(settings)
    return ocr, resolver


def _main(argv: list[str]) -> int:
    import argparse

    parser = argparse.ArgumentParser(description="Run the OCR->ore pipeline on an image or video.")
    parser.add_argument("path", type=Path, help="Image or video file to analyze")
    parser.add_argument("--stride", type=int, default=15, help="Sample every Nth video frame")
    args = parser.parse_args(argv)

    ocr, resolver = build_engines()
    results = detect_ores_in_file(args.path, ocr, resolver, stride=args.stride)

    if not results:
        print(f"No ores detected in {args.path.name}")
        return 0

    print(f"Detected ores in {args.path.name}:")
    for ore_id, match in sorted(results.items(), key=lambda kv: -kv[1].confidence):
        print(
            f"  {match.quantity}x {match.ore.name:<12} "
            f"(RS {match.detected_rs}, tier {match.ore.tier}, conf {match.confidence:.2f})"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(_main(sys.argv[1:]))
