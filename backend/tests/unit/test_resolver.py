"""Unit tests for the RS signature resolver.

These exercise the core "number -> ore" math against the real signatures.json
database, so they double as a regression guard on the ore data itself.
"""

import pytest

from src.resolver import RSResolver, OreMatch


@pytest.fixture
def resolver(settings):
    return RSResolver(settings)


def test_signatures_loaded(resolver):
    # signatures.json currently ships 27 ores; at minimum the DB must be non-empty.
    assert len(resolver.signatures) > 0
    assert "beryl" in resolver.signatures_by_id
    assert resolver.signatures_by_id["beryl"].base_rs == 3540


@pytest.mark.parametrize(
    "detected_rs, expected_ore, expected_qty",
    [
        (3170, "Quantainium", 1),   # exact single node
        (10620, "Beryl", 3),        # 3 x 3540
        (17140, "Aluminium", 4),    # 4 x 4285
        (3540, "Beryl", 1),         # exact single node
        (6340, "Quantainium", 2),   # 2 x 3170
        (3840, "Aslarite", 1),      # 4.7 ore (clustered near Laranite 3825)
        (4195, "Tin", 1),           # 4.7 ore (clustered near Hephestanite 4180)
        (4210, "Quartz", 1),        # 4.7 ore (clustered near Corundum 4225)
        (4255, "Silicon", 1),       # 4.7 ore (clustered near Copper 4240)
        (4700, "C-Type Asteroid", 1),  # asteroid type signature
        (4900, "E-Type Asteroid", 1),  # asteroid type signature
    ],
)
def test_exact_division_match(resolver, detected_rs, expected_ore, expected_qty):
    matches = resolver.resolve(detected_rs)
    assert matches, f"expected a match for {detected_rs}"
    top = matches[0]
    assert top.ore.name == expected_ore
    assert top.quantity == expected_qty
    # Exact divisions should be maximally confident at ocr_confidence=1.0.
    assert top.confidence == pytest.approx(1.0)
    assert top.error_margin == 0


def test_matches_sorted_by_confidence_desc(resolver):
    matches = resolver.resolve(10620)
    confidences = [m.confidence for m in matches]
    assert confidences == sorted(confidences, reverse=True)


def test_confidence_scales_with_ocr_confidence(resolver):
    full = resolver.resolve(3540, ocr_confidence=1.0)[0]
    half = resolver.resolve(3540, ocr_confidence=0.5)[0]
    assert half.confidence == pytest.approx(full.confidence * 0.5)


def test_quantity_out_of_range_no_match(resolver):
    # 3540 * 11 exceeds max_quantity (10) and isn't a multiple of any other base.
    matches = resolver.resolve(3540 * 11)
    assert all(m.ore.name != "Beryl" for m in matches)


def test_ocr_correction_split(resolver):
    # "33170" -> quantity 3 + signature 3170 (Quantainium). Pure division finds
    # nothing (33170 isn't a clean multiple), so the correction path must fire.
    matches = resolver.resolve(33170)
    names = {m.ore.name for m in matches}
    assert "Quantainium" in names


def test_ocr_correction_extra_digit(resolver):
    # "105620" with a spurious '5' -> "10620" -> 3 x Beryl.
    matches = resolver.resolve(105620)
    names = {m.ore.name for m in matches}
    assert "Beryl" in names


def test_aggregate_detections_keeps_highest_confidence(resolver):
    ore = resolver.signatures_by_id["beryl"]
    low = OreMatch(ore=ore, quantity=1, detected_rs=3540, confidence=0.4)
    high = OreMatch(ore=ore, quantity=3, detected_rs=10620, confidence=0.95)
    aggregated = resolver.aggregate_detections([low, high])
    assert set(aggregated.keys()) == {"beryl"}
    assert aggregated["beryl"].confidence == pytest.approx(0.95)
    assert aggregated["beryl"].quantity == 3


def test_get_ore_by_id(resolver):
    assert resolver.get_ore_by_id("quantanium").name == "Quantainium"
    assert resolver.get_ore_by_id("does-not-exist") is None
