import { useState } from 'react';
import { useOreStore } from '../store/useOreStore';
import { useScanEvents } from '../hooks/useScanEvents';
import { OreCard } from './OreCard';
import { Settings } from './Settings';

export function Overlay() {
  const { ores, scannerActive, configured, connected } = useOreStore();
  useScanEvents();
  const [showSettings, setShowSettings] = useState(false);

  async function closeOverlay() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('quit');
    } catch {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().close();
      } catch (err) {
        console.warn('quit/close unavailable outside Tauri:', err);
      }
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
      {/* Header. Only the title/status area is a drag region — the buttons must
          NOT be inside it, or Tauri's drag handler swallows their clicks. */}
      <div className="overlay-header">
        <div className="header-drag" data-tauri-drag-region>
          <div className="title">SC ORE SCANNER</div>
          <div className="status">
            <div className={`status-dot ${connected ? 'connected' : 'disconnected'}`} />
            <span className="status-text">
              {connected ? (scannerActive ? 'SCANNING' : 'READY') : 'OFFLINE'}
            </span>
          </div>
        </div>
        <div className="header-actions">
          <button
            className={`icon-btn ${showSettings ? 'active' : ''}`}
            onClick={() => setShowSettings((s) => !s)}
            onMouseDown={(e) => e.stopPropagation()}
            title="Settings"
            aria-label="Settings"
          >
            <svg
              width="15"
              height="15"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <circle cx="12" cy="12" r="3" />
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
          </button>
          <button
            className="calibrate-btn"
            onClick={openCalibration}
            onMouseDown={(e) => e.stopPropagation()}
            title="Set scan region"
            aria-label="Set scan region"
          >
            <svg
              width="14"
              height="14"
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
            <span>Set region</span>
          </button>
          <button
            className="close-btn"
            onClick={closeOverlay}
            onMouseDown={(e) => e.stopPropagation()}
            title="Close / quit"
            aria-label="Close overlay"
          >
            ✕
          </button>
        </div>
      </div>

      {showSettings ? (
        <Settings onClose={() => setShowSettings(false)} />
      ) : (
      /* Ore List */
      <div className="ore-list">
        {!connected && (
          <div className="message">
            <p>Starting scanner…</p>
            <p className="hint">Loading the OCR engine (~15–20s on first launch). If
              this persists, check logs/scanner.log.</p>
          </div>
        )}

        {connected && !configured && (
          <div className="message">
            <p>Set your scan region</p>
            <p className="hint">Click <b>Set region</b> (top-right), then drag a box
              around the mining scanner's <b>RS</b> number.</p>
          </div>
        )}

        {connected && configured && !hasOres && (
          <div className="message">
            <p>{scannerActive ? 'No ores detected' : 'Waiting for scanner...'}</p>
            <p className="hint">Point your scanner at ore deposits in-game.</p>
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
      )}
    </div>
  );
}
