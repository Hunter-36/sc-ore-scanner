"""Unit test for the background scanning loop wiring (capture -> OCR -> resolve
-> broadcast), using fakes for capture/OCR and the real resolver."""

import asyncio

from PIL import Image

import src.server.app as app_module
from src.resolver import RSResolver


class _FakeCapture:
    def __init__(self, img):
        self._img = img

    def capture_region(self, force=False):
        return self._img


class _FakeOCR:
    """Returns a fixed set of confirmed numbers; no real OCR."""
    def __init__(self, confirmed):
        self._confirmed = confirmed

    def detect_numbers(self, img):
        return []

    def get_confirmed_numbers(self):
        return self._confirmed

    def reset_debouncing(self):
        pass


async def test_scanning_loop_broadcasts_resolved_ore(settings, monkeypatch):
    settings.scan_interval = 0.01
    sent = []

    async def fake_broadcast(msg):
        sent.append(msg)
        app_module.scan_enabled = False  # stop after the first broadcast

    monkeypatch.setattr(app_module.manager, "broadcast", fake_broadcast)
    monkeypatch.setattr(app_module, "scan_enabled", True)

    capture = _FakeCapture(Image.new("RGB", (100, 40)))
    ocr = _FakeOCR(confirmed=[10620])  # 3 x Beryl (3540)
    resolver = RSResolver(settings)

    await asyncio.wait_for(
        app_module.scanning_loop(settings, capture, ocr, resolver),
        timeout=5,
    )

    assert sent, "loop should have broadcast at least one result"
    result = sent[0]
    assert result["scanner_active"] is True
    assert "beryl" in result["ores"]
    assert result["ores"]["beryl"]["name"] == "Beryl"
    assert result["ores"]["beryl"]["quantity"] == 3
