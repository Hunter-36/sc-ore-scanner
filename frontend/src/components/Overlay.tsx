import { useOreStore } from '../store/useOreStore';
import { useScanEvents } from '../hooks/useScanEvents';
import { OreCard } from './OreCard';

async function closeOverlay() {
  // The scan loop is in-process (v2), so closing the window exits the whole app.
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().close();
  } catch (err) {
    // Not running inside Tauri (e.g. browser/dev) — nothing to close.
    console.warn('close() unavailable outside Tauri:', err);
  }
}

async function openCalibration() {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('open_calibration');
  } catch (err) {
    console.warn('open_calibration unavailable outside Tauri:', err);
  }
}

export function Overlay() {
  const { ores, scannerActive, connected, session } = useOreStore();
  useScanEvents();

  const oreList = Object.entries(ores);
  const hasOres = oreList.length > 0;

  // Sort by tier value (S > A > B > C) then by quantity
  const sortedOres = oreList.sort(([, a], [, b]) => {
    if (a.tier_value !== b.tier_value) {
      return b.tier_value - a.tier_value; // Higher tier first
    }
    return b.quantity - a.quantity; // Higher quantity first
  });

  return (
    <div className="overlay">
      {/* Header */}
      <div className="overlay-header" data-tauri-drag-region>
        <div className="title">SC ORE SCANNER</div>
        <div className="status">
          <div className={`status-dot ${connected ? 'connected' : 'disconnected'}`} />
          <span className="status-text">
            {connected ? (scannerActive ? 'SCANNING' : 'READY') : 'OFFLINE'}
          </span>
        </div>
        <div className="header-actions">
          <button
            className="calibrate-btn"
            onClick={openCalibration}
            title="Set scan region"
            aria-label="Set scan region"
          >
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
            >
              <circle cx="12" cy="12" r="7" />
              <line x1="12" y1="1" x2="12" y2="4" />
              <line x1="12" y1="20" x2="12" y2="23" />
              <line x1="1" y1="12" x2="4" y2="12" />
              <line x1="20" y1="12" x2="23" y2="12" />
            </svg>
          </button>
          <button
            className="close-btn"
            onClick={closeOverlay}
            title="Close overlay"
            aria-label="Close overlay"
          >
            ✕
          </button>
        </div>
      </div>

      {/* Ore List */}
      <div className="ore-list">
        {!connected && (
          <div className="message">
            <p>Starting scanner…</p>
            <p className="hint">First launch loads the OCR engine (~15–20s). If this
              persists, check logs/scanner.log.</p>
          </div>
        )}

        {connected && !hasOres && (
          <div className="message">
            <p>{scannerActive ? 'No ores detected' : 'Waiting for scanner...'}</p>
            <p className="hint">Point scanner at ores in-game</p>
          </div>
        )}

        {connected && hasOres && (
          <div className="ores-grid">
            {sortedOres.map(([id, ore]) => (
              <OreCard key={id} ore={ore} />
            ))}
          </div>
        )}
      </div>

      {/* Session footer */}
      {connected && session.total_detections > 0 && (
        <div className="session-footer">
          Session: {session.total_detections} detections · {session.distinct_ores} types
        </div>
      )}
    </div>
  );
}
