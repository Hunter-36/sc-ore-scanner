import { useEffect, useRef, useState } from 'react';

type Rect = { x: number; y: number; w: number; h: number };
type Region = { x: number; y: number; w: number; h: number };

// Matches calibrate.py's 50px minimum (in capture px; ~equivalent here). Filters
// an accidental click/jitter from an intentional drag.
const MIN = 20;

// Below this region height (in capture px) the RS text is too few pixels for a
// reliable read even at the detection pipeline's max upscale — mirrors
// scanner-core's MIN_READABLE_REGION_HEIGHT (TARGET_REGION_HEIGHT / MAX_AUTO_SCALE
// = 192 / 8). We warn (not block) so ultrawide / high-FOV users learn the RS text
// is too small *before* wondering why detection is flaky (issue #110).
const MIN_READABLE_PX = 24;

async function closeCalibration() {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().close();
  } catch (err) {
    console.warn('close() unavailable outside Tauri:', err);
  }
}

/**
 * Direct port of the Python calibrate.py: a full-screen, semi-transparent
 * overlay over the primary monitor. Click-drag a box around the RS readout;
 * release saves it and closes. Esc / Cancel aborts. The window is sized to the
 * monitor's physical pixels, so CSS px * devicePixelRatio = capture px.
 *
 * If the released box is too short to read reliably (issue #110: ultrawide +
 * high FOV shrinks the HUD text in pixels), we surface a warning and let the
 * user redraw or save anyway, instead of silently saving a region detection
 * will struggle with.
 */
export function Calibrate() {
  const surfaceRef = useRef<HTMLDivElement>(null);
  const start = useRef<{ x: number; y: number } | null>(null);
  const [rect, setRect] = useState<Rect | null>(null);
  // A valid-but-small region awaiting the user's confirmation (warning shown).
  const [pending, setPending] = useState<Region | null>(null);

  useEffect(() => {
    surfaceRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeCalibration();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  async function save(region: Region) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_scan_region', region);
    } catch (err) {
      console.error('save_scan_region failed:', err);
    }
    await closeCalibration();
  }

  function onMouseDown(e: React.MouseEvent) {
    setPending(null); // a new drag dismisses any standing warning
    start.current = { x: e.clientX, y: e.clientY };
    setRect({ x: e.clientX, y: e.clientY, w: 0, h: 0 });
  }

  function onMouseMove(e: React.MouseEvent) {
    const s = start.current;
    if (!s) return;
    setRect({
      x: Math.min(s.x, e.clientX),
      y: Math.min(s.y, e.clientY),
      w: Math.abs(e.clientX - s.x),
      h: Math.abs(e.clientY - s.y),
    });
  }

  async function onMouseUp() {
    const r = rect;
    start.current = null;
    if (!r || r.w < MIN || r.h < MIN) {
      setRect(null);
      return;
    }
    const dpr = window.devicePixelRatio || 1;
    const region = {
      x: Math.round(r.x * dpr),
      y: Math.round(r.y * dpr),
      w: Math.round(r.w * dpr),
      h: Math.round(r.h * dpr),
    };
    // Too short to read reliably: warn and wait for the user to redraw or confirm.
    if (region.h < MIN_READABLE_PX) {
      setPending(region);
      return;
    }
    await save(region);
  }

  return (
    <div
      ref={surfaceRef}
      className="calibrate"
      tabIndex={0}
      onMouseDown={onMouseDown}
      onMouseMove={onMouseMove}
      onMouseUp={onMouseUp}
    >
      <div
        className="calibrate-panel"
        onMouseDown={(e) => e.stopPropagation()}
        onMouseUp={(e) => e.stopPropagation()}
      >
        <div className="calibrate-title">Set scan region</div>
        <p className="calibrate-steps">
          Click and <b>drag a box</b> around the mining scanner's <b>RS</b> number,
          then release. It saves and closes automatically.
        </p>

        {pending && (
          <div className="calibrate-warning" role="alert">
            <p>
              ⚠ That box is only <b>{pending.h}px</b> tall — the <b>RS</b> text may be
              too small to read reliably. Draw a tighter box around just the RS number,
              or raise your in-game <b>HUD / Render scale</b> (or lower FOV), then try
              again.
            </p>
            <button className="calibrate-saveanyway" onClick={() => save(pending)}>
              Save anyway
            </button>
          </div>
        )}

        <button className="calibrate-cancel" onClick={closeCalibration}>
          Cancel (Esc)
        </button>
      </div>

      {rect && (
        <div
          className="calibrate-box"
          style={{ left: rect.x, top: rect.y, width: rect.w, height: rect.h }}
        />
      )}
    </div>
  );
}
