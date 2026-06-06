import { useEffect, useRef, useState } from 'react';

/** The tunable subset mirrored from the Rust Config (see main.rs SettingsUpdate). */
export interface Cfg {
  scan_interval_secs: number;
  min_consecutive_frames: number;
  upscale: number;
  clahe_clip_limit: number;
}

const DEFAULTS: Cfg = {
  scan_interval_secs: 0.75,
  min_consecutive_frames: 3,
  upscale: 4,
  clahe_clip_limit: 0,
};

type PresetKey = 'Responsive' | 'Balanced' | 'Low-impact';
const PRESETS: Record<PresetKey, Pick<Cfg, 'scan_interval_secs' | 'min_consecutive_frames'>> = {
  Responsive: { scan_interval_secs: 0.4, min_consecutive_frames: 2 },
  Balanced: { scan_interval_secs: 0.75, min_consecutive_frames: 3 },
  'Low-impact': { scan_interval_secs: 1.5, min_consecutive_frames: 3 },
};

async function loadConfig(): Promise<Cfg> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const c = await invoke<Cfg>('get_config');
    return {
      scan_interval_secs: c.scan_interval_secs ?? DEFAULTS.scan_interval_secs,
      min_consecutive_frames: c.min_consecutive_frames ?? DEFAULTS.min_consecutive_frames,
      upscale: c.upscale ?? DEFAULTS.upscale,
      clahe_clip_limit: c.clahe_clip_limit ?? DEFAULTS.clahe_clip_limit,
    };
  } catch {
    return DEFAULTS;
  }
}

/// Persist settings. Returns null on success OR when not running under Tauri
/// (e.g. browser dev — nothing to surface), and an error string when a genuine
/// `set_config` call is rejected (I/O / permission) so the UI can show it.
async function saveConfig(update: Cfg): Promise<string | null> {
  let core: typeof import('@tauri-apps/api/core');
  try {
    core = await import('@tauri-apps/api/core');
  } catch {
    return null; // not in Tauri — indistinguishable from "no backend", don't alarm
  }
  try {
    await core.invoke('set_config', { update });
    return null;
  } catch (e) {
    console.warn('set_config failed:', e);
    return String(e);
  }
}

const near = (a: number, b: number) => Math.abs(a - b) < 1e-9;
// Estimated time from looking at a rock to it showing: interval x frames.
const detectSeconds = (c: Cfg) => (c.scan_interval_secs * c.min_consecutive_frames).toFixed(1);

export function Settings({ onClose }: { onClose: () => void }) {
  const [cfg, setCfg] = useState<Cfg | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const dirty = useRef(false);
  const saveTimer = useRef<number | undefined>(undefined);
  // Latest cfg, readable from the unmount cleanup without making it a dep.
  const latestCfg = useRef<Cfg | null>(null);

  useEffect(() => {
    loadConfig().then(setCfg);
  }, []);

  useEffect(() => {
    latestCfg.current = cfg;
  }, [cfg]);

  // Persist (debounced) after a user change. The `dirty` gate skips the initial
  // load; the timer batches a slider drag into a single write ~200ms after it
  // settles, and the write never runs inside a state updater. The scan loop
  // hot-reloads config, so the value still applies live.
  useEffect(() => {
    if (!cfg || !dirty.current) return;
    if (saveTimer.current !== undefined) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      void saveConfig(cfg).then(setSaveError);
    }, 200);
    return () => {
      if (saveTimer.current !== undefined) window.clearTimeout(saveTimer.current);
    };
  }, [cfg]);

  // Flush a pending edit when the panel unmounts — closing via the gear toggle
  // unmounts without going through handleDone, so the debounce cleanup would
  // otherwise drop an edit made <200ms before the click. Empty deps → this
  // cleanup runs only on unmount.
  useEffect(() => {
    return () => {
      if (saveTimer.current !== undefined) window.clearTimeout(saveTimer.current);
      if (dirty.current && latestCfg.current) void saveConfig(latestCfg.current);
    };
  }, []);

  function update(patch: Partial<Cfg>) {
    dirty.current = true;
    setCfg((prev) => (prev ? { ...prev, ...patch } : prev));
  }

  // Flush any pending debounced write when leaving the panel via Done.
  function handleDone() {
    if (saveTimer.current !== undefined) window.clearTimeout(saveTimer.current);
    if (dirty.current && cfg) void saveConfig(cfg);
    onClose();
  }

  if (!cfg) {
    return <div className="settings message">Loading settings…</div>;
  }

  const activePreset = (Object.keys(PRESETS) as PresetKey[]).find(
    (k) =>
      near(PRESETS[k].scan_interval_secs, cfg.scan_interval_secs) &&
      PRESETS[k].min_consecutive_frames === cfg.min_consecutive_frames,
  );

  return (
    <div className="settings">
      <div className="settings-row settings-presets">
        {(Object.keys(PRESETS) as PresetKey[]).map((k) => (
          <button
            key={k}
            className={`preset-btn ${activePreset === k ? 'active' : ''}`}
            onClick={() => update(PRESETS[k])}
          >
            {k}
          </button>
        ))}
      </div>
      <div className="settings-detect">≈ {detectSeconds(cfg)}s to confirm a reading</div>

      <label className="settings-field">
        <span>
          Scan interval <b>{cfg.scan_interval_secs.toFixed(2)}s</b>
        </span>
        <input
          type="range"
          min={0.3}
          max={3}
          step={0.05}
          value={cfg.scan_interval_secs}
          onChange={(e) => update({ scan_interval_secs: parseFloat(e.target.value) })}
        />
        <span className="settings-hint">lower = faster, more CPU</span>
      </label>

      <label className="settings-field">
        <span>
          Confirm frames <b>{cfg.min_consecutive_frames}</b>
        </span>
        <input
          type="range"
          min={1}
          max={6}
          step={1}
          value={cfg.min_consecutive_frames}
          onChange={(e) => update({ min_consecutive_frames: parseInt(e.target.value, 10) })}
        />
        <span className="settings-hint">lower = faster, more phantom risk</span>
      </label>

      <label className="settings-field">
        <span>
          Upscale <b>{cfg.upscale}×</b>
        </span>
        <input
          type="range"
          min={1}
          max={6}
          step={1}
          value={cfg.upscale}
          onChange={(e) => update({ upscale: parseInt(e.target.value, 10) })}
        />
        <span className="settings-hint">higher = better OCR, more CPU</span>
      </label>

      <label className="settings-field">
        <span>
          Contrast (CLAHE) <b>{cfg.clahe_clip_limit === 0 ? 'off' : cfg.clahe_clip_limit.toFixed(1)}</b>
        </span>
        <input
          type="range"
          min={0}
          max={4}
          step={0.5}
          value={cfg.clahe_clip_limit}
          onChange={(e) => update({ clahe_clip_limit: parseFloat(e.target.value) })}
        />
        <span className="settings-hint">0 = off (recommended); can hurt detection — only for very dark HUDs</span>
      </label>

      {saveError && (
        <div className="settings-error" role="alert" style={{ color: '#ff6b6b', fontSize: '0.85rem' }}>
          Couldn't save settings: {saveError}
        </div>
      )}

      <button className="settings-done" onClick={handleDone}>
        Done
      </button>
    </div>
  );
}
