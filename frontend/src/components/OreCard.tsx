import { OreData } from '../store/useOreStore';

interface OreCardProps {
  ore: OreData;
}

// Tier colors
const tierColors: Record<string, string> = {
  S: '#ff3366', // Red/pink
  A: '#ffaa00', // Orange/gold
  B: '#00ccff', // Cyan
  C: '#999999', // Gray
  Type: '#9a7dff', // Asteroid type
  Salvage: '#66ff66', // Green
};
const GROUP_COLOR = '#9a7dff';

// Compact aUEC value, e.g. 1_800_000 -> "1.8M", 600_000 -> "600k".
function fmtValue(v?: number | null): string | null {
  if (v == null || v <= 0) return null;
  if (v >= 1_000_000) return `${(v / 1_000_000).toFixed(1)}M`;
  if (v >= 1000) return `${Math.round(v / 1000)}k`;
  return `${v}`;
}

export function OreCard({ ore }: OreCardProps) {
  const candidates = ore.candidates ?? [];

  // Signature-degenerate reading: the RS can't pick one ore (e.g. all FPS gems = 3000).
  // Show every candidate with its value, ranked by spawn probability when a location is
  // set, else by value — honest ("could be any of these") and useful for deciding.
  if (candidates.length >= 2) {
    const located = candidates.some((c) => c.probability != null);
    const sorted = [...candidates].sort((a, b) =>
      located
        ? (b.probability ?? -1) - (a.probability ?? -1)
        : (b.unit_price ?? 0) - (a.unit_price ?? 0),
    );
    const sameQty = candidates.every((c) => c.quantity === candidates[0].quantity);

    return (
      <div className="ore-card ore-card-group" style={{ borderLeftColor: GROUP_COLOR }}>
        <div className="ore-header">
          <span className="ore-name">
            {ore.group_label ?? 'Ambiguous'}
            {sameQty ? ` ${candidates[0].quantity}x` : ''}
          </span>
          <span className="ore-sig" title="Detected radar signature">
            sig {ore.detected_rs}
          </span>
        </div>
        <div className="candidates">
          {sorted.map((c) => {
            const val = fmtValue(c.unit_price);
            return (
              <div className="candidate" key={c.name}>
                <span className="candidate-name" style={{ color: tierColors[c.tier] || '#fff' }}>
                  {c.volatile && <span className="volatile-badge">⚠</span>}
                  {c.name}
                  {sameQty ? '' : ` ${c.quantity}x`}
                </span>
                <span className="candidate-meta">
                  {val && (
                    <span className="candidate-value" title="Market sell price per SCU (UEX Corp)">
                      ≈{val}/SCU
                    </span>
                  )}
                  {c.probability != null && (
                    <span className="candidate-prob">{Math.round(c.probability)}%</span>
                  )}
                </span>
              </div>
            );
          })}
        </div>
        {!located && (
          <div className="candidate-hint">set your location in ⚙ to rank by spawn chance</div>
        )}
      </div>
    );
  }

  // Unambiguous reading — a single ore card.
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
      {ore.alternatives && ore.alternatives.length > 0 && (
        <div className="ore-alt" title="Ambiguous radar signature — could be either reading">
          ⇄ or {ore.alternatives.join(' / ')}
        </div>
      )}
      {ore.unit_price != null && (
        <div className="ore-value" title="Market sell price per SCU (UEX Corp)">
          ≈ {ore.unit_price.toLocaleString()} aUEC/SCU
        </div>
      )}
      {ore.confidence < 0.9 && (
        <div className="ore-confidence">~{Math.round(ore.confidence * 100)}%</div>
      )}
    </div>
  );
}
