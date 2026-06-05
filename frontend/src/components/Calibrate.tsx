import { useEffect, useRef, useState } from 'react';

type Point = { x: number; y: number };
type Rect = { x: number; y: number; w: number; h: number };

async function closeCalibration() {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().close();
  } catch (err) {
    console.warn('close() unavailable outside Tauri:', err);
  }
}

/**
 * Full-screen drag-select for the scan region. The window covers the primary
 * monitor, so a rectangle in CSS pixels maps to physical capture pixels by
 * multiplying by devicePixelRatio. On release it saves the region and closes.
 */
export function Calibrate() {
  const surfaceRef = useRef<HTMLDivElement>(null);
  const start = useRef<Point | null>(null);
  const [rect, setRect] = useState<Rect | null>(null);

  useEffect(() => {
    // Grab focus so Escape works immediately.
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
    if (!start.current) return;
    const s = start.current;
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
    if (!r || r.w < 5 || r.h < 5) {
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
      {/* Instruction panel — clicking it must not start a drag. */}
      <div
        className="calibrate-panel"
        onMouseDown={(e) => e.stopPropagation()}
        onMouseUp={(e) => e.stopPropagation()}
      >
        <div className="calibrate-title">Set scan region</div>
        <p className="calibrate-steps">
          Click and <b>drag a box</b> around the mining scanner's <b>RS</b> number.
          Release to save — it applies immediately and this window closes.
        </p>
        <button className="calibrate-cancel" onClick={closeCalibration}>
          Cancel (Esc)
        </button>
      </div>

      {rect && (
        <div
          className="calibrate-rect"
          style={{ left: rect.x, top: rect.y, width: rect.w, height: rect.h }}
        />
      )}
    </div>
  );
}
