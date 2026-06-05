//! Runtime configuration (scan region, interval). Stored as JSON in the app
//! config dir. The scan region is [x, y, width, height] in screen pixels.

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
}

fn default_interval() -> f64 {
    2.0
}
fn default_scale() -> u32 {
    4
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan_region: None,
            scan_interval_secs: default_interval(),
            upscale: default_scale(),
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
