"""End-to-end OCR pipeline tests against real Star Citizen scan captures.

For each fixture in manifest.json we crop to the calibrated scan region, run the
*real* OCREngine + RSResolver, and repeat `runs` times. The expected ore must be
the top (highest-confidence) match in at least `min_pass_rate` of runs. This both
validates detection accuracy and guards against run-to-run flakiness.
"""

import json
from pathlib import Path

import pytest
from PIL import Image

from pipeline import crop_to_region, detect_ores_in_frame, top_match

pytestmark = pytest.mark.e2e

E2E_DIR = Path(__file__).resolve().parent
_MANIFEST = json.loads((E2E_DIR / "manifest.json").read_text())
_FIXTURES = _MANIFEST["fixtures"]


def _describe(top):
    if top is None:
        return None
    return (top.ore.name, top.quantity, round(top.confidence, 2))


@pytest.mark.parametrize("entry", _FIXTURES, ids=[f["file"] for f in _FIXTURES])
def test_detection_consistency(entry, engines, manifest, image_dir):
    ocr, resolver = engines
    region = entry.get("scan_region", manifest["default_scan_region"])
    runs = manifest["runs"]
    min_rate = manifest["min_pass_rate"]
    expected = entry["expected_top"]

    path = image_dir / entry["file"]
    if not path.exists():
        pytest.skip(f"fixture not found: {path}")

    img = Image.open(path).convert("RGB")
    crop = crop_to_region(img, region)

    passes = 0
    observed = []
    for _ in range(runs):
        ocr.reset_debouncing()  # keep each run independent
        top = top_match(detect_ores_in_frame(crop, ocr, resolver))
        observed.append(_describe(top))
        if expected is None:
            passes += top is None
        else:
            passes += (
                top is not None
                and top.ore.name == expected["name"]
                and top.quantity == expected["quantity"]
            )

    rate = passes / runs
    assert rate >= min_rate, (
        f"{entry['file']}: {passes}/{runs} runs matched expected_top={expected} "
        f"(need {min_rate:.0%}). Observed top matches: {observed}"
    )


def test_manifest_references_existing_fixtures(image_dir):
    """Every manifest entry should point at a real file (catches typos/renames)."""
    missing = [f["file"] for f in _FIXTURES if not (image_dir / f["file"]).exists()]
    assert not missing, f"manifest references missing fixtures: {missing}"
