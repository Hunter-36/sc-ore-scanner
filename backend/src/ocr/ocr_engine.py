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
        """Preprocess image for OCR with CLAHE.

        Pipeline:
        1. Convert to RGB
        2. Upscale for better OCR
        3. Convert to grayscale
        4. Apply CLAHE (Contrast Limited Adaptive Histogram Equalization)
        5. Adaptive thresholding
        6. Morphological noise removal
        7. Remove small components

        Args:
            img: Input PIL Image

        Returns:
            Preprocessed image as numpy array (RGB)
        """
        config = self.settings.ocr

        # Ensure RGB
        if img.mode != 'RGB':
            img = img.convert('RGB')

        # Upscale for better OCR
        if config.upscale_factor > 1:
            new_size = (
                img.width * config.upscale_factor,
                img.height * config.upscale_factor
            )
            img = img.resize(new_size, Image.Resampling.LANCZOS)

        img_array = np.array(img)

        # Convert to grayscale
        gray = cv2.cvtColor(img_array, cv2.COLOR_RGB2GRAY)

        # Apply CLAHE for better contrast
        clahe = cv2.createCLAHE(
            clipLimit=config.clahe_clip_limit,
            tileGridSize=config.clahe_grid_size
        )
        gray = clahe.apply(gray)

        # Adaptive thresholding (bright text on dark background)
        thresh = cv2.adaptiveThreshold(
            gray, 255,
            cv2.ADAPTIVE_THRESH_GAUSSIAN_C,
            cv2.THRESH_BINARY,
            11, 2
        )

        # Morphological opening to remove noise
        kernel = np.ones((2, 2), np.uint8)
        thresh = cv2.morphologyEx(thresh, cv2.MORPH_OPEN, kernel)

        # Convert back to RGB for EasyOCR
        img_array = cv2.cvtColor(thresh, cv2.COLOR_GRAY2RGB)

        # Remove small components (commas, periods, noise)
        img_array = self._remove_small_components(img_array)

        return img_array

    def _remove_small_components(self, img_array: np.ndarray) -> np.ndarray:
        """Remove small connected components (punctuation, noise).

        Args:
            img_array: Input image (RGB)

        Returns:
            Cleaned image
        """
        config = self.settings.ocr

        # Convert to grayscale
        gray = cv2.cvtColor(img_array, cv2.COLOR_RGB2GRAY)

        # Binary threshold
        _, binary = cv2.threshold(gray, 0, 255, cv2.THRESH_BINARY_INV + cv2.THRESH_OTSU)

        # Find connected components
        num_labels, labels, stats, _ = cv2.connectedComponentsWithStats(binary, connectivity=8)

        # Create mask of components to keep
        mask = np.zeros(binary.shape, dtype=np.uint8)

        for i in range(1, num_labels):  # Skip background
            area = stats[i, cv2.CC_STAT_AREA]
            if area >= config.min_component_area:
                mask[labels == i] = 255

        # Estimate background color
        bg_mask = binary == 0
        if np.any(bg_mask):
            bg_color = np.median(img_array[bg_mask], axis=0).astype(np.uint8)
        else:
            bg_color = np.array([128, 128, 128], dtype=np.uint8)

        # Apply mask
        result = img_array.copy()
        removed_pixels = (binary == 255) & (mask == 0)
        result[removed_pixels] = bg_color

        return result

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
