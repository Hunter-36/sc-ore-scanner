import { useEffect, useRef, useState } from 'react';

type Rect = { x: number; y: number; w: number; h: number };

// Matches calibrate.py's 50px minimum (in capture px; ~equivalent here).
const MIN = 20;

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
 */
export function Calibrate() {
  const surfaceRef = useRef<HTMLDivElement>(null);
  const start = useRef<{ x: number; y: number } | null>(null);
  const [rect, setRect] = useState<Rect | null>(null);

  useEffect(() => {
    surfaceRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeCalibration();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  function onMouseDown(e: React.MouseEvent) {
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
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_scan_region', region);
    } catch (err) {
      console.error('save_scan_region failed:', err);
    }
    await closeCalibration();
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
