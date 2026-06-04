"""Cached Star Citizen ore prices, read from the published UEX price feed.

The feed (a small prices.json on GitHub Pages) is produced hourly by the prices
workflow from UEX Corp's public data. This cache fetches it on a timer so we
never hit UEX per scan; if a fetch fails we keep the last good values.

Price data: UEX Corp (https://uexcorp.space).
"""

import asyncio
import json
import logging
import time
import urllib.request
from typing import Dict, Optional

logger = logging.getLogger(__name__)


class PriceCache:
    """Holds the latest ore prices fetched from the feed URL."""

    def __init__(self, url: str, refresh_seconds: int = 3600):
        self.url = url
        self.refresh_seconds = refresh_seconds
        self.prices: Dict[str, dict] = {}   # ore_id -> {name, sell, buy}
        self.updated_at: Optional[int] = None
        self.last_fetch: float = 0.0

    def _apply(self, data: dict) -> None:
        """Apply a parsed feed payload to the cache (separated for testing)."""
        self.prices = data.get("prices", {}) or {}
        self.updated_at = data.get("updated_at")
        self.last_fetch = time.time()

    def _fetch(self) -> None:
        req = urllib.request.Request(self.url, headers={"User-Agent": "sc-ore-scanner"})
        with urllib.request.urlopen(req, timeout=15) as resp:
            self._apply(json.load(resp))
        logger.info(f"Loaded {len(self.prices)} ore prices from {self.url}")

    async def refresh(self) -> None:
        """Refresh the cache; on failure keep the previously cached prices."""
        try:
            await asyncio.to_thread(self._fetch)
        except Exception as e:
            logger.warning(f"Price refresh failed (keeping cached prices): {e}")

    def sell_price(self, ore_id: str) -> Optional[int]:
        entry = self.prices.get(ore_id)
        return entry.get("sell") if entry else None

    def value_of(self, ore_id: str, quantity: int) -> Optional[int]:
        sell = self.sell_price(ore_id)
        return sell * quantity if sell else None

    def summary(self) -> dict:
        return {
            "url": self.url,
            "updated_at": self.updated_at,
            "count": len(self.prices),
            "prices": self.prices,
        }
