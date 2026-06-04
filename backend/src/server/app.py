"""FastAPI application with WebSocket support."""

import asyncio
import logging
from contextlib import asynccontextmanager
from typing import Dict, Set

from fastapi import FastAPI, WebSocket, WebSocketDisconnect, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

from ..config import Settings, ScanRegion, get_settings
from ..capture import ScreenCapture
from ..ocr import OCREngine
from ..resolver import RSResolver

logger = logging.getLogger(__name__)


class ScanResult(BaseModel):
    """Scan result sent to frontend."""
    ores: Dict[str, dict]  # ore_id -> {name, quantity, tier, confidence}
    scanner_active: bool
    timestamp: float


class ConnectionManager:
    """Manage WebSocket connections."""

    def __init__(self):
        self.active_connections: Set[WebSocket] = set()

    async def connect(self, websocket: WebSocket):
        """Accept new WebSocket connection."""
        await websocket.accept()
        self.active_connections.add(websocket)
        logger.info(f"Client connected. Active connections: {len(self.active_connections)}")

    def disconnect(self, websocket: WebSocket):
        """Remove WebSocket connection."""
        self.active_connections.discard(websocket)
        logger.info(f"Client disconnected. Active connections: {len(self.active_connections)}")

    async def broadcast(self, message: dict):
        """Broadcast message to all connected clients."""
        disconnected = set()

        for connection in self.active_connections:
            try:
                await connection.send_json(message)
            except Exception as e:
                logger.error(f"Failed to send to client: {e}")
                disconnected.add(connection)

        # Clean up disconnected clients
        for conn in disconnected:
            self.disconnect(conn)


# Global state
manager = ConnectionManager()
scanning_task = None
scan_enabled = False


