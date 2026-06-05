//! Background scan loop: capture the screen, detect ore via scanner-core, and
//! emit results to the overlay through a Tauri event. Replaces the v1 Python
//! backend + WebSocket entirely.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use scanner_core::{config::Config, ocr::Ocr, pipeline::detect_ores, resolver::Resolver};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Clone)]
struct OreOut {
    name: String,
    quantity: i64,
    tier: String,
    tier_value: i64,
    volatile: bool,
    confidence: f64,
    detected_rs: i64,
}

#[derive(Serialize, Clone)]
struct ScanResult {
    ores: HashMap<String, OreOut>,
    scanner_active: bool,
    timestamp: f64,
}

fn models_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("models")))
        .unwrap_or_else(|| PathBuf::from("models"))
}

/// Capture the primary monitor as an RGB image (alpha dropped).
fn capture_primary() -> anyhow::Result<image::RgbImage> {
    let monitor = xcap::Monitor::all()?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("no monitor found"))?;
    let rgba = monitor.capture_image()?;
    let (w, h) = (rgba.width(), rgba.height());
    let raw = rgba.into_raw();
    let mut rgb = image::RgbImage::new(w, h);
    for (i, px) in rgb.pixels_mut().enumerate() {
        let o = i * 4;
        *px = image::Rgb([raw[o], raw[o + 1], raw[o + 2]]);
    }
    Ok(rgb)
}

/// Spawn the scan loop on a background thread. It reads the scan region from the
/// app config each cycle, so calibration takes effect without a restart.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let ocr = match Ocr::new(&models_dir()) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[scan] OCR init failed: {e}");
                return;
            }
        };
        let resolver = Resolver::new();
        let config_path = app
            .path()
            .app_config_dir()
            .map(|d| d.join("config.json"))
            .unwrap_or_else(|_| PathBuf::from("config.json"));

        loop {
            let cfg = Config::load(&config_path);
            let interval = Duration::from_secs_f64(cfg.scan_interval_secs.max(0.2));

            if let Some(region) = cfg.scan_region {
                match capture_primary().and_then(|img| detect_ores(&img, Some(region), cfg.upscale, &ocr, &resolver)) {
                    Ok(agg) => {
                        let ores: HashMap<String, OreOut> = agg
                            .into_iter()
                            .map(|(id, m)| {
                                (
                                    id,
                                    OreOut {
                                        name: m.ore.name,
                                        quantity: m.quantity,
                                        tier: m.ore.tier,
                                        tier_value: m.ore.tier_value,
                                        volatile: m.ore.volatile,
                                        confidence: (m.confidence * 100.0).round() / 100.0,
                                        detected_rs: m.detected_rs,
                                    },
                                )
                            })
                            .collect();
                        let result = ScanResult { ores, scanner_active: true, timestamp: 0.0 };
                        let _ = app.emit("scan-result", result);
                    }
                    Err(e) => eprintln!("[scan] {e}"),
                }
            }

            std::thread::sleep(interval);
        }
    });
}
