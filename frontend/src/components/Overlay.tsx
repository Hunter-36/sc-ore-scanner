import { useOreStore } from '../store/useOreStore';
import { useWebSocket } from '../hooks/useWebSocket';
import { OreCard } from './OreCard';

async function closeOverlay() {
  // Stop the (windowless) backend first, so it doesn't linger after the overlay
  // closes. Fire-and-forget; ignore errors (backend may already be down).
  try {
    await fetch('http://127.0.0.1:8765/shutdown', { method: 'POST' });
  } catch {
    /* backend not reachable — nothing to stop */
  }
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().close();
  } catch (err) {
    // Not running inside Tauri (e.g. browser/dev) — nothing to close.
    console.warn('close() unavailable outside Tauri:', err);
  }
}

export function Overlay() {
  const { ores, scannerActive, connected } = useOreStore();
  useWebSocket();

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
        <button
          className="close-btn"
          onClick={closeOverlay}
          title="Close overlay"
          aria-label="Close overlay"
        >
          ✕
        </button>
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
    </div>
  );
}
