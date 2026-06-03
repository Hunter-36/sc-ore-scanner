# SC Ore Scanner

Real-time Star Citizen mining overlay that automatically detects RS (Radar Signature) numbers and displays ore names with ZERO typing required.

![Version](https://img.shields.io/badge/version-1.0.0-blue)
![Python](https://img.shields.io/badge/python-3.10+-green)
![Tauri](https://img.shields.io/badge/tauri-2.0-orange)

## Features

### Backend (Python + FastAPI)
- 🎯 **Screen Capture**: Fast capture with `mss` library
- 🔍 **Scan-State Gating**: Only runs OCR when scanner HUD is active
- 🤖 **EasyOCR**: Deep learning OCR with CLAHE preprocessing
- 📊 **Debouncing**: Requires 3 consecutive detections before confirming
- 🧮 **RS Resolution**: Division-based matching (e.g., 10620 = 3 × 3540 Beryl)
- ⚡ **WebSocket**: Real-time ore streaming to frontend
- 🎛️ **REST API**: Configuration and control endpoints

### Frontend (Tauri v2 + React)
- 🪟 **Transparent Overlay**: Always-on-top, positioned in top-right corner
- 🎨 **Tier Visualization**: Color-coded (S/A/B/C tiers)
- 📡 **Auto-Reconnect**: Automatically reconnects to backend
- ⚠️ **Volatile Warning**: Special indicator for Quantainium
- 🎯 **Minimal UI**: Clean, sci-fi themed interface

## Quick Start

### Prerequisites

**Backend:**
- Python 3.10+
- Windows (for screen capture)
- [`uv`](https://github.com/astral-sh/uv) package manager

**Frontend:**
- Node.js 18+
- Rust (for Tauri)

### Installation

```bash
# Clone or download the project
cd sc-ore-scanner

# Install backend dependencies
cd backend
uv venv
uv pip install -r requirements.txt

# Install frontend dependencies
cd ../frontend
npm install
```

### Usage

#### Option 1: Use Launcher (Recommended)

Double-click `launch.bat` or run:

```bash
launch.bat
```

This starts both backend and frontend automatically.

#### Option 2: Manual Start

**Terminal 1 - Backend:**
```bash
cd backend
.venv\Scripts\python.exe calibrate.py  # First time only - select scan region
.venv\Scripts\python.exe main.py
```

**Terminal 2 - Frontend:**
```bash
cd frontend
npm run tauri dev
```

### First-Time Calibration

On first run, you need to calibrate the scan region:

1. Run `python calibrate.py` in the backend folder
2. A fullscreen overlay will appear
3. Click and drag to select where RS numbers appear in-game
4. Release to confirm

The configuration is saved to `backend/src/config/settings.json`

## Project Structure

```
sc-ore-scanner/
├── backend/               # Python FastAPI backend
│   ├── src/
│   │   ├── config/       # Settings management
│   │   ├── capture/      # Screen capture + gating
│   │   ├── ocr/          # OCR engine
│   │   ├── resolver/     # Signature matching
│   │   └── server/       # FastAPI + WebSocket
│   ├── data/
│   │   └── signatures.json   # 27 ore definitions
│   ├── main.py           # Backend entry point
│   ├── calibrate.py      # Region selector
│   └── requirements.txt  # Python dependencies
│
├── frontend/             # Tauri + React frontend
│   ├── src/
│   │   ├── components/   # React components
│   │   ├── hooks/        # WebSocket hook
│   │   ├── store/        # Zustand state
│   │   └── App.tsx       # Root component
│   ├── src-tauri/        # Tauri/Rust backend
│   └── package.json      # Node dependencies
│
├── launch.bat            # Windows launcher script
└── README.md             # This file
```

## How It Works

1. **Screen Capture**: Backend captures configured screen region every 2 seconds
2. **Scan-State Gating**: Checks if scanner HUD is active (by pixel color detection)
3. **OCR Processing**: If scanner active, runs EasyOCR on captured image
4. **Preprocessing**: CLAHE enhancement, adaptive thresholding, noise removal
5. **Number Detection**: Extracts 3-6 digit numbers from OCR results
6. **Debouncing**: Requires 3 consecutive frames showing same number
7. **RS Resolution**: Divides detected number by known signatures
   - Example: 10620 ÷ 3540 = 3 → **3x Beryl**
8. **WebSocket Broadcast**: Sends results to frontend in real-time
9. **UI Display**: Frontend shows ores sorted by tier and quantity

## Configuration

### Backend Settings

Edit `backend/src/config/settings.json`:

```json
{
  "scan_interval": 2.0,
  "ocr": {
    "confidence_threshold": 0.5,
    "min_consecutive_frames": 3,
    "gpu_enabled": false
  },
  "scan_gating": {
    "enabled": true
  },
  "signature": {
    "max_error_margin": 50,
    "error_margin_percent": 0.01
  }
}
```

### Frontend Window

Edit `frontend/src-tauri/tauri.conf.json`:

```json
{
  "tauri": {
    "windows": [{
      "width": 450,
      "height": 300,
      "x": 1450,
      "y": 20,
      "alwaysOnTop": true
    }]
  }
}
```

## API Reference

### WebSocket

**Endpoint:** `ws://127.0.0.1:8765/ws`

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
- `GET /config` - Get configuration
- `POST /config/scan-region` - Set scan region
- `POST /scan/start` - Start scanning
- `POST /scan/stop` - Stop scanning
- `GET /monitors` - Get available monitors
- `GET /signatures` - Get ore database

## Supported Ores

**27 ores total:**

**S Tier:**
- Quantainium (3170) - Volatile
- Bexalite (3600)
- Hadanite (5415) - FPS only

**A Tier:**
- Stileron, Savrilium, Ouratite, Beryl, Taranite, Gold, Laranite, Agricium
- Dolivine, Felinite (FPS only)

**B Tier:**
- Riccite, Lindinium, Borase, Titanium, Tungsten, Torite, Hephestanite
- Aphorite (FPS only)

**C Tier:**
- Corundum, Copper, Iron, Aluminium, Ice

**Salvage:**
- Salvage Panels (2000)

## Troubleshooting

### Backend Issues

**OCR not working:**
- Run: `uv pip install easyocr`
- Check logs for initialization errors
- Try disabling GPU: `ocr.gpu_enabled: false`

**No detections:**
- Recalibrate scan region: `python calibrate.py`
- Disable scan-state gating: `scan_gating.enabled: false`
- Lower confidence threshold: `ocr.confidence_threshold: 0.3`

### Frontend Issues

**Window not showing:**
- Check if within screen bounds
- Verify Tauri is in dev/debug mode

**WebSocket not connecting:**
- Ensure backend running on port 8765
- Check firewall settings
- View console logs (Ctrl+Shift+I)

**Transparent background not working:**
- Requires Windows 10+
- Enable GPU acceleration

## Performance

- **Memory**: ~200MB (EasyOCR models loaded)
- **CPU**: Low (2-second intervals)
- **Latency**: <100ms from detection to display

## Roadmap

- [ ] Hand-minable gems detection (FPS mode)
- [ ] Historical tracking & statistics
- [ ] Multiple monitor support
- [ ] Customizable overlay position
- [ ] Sound notifications
- [ ] Price integration

## License

MIT

## Credits

Built with:
- [FastAPI](https://fastapi.tiangolo.com/)
- [EasyOCR](https://github.com/JaidedAI/EasyOCR)
- [Tauri](https://tauri.app/)
- [React](https://react.dev/)
- [Zustand](https://github.com/pmndrs/zustand)

Ore data from Star Citizen community resources.
