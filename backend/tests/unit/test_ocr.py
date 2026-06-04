"""Unit tests for OCR preprocessing and debouncing.

These avoid loading the OCR engine entirely: image preprocessing only touches
OpenCV/Pillow/NumPy, and the debouncing logic is pure Python. The actual
text recognition is covered by the end-to-end suite.
"""

import numpy as np
import pytest
from PIL import Image

from src.ocr import OCREngine, OCRResult


@pytest.fixture
def engine(settings):
    return OCREngine(settings)


def test_preprocess_upscales_and_returns_rgb(engine, settings):
    img = Image.new("RGB", (40, 20), color=(0, 0, 0))
    out = engine.preprocess_image(img)

    assert isinstance(out, np.ndarray)
    assert out.ndim == 3 and out.shape[2] == 3  # RGB
    factor = settings.ocr.upscale_factor
    assert out.shape[0] == 20 * factor
    assert out.shape[1] == 40 * factor


def test_preprocess_handles_non_rgb_input(engine):
    # Grayscale input must be converted, not crash.
    img = Image.new("L", (30, 15), color=128)
    out = engine.preprocess_image(img)
    assert out.shape[2] == 3


def _confirm(engine, number, frames):
    """Feed `number` as detected for `frames` consecutive frames."""
    for _ in range(frames):
        engine._update_debouncing([OCRResult(number=number, confidence=0.9)])


def test_debouncing_requires_min_consecutive_frames(engine, settings):
    n = settings.ocr.min_consecutive_frames

    _confirm(engine, 10620, n - 1)
    assert 10620 not in engine.get_confirmed_numbers()

    engine._update_debouncing([OCRResult(number=10620, confidence=0.9)])
    assert 10620 in engine.get_confirmed_numbers()


def test_debouncing_reset_clears_state(engine, settings):
    _confirm(engine, 3170, settings.ocr.min_consecutive_frames)
    assert 3170 in engine.get_confirmed_numbers()

    engine.reset_debouncing()
    assert engine.get_confirmed_numbers() == []


def test_absent_number_not_confirmed(engine, settings):
    n = settings.ocr.min_consecutive_frames
    # Number appears, then disappears for a frame -> not N consecutive.
    _confirm(engine, 4285, n - 1)
    engine._update_debouncing([])  # frame with no detections
    assert 4285 not in engine.get_confirmed_numbers()
