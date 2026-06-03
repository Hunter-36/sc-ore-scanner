"""Unit tests for settings loading, validation, and persistence."""

import pytest
from pydantic import ValidationError

from src.config import Settings, ScanRegion


def test_defaults(settings):
    assert settings.scan_interval == 2.0
    assert settings.scan_region is None
    assert settings.server.port == 8765
    assert settings.ocr.confidence_threshold == 0.5
    assert settings.signatures_path.exists()


def test_scan_region_validation():
    region = ScanRegion(x=10, y=20, width=300, height=150)
    assert region.width == 300

    with pytest.raises(ValidationError):
        ScanRegion(x=-1, y=0, width=100, height=100)  # x must be >= 0

    with pytest.raises(ValidationError):
        ScanRegion(x=0, y=0, width=0, height=100)  # width must be > 0


def test_save_and_load_roundtrip(tmp_path, settings):
    settings.config_file = tmp_path / "settings.json"
    settings.scan_interval = 1.5
    settings.scan_region = ScanRegion(x=100, y=200, width=400, height=300)
    settings.save_user_config()

    assert settings.config_file.exists()

    loaded = Settings.load_user_config(settings.config_file)
    assert loaded.scan_interval == 1.5
    assert loaded.scan_region is not None
    assert loaded.scan_region.x == 100
    assert loaded.scan_region.width == 400


def test_load_missing_file_returns_defaults(tmp_path):
    loaded = Settings.load_user_config(tmp_path / "nope.json")
    assert loaded.scan_interval == 2.0
    assert loaded.scan_region is None


def test_env_override(monkeypatch):
    monkeypatch.setenv("SC_SCANNER_SCAN_INTERVAL", "5.0")
    assert Settings().scan_interval == 5.0
