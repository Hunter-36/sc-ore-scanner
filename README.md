# SC Ore Scanner

Real-time Star Citizen mining overlay. It reads the RS (Radar Signature) number off the mining scanner HUD on screen, matches it to the corresponding ore type, and shows the ore name and quantity in an always-on-top overlay.

![CI](https://github.com/Hunter-36/sc-ore-scanner/actions/workflows/ci.yml/badge.svg)
![E2E](https://github.com/Hunter-36/sc-ore-scanner/actions/workflows/e2e.yml/badge.svg)
![Version](https://img.shields.io/badge/version-1.0.0-blue)
![Python](https://img.shields.io/badge/python-3.11+-green)
![Tauri](https://img.shields.io/badge/tauri-2.0-orange)

## Features

### Backend (Python + FastAPI)
- 🎯 **Screen Capture**: Fast capture with `mss` library
- 🔍 **Scan-State Gating**: Only runs OCR when scanner HUD is active
- 🤖 **RapidOCR (ONNX)**: lightweight OCR with CLAHE preprocessing — no PyTorch, ~150 MB install
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
- Python 3.11+
- Windows (for screen capture)
- [`uv`](https://github.com/astral-sh/uv) package manager

**Frontend:**
- Node.js 18+
- [`pnpm`](https://pnpm.io/) package manager (`corepack enable pnpm`)
- Rust (for Tauri)

### Installation

```bash
# Clone the project
git clone https://github.com/Hunter-36/sc-ore-scanner.git
cd sc-ore-scanner

# Install backend dependencies (full stack incl. OCR/ML)
cd backend
uv venv
uv pip install -r requirements.txt

# Install frontend dependencies
cd ../frontend
pnpm install
```

> Dependencies are split for faster installs: `requirements-core.txt` (app, no ML),
> `requirements-ml.txt` (rapidocr-onnxruntime — the OCR engine, no PyTorch), and
> `requirements-dev.txt` (test + lint tooling). `requirements.txt` pulls in core + OCR.

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
pnpm tauri dev
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
├── backend/                  # Python FastAPI backend
│   ├── src/
│   │   ├── config/          # Settings management
│   │   ├── capture/         # Screen capture + gating
│   │   ├── ocr/             # OCR engine
│   │   ├── resolver/        # Signature matching
│   │   └── server/          # FastAPI + WebSocket
│   ├── data/
│   │   └── signatures.json  # 27 ore definitions
│   ├── tests/
│   │   ├── unit/            # resolver / config / ocr / server tests
│   │   ├── e2e/             # OCR pipeline tests + manifest
│   │   └── test_images/     # real scan captures (fixtures)
│   ├── main.py              # Backend entry point
│   ├── calibrate.py         # Region selector
│   ├── pyproject.toml       # pytest + ruff config
│   └── requirements*.txt    # core / ml / dev dependency sets
│
├── frontend/                # Tauri + React frontend
│   ├── src/
│   │   ├── components/      # React components
│   │   ├── hooks/           # WebSocket hook
│   │   ├── store/           # Zustand state (+ vitest tests)
│   │   └── App.tsx          # Root component
│   ├── tests/e2e/           # Playwright overlay display tests
│   ├── src-tauri/           # Tauri/Rust backend
│   └── package.json         # Node dependencies (pnpm)
│
├── .github/workflows/       # CI, E2E, and Release pipelines
├── docs/                    # Architecture, testing, and CI docs
├── launch.bat               # Windows launcher script
└── README.md                # This file
```

## How It Works

1. **Screen Capture**: Backend captures configured screen region every 2 seconds
2. **Scan-State Gating**: Checks if scanner HUD is active (by pixel color detection)
3. **OCR Processing**: If scanner active, runs RapidOCR on the captured image
4. **Preprocessing**: upscale (LANCZOS) → grayscale → CLAHE contrast + histogram normalize → a height-based component mask that strips the thousands comma, the location-pin glyph, and floating particles while keeping the digit strokes intact
5. **Number Detection**: Extracts 3-6 digit numbers from OCR results
6. **Debouncing**: Requires 3 consecutive frames showing same number
7. **RS Resolution**: Divides detected number by known signatures
   - Example: 10620 ÷ 3540 = 3 → **3x Beryl**
8. **WebSocket Broadcast**: Sends results to frontend in real-time
9. **UI Display**: Frontend shows ores sorted by tier and quantity

## Testing

See [`docs/testing.md`](docs/testing.md) for the full guide. Quick reference:

**Backend** (from `backend/`):
```bash
uv pip install -r requirements-dev.txt   # core + test tooling (no ML)
pytest tests/unit                         # fast unit tests (resolver, config, ocr, server)

uv pip install -r requirements-ml.txt    # OCR/ML stack
pytest tests/e2e                          # OCR pipeline over real captures
```

The end-to-end suite is **manifest-driven** ([`backend/tests/e2e/manifest.json`](backend/tests/e2e/manifest.json)):
each real capture in `backend/tests/test_images/` is cropped to its scan region, run through the
**real** OCR + resolver pipeline **10 times**, and must produce the expected ore as the
top match in **≥90%** of runs. To add a case, drop an image (or video) in `test_images/` and add a
manifest entry. You can also run the pipeline on any file directly:

```bash
python -m tests.e2e.pipeline tests/test_images/sc_mining_scan_rs_10620_some_particles.png
```

**Frontend** (from `frontend/`):
```bash
pnpm test          # vitest unit tests (store logic)
pnpm typecheck     # tsc --noEmit
pnpm test:e2e      # Playwright overlay display tests (mock backend WebSocket)
```

## CI/CD

GitHub Actions runs three pipelines (see [`docs/ci-cd.md`](docs/ci-cd.md)):

| Workflow | Trigger | What it does |
|---|---|---|
| **CI** (`ci.yml`) | push / PR | ruff + backend unit tests, frontend typecheck + vitest, Tauri `cargo check` |
| **E2E** (`e2e.yml`) | push / PR | OCR pipeline over real captures (RapidOCR), Playwright overlay tests |
| **Release** (`release.yml`) | tag `v*` | builds the Windows installer and drafts a GitHub Release |

Cutting a release:
```bash
git tag v1.0.1
git push origin v1.0.1   # -> builds .msi/.exe and drafts a Release
```

## Configuration

### Backend Settings

Edit `backend/src/config/settings.json`:

```json
{
  "scan_interval": 2.0,
  "ocr": {
    "confidence_threshold": 0.5,
    "min_consecutive_frames": 3,
    "upscale_factor": 4
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
  "app": {
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
- Run: `uv pip install -r requirements-ml.txt`
- Check logs for initialization errors

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

- **Memory**: ~150MB (RapidOCR ONNX models loaded)
- **CPU**: Low (2-second intervals)
- **Latency**: <100ms from detection to display

## Roadmap

- [ ] Hand-minable gems detection (FPS mode)
- [ ] Historical tracking & statistics
- [ ] Multiple monitor support
- [ ] Customizable overlay position
- [ ] Sound notifications
- [ ] Price integration

## Support

SC Ore Scanner is free and built/maintained on personal time. If it saves you
some aUEC and you'd like to help keep it updated as Star Citizen changes (HUD
tweaks, new ores, features), any support is hugely appreciated — but never
required. o7

- ☕ **Ko-fi** (one-off tip): <!-- add your link --> _coming soon_
- 💜 **GitHub Sponsors**: use the **Sponsor** button at the top of the repo <!-- enable in .github/FUNDING.yml -->

> Maintainer: fill in your donation links in [`.github/FUNDING.yml`](.github/FUNDING.yml)
> (enables the Sponsor button) and replace the placeholder above.

Not in a position to donate? Starring the repo, filing good bug reports, and
sharing it with fellow miners helps just as much.

## License

MIT

## Credits

Built with:
- [FastAPI](https://fastapi.tiangolo.com/)
- [RapidOCR](https://github.com/RapidAI/RapidOCR)
- [Tauri](https://tauri.app/)
- [React](https://react.dev/)
- [Zustand](https://github.com/pmndrs/zustand)

Ore data from Star Citizen community resources.
