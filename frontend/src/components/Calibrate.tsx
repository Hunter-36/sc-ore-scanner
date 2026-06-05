import { useEffect, useRef, useState } from 'react';

type Rect = { x: number; y: number; w: number; h: number };
type Handle = 'nw' | 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w';
type Drag =
  | { kind: 'draw'; ox: number; oy: number }
  | { kind: 'move'; mx: number; my: number; orig: Rect }
  | { kind: 'resize'; handle: Handle; mx: number; my: number; orig: Rect };

const MIN = 8;
const HANDLES: Handle[] = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];

async function closeCalibration() {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().close();
  } catch (err) {
    console.warn('close() unavailable outside Tauri:', err);
  }
}

function clampToViewport(r: Rect): Rect {
  const W = window.innerWidth;
  const H = window.innerHeight;
  const w = Math.max(MIN, r.w);
  const h = Math.max(MIN, r.h);
  return {
    w,
    h,
    x: Math.min(Math.max(0, r.x), Math.max(0, W - w)),
    y: Math.min(Math.max(0, r.y), Math.max(0, H - h)),
  };
}

function resizeRect(orig: Rect, handle: Handle, dx: number, dy: number): Rect {
  let left = orig.x;
  let top = orig.y;
  let right = orig.x + orig.w;
  let bottom = orig.y + orig.h;
  if (handle.includes('w')) left = orig.x + dx;
  if (handle.includes('e')) right = orig.x + orig.w + dx;
  if (handle.includes('n')) top = orig.y + dy;
  if (handle.includes('s')) bottom = orig.y + orig.h + dy;
  return {
    x: Math.min(left, right),
    y: Math.min(top, bottom),
    w: Math.max(MIN, Math.abs(right - left)),
    h: Math.max(MIN, Math.abs(bottom - top)),
  };
}

/**
 * Full-screen calibration: drag to draw a box around the scanner's RS readout,
 * then nudge/resize it and Confirm. The window covers the primary monitor, so a
 * rectangle in CSS pixels maps to physical capture pixels via devicePixelRatio.
 */
export function Calibrate() {
  const surfaceRef = useRef<HTMLDivElement>(null);
  const drag = useRef<Drag | null>(null);
  const [rect, setRect] = useState<Rect | null>(null);
  // Mirror rect into a ref so the keydown handler (bound once) sees the latest.
  const rectRef = useRef<Rect | null>(null);
  rectRef.current = rect;

  useEffect(() => {
    surfaceRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeCalibration();
      if (e.key === 'Enter') confirm();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Start a fresh draw when pressing on empty space.
  function onSurfaceDown(e: React.MouseEvent) {
    drag.current = { kind: 'draw', ox: e.clientX, oy: e.clientY };
    setRect({ x: e.clientX, y: e.clientY, w: 0, h: 0 });
  }

  function onBoxDown(e: React.MouseEvent) {
    e.stopPropagation();
    if (!rect) return;
    drag.current = { kind: 'move', mx: e.clientX, my: e.clientY, orig: rect };
  }

  function onHandleDown(e: React.MouseEvent, handle: Handle) {
    e.stopPropagation();
    if (!rect) return;
    drag.current = { kind: 'resize', handle, mx: e.clientX, my: e.clientY, orig: rect };
  }

  function onMouseMove(e: React.MouseEvent) {
    const d = drag.current;
    if (!d) return;
    if (d.kind === 'draw') {
      setRect({
        x: Math.min(d.ox, e.clientX),
        y: Math.min(d.oy, e.clientY),
        w: Math.abs(e.clientX - d.ox),
        h: Math.abs(e.clientY - d.oy),
      });
    } else if (d.kind === 'move') {
      setRect(clampToViewport({ ...d.orig, x: d.orig.x + (e.clientX - d.mx), y: d.orig.y + (e.clientY - d.my) }));
    } else {
      setRect(clampToViewport(resizeRect(d.orig, d.handle, e.clientX - d.mx, e.clientY - d.my)));
    }
  }

  function onMouseUp() {
    const d = drag.current;
    drag.current = null;
    // A click or too-tiny drag clears the box instead of leaving a sliver.
    if (d?.kind === 'draw') {
      setRect((r) => (r && (r.w < MIN || r.h < MIN) ? null : r));
    }
  }

  async function confirm() {
    const r = rectRef.current;
    if (!r || r.w < MIN || r.h < MIN) return;
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

  const hasBox = !!rect;

  return (
    <div
      ref={surfaceRef}
      className="calibrate"
      tabIndex={0}
      onMouseDown={onSurfaceDown}
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
          {hasBox
            ? 'Drag the box to move it, or its handles to resize, until it frames the RS number. Then Confirm.'
            : "Drag a box around the mining scanner's RS number."}
        </p>
        <div className="calibrate-actions">
          {hasBox && (
            <>
              <button className="calibrate-confirm" onClick={confirm}>
                Confirm (Enter)
              </button>
              <button className="calibrate-redraw" onClick={() => setRect(null)}>
                Redraw
              </button>
            </>
          )}
          <button className="calibrate-cancel" onClick={closeCalibration}>
            Cancel (Esc)
          </button>
        </div>
      </div>

      {rect && (
        <div
          className="calibrate-box"
          style={{ left: rect.x, top: rect.y, width: rect.w, height: rect.h }}
          onMouseDown={onBoxDown}
        >
          <span className="calibrate-size">
            {Math.round(rect.w)} × {Math.round(rect.h)}
          </span>
          {HANDLES.map((h) => (
            <span
              key={h}
              className={`calibrate-handle handle-${h}`}
              onMouseDown={(e) => onHandleDown(e, h)}
            />
          ))}
        </div>
      )}
    </div>
  );
}
