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
}

fn default_interval() -> f64 {
    2.0
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
        }
    }
}

impl Config {
    /// Load config from `path`, falling back to defaults if missing/invalid.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self).unwrap())
    }
}
