"""Screen capture with scan-state gating."""

import logging
from typing import Optional, Tuple
import numpy as np
from PIL import Image
import mss

from ..config import Settings

logger = logging.getLogger(__name__)


class ScreenCapture:
    """Screen capture service with scan-state gating.

    Captures screen regions using mss and detects if the scanner HUD is active
    by checking specific pixel colors.
    """

    def __init__(self, settings: Settings):
        """Initialize screen capture.

        Args:
            settings: Application settings
        """
        self.settings = settings
        self.sct = mss.mss()
        self._last_gate_status = False

    def capture_region(self, force: bool = False) -> Optional[Image.Image]:
        """Capture configured scan region.

        Args:
            force: Skip scan-state gating and capture anyway

        Returns:
            PIL Image if scan region configured and (force or scanner active), else None
        """
        if not self.settings.scan_region:
            logger.warning("No scan region configured")
            return None

        region = self.settings.scan_region

        # Define monitor region for mss
        monitor = {
            "left": region.x,
            "top": region.y,
            "width": region.width,
            "height": region.height
        }

        try:
            # Capture screenshot
            screenshot = self.sct.grab(monitor)

            # Convert to PIL Image
            img = Image.frombytes("RGB", screenshot.size, screenshot.rgb)

            # Check scan-state gating (unless forced)
            if not force and self.settings.scan_gating.enabled:
                if not self._is_scanner_active(img):
                    # Only log state changes to reduce spam
                    if self._last_gate_status:
                        logger.debug("Scanner HUD no longer detected - skipping OCR")
                        self._last_gate_status = False
                    return None
                else:
                    if not self._last_gate_status:
                        logger.debug("Scanner HUD detected - starting OCR")
                        self._last_gate_status = True

            return img

        except Exception as e:
            logger.error(f"Screen capture failed: {e}")
            return None

    def _is_scanner_active(self, img: Image.Image) -> bool:
        """Detect if scanner HUD is active by checking pixel colors.

        The Star Citizen scanner displays bright white/cyan UI elements when active.
        We sample specific pixels and check if they match the expected color range.

        Args:
            img: Captured screen region

        Returns:
            True if scanner appears active
        """
        config = self.settings.scan_gating

        # Convert to numpy for fast pixel access
        img_array = np.array(img)
        height, width = img_array.shape[:2]

        active_points = 0

        for x, y in config.sample_points:
            # Ensure coordinates are within bounds
            if x < 0 or x >= width or y < 0 or y >= height:
                logger.warning(f"Sample point ({x}, {y}) out of bounds ({width}x{height})")
                continue

            # Get pixel RGB values
            r, g, b = img_array[y, x][:3]

            # Check if pixel matches scanner UI colors (bright white/cyan)
            if (r >= config.color_threshold_r and
                g >= config.color_threshold_g and
                b >= config.color_threshold_b):
                active_points += 1

        # Scanner is active if enough points match
        is_active = active_points >= config.min_active_points

        return is_active

    def get_screen_info(self) -> dict:
        """Get information about available monitors.

        Returns:
            Dict with monitor information
        """
        monitors = []
        for idx, monitor in enumerate(self.sct.monitors[1:], start=1):  # Skip "all monitors"
            monitors.append({
                "id": idx,
                "left": monitor["left"],
                "top": monitor["top"],
                "width": monitor["width"],
                "height": monitor["height"]
            })

        return {
            "monitors": monitors,
            "primary": monitors[0] if monitors else None
        }

    def close(self):
        """Release screen capture resources."""
        if self.sct:
            self.sct.close()

    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.close()
