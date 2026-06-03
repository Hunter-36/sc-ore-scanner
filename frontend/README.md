# SC Ore Scanner - Frontend

Tauri v2 + React + TypeScript overlay application for displaying Star Citizen mining data.

## Features

- **Transparent Overlay**: Always-on-top window positioned in top-right corner
- **Real-time Updates**: WebSocket connection to backend for live ore detection
- **Tier Visualization**: Color-coded ore tiers (S/A/B/C/Salvage)
- **Auto-reconnect**: Automatically reconnects to backend if connection drops
- **Minimal UI**: Clean, sci-fi themed interface

## Tech Stack

- **Tauri v2**: Native desktop application framework
- **React 18**: UI library
- **TypeScript**: Type-safe development
- **Zustand**: Lightweight state management
- **Vite**: Fast build tool

## Development

### Prerequisites

- Node.js 18+
- Rust (for Tauri)

### Install Dependencies

```bash
npm install
```

### Run in Development Mode

```bash
npm run tauri dev
```

This will:
1. Start Vite dev server on port 1420
2. Launch Tauri app in development mode
3. Enable hot reload for React components

### Build Production

```bash
npm run tauri build
```

Outputs executable to `src-tauri/target/release/`

## Configuration

### Window Settings

Edit `src-tauri/tauri.conf.json`:

```json
{
  "tauri": {
    "windows": [{
      "width": 450,
      "height": 300,
      "x": 1450,  // Position from left
      "y": 20,    // Position from top
      "decorations": false,  // No title bar
      "transparent": true,   // Transparent background
      "alwaysOnTop": true    // Stay on top
    }]
  }
}
```

### Backend Connection

WebSocket URL is hardcoded in `src/hooks/useWebSocket.ts`:

```typescript
const WS_URL = 'ws://127.0.0.1:8765/ws';
```

Change this if backend runs on different port.

## Project Structure

```
frontend/
├── src/
│   ├── components/
│   │   ├── Overlay.tsx       # Main overlay component
│   │   └── OreCard.tsx       # Individual ore display
│   ├── hooks/
│   │   └── useWebSocket.ts   # WebSocket connection hook
│   ├── store/
│   │   └── useOreStore.ts    # Zustand state management
│   ├── App.tsx               # Root component
│   ├── App.css               # Global styles
│   └── main.tsx              # React entry point
├── src-tauri/
│   ├── src/
│   │   └── main.rs           # Rust/Tauri backend
│   ├── Cargo.toml            # Rust dependencies
│   └── tauri.conf.json       # Tauri configuration
├── package.json
├── tsconfig.json
└── vite.config.ts
```

## State Management

Uses Zustand for simple, fast state management:

```typescript
interface OreStore {
  ores: Record<string, OreData>;
  scannerActive: boolean;
  connected: boolean;
  lastUpdate: number;
}
```

## Styling

- **Transparent background** with backdrop blur
- **Tier colors**:
  - S tier: Red (#ff3366)
  - A tier: Orange (#ffaa00)
  - B tier: Cyan (#00ccff)
  - C tier: Gray (#999999)
  - Salvage: Green (#66ff66)
- **Animations**: Slide-in for new ores, pulse for volatile indicator

## Troubleshooting

### Window not showing
- Check if Tauri build is in release mode
- Verify window position is within screen bounds

### WebSocket not connecting
- Ensure backend is running on port 8765
- Check firewall settings
- Look at console logs (Ctrl+Shift+I in dev mode)

### Transparent background not working
- Windows 10+ required for transparency
- GPU acceleration must be enabled

## License

MIT
