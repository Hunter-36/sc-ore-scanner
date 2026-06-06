# SC Ore Scanner — Frontend

The Tauri v2 + React + TypeScript app. This is the whole app: the React UI in `src/`
is the overlay, and the Rust shell in `src-tauri/src/` runs the in-process scan loop
(capture → OCR via the `core` crate → detect) and pushes results to the UI through a
Tauri event. There is no separate backend and no WebSocket.

## Tech stack

- **Tauri v2** — native desktop shell (Rust)
- **React 18 + TypeScript + Vite** — overlay UI
- **Zustand** — state
- **scanner-core** (`../core`) — detection (OCR, resolver, signatures)

## Development

Prerequisites: Node 18+, [`pnpm`](https://pnpm.io/), Rust (on Windows, build under
`vcvars64`).

```bash
pnpm install
pnpm tauri dev      # run the app (Rust + React, hot reload on the UI)
pnpm tauri build    # release exe in src-tauri/target/release;
                    # NSIS/MSI installers under src-tauri/target/release/bundle/{nsis,msi}
pnpm typecheck
pnpm test           # vitest (store)
pnpm test:e2e       # Playwright (overlay + calibration)
```

## How data reaches the UI

The scan loop (`src-tauri/src/scan.rs`) emits a `scan-result` Tauri event each cycle.
`src/hooks/useScanEvents.ts` listens for it and updates the Zustand store
(`src/store/useOreStore.ts`); `src/components/Overlay.tsx` renders one `OreCard` per
ore, sorted by tier then quantity. `src/components/Calibrate.tsx` is the calibration
window (opened from the overlay's "Set region" button), and the overlay's **gear**
button toggles `src/components/Settings.tsx` — an in-overlay panel that edits the
runtime scan config (interval, confirm frames, upscale, CLAHE) live via `set_config`.
The whole UI is wrapped in `src/components/ErrorBoundary.tsx` (mounted in `main.tsx`).

> In a browser (vitest/Playwright) there's no Tauri runtime, so tests drive the store
> via a dev-only `mock-scan` event and reach the calibration view via `?calibrate`.
> See [`../docs/testing.md`](../docs/testing.md).

## Project structure

```
frontend/
├── src/
│   ├── components/  Overlay.tsx, OreCard.tsx, Settings.tsx, Calibrate.tsx, ErrorBoundary.tsx
│   ├── hooks/       useScanEvents.ts  (Tauri scan-result listener)
│   ├── store/       useOreStore.ts    (Zustand)
│   ├── App.tsx      overlay vs. calibrate routing
│   ├── main.tsx     React entry (mounts <App/> inside ErrorBoundary)
│   └── App.css
├── src-tauri/
│   └── src/         scan.rs (scan loop), main.rs (windows, calibration, quit, logging)
└── tests/e2e/       Playwright specs
```

## Configuration

- Overlay window size/position: `src-tauri/tauri.conf.json` (position is also remembered
  in `%APPDATA%\com.scorescanner.app\.window-state.json`).
- Runtime scan config (region, interval, tuning): `%APPDATA%\com.scorescanner.app\config.json`
  (written by calibration). See the root README's Configuration section.

## License

MIT
