"""Main entry point for SC Ore Scanner backend."""

import logging
import sys
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent / "src"))

import uvicorn
from src.config import get_settings
from src.server import create_app

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler(sys.stdout)
    ]
)

logger = logging.getLogger(__name__)


def main():
    """Start the FastAPI server."""
    settings = get_settings()

    logger.info("=" * 60)
    logger.info("SC ORE SCANNER - Backend Server")
    logger.info("=" * 60)
    logger.info(f"Version: 1.0.0")
    logger.info(f"WebSocket: ws://{settings.server.host}:{settings.server.port}/ws")
    logger.info(f"API: http://{settings.server.host}:{settings.server.port}")
    logger.info("=" * 60)

    # Create app
    app = create_app()

    # Run server
    uvicorn.run(
        app,
        host=settings.server.host,
        port=settings.server.port,
        log_level=settings.server.log_level.lower(),
        access_log=False  # Reduce noise
    )


if __name__ == "__main__":
    main()
