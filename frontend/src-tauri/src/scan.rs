//! Background scan loop: capture the screen, detect ore via scanner-core, and
//! emit results to the overlay through a Tauri event. Replaces the v1 Python
//! backend + WebSocket entirely.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use scanner_core::{
    config::Config,
    debounce::Debouncer,
    ocr::Ocr,
    pipeline::{recognize_rs_numbers_from_processed, resolve_and_aggregate},
    preprocess::preprocess_for_ocr,
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
    /// False until a scan region has been calibrated — lets the overlay prompt
    /// the user to set one instead of sitting on "Starting scanner…".
    configured: bool,
    timestamp: f64,
}

fn logs_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("logs")))
}

/// Capture the primary monitor as an RGB image (alpha dropped). Targets the
/// primary monitor explicitly so it matches the calibration overlay's coords.
pub fn capture_primary() -> anyhow::Result<image::RgbImage> {
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
        log::info!("Scan loop started; config at {}", config_path.display());

        // Debounce raw RS numbers: a number must appear in N consecutive frames
        // before it's reported (faithful to v1; filters transient OCR misreads).
        let mut debouncer = Debouncer::new(Config::load(&config_path).min_consecutive_frames);

        // Track state so we log on change instead of every cycle (no spam).
        let mut last_region: Option<Option<[u32; 4]>> = None;
        let mut last_summary = String::new();

        loop {
            let cfg = Config::load(&config_path);
            let interval = Duration::from_secs_f64(cfg.scan_interval_secs.max(0.2));

            if last_region != Some(cfg.scan_region) {
                match cfg.scan_region {
                    Some(r) => log::info!("Scan region set to [{}, {}, {}, {}].", r[0], r[1], r[2], r[3]),
                    None => log::info!("No scan region set — click ‘Set region’ to calibrate."),
                }
                last_region = Some(cfg.scan_region);
                last_summary.clear();
            }

            // Refresh prices hourly. Reset the timer even on failure so we don't
            // hammer the feed every cycle while it's down.
            if last_price_refresh.elapsed() >= PRICE_REFRESH {
                match prices.refresh() {
                    Ok(()) => log::info!("Refreshed prices ({} ores).", prices.len()),
                    Err(e) => log::warn!("price refresh failed: {e}"),
                }
                last_price_refresh = Instant::now();
            }

            match cfg.scan_region {
                // No region yet — tell the overlay so it can prompt for calibration.
                None => {
                    debouncer.reset();
                    let _ = app.emit(
                        "scan-result",
                        ScanResult {
                            ores: HashMap::new(),
                            scanner_active: false,
                            configured: false,
                            timestamp: 0.0,
                        },
                    );
                }
                Some(region) => match capture_primary() {
                    Ok(img) => {
                        // Preprocess once (crop/upscale/grayscale/CLAHE); save it for
                        // inspection, then OCR it.
                        let processed = preprocess_for_ocr(
                            &img,
                            Some(region),
                            cfg.upscale,
                            cfg.clahe_clip_limit,
                            cfg.clahe_grid,
                        );
                        if let Some(dir) = logs_dir() {
                            let _ = processed.save(dir.join("last_scan.png"));
                        }

                        match recognize_rs_numbers_from_processed(&processed, &ocr, &resolver) {
                            Ok(candidates) => {
                                // Debounce, then resolve only the confirmed numbers.
                                debouncer.update(&candidates);
                                let confirmed = debouncer.confirmed();
                                let agg = resolve_and_aggregate(&confirmed, &resolver);

                                // Log only when the detected set changes.
                                let mut names: Vec<String> = agg
                                    .values()
                                    .map(|m| {
                                        format!("{}x {} (rs {})", m.quantity, m.ore.name, m.detected_rs)
                                    })
                                    .collect();
                                names.sort();
                                let summary = if names.is_empty() {
                                    "no ores in view".to_string()
                                } else {
                                    names.join(", ")
                                };
                                if summary != last_summary {
                                    log::info!("Detected: {summary}");
                                    last_summary = summary;
                                }

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
                                let _ = app.emit(
                                    "scan-result",
                                    ScanResult {
                                        ores,
                                        scanner_active: true,
                                        configured: true,
                                        timestamp: 0.0,
                                    },
                                );
                            }
                            Err(e) => {
                                let msg = format!("ocr error: {e}");
                                if msg != last_summary {
                                    log::error!("{msg}");
                                    last_summary = msg;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let msg = format!("capture error: {e}");
                        if msg != last_summary {
                            log::error!("{msg}");
                            last_summary = msg;
                        }
                    }
                },
            }

            std::thread::sleep(interval);
        }
    });
}
