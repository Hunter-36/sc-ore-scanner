import { OreData } from '../store/useOreStore';

interface OreCardProps {
  ore: OreData;
}

// Tier colors
const tierColors: Record<string, string> = {
  'S': '#ff3366',  // Red/pink
  'A': '#ffaa00',  // Orange/gold
  'B': '#00ccff',  // Cyan
  'C': '#999999',  // Gray
  'Salvage': '#66ff66' // Green
};

export function OreCard({ ore }: OreCardProps) {
  const tierColor = tierColors[ore.tier] || '#ffffff';

  return (
    <div className="ore-card" style={{ borderLeftColor: tierColor }}>
      <div className="ore-header">
        <span className="ore-name">{ore.name}</span>
        <span className="ore-tier" style={{ color: tierColor }}>
          {ore.tier}
        </span>
      </div>
      <div className="ore-quantity">
        {ore.quantity}x
        {ore.volatile && <span className="volatile-badge">⚠</span>}
      </div>
      {ore.value != null && (
        <div className="ore-value" title="Estimated value (UEX Corp)">
          ≈ {ore.value.toLocaleString()} aUEC
        </div>
      )}
      {ore.confidence < 0.9 && (
        <div className="ore-confidence">
          ~{Math.round(ore.confidence * 100)}%
        </div>
      )}
    </div>
  );
}
