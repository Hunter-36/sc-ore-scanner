# SC Ore Scanner - Backend

Python FastAPI backend for real-time Star Citizen mining overlay.

## Features

- **Screen Capture**: Low-latency capture with `mss`
- **Scan-State Gating**: Only runs OCR when scanner HUD detected
- **OCR**: EasyOCR with CLAHE preprocessing
- **Debouncing**: Requires N consecutive detections before confirming
- **RS Resolution**: Division-based matching (e.g., 10620 = 3 × 3540 Beryl)
- **WebSocket**: Real-time ore detection streaming
- **REST API**: Configuration and control endpoints

## Requirements

- Python 3.10+
- Windows (for screen capture)

## Installation

### Option 1: Using uv (recommended)

```bash
cd backend
uv venv
.venv\Scripts\activate
uv pip install -r requirements.txt
```

### Option 2: Using pip

```bash
cd backend
python -m venv .venv
.venv\Scripts\activate
pip install -r requirements.txt
```

## Usage

### 1. Calibrate Scan Region

Run the calibration tool to select the screen region where RS signatures appear:

```bash
python calibrate.py
```

This will:
- Show a fullscreen overlay
- Let you click and drag to select the scan area
- Save the configuration to `src/config/settings.json`

### 2. Start Backend Server

```bash
python main.py
```

The server will start on `ws://127.0.0.1:8765`

### 3. Connect Frontend

The backend exposes:
- WebSocket: `ws://127.0.0.1:8765/ws` (ore detection stream)
- REST API: `http://127.0.0.1:8765/` (configuration, control)

## API Endpoints

### WebSocket

**`ws://127.0.0.1:8765/ws`**

Streams real-time ore detections:

```json
{
  "ores": {
    "beryl": {
      "name": "Beryl",
      "quantity": 3,
      "tier": "A",
      "tier_value": 3,
      "volatile": false,
      "confidence": 0.95,
      "detected_rs": 10620
    }
  },
  "scanner_active": true,
  "timestamp": 1234567890.123
}
```

### REST Endpoints

- `GET /health` - Health check
- `GET /config` - Get current configuration
- `POST /config/scan-region` - Set scan region
- `POST /scan/start` - Start scanning
- `POST /scan/stop` - Stop scanning
- `GET /monitors` - Get available monitors
- `GET /signatures` - Get ore signature database

## Configuration

Configuration is stored in `src/config/settings.json`:

- `scan_region`: Screen area to scan (x, y, width, height)
- `scan_interval`: Seconds between scans (default: 2.0)
- `ocr.confidence_threshold`: Minimum OCR confidence (default: 0.5)
- `ocr.min_consecutive_frames`: Frames required for confirmation (default: 3)
- `scan_gating.enabled`: Enable scanner HUD detection (default: true)
- `signature.max_error_margin`: RS matching tolerance (default: 50)

## Development

### Project Structure

```
backend/
├── src/
│   ├── config/          # Configuration management
│   ├── capture/         # Screen capture + gating
│   ├── ocr/             # OCR engine (CLAHE + EasyOCR)
│   ├── resolver/        # RS signature matching
│   └── server/          # FastAPI + WebSocket
├── data/
│   └── signatures.json  # Ore signature database
├── main.py              # Entry point
├── calibrate.py         # Region selector tool
└── requirements.txt     # Dependencies
```

### Adding New Ores

Edit `data/signatures.json`:

```json
{
  "ores": [
    {
      "id": "new_ore",
      "name": "New Ore",
      "base_rs": 3000,
      "tier": "S",
      "tier_value": 4,
      "volatile": false,
      "context": ["ship", "vehicle"],
      "notes": "Description"
    }
  ]
}
```

## Troubleshooting

### OCR Not Working

1. Ensure EasyOCR is installed: `uv pip install easyocr`
2. Check logs for initialization errors
3. Try disabling GPU: Set `ocr.gpu_enabled: false` in settings

### No Detections

1. Run calibration again - ensure scan region is correct
2. Check if scan-state gating is working (disable with `scan_gating.enabled: false`)
3. Adjust `ocr.confidence_threshold` (lower = more detections, less accurate)

### High Memory Usage

- EasyOCR loads models into memory (~200MB)
- This is normal and required for fast OCR

## License

MIT
