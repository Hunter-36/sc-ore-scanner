//! Background scan loop: capture the screen, detect ore via scanner-core, and
//! emit results to the overlay through a Tauri event. Replaces the v1 Python
//! backend + WebSocket entirely.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use scanner_core::{
    config::Config,
    ocr::Ocr,
    pipeline::detect_ores,
    prices::{PriceCache, DEFAULT_FEED_URL},
    resolver::Resolver,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

/// Re-fetch the price feed at most once an hour (it updates hourly upstream).
const PRICE_REFRESH: Duration = Duration::from_secs(3600);

#[derive(Serialize, Clone)]
struct OreOut {
    name: String,
    quantity: i64,
    tier: String,
    tier_value: i64,
    volatile: bool,
    confidence: f64,
    detected_rs: i64,
    /// Best sell price per SCU (aUEC), if the feed knows this ore.
    unit_price: Option<i64>,
}

#[derive(Serialize, Clone)]
struct ScanResult {
    ores: HashMap<String, OreOut>,
    scanner_active: bool,
    timestamp: f64,
}

/// Capture the primary monitor as an RGB image (alpha dropped). Targets the
/// primary monitor explicitly so it matches the calibration overlay's coords.
fn capture_primary() -> anyhow::Result<image::RgbImage> {
    let monitors = xcap::Monitor::all()?;
    let monitor = monitors
        .iter()
        .find(|m| m.is_primary())
        .or_else(|| monitors.first())
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
        log::info!("Loading OCR engine (embedded models)...");
        let ocr = match Ocr::new() {
            Ok(o) => o,
            Err(e) => {
                log::error!("OCR init failed: {e}");
                return;
            }
        };
        log::info!("OCR engine ready.");
        let resolver = Resolver::new();

        let mut prices = PriceCache::new(DEFAULT_FEED_URL);
        match prices.refresh() {
            Ok(()) => log::info!("Loaded {} ore prices (UEX).", prices.len()),
            Err(e) => log::warn!("price feed unavailable ({e}); cards will omit price."),
        }
        let mut last_price_refresh = Instant::now();

        let config_path = app
            .path()
            .app_config_dir()
            .map(|d| d.join("config.json"))
            .unwrap_or_else(|_| PathBuf::from("config.json"));

        loop {
            let cfg = Config::load(&config_path);
            let interval = Duration::from_secs_f64(cfg.scan_interval_secs.max(0.2));

            // Refresh prices hourly. Reset the timer even on failure so we don't
            // hammer the feed every cycle while it's down.
            if last_price_refresh.elapsed() >= PRICE_REFRESH {
                if let Err(e) = prices.refresh() {
                    log::warn!("price refresh failed: {e}");
                }
                last_price_refresh = Instant::now();
            }

            if let Some(region) = cfg.scan_region {
                match capture_primary()
                    .and_then(|img| detect_ores(&img, Some(region), cfg.upscale, &ocr, &resolver))
                {
                    Ok(agg) => {
                        let ores: HashMap<String, OreOut> = agg
                            .into_iter()
                            .map(|(id, m)| {
                                let unit_price = prices.sell_price(&id);
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
                                        unit_price,
                                    },
                                )
                            })
                            .collect();
                        let result = ScanResult { ores, scanner_active: true, timestamp: 0.0 };
                        let _ = app.emit("scan-result", result);
                    }
                    Err(e) => log::error!("scan: {e}"),
                }
            }

            std::thread::sleep(interval);
        }
    });
}
