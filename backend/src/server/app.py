"""FastAPI application with WebSocket support."""

import asyncio
import logging
import os
from contextlib import asynccontextmanager
from typing import Dict, Set
from urllib.parse import urlparse

from fastapi import FastAPI, WebSocket, WebSocketDisconnect, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse, PlainTextResponse
from pydantic import BaseModel

# Hosts allowed to make mutating requests (CSRF guard). The Tauri production
# webview's origin is http://tauri.localhost; dev runs on localhost.
_TRUSTED_ORIGIN_HOSTS = {"localhost", "127.0.0.1", "tauri.localhost"}


def _origin_is_trusted(origin: str) -> bool:
    """True if an HTTP Origin header belongs to the app (not a foreign website)."""
    try:
        parsed = urlparse(origin)
    except ValueError:
        return False
    if parsed.scheme == "tauri":  # tauri://localhost (non-Windows)
        return True
    return (parsed.hostname or "") in _TRUSTED_ORIGIN_HOSTS

from ..config import Settings, ScanRegion, get_settings
from ..capture import ScreenCapture
from ..ocr import OCREngine
from ..resolver import RSResolver
from ..stats import SessionStats
from ..prices import PriceCache

logger = logging.getLogger(__name__)


class ScanResult(BaseModel):
    """Scan result sent to frontend."""
    ores: Dict[str, dict]  # ore_id -> {name, quantity, tier, confidence}
    scanner_active: bool
    timestamp: float
    session: dict = {}  # compact session summary (distinct_ores, total_detections)


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
session_stats = SessionStats()  # per-session ore detection stats
price_cache = None              # PriceCache, set up in lifespan
price_task = None               # background price-refresh task
scanning_task = None
scan_enabled = False


async def price_refresh_loop(cache: PriceCache):
    """Periodically refresh the ore price cache from the feed."""
    while True:
        await cache.refresh()
        await asyncio.sleep(cache.refresh_seconds)


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

            # Observability: log what the OCR read and what it resolved to.
            if detections:
                logger.info(
                    "OCR raw=%s confirmed=%s -> %s",
                    [(d.number, round(d.confidence, 2)) for d in detections],
                    confirmed,
                    {oid: f"{m.quantity}x {m.ore.name} ({round(m.confidence, 2)})"
                     for oid, m in aggregated.items()},
                )

            # Update session statistics
            session_stats.record(aggregated)

            # Build result message
            ores_data = {}
            for ore_id, match in aggregated.items():
                unit_price = price_cache.sell_price(ore_id) if price_cache else None
                ores_data[ore_id] = {
                    "name": match.ore.name,
                    "quantity": match.quantity,
                    "tier": match.ore.tier,
                    "tier_value": match.ore.tier_value,
                    "volatile": match.ore.volatile,
                    "confidence": round(match.confidence, 2),
                    "detected_rs": match.detected_rs,
                    "unit_price": unit_price,
                    "value": unit_price * match.quantity if unit_price else None,
                }

            result = ScanResult(
                ores=ores_data,
                scanner_active=True,
                timestamp=asyncio.get_event_loop().time(),
                session={
                    "distinct_ores": len(session_stats.ores),
                    "total_detections": session_stats.total_detections,
                },
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

    # Ore price cache (UEX feed). Refresh in the background so we never block.
    global price_cache, price_task
    if settings.prices.enabled:
        price_cache = PriceCache(
            settings.prices.feed_url,
            refresh_seconds=settings.prices.refresh_minutes * 60,
        )
        price_task = asyncio.create_task(price_refresh_loop(price_cache))

    # Start scanning if region configured
    global scan_enabled, scanning_task
    if settings.scan_region:
        # Warm up the OCR engine before we start serving, so the first real scan
        # is responsive. Blocking here keeps the overlay in its "starting up"
        # (OFFLINE) state during the ~15-20s first load, instead of connecting
        # and then sitting idle — clearer for the user.
        try:
            from PIL import Image
            logger.info("Loading OCR engine - first start can take ~15-20s...")
            app.state.ocr.initialize()
            app.state.ocr.detect_numbers(Image.new("RGB", (200, 80)))  # warm the model
            app.state.ocr.reset_debouncing()
            logger.info("OCR engine ready.")
        except Exception as e:
            logger.error(f"OCR warmup failed (will retry lazily): {e}")

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

    if price_task:
        price_task.cancel()
        try:
            await price_task
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
        version="1.4.1",
        lifespan=lifespan
    )

    # CORS: read endpoints are open, but NO credentials (the previous
    # allow_origins="*" + allow_credentials=True combo is invalid and is rejected
    # by browsers). The CSRF guard below is what actually protects mutating calls.
    app.add_middleware(
        CORSMiddleware,
        allow_origins=["*"],
        allow_credentials=False,
        allow_methods=["GET", "POST", "OPTIONS"],
        allow_headers=["*"],
    )

    # CSRF guard: the API binds to localhost, but a web page the user visits could
    # still fire a simple cross-origin POST at it (e.g. /shutdown, /config/scan-region).
    # Block mutating requests that carry a foreign web Origin; allow the Tauri webview
    # (`http://tauri.localhost` / `tauri://localhost`), localhost dev, and non-browser
    # callers (no Origin header).
    @app.middleware("http")
    async def csrf_guard(request, call_next):
        if request.method in ("POST", "PUT", "PATCH", "DELETE"):
            origin = request.headers.get("origin")
            if origin and not _origin_is_trusted(origin):
                logger.warning(f"Blocked cross-origin {request.method} from origin {origin!r}")
                return JSONResponse(status_code=403, content={"detail": "Cross-origin request blocked"})
        return await call_next(request)

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

    @app.post("/shutdown")
    async def shutdown():
        """Stop the backend (called by the overlay's close button).

        The backend runs windowless, so closing the overlay must also stop it.
        Prefer a graceful uvicorn shutdown (runs lifespan teardown — closes the
        capture/OCR resources); fall back to a hard exit if it doesn't stop.
        """
        logger.info("Shutdown requested by client — stopping.")

        server = getattr(app.state, "server", None)
        if server is not None:
            server.should_exit = True

        # Backstop: if graceful shutdown hasn't taken effect shortly, force it so
        # we never leave an invisible backend running.
        asyncio.get_event_loop().call_later(3.0, lambda: os._exit(0))
        return {"message": "shutting down"}

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

    @app.get("/prices")
    async def get_prices():
        """Cached ore prices (UEX Corp data). Empty if disabled or not yet loaded."""
        if price_cache is None:
            return {"enabled": False, "prices": {}}
        return {"enabled": True, **price_cache.summary()}

    @app.get("/stats")
    async def get_stats():
        """Session statistics: per-ore counts, totals, and timing."""
        return session_stats.summary()

    @app.get("/stats/export.csv")
    async def export_stats():
        """Session statistics as a downloadable CSV."""
        return PlainTextResponse(
            session_stats.to_csv(),
            media_type="text/csv",
            headers={"Content-Disposition": "attachment; filename=sc-ore-scanner-session.csv"},
        )

    @app.post("/stats/reset")
    async def reset_stats():
        """Clear the current session's statistics."""
        session_stats.reset()
        return {"message": "Session stats reset"}

    return app
