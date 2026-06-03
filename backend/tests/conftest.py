"""Shared pytest fixtures and path setup for the backend test suite.

Adds the backend root to sys.path so `import src...` works regardless of the
working directory the tests are invoked from.
"""

import sys
from pathlib import Path

import pytest

BACKEND_ROOT = Path(__file__).resolve().parent.parent
if str(BACKEND_ROOT) not in sys.path:
    sys.path.insert(0, str(BACKEND_ROOT))

from src.config import Settings  # noqa: E402


@pytest.fixture
def settings() -> Settings:
    """Fresh default Settings instance (does not read the user's config file).

    Constructing Settings() directly bypasses load_user_config(), so tests are
    deterministic and unaffected by any local settings.json.
    """
    return Settings()
