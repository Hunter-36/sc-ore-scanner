"""Application settings and configuration management."""

import json
from pathlib import Path
from typing import Optional, Tuple
from pydantic import BaseModel, Field, field_validator
from pydantic_settings import BaseSettings


class ScanRegion(BaseModel):
    """Screen region to scan for RS signatures."""
    x: int = Field(ge=0, description="X coordinate of top-left corner")
    y: int = Field(ge=0, description="Y coordinate of top-left corner")
    width: int = Field(gt=0, description="Width of scan region")
    height: int = Field(gt=0, description="Height of scan region")


class OCRConfig(BaseModel):
    """OCR engine configuration."""
    confidence_threshold: float = Field(default=0.8, ge=0.0, le=1.0, description="Minimum OCR confidence (0-1)")
    min_consecutive_frames: int = Field(default=3, ge=1, description="Frames required for confirmation")

    # CLAHE preprocessing parameters
    clahe_clip_limit: float = Field(default=2.0, ge=0.0, description="CLAHE contrast limit")
    clahe_grid_size: Tuple[int, int] = Field(default=(8, 8), description="CLAHE tile grid size")

    # Image preprocessing
    upscale_factor: int = Field(default=4, ge=1, le=5, description="Image upscale multiplier")


class ScanGatingConfig(BaseModel):
    """Scanner HUD detection configuration.

    Off by default: the corner-pixel sampling is fragile and tends to suppress
    capture entirely on a normally-calibrated region. OCR over the (calibrated)
    region is cheap, so we just run it every scan interval.
    """
    enabled: bool = Field(default=False, description="Enable scan-state gating")

    # Scanner HUD color detection (detect white/cyan scanner UI)
    sample_points: list[Tuple[int, int]] = Field(
        default=[(10, 10), (20, 20), (30, 30)],
        description="Relative pixel coordinates to check (within scan region)"
    )
    color_threshold_r: int = Field(default=180, ge=0, le=255, description="Min R value for scanner UI")
    color_threshold_g: int = Field(default=180, ge=0, le=255, description="Min G value for scanner UI")
    color_threshold_b: int = Field(default=200, ge=0, le=255, description="Min B value for scanner UI")

    min_active_points: int = Field(default=1, ge=1, description="Minimum sample points that must match")


class SignatureConfig(BaseModel):
    """RS signature matching configuration."""
    max_error_margin: int = Field(default=50, ge=0, description="Max RS value error tolerance")
    error_margin_percent: float = Field(default=0.01, ge=0.0, le=1.0, description="Max error as percentage (1%)")

    min_quantity: int = Field(default=1, ge=1, description="Minimum node quantity")
    max_quantity: int = Field(default=10, ge=1, description="Maximum node quantity")

    valid_rs_min: int = Field(default=100, description="Minimum valid RS value")
    valid_rs_max: int = Field(default=200000, description="Maximum valid RS value")


class ServerConfig(BaseModel):
    """FastAPI server configuration."""
    host: str = Field(default="127.0.0.1", description="Server bind address")
    port: int = Field(default=8765, ge=1024, le=65535, description="WebSocket port")
    log_level: str = Field(default="INFO", description="Logging level")


class PriceConfig(BaseModel):
    """Ore price feed (UEX Corp data, served via GitHub Pages)."""
    enabled: bool = Field(default=True, description="Show estimated ore values")
    feed_url: str = Field(
        default="https://hunter-36.github.io/sc-ore-scanner/prices.json",
        description="URL of the published prices.json feed"
    )
    refresh_minutes: int = Field(default=60, ge=1, description="How often to re-read the feed")


class Settings(BaseSettings):
    """Main application settings."""

    # Core configuration
    scan_interval: float = Field(default=2.0, ge=0.1, description="Scan interval in seconds")
    scan_region: Optional[ScanRegion] = Field(default=None, description="Configured scan region")

    # Module configs
    ocr: OCRConfig = Field(default_factory=OCRConfig)
    scan_gating: ScanGatingConfig = Field(default_factory=ScanGatingConfig)
    signature: SignatureConfig = Field(default_factory=SignatureConfig)
    server: ServerConfig = Field(default_factory=ServerConfig)
    prices: PriceConfig = Field(default_factory=PriceConfig)

    # Paths
    signatures_path: Path = Field(
        default=Path(__file__).parent.parent.parent / "data" / "signatures.json",
        description="Path to ore signatures database"
    )
    config_file: Path = Field(
        default=Path(__file__).parent / "settings.json",
        description="Path to user configuration file"
    )

    model_config = {
        "env_prefix": "SC_SCANNER_",
        "env_nested_delimiter": "__",
    }

    @field_validator("signatures_path", "config_file")
    @classmethod
    def resolve_path(cls, v: Path) -> Path:
        """Resolve path to absolute."""
        return v.resolve()

    def save_user_config(self) -> None:
        """Save user-specific settings to config file."""
        config_data = {
            "scan_interval": self.scan_interval,
            "scan_region": self.scan_region.model_dump() if self.scan_region else None,
            "ocr": self.ocr.model_dump(),
            "scan_gating": self.scan_gating.model_dump(),
            "signature": self.signature.model_dump(),
            "server": self.server.model_dump(),
            "prices": self.prices.model_dump(),
        }

        self.config_file.parent.mkdir(parents=True, exist_ok=True)
        with open(self.config_file, "w") as f:
            json.dump(config_data, f, indent=2)

    @classmethod
    def load_user_config(cls, config_path: Optional[Path] = None) -> "Settings":
        """Load settings from config file if it exists."""
        settings = cls()

        config_file = config_path or settings.config_file

        if config_file.exists():
            try:
                with open(config_file, "r") as f:
                    config_data = json.load(f)

                # Update settings from file
                if "scan_interval" in config_data:
                    settings.scan_interval = config_data["scan_interval"]

                if config_data.get("scan_region"):
                    settings.scan_region = ScanRegion(**config_data["scan_region"])

                if "ocr" in config_data:
                    settings.ocr = OCRConfig(**config_data["ocr"])

                if "scan_gating" in config_data:
                    settings.scan_gating = ScanGatingConfig(**config_data["scan_gating"])

                if "signature" in config_data:
                    settings.signature = SignatureConfig(**config_data["signature"])

                if "server" in config_data:
                    settings.server = ServerConfig(**config_data["server"])

                if "prices" in config_data:
                    settings.prices = PriceConfig(**config_data["prices"])

            except Exception as e:
                print(f"Warning: Failed to load config from {config_file}: {e}")

        return settings


# Singleton instance
_settings: Optional[Settings] = None


def get_settings() -> Settings:
    """Get or create settings singleton."""
    global _settings
    if _settings is None:
        _settings = Settings.load_user_config()
    return _settings
