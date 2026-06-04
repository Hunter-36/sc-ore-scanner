"""Unit tests for ScreenCapture: scan-state gating and region handling.

Constructing ScreenCapture opens mss, which needs a display — CI runs the unit
suite under xvfb, and it works natively on Windows/macOS.
"""

import pytest
from PIL import Image

from src.capture import ScreenCapture


@pytest.fixture
def capture(settings):
    cap = ScreenCapture(settings)
    yield cap
    cap.close()


def test_capture_region_none_without_region(capture, settings):
    # Default settings have no scan_region configured -> nothing to capture.
    assert settings.scan_region is None
    assert capture.capture_region() is None


def test_scanner_active_on_bright_sample_points(capture, settings):
    img = Image.new("RGB", (50, 50), (0, 0, 0))
    px = img.load()
    for (x, y) in settings.scan_gating.sample_points:  # default (10,10),(20,20),(30,30)
        px[x, y] = (255, 255, 255)
    assert capture._is_scanner_active(img) is True


def test_scanner_inactive_on_dark_image(capture):
    img = Image.new("RGB", (50, 50), (0, 0, 0))
    assert capture._is_scanner_active(img) is False
