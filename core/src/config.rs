//! Runtime configuration (scan region, interval, OCR tuning). Stored as JSON in
//! the app config dir. Defaults mirror the v1 Python `settings.json`.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Calibrated capture region [x, y, width, height]; None until calibrated.
    #[serde(default)]
    pub scan_region: Option<[u32; 4]>,
    #[serde(default = "default_interval")]
    pub scan_interval_secs: f64,
    /// Upscale factor applied to the cropped region before OCR.
    #[serde(default = "default_scale")]
    pub upscale: u32,
    /// Number must be detected this many consecutive frames before it's reported.
    #[serde(default = "default_min_frames")]
    pub min_consecutive_frames: u32,
    /// CLAHE contrast clip limit.
    #[serde(default = "default_clahe_clip")]
    pub clahe_clip_limit: f64,
    /// CLAHE tile grid [cols, rows].
    #[serde(default = "default_clahe_grid")]
    pub clahe_grid: [u32; 2],
    /// Body the user is mining at (e.g. "Cellin"). When set, ambiguous candidates are
    /// ranked/filtered by per-location spawn probability. None = no location filtering.
    #[serde(default)]
    pub mining_location: Option<String>,
}

fn default_interval() -> f64 {
    // Rust scans far faster than the Python v1, so we don't need its 2s gap.
    // 0.75s x 3-frame debounce -> ~2.25s to confirm, while staying light on CPU.
    0.75
}
fn default_scale() -> u32 {
    4
}
fn default_min_frames() -> u32 {
    3
}
fn default_clahe_clip() -> f64 {
    0.0 // CLAHE off by default; ocrs reads raw upscaled text better (see preprocess)
}
fn default_clahe_grid() -> [u32; 2] {
    [8, 8]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan_region: None,
            scan_interval_secs: default_interval(),
            upscale: default_scale(),
            min_consecutive_frames: default_min_frames(),
            clahe_clip_limit: default_clahe_clip(),
            clahe_grid: default_clahe_grid(),
            mining_location: None,
        }
    }
}

impl Config {
    /// Load config from `path`, falling back to defaults if missing/invalid.
    ///
    /// Values are clamped to the same ranges the `set_config` Tauri command
    /// enforces, so a hand-edited or migrated `config.json` can't feed
    /// out-of-range values into the scan loop. The worst case this guards
    /// against: `min_consecutive_frames` above the debouncer's history window,
    /// which would make detection silently never confirm.
    pub fn load(path: &Path) -> Self {
        let mut cfg: Self = std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        cfg.clamp();
        cfg
    }

    /// Clamp every tunable to a sane range (mirrors `set_config`). Non-finite
    /// floats fall back to their defaults rather than propagating NaN.
    pub fn clamp(&mut self) {
        if !self.scan_interval_secs.is_finite() {
            self.scan_interval_secs = default_interval();
        }
        self.scan_interval_secs = self.scan_interval_secs.clamp(0.3, 5.0);

        self.min_consecutive_frames = self.min_consecutive_frames.clamp(1, 6);
        self.upscale = self.upscale.clamp(1, 6);

        if !self.clahe_clip_limit.is_finite() {
            self.clahe_clip_limit = default_clahe_clip();
        }
        self.clahe_clip_limit = self.clahe_clip_limit.clamp(0.0, 8.0);

        // A zero-dimension CLAHE tile grid would divide by zero in preprocess.
        self.clahe_grid = [self.clahe_grid[0].max(1), self.clahe_grid[1].max(1)];

        // A degenerate (zero-width or zero-height) region can't be cropped;
        // drop it so the app behaves as if uncalibrated rather than crashing.
        if let Some([_, _, w, h]) = self.scan_region {
            if w == 0 || h == 0 {
                self.scan_region = None;
            }
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        // Write to a temp file then rename, so a concurrent reader (the scan loop
        // re-reads config every cycle) never sees a half-written file and falls
        // back to defaults — which would briefly drop the scan region. The temp
        // name is unique per write (pid + counter) so rapid concurrent saves from
        // the settings UI never collide on the same temp file.
        static SAVE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SAVE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp = path.with_extension(format!("{}.{seq}.tmp", std::process::id()));
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, path)
    }
}
