"""Smoke tests for the FastAPI server via TestClient.

The lifespan constructs a real ScreenCapture (mss). On headless Linux CI this
requires a virtual display, so the backend job runs under xvfb. We patch
get_settings() so the app starts with a clean, scan-region-less config and the
background scanning loop never auto-starts (keeping these tests deterministic).
"""

import pytest
from fastapi.testclient import TestClient

import src.server.app as app_module
from src.config import Settings
from src.server import create_app


@pytest.fixture
def client(monkeypatch, tmp_path):
    def _settings():
        s = Settings()
        # Redirect persistence to a temp file so tests never touch the real
        # src/config/settings.json.
        s.config_file = tmp_path / "settings.json"
        return s

    monkeypatch.setattr(app_module, "get_settings", _settings)
    # Ensure no scanning state leaks in from a previous test/module.
    monkeypatch.setattr(app_module, "scan_enabled", False, raising=False)
    app = create_app()
    with TestClient(app) as c:
        yield c


def test_health(client):
    resp = client.get("/health")
    assert resp.status_code == 200
    body = resp.json()
    assert body["status"] == "ok"
    assert body["scanning"] is False


def test_signatures_endpoint(client):
    resp = client.get("/signatures")
    assert resp.status_code == 200
    sigs = resp.json()["signatures"]
    assert len(sigs) > 0
    names = {s["name"] for s in sigs}
    assert "Beryl" in names


def test_config_endpoint(client):
    resp = client.get("/config")
    assert resp.status_code == 200
    body = resp.json()
    assert body["scan_region"] is None
    assert body["scan_interval"] == 2.0
    assert "ocr" in body


def test_start_without_region_returns_400(client):
    resp = client.post("/scan/start")
    assert resp.status_code == 400


def test_set_scan_region_then_config_reflects_it(client, tmp_path, monkeypatch):
    region = {"x": 100, "y": 200, "width": 400, "height": 300}
    resp = client.post("/config/scan-region", json=region)
    assert resp.status_code == 200
    assert resp.json()["region"] == region

    # Stop the scanning loop the POST kicked off so it doesn't leak across tests.
    client.post("/scan/stop")


# --- CSRF guard on mutating endpoints (uses the safe /scan/stop, never /shutdown) ---

def test_mutating_request_blocked_from_foreign_origin(client):
    resp = client.post("/scan/stop", headers={"origin": "https://evil.example"})
    assert resp.status_code == 403


def test_mutating_request_allowed_from_tauri_origin(client):
    resp = client.post("/scan/stop", headers={"origin": "http://tauri.localhost"})
    assert resp.status_code == 200


def test_mutating_request_allowed_without_origin(client):
    resp = client.post("/scan/stop")
    assert resp.status_code == 200


def test_mutating_request_allowed_from_localhost_dev(client):
    resp = client.post("/scan/stop", headers={"origin": "http://localhost:1420"})
    assert resp.status_code == 200


# --- session stats endpoints ---

def test_stats_endpoint(client):
    import src.server.app as app_module
    app_module.session_stats.reset()
    resp = client.get("/stats")
    assert resp.status_code == 200
    body = resp.json()
    assert body["distinct_ores"] == 0
    assert body["total_detections"] == 0
    assert "ores" in body


def test_stats_export_csv(client):
    resp = client.get("/stats/export.csv")
    assert resp.status_code == 200
    assert "text/csv" in resp.headers["content-type"]
    assert resp.text.startswith("ore_id,name,tier")
