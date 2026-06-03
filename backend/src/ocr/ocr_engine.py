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
    - CLAHE preprocessing for better contrast
    - EasyOCR with digit-only detection
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
        """Initialize EasyOCR (lazy loading)."""
        if self._initialized:
            return

        logger.info("Initializing EasyOCR...")

        try:
            import easyocr
            self.reader = easyocr.Reader(
                ['en'],
                gpu=self.settings.ocr.gpu_enabled,
                verbose=False
            )
            self._initialized = True
            logger.info("EasyOCR initialized successfully")

        except Exception as e:
            logger.error(f"Failed to initialize EasyOCR: {e}")
            raise

    def preprocess_image(self, img: Image.Image) -> np.ndarray:
        """Preprocess a scan-region image for OCR.

        The Star Citizen RS readout is bright teal digits (with a thousands
        comma and a location-pin glyph) on a dark, particle-flecked background.
        The goal is to maximize digit contrast while stripping the comma, the
        pin, and floating particles -- WITHOUT destroying the thin digit strokes.

        Pipeline:
        1. Convert to RGB
        2. Upscale (LANCZOS) for better small-text OCR
        3. Convert to grayscale
        4. CLAHE + min-max normalize to boost digit contrast
        5. Build a "digit-like" mask from connected components, keeping only
           tall, sufficiently-large blobs (drops the short comma + tiny specks)
        6. Apply the mask to the contrast-enhanced grayscale (strokes intact)

        The pin glyph survives as a single component but reads as a lone digit,
        which the 3-6 digit regex in detect_numbers() discards.

        Args:
            img: Input PIL Image (the captured scan region)

        Returns:
            Preprocessed image as a numpy array (RGB, suitable for EasyOCR)
        """
        config = self.settings.ocr

        # Ensure RGB
        if img.mode != 'RGB':
            img = img.convert('RGB')

        img_array = np.array(img)

        # Upscale for better OCR. cv2's LANCZOS4 preserves the digit edges more
        # faithfully than PIL's resampler here -- enough to keep borderline
        # glyphs (e.g. 6 vs 8) from flipping.
        if config.upscale_factor > 1:
            f = config.upscale_factor
            img_array = cv2.resize(
                img_array,
                (img_array.shape[1] * f, img_array.shape[0] * f),
                interpolation=cv2.INTER_LANCZOS4,
            )

        # Grayscale
        gray = cv2.cvtColor(img_array, cv2.COLOR_RGB2GRAY)

        # Boost contrast: CLAHE then stretch the histogram to the full range.
        clahe = cv2.createCLAHE(
            clipLimit=config.clahe_clip_limit,
            tileGridSize=config.clahe_grid_size
        )
        gray = clahe.apply(gray)
        gray = cv2.normalize(gray, None, 0, 255, cv2.NORM_MINMAX)

        # Strip non-digit noise (comma, pin remnants, particles) while keeping
        # the grayscale digit shapes intact.
        gray = self._mask_digit_components(gray)

        # EasyOCR expects an RGB-shaped array.
        return cv2.cvtColor(gray, cv2.COLOR_GRAY2RGB)

    def _mask_digit_components(self, gray: np.ndarray) -> np.ndarray:
        """Zero out everything that doesn't look like a digit stroke.

        Otsu-thresholds the (already contrast-boosted) grayscale to find bright
        blobs, then keeps only those that are tall enough relative to the
        tallest blob (digits) and large enough in area. The short thousands
        comma and small floating particles fall below the height/area cutoffs
        and are removed. The mask is applied back to the grayscale image so the
        surviving digits retain their anti-aliased shape.

        Args:
            gray: Contrast-enhanced grayscale image

        Returns:
            Grayscale image with non-digit components zeroed out
        """
        config = self.settings.ocr

        _, binary = cv2.threshold(gray, 0, 255, cv2.THRESH_BINARY + cv2.THRESH_OTSU)

        num_labels, labels, stats, _ = cv2.connectedComponentsWithStats(binary, connectivity=8)
        if num_labels <= 1:
            return gray  # nothing bright found

        max_height = max(stats[i, cv2.CC_STAT_HEIGHT] for i in range(1, num_labels))
        height_cutoff = config.min_component_height_frac * max_height

        mask = np.zeros(binary.shape, dtype=np.uint8)
        for i in range(1, num_labels):  # skip background
            height = stats[i, cv2.CC_STAT_HEIGHT]
            area = stats[i, cv2.CC_STAT_AREA]
            if height >= height_cutoff and area >= config.min_component_area:
                mask[labels == i] = 255

        return cv2.bitwise_and(gray, gray, mask=mask)

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

        # Run OCR
        try:
            results = self.reader.readtext(
                processed,
                allowlist=self.settings.ocr.allowlist,
                paragraph=False,
                detail=1  # Return bbox + confidence
            )
        except Exception as e:
            logger.error(f"OCR detection failed: {e}")
            return []

        # Extract numbers
        detections = []
        for detection in results:
            if len(detection) >= 3:
                bbox_points = detection[0]  # [[x1,y1], [x2,y2], [x3,y3], [x4,y4]]
                text = detection[1]
                confidence = detection[2]

                # Filter by confidence threshold
                if confidence < self.settings.ocr.confidence_threshold:
                    continue

                # Extract all 3-6 digit numbers from text
                matches = re.findall(r'\d{3,6}', text)
                for match in matches:
                    num = int(match)

                    # Valid signature range
                    sig_config = self.settings.signature
                    if sig_config.valid_rs_min <= num <= sig_config.valid_rs_max:
                        # Calculate bounding box
                        bbox = self._calculate_bbox(bbox_points)

                        detections.append(OCRResult(
                            number=num,
                            confidence=confidence,
                            bbox=bbox
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
