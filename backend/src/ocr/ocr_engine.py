"""OCR engine with CLAHE preprocessing and debouncing."""

import logging
import re
from collections import defaultdict, deque
from typing import List, Optional, Tuple
import numpy as np
import cv2
from PIL import Image
from pydantic import BaseModel

from ..config import Settings

logger = logging.getLogger(__name__)


class OCRResult(BaseModel):
    """OCR detection result."""
    number: int
    confidence: float
    bbox: Optional[Tuple[int, int, int, int]] = None  # (x, y, width, height)


class OCREngine:
    """OCR engine for detecting RS signature numbers.

    Features:
    - CLAHE contrast preprocessing
    - RapidOCR (ONNX) text detection -- lightweight, no PyTorch
    - Confidence scoring
    - Debouncing (N consecutive frames required)
    """

    def __init__(self, settings: Settings):
        """Initialize OCR engine.

        Args:
            settings: Application settings
        """
        self.settings = settings
        self.reader = None  # Lazy load
        self._initialized = False

        # Debouncing state: {number: deque of recent detections}
        self._detection_history: dict[int, deque] = defaultdict(lambda: deque(maxlen=10))
        self._confirmed_numbers: set[int] = set()

    def initialize(self):
        """Initialize RapidOCR (lazy loading).

        RapidOCR ships its ONNX detection/recognition models inside the wheel,
        so there's no runtime model download and no PyTorch dependency.
        """
        if self._initialized:
            return

        logger.info("Initializing RapidOCR...")

        try:
            from rapidocr_onnxruntime import RapidOCR
            self.reader = RapidOCR()
            self._initialized = True
            logger.info("RapidOCR initialized successfully")

        except Exception as e:
            logger.error(f"Failed to initialize RapidOCR: {e}")
            raise

    def preprocess_image(self, img: Image.Image) -> np.ndarray:
        """Preprocess a scan-region image for OCR.

        The Star Citizen RS readout is bright teal digits (with a thousands
        comma and a location-pin glyph) on a dark, particle-flecked background.
        RapidOCR handles the comma and stylized glyphs natively, so we only need
        to boost contrast and scale the small text up -- no aggressive masking.

        Pipeline:
        1. Convert to RGB
        2. Upscale with cv2 LANCZOS4 for better small-text OCR (cv2 preserves
           digit edges better than PIL -- keeps borderline glyphs like 6 vs 8
           from flipping)
        3. Convert to grayscale
        4. CLAHE to boost digit contrast against the dark background

        The thousands comma is stripped downstream (digits-only extraction in
        detect_numbers); the location pin reads as a separate lone digit, which
        the length filter discards.

        Args:
            img: Input PIL Image (the captured scan region)

        Returns:
            Preprocessed image as a numpy array (RGB, suitable for RapidOCR)
        """
        config = self.settings.ocr

        # Ensure RGB
        if img.mode != 'RGB':
            img = img.convert('RGB')

        img_array = np.array(img)

        # Upscale for better small-text OCR.
        if config.upscale_factor > 1:
            f = config.upscale_factor
            img_array = cv2.resize(
                img_array,
                (img_array.shape[1] * f, img_array.shape[0] * f),
                interpolation=cv2.INTER_LANCZOS4,
            )

        # Grayscale + CLAHE contrast boost.
        gray = cv2.cvtColor(img_array, cv2.COLOR_RGB2GRAY)
        clahe = cv2.createCLAHE(
            clipLimit=config.clahe_clip_limit,
            tileGridSize=config.clahe_grid_size
        )
        gray = clahe.apply(gray)

        # RapidOCR expects an RGB-shaped array.
        return cv2.cvtColor(gray, cv2.COLOR_GRAY2RGB)

    def detect_numbers(self, img: Image.Image) -> List[OCRResult]:
        """Detect RS signature numbers in image.

        Args:
            img: Screenshot image

        Returns:
            List of detected numbers with confidence scores
        """
        # Initialize OCR if needed
        if not self._initialized:
            self.initialize()

        # Preprocess image
        processed = self.preprocess_image(img)

        # Run OCR. RapidOCR returns (results, elapse) where results is a list of
        # [box_points, text, score] (or None when nothing is found).
        try:
            results, _ = self.reader(processed)
        except Exception as e:
            logger.error(f"OCR detection failed: {e}")
            return []

        # Extract numbers
        detections = []
        sig_config = self.settings.signature
        for box_points, text, score in (results or []):
            confidence = float(score)

            # Filter by confidence threshold
            if confidence < self.settings.ocr.confidence_threshold:
                continue

            # Strip the thousands comma (and any stray punctuation/space) so a
            # read like "10,620" becomes "10620".
            digits = re.sub(r"[^0-9]", "", text)
            if len(digits) not in (3, 4, 5, 6):
                continue

            num = int(digits)

            # Valid signature range
            if sig_config.valid_rs_min <= num <= sig_config.valid_rs_max:
                detections.append(OCRResult(
                    number=num,
                    confidence=confidence,
                    bbox=self._calculate_bbox(box_points)
                ))

        # Update debouncing state
        self._update_debouncing(detections)

        return detections

    def _calculate_bbox(self, points: List[List[float]]) -> Tuple[int, int, int, int]:
        """Calculate bounding box from corner points.

        Args:
            points: List of [x, y] corner coordinates

        Returns:
            (x, y, width, height) tuple
        """
        xs = [p[0] for p in points]
        ys = [p[1] for p in points]

        x = int(min(xs))
        y = int(min(ys))
        width = int(max(xs) - x)
        height = int(max(ys) - y)

        return (x, y, width, height)

    def _update_debouncing(self, detections: List[OCRResult]):
        """Update debouncing state for detected numbers.

        Args:
            detections: Current frame detections
        """
        detected_numbers = {d.number for d in detections}

        # Add current detections to history
        for num in detected_numbers:
            self._detection_history[num].append(True)

        # Mark absent numbers
        for num in list(self._detection_history.keys()):
            if num not in detected_numbers:
                self._detection_history[num].append(False)

                # Clean up old entries
                if len(self._detection_history[num]) >= 10 and not any(self._detection_history[num]):
                    del self._detection_history[num]
                    self._confirmed_numbers.discard(num)

    def get_confirmed_numbers(self) -> List[int]:
        """Get numbers confirmed by debouncing.

        Returns:
            List of numbers detected in N consecutive frames
        """
        min_frames = self.settings.ocr.min_consecutive_frames
        confirmed = []

        for num, history in self._detection_history.items():
            # Check last N frames
            recent = list(history)[-min_frames:]

            if len(recent) >= min_frames and all(recent):
                confirmed.append(num)
                self._confirmed_numbers.add(num)

        return confirmed

    def reset_debouncing(self):
        """Reset debouncing state (e.g., when scanner becomes inactive)."""
        self._detection_history.clear()
        self._confirmed_numbers.clear()

    def close(self):
        """Release OCR resources."""
        self.reader = None
        self._initialized = False
