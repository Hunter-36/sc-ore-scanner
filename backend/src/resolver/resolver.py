"""RS signature resolver - matches detected numbers to ore types."""

import json
import logging
from typing import Dict, List, Optional
from pydantic import BaseModel

from ..config import Settings

logger = logging.getLogger(__name__)


class OreSignature(BaseModel):
    """Ore signature definition."""
    id: str
    name: str
    base_rs: int
    tier: str
    tier_value: int
    volatile: bool
    context: List[str]
    notes: str = ""


class OreMatch(BaseModel):
    """Matched ore detection."""
    ore: OreSignature
    quantity: int
    detected_rs: int
    confidence: float
    error_margin: int = 0


class RSResolver:
    """Resolves detected RS numbers to ore types.

    Uses division math: detected_rs = base_rs × quantity
    Example: 10620 = 3540 × 3 → 3x Beryl
    """

    def __init__(self, settings: Settings):
        """Initialize resolver.

        Args:
            settings: Application settings
        """
        self.settings = settings
        self.signatures: Dict[int, OreSignature] = {}
        self.signatures_by_id: Dict[str, OreSignature] = {}

        # Load signatures database
        self._load_signatures()

    def _load_signatures(self):
        """Load ore signatures from JSON database."""
        sig_path = self.settings.signatures_path

        if not sig_path.exists():
            logger.error(f"Signatures database not found: {sig_path}")
            return

        try:
            with open(sig_path, 'r') as f:
                data = json.load(f)

            for ore_data in data.get('ores', []):
                ore = OreSignature(**ore_data)
                self.signatures[ore.base_rs] = ore
                self.signatures_by_id[ore.id] = ore

            logger.info(f"Loaded {len(self.signatures)} ore signatures")

        except Exception as e:
            logger.error(f"Failed to load signatures: {e}")

    def resolve(self, detected_rs: int, ocr_confidence: float = 1.0) -> List[OreMatch]:
        """Resolve detected RS number to ore matches.

        Tries multiple strategies:
        1. Exact division match (e.g., 10620 / 3540 = 3)
        2. Fuzzy match with error tolerance
        3. OCR error correction (digit removal/addition)

        Args:
            detected_rs: Detected RS number
            ocr_confidence: OCR confidence score (0-1)

        Returns:
            List of possible ore matches, sorted by confidence
        """
        matches = []

        # Strategy 1: Try exact division match
        matches.extend(self._try_division_match(detected_rs, ocr_confidence))

        # Strategy 2: Try OCR error correction (5-6 digit numbers)
        if len(str(detected_rs)) in [5, 6]:
            matches.extend(self._try_ocr_correction(detected_rs, ocr_confidence))

        # Sort by confidence (descending)
        matches.sort(key=lambda m: m.confidence, reverse=True)

        return matches

    def _try_division_match(self, detected_rs: int, ocr_confidence: float) -> List[OreMatch]:
        """Try matching by dividing detected RS by known signatures.

        Args:
            detected_rs: Detected RS number
            ocr_confidence: OCR confidence

        Returns:
            List of matches
        """
        matches = []
        config = self.settings.signature

        for base_rs, ore in self.signatures.items():
            # Calculate quantity
            quantity = detected_rs / base_rs

            # Valid if quantity is in range [1, max_quantity]
            if config.min_quantity <= quantity <= config.max_quantity:
                remainder = detected_rs % base_rs
                error_margin = min(config.max_error_margin, base_rs * config.error_margin_percent)

                # Check if division is close to whole number
                if remainder == 0 or remainder <= error_margin or (base_rs - remainder) <= error_margin:
                    final_quantity = round(quantity)
                    actual_error = abs(detected_rs - (base_rs * final_quantity))

                    # Calculate confidence score
                    # Factors: OCR confidence, error margin, quantity validity
                    error_penalty = 1.0 - (actual_error / (base_rs * 0.1))  # 10% max penalty
                    error_penalty = max(0.0, min(1.0, error_penalty))

                    confidence = ocr_confidence * error_penalty

                    matches.append(OreMatch(
                        ore=ore,
                        quantity=final_quantity,
                        detected_rs=detected_rs,
                        confidence=confidence,
                        error_margin=actual_error
                    ))

        return matches

    def _try_ocr_correction(self, detected_rs: int, ocr_confidence: float) -> List[OreMatch]:
        """Try OCR error correction by removing/adding digits.

        Common OCR errors:
        - Extra digit: 105620 → 10620 (5-digit → 5-digit split)
        - Missing digit: 1062 → 10620 (4-digit → add 0)

        Args:
            detected_rs: Detected RS number
            ocr_confidence: OCR confidence

        Returns:
            List of corrected matches
        """
        matches = []
        detected_str = str(detected_rs)

        # Try removing each digit
        for i in range(len(detected_str)):
            candidate_str = detected_str[:i] + detected_str[i+1:]

            # Must result in valid 4-5 digit number
            if len(candidate_str) in [4, 5]:
                try:
                    candidate_rs = int(candidate_str)

                    # Apply reduced confidence for corrections
                    corrected_confidence = ocr_confidence * 0.8

                    # Try matching corrected number
                    candidate_matches = self._try_division_match(candidate_rs, corrected_confidence)
                    matches.extend(candidate_matches)

                except ValueError:
                    pass

        # Try splitting 5-digit into quantity + 4-digit signature
        if len(detected_str) == 5:
            for split_pos in [1, 2]:
                quantity_str = detected_str[:split_pos]
                sig_str = detected_str[split_pos:]

                if len(sig_str) == 4:
                    try:
                        quantity = int(quantity_str)
                        base_rs = int(sig_str)

                        # Check if signature exists exactly
                        if base_rs in self.signatures:
                            ore = self.signatures[base_rs]
                            config = self.settings.signature

                            if config.min_quantity <= quantity <= config.max_quantity:
                                matches.append(OreMatch(
                                    ore=ore,
                                    quantity=quantity,
                                    detected_rs=detected_rs,
                                    confidence=ocr_confidence * 0.9,  # High confidence for exact sig match
                                    error_margin=0
                                ))

                    except ValueError:
                        pass

        return matches

    def aggregate_detections(self, matches: List[OreMatch]) -> Dict[str, OreMatch]:
        """Aggregate multiple detections of the same ore.

        Args:
            matches: List of ore matches

        Returns:
            Dict mapping ore_id to best match (highest confidence)
        """
        aggregated = {}

        for match in matches:
            ore_id = match.ore.id

            if ore_id not in aggregated or match.confidence > aggregated[ore_id].confidence:
                aggregated[ore_id] = match

        return aggregated

    def get_ore_by_id(self, ore_id: str) -> Optional[OreSignature]:
        """Get ore signature by ID.

        Args:
            ore_id: Ore identifier

        Returns:
            Ore signature or None
        """
        return self.signatures_by_id.get(ore_id)
