"""Fixtures for the end-to-end OCR pipeline tests.

These tests need the heavy ML stack (easyocr + torch) and the real capture
fixtures. If either is missing the whole module is skipped, so the light
unit-test CI job (which doesn't install the ML deps) stays green.
"""

import json
import sys
from pathlib import Path

import pytest

E2E_DIR = Path(__file__).resolve().parent
BACKEND_ROOT = E2E_DIR.parent.parent
IMAGE_DIR = BACKEND_ROOT / "tests" / "test_images"
MANIFEST = E2E_DIR / "manifest.json"

# Make `import pipeline` (this directory) and `import src...` resolvable.
for p in (str(E2E_DIR), str(BACKEND_ROOT)):
    if p not in sys.path:
        sys.path.insert(0, p)


def _ml_available() -> bool:
    try:
        import easyocr  # noqa: F401
        import torch  # noqa: F401
    except Exception:
        return False
    return True


# Skip the entire e2e package if the ML stack isn't installed.
pytestmark = pytest.mark.skipif(
    not _ml_available(),
    reason="OCR/ML stack (easyocr + torch) not installed; run `pip install -r requirements-ml.txt`",
)


@pytest.fixture(scope="session")
def engines():
    """Initialized (OCREngine, RSResolver) shared across the whole e2e session.

    EasyOCR model load is expensive, so build it once.
    """
    from pipeline import build_engines

    return build_engines()


@pytest.fixture(scope="session")
def manifest() -> dict:
    return json.loads(MANIFEST.read_text())


@pytest.fixture(scope="session")
def image_dir() -> Path:
    return IMAGE_DIR
