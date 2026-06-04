"""Fixtures for the end-to-end OCR pipeline tests.

These tests need the OCR engine (rapidocr-onnxruntime) and the real capture
fixtures. If it's missing the whole module is skipped, so the light unit-test
CI job (which doesn't install the OCR deps) stays green.
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


def _ocr_available() -> bool:
    try:
        import rapidocr_onnxruntime  # noqa: F401
    except Exception:
        return False
    return True


# Skip the entire e2e package if the OCR engine isn't installed.
pytestmark = pytest.mark.skipif(
    not _ocr_available(),
    reason="OCR engine (rapidocr-onnxruntime) not installed; run `pip install -r requirements-ml.txt`",
)


@pytest.fixture(scope="session")
def engines():
    """Initialized (OCREngine, RSResolver) shared across the whole e2e session.

    OCR engine init is relatively expensive, so build it once.
    """
    from pipeline import build_engines

    return build_engines()


@pytest.fixture(scope="session")
def manifest() -> dict:
    return json.loads(MANIFEST.read_text())


@pytest.fixture(scope="session")
def image_dir() -> Path:
    return IMAGE_DIR
