import { useOreStore } from '../store/useOreStore';
import { useWebSocket } from '../hooks/useWebSocket';
import { OreCard } from './OreCard';

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
      </div>

      {/* Ore List */}
      <div className="ore-list">
        {!connected && (
          <div className="message">
            <p>Connecting to backend...</p>
            <p className="hint">Make sure backend is running on port 8765</p>
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