async def scanning_loop(settings: Settings, capture: ScreenCapture, ocr: OCREngine, resolver: RSResolver):
    """Background scanning loop.

    Continuously captures screen, runs OCR, resolves signatures, and broadcasts results.
    """
    global scan_enabled

    logger.info("Scanning loop started")

    while scan_enabled:
        try:
            # Capture screen region
            img = capture.capture_region()

            if img is None:
                # Scanner not active or no region configured
                await asyncio.sleep(settings.scan_interval)
                continue

            # Run OCR
            detections = ocr.detect_numbers(img)

            # Get confirmed numbers (debounced)
            confirmed = ocr.get_confirmed_numbers()

            # Resolve to ores. Each reading maps to its single best ore — ore
            # base signatures are clustered (~15 RS apart), so a loose match
            # surfaces neighbours (e.g. 7080 = exact 2x Beryl but also near-2x
            # Taranite). The top match (exact beats fuzzy) is the real one.
            all_matches = []
            for number in confirmed:
                matches = resolver.resolve(number, ocr_confidence=1.0)
                if matches:
                    all_matches.append(matches[0])

            # Aggregate duplicates
            aggregated = resolver.aggregate_detections(all_matches)

            # Build result message
            ores_data = {}
            for ore_id, match in aggregated.items():
                ores_data[ore_id] = {
                    "name": match.ore.name,
                    "quantity": match.quantity,
                    "tier": match.ore.tier,
                    "tier_value": match.ore.tier_value,
                    "volatile": match.ore.volatile,
                    "confidence": round(match.confidence, 2),
                    "detected_rs": match.detected_rs
                }

            result = ScanResult(
                ores=ores_data,
                scanner_active=True,
                timestamp=asyncio.get_event_loop().time()
            )

            # Broadcast to all clients
            await manager.broadcast(result.model_dump())

        except Exception as e:
            logger.error(f"Error in scanning loop: {e}")

        # Wait before next scan
        await asyncio.sleep(settings.scan_interval)

    logger.info("Scanning loop stopped")


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Application lifespan manager."""
    logger.info("Starting SC Ore Scanner backend...")

    # Initialize modules
    settings = get_settings()
    app.state.settings = settings
    app.state.capture = ScreenCapture(settings)
    app.state.ocr = OCREngine(settings)
    app.state.resolver = RSResolver(settings)

    # Start scanning if region configured
    global scan_enabled, scanning_task
    if settings.scan_region:
        scan_enabled = True
        scanning_task = asyncio.create_task(
            scanning_loop(settings, app.state.capture, app.state.ocr, app.state.resolver)
        )
        logger.info("Auto-scan enabled")

    yield

    # Cleanup
    logger.info("Shutting down...")
    scan_enabled = False
    if scanning_task:
        scanning_task.cancel()
        try:
            await scanning_task
        except asyncio.CancelledError:
            pass

    app.state.capture.close()
    app.state.ocr.close()


def create_app() -> FastAPI:
    """Create FastAPI application.

    Returns:
        Configured FastAPI app
    """
    app = FastAPI(
        title="SC Ore Scanner",
        description="Real-time Star Citizen mining overlay backend",
        version="1.0.0",
        lifespan=lifespan
    )

    # CORS middleware
    app.add_middleware(
        CORSMiddleware,
        allow_origins=["*"],  # Tauri localhost
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )

    # WebSocket endpoint
    @app.websocket("/ws")
    async def websocket_endpoint(websocket: WebSocket):
        """WebSocket connection for real-time ore detection."""
        await manager.connect(websocket)

        try:
            while True:
                # Keep connection alive, receive client messages if needed
                data = await websocket.receive_text()

                # Handle client commands
                if data == "ping":
                    await websocket.send_json({"type": "pong"})

        except WebSocketDisconnect:
            manager.disconnect(websocket)

    # REST endpoints
    @app.get("/health")
    async def health_check():
        """Health check endpoint."""
        return {"status": "ok", "scanning": scan_enabled}

    @app.get("/config")
    async def get_config():
        """Get current configuration."""
        settings = app.state.settings
        return {
            "scan_region": settings.scan_region.model_dump() if settings.scan_region else None,
            "scan_interval": settings.scan_interval,
            "ocr": settings.ocr.model_dump(),
            "signature": settings.signature.model_dump(),
        }

    @app.post("/config/scan-region")
    async def set_scan_region(region: ScanRegion):
        """Configure scan region.

        Args:
            region: Screen region coordinates

        Returns:
            Success message
        """
        global scan_enabled, scanning_task

        settings = app.state.settings
        settings.scan_region = region
        settings.save_user_config()

        # Restart scanning
        if scanning_task:
            scan_enabled = False
            scanning_task.cancel()
            try:
                await scanning_task
            except asyncio.CancelledError:
                pass

        scan_enabled = True
        scanning_task = asyncio.create_task(
            scanning_loop(settings, app.state.capture, app.state.ocr, app.state.resolver)
        )

        logger.info(f"Scan region configured: {region}")
        return {"message": "Scan region configured", "region": region.model_dump()}

    @app.post("/scan/start")
    async def start_scanning():
        """Start scanning."""
        global scan_enabled, scanning_task

        if not app.state.settings.scan_region:
            raise HTTPException(400, "Scan region not configured")

        if scan_enabled:
            return {"message": "Scanning already active"}

        scan_enabled = True
        scanning_task = asyncio.create_task(
            scanning_loop(app.state.settings, app.state.capture, app.state.ocr, app.state.resolver)
        )

        logger.info("Scanning started")
        return {"message": "Scanning started"}

    @app.post("/scan/stop")
    async def stop_scanning():
        """Stop scanning."""
        global scan_enabled, scanning_task

        if not scan_enabled:
            return {"message": "Scanning already stopped"}

        scan_enabled = False
        if scanning_task:
            scanning_task.cancel()
            try:
                await scanning_task
            except asyncio.CancelledError:
                pass
            scanning_task = None

        # Reset OCR debouncing
        app.state.ocr.reset_debouncing()

        logger.info("Scanning stopped")
        return {"message": "Scanning stopped"}

    @app.get("/monitors")
    async def get_monitors():
        """Get available monitors."""
        return app.state.capture.get_screen_info()

    @app.get("/signatures")
    async def get_signatures():
        """Get all ore signatures."""
        resolver = app.state.resolver
        return {
            "signatures": [ore.model_dump() for ore in resolver.signatures.values()]
        }

    return app
