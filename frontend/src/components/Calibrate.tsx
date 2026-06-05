import { useEffect, useLayoutEffect, useRef, useState } from 'react';

type Rect = { x: number; y: number; w: number; h: number };
type Handle = 'nw' | 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w';
type Drag =
  | { kind: 'draw'; ox: number; oy: number }
  | { kind: 'move'; mx: number; my: number; orig: Rect }
  | { kind: 'resize'; handle: Handle; mx: number; my: number; orig: Rect };

type Capture = { width: number; height: number; dataUrl: string };

const MIN = 8;
const HANDLES: Handle[] = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];

// Dev/test fallback (a 1×1 transparent pixel) when there's no Tauri capture.
const MOCK: Capture = {
  width: 1920,
  height: 1080,
  dataUrl:
    'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==',
};

const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), hi);

async function closeCalibration() {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().close();
  } catch (err) {
    console.warn('close() unavailable outside Tauri:', err);
  }
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
 * Calibration shown in a normal, movable window: a screenshot of the scanned
 * monitor with a draggable/resizable selection box. Drawing on the actual image
 * sidesteps every transparent-overlay/multi-monitor problem — the box is mapped
 * from displayed pixels back to capture pixels on Confirm.
 */
export function Calibrate() {
  const wrapRef = useRef<HTMLDivElement>(null);
  const stageRef = useRef<HTMLDivElement>(null);
  const drag = useRef<Drag | null>(null);

  const [cap, setCap] = useState<Capture | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [stage, setStage] = useState<{ w: number; h: number }>({ w: 0, h: 0 });
  const [rect, setRect] = useState<Rect | null>(null);

  // Refs so the once-bound keydown handler always sees the latest values.
  const capRef = useRef<Capture | null>(null);
  const stageRef2 = useRef({ w: 0, h: 0 });
  const rectRef = useRef<Rect | null>(null);
  capRef.current = cap;
  stageRef2.current = stage;
  rectRef.current = rect;

  // Load the screenshot (and reload when Rust emits "recapture").
  useEffect(() => {
    let alive = true;
    async function load() {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const c = await invoke<Capture>('get_capture');
        if (alive) {
          setCap(c);
          setRect(null);
          setErr(null);
        }
      } catch (e) {
        if (import.meta.env.DEV) {
          if (alive) setCap(MOCK);
        } else if (alive) {
          setErr(String(e));
        }
      }
    }
    load();
    let un: (() => void) | undefined;
    (async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        un = await listen('recapture', () => load());
      } catch {
        /* not in Tauri */
      }
    })();
    return () => {
      alive = false;
      if (un) un();
    };
  }, []);

  // Fit the stage to the available area, preserving the capture's aspect ratio.
  useLayoutEffect(() => {
    if (!cap) return;
    const compute = () => {
      const wrap = wrapRef.current;
      if (!wrap) return;
      const availW = wrap.clientWidth;
      const availH = wrap.clientHeight;
      if (availW <= 0 || availH <= 0) return;
      const aspect = cap.width / cap.height;
      const w = Math.floor(Math.min(availW, availH * aspect));
      setStage({ w, h: Math.floor(w / aspect) });
    };
    compute();
    window.addEventListener('resize', compute);
    return () => window.removeEventListener('resize', compute);
  }, [cap]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeCalibration();
      if (e.key === 'Enter') confirm();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function local(e: React.MouseEvent) {
    const r = stageRef.current!.getBoundingClientRect();
    return {
      x: clamp(e.clientX - r.left, 0, stage.w),
      y: clamp(e.clientY - r.top, 0, stage.h),
    };
  }

  function onStageDown(e: React.MouseEvent) {
    const p = local(e);
    drag.current = { kind: 'draw', ox: p.x, oy: p.y };
    setRect({ x: p.x, y: p.y, w: 0, h: 0 });
  }

  function onBoxDown(e: React.MouseEvent) {
    e.stopPropagation();
    if (!rect) return;
    const p = local(e);
    drag.current = { kind: 'move', mx: p.x, my: p.y, orig: rect };
  }

  function onHandleDown(e: React.MouseEvent, handle: Handle) {
    e.stopPropagation();
    if (!rect) return;
    const p = local(e);
    drag.current = { kind: 'resize', handle, mx: p.x, my: p.y, orig: rect };
  }

  function onMove(e: React.MouseEvent) {
    const d = drag.current;
    if (!d) return;
    const p = local(e);
    if (d.kind === 'draw') {
      setRect({
        x: Math.min(d.ox, p.x),
        y: Math.min(d.oy, p.y),
        w: Math.abs(p.x - d.ox),
        h: Math.abs(p.y - d.oy),
      });
    } else if (d.kind === 'move') {
      const nx = clamp(d.orig.x + (p.x - d.mx), 0, stage.w - d.orig.w);
      const ny = clamp(d.orig.y + (p.y - d.my), 0, stage.h - d.orig.h);
      setRect({ ...d.orig, x: nx, y: ny });
    } else {
      setRect(resizeRect(d.orig, d.handle, p.x - d.mx, p.y - d.my));
    }
  }

  function onUp() {
    const d = drag.current;
    drag.current = null;
    if (d?.kind === 'draw') {
      setRect((r) => (r && (r.w < MIN || r.h < MIN) ? null : r));
    }
  }

  async function confirm() {
    const r = rectRef.current;
    const c = capRef.current;
    const s = stageRef2.current;
    if (!r || !c || s.w <= 0 || r.w < MIN || r.h < MIN) return;
    const scale = c.width / s.w; // displayed px -> capture px (aspect preserved)
    const region = {
      x: Math.round(r.x * scale),
      y: Math.round(r.y * scale),
      w: Math.round(r.w * scale),
      h: Math.round(r.h * scale),
    };
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_scan_region', region);
    } catch (e) {
      console.error('save_scan_region failed:', e);
    }
    await closeCalibration();
  }

  const scale = cap && stage.w > 0 ? cap.width / stage.w : 1;

  return (
    <div className="calibrate">
      <div className="calibrate-panel">
        <div className="calibrate-title">Set scan region</div>
        <p className="calibrate-steps">
          {rect
            ? 'Move the box or drag its handles to frame the RS number, then Confirm.'
            : "Drag a box around the mining scanner's RS number on the screenshot below."}
        </p>
        <div className="calibrate-actions">
          {rect && (
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

      <div className="calibrate-stage-wrap" ref={wrapRef}>
        {err && <div className="calibrate-error">Couldn't capture the screen: {err}</div>}
        {!err && !cap && <div className="calibrate-error">Capturing screen…</div>}
        {cap && stage.w > 0 && (
          <div
            className="calibrate-stage"
            ref={stageRef}
            style={{ width: stage.w, height: stage.h }}
            onMouseDown={onStageDown}
            onMouseMove={onMove}
            onMouseUp={onUp}
          >
            <img className="calibrate-shot" src={cap.dataUrl} draggable={false} alt="" />
            {rect && (
              <div
                className="calibrate-box"
                style={{ left: rect.x, top: rect.y, width: rect.w, height: rect.h }}
                onMouseDown={onBoxDown}
              >
                <span className="calibrate-size">
                  {Math.round(rect.w * scale)} × {Math.round(rect.h * scale)} px
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
        )}
      </div>
    </div>
  );
}
