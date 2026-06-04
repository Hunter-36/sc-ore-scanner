"""Main entry point for SC Ore Scanner backend."""

import logging
import os
import sys
from logging.handlers import RotatingFileHandler
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent / "src"))

# When launched with pythonw.exe (no console), sys.stdout/stderr are None.
# Point them at a sink so any stray prints / library output can't crash the app.
if sys.stdout is None:
    sys.stdout = open(os.devnull, "w")
if sys.stderr is None:
    sys.stderr = open(os.devnull, "w")

import uvicorn
from src.config import get_settings
from src.server import create_app

# Log to a size-capped, self-pruning file (no console needed). Lives in
# <app root>/logs/scanner.log next to the executable folder.
LOG_DIR = Path(__file__).resolve().parent.parent / "logs"
LOG_DIR.mkdir(parents=True, exist_ok=True)
LOG_FILE = LOG_DIR / "scanner.log"

_file_handler = RotatingFileHandler(
    LOG_FILE, maxBytes=1_000_000, backupCount=3, encoding="utf-8"
)
_file_handler.setFormatter(
    logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
)

logging.basicConfig(level=logging.INFO, handlers=[_file_handler])

logger = logging.getLogger(__name__)


def main():
    """Start the FastAPI server."""
    settings = get_settings()

    logger.info("=" * 60)
    logger.info("SC ORE SCANNER - Backend Server")
    logger.info("=" * 60)
    logger.info(f"Version: 1.1.1")
    logger.info(f"WebSocket: ws://{settings.server.host}:{settings.server.port}/ws")
    logger.info(f"API: http://{settings.server.host}:{settings.server.port}")
    logger.info("=" * 60)

    # Create app
    app = create_app()

    # Use an explicit Server so /shutdown can request a graceful stop (it sets
    # server.should_exit). log_config=None routes uvicorn through the root logger
    # (our rotating file handler) instead of its own console logging.
    config = uvicorn.Config(
        app,
        host=settings.server.host,
        port=settings.server.port,
        log_level=settings.server.log_level.lower(),
        access_log=False,
        log_config=None,
    )
    server = uvicorn.Server(config)
    app.state.server = server

    # Surface startup failures clearly — the backend runs windowless, so without
    # this a bind error (e.g. port already in use) would vanish and the overlay
    # would sit on "Starting scanner…" forever. The log lands in logs/scanner.log.
    try:
        server.run()
    except OSError as e:
        logger.error(
            f"Could not start the backend on {settings.server.host}:{settings.server.port} "
            f"— is it already running, or is the port in use? ({e})"
        )
        raise
    except Exception:
        logger.exception("Backend stopped unexpectedly")
        raise


if __name__ == "__main__":
    main()
