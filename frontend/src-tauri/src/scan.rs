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
    /// Equally-likely alternative readings of the same RS (ambiguous signature),
    /// e.g. ["5x Aslarite"]; empty when unambiguous. Mirrors OreData.alternatives.
    alternatives: Vec<String>,
}

/// Emitted as the "scan-result" Tauri event. Mirrors `ScanResult` in
/// frontend/src/store/useOreStore.ts — keep the two in sync.
#[derive(Serialize, Clone)]
struct ScanResult {
    ores: HashMap<String, OreOut>,
    scanner_active: bool,
    /// False until a scan region has been calibrated — lets the overlay prompt
    /// the user to set one instead of sitting on "Starting scanner…".
    configured: bool,
}

/// Capture the calibrated `region` of the primary monitor as RGB. The full frame
/// is grabbed (xcap), but only the region's pixels are converted to RGB — a 4K
/// grab is ~33 MB while the scan region is a few thousand pixels.
pub fn capture_region(region: [u32; 4]) -> anyhow::Result<image::RgbImage> {
    let monitors = xcap::Monitor::all()?;
    let monitor = monitors
        .iter()
        .find(|m| m.is_primary())
        .or_else(|| monitors.first())
        .ok_or_else(|| anyhow::anyhow!("no monitor found"))?;
    let rgba = monitor.capture_image()?;
    let (fw, fh) = (rgba.width(), rgba.height());
    Ok(scanner_core::preprocess::crop_rgba_to_rgb(
        rgba.as_raw(),
        fw,
        fh,
        region,
    ))
}

/// Run one scan cycle's work under panic protection. Returns `true` if it
/// completed, `false` if it panicked (which is logged). The scan loop runs on a
/// detached thread, so without this a single bad frame (a panic anywhere in
/// capture/OCR/resolve) would end scanning permanently and the overlay would
/// just go quiet. `AssertUnwindSafe` is sound here: on a panic we discard the
/// frame and reset the debouncer, so no observer sees inconsistent state.
fn run_guarded(f: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_ok()
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
        let mut min_frames = Config::load(&config_path).min_consecutive_frames;
        let mut debouncer = Debouncer::new(min_frames);

        // Track state so we log on change instead of every cycle (no spam).
        let mut last_region: Option<Option<[u32; 4]>> = None;
        let mut last_summary = String::new();

        loop {
            let cfg = Config::load(&config_path);
            let interval = Duration::from_secs_f64(cfg.scan_interval_secs.max(0.2));

            // Live-apply a changed confirm-frames setting WITHOUT rebuilding the
            // debouncer — set it in place so the currently-shown ore doesn't drop
            // while the user is tuning the slider.
            if cfg.min_consecutive_frames != min_frames {
                min_frames = cfg.min_consecutive_frames;
                debouncer.set_min_frames(min_frames);
            }

            if last_region != Some(cfg.scan_region) {
                match cfg.scan_region {
                    Some(r) => log::info!(
                        "Scan region set to [{}, {}, {}, {}].",
                        r[0],
                        r[1],
                        r[2],
                        r[3]
                    ),
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

            let cycle_ok = run_guarded(|| {
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
                            },
                        );
                    }
                    Some(region) => match capture_region(region) {
                        Ok(img) => {
                            // Already cropped to the region, so just upscale (+CLAHE).
                            let processed = preprocess_for_ocr(
                                &img,
                                None,
                                cfg.upscale,
                                cfg.clahe_clip_limit,
                                cfg.clahe_grid,
                            );

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
                                            let alt = if m.alternatives.is_empty() {
                                                String::new()
                                            } else {
                                                format!(" [or {}]", m.alternatives.join(", "))
                                            };
                                            format!(
                                                "{}x {} (rs {}){}",
                                                m.quantity, m.ore.name, m.detected_rs, alt
                                            )
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
                                                    confidence: (m.confidence * 100.0).round()
                                                        / 100.0,
                                                    detected_rs: m.detected_rs,
                                                    unit_price,
                                                    alternatives: m.alternatives,
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
            });
            if !cycle_ok {
                // A panic in capture/OCR/resolve must not kill the session. Log,
                // drop the frame, and reset the debouncer (its in-place state may
                // be inconsistent) so the next cycle starts clean.
                log::error!("scan cycle panicked; resetting debouncer and continuing");
                debouncer.reset();
            }

            std::thread::sleep(interval);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::run_guarded;

    #[test]
    fn guarded_cycle_survives_panic() {
        // Silence the default panic hook so the deliberate panic doesn't spam
        // test output; restore it afterwards.
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        let mut ran_after = false;
        let panicked = !run_guarded(|| panic!("boom"));
        // A subsequent cycle still runs — the loop is not dead.
        let ok = run_guarded(|| ran_after = true);

        std::panic::set_hook(prev);

        assert!(panicked, "a panicking cycle is caught, not propagated");
        assert!(
            ok && ran_after,
            "the next cycle runs normally after a panic"
        );
    }
}
