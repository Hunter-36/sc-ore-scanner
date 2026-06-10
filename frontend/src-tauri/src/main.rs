// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};
use tauri_plugin_window_state::StateFlags;

mod scan;

/// Open the calibration overlay: a full-screen, semi-transparent window over the
/// PRIMARY monitor (the one the scan loop captures). Drag a box and release to
/// save — mirrors the old Python calibrate.py. Re-uses the window if already open.
///
/// MUST be async: a synchronous command runs on the main thread, and building a
/// window there deadlocks (the window appears but build() never returns, freezing
/// the whole app — which is why save/quit stopped working). As an async command
/// this runs off the main thread, so window creation dispatches to the free event
/// loop.
#[tauri::command]
async fn open_calibration(app: AppHandle) -> Result<(), String> {
    log::info!("open_calibration requested");
    if let Some(w) = app.get_webview_window("calibrate") {
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }

    log::info!("building calibration window…");
    let win =
        match WebviewWindowBuilder::new(&app, "calibrate", WebviewUrl::App("index.html".into()))
            .title("Calibrate scan region")
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .build()
        {
            Ok(w) => w,
            Err(e) => {
                log::error!("calibration window build failed: {e}");
                return Err(e.to_string());
            }
        };
    log::info!("calibration window built");

    // Cover the PRIMARY monitor — the same one the scan loop captures — so the
    // dragged box maps 1:1 to the capture, regardless of which monitor the
    // overlay window happens to live on.
    if let Ok(Some(m)) = win.primary_monitor() {
        let p = m.position();
        let s = m.size();
        log::info!(
            "calibration overlay -> primary monitor ({}, {}) {}x{}",
            p.x,
            p.y,
            s.width,
            s.height
        );
        let _ = win.set_position(PhysicalPosition::new(p.x, p.y));
        let _ = win.set_size(PhysicalSize::new(s.width, s.height));
    } else {
        log::warn!("no primary monitor info; maximizing calibration overlay");
        let _ = win.maximize();
    }
    let _ = win.show();
    let _ = win.set_focus();
    Ok(())
}

/// Persist a calibrated scan region [x, y, w, h] (physical px) into config.json,
/// preserving other settings. The scan loop re-reads config each cycle.
#[tauri::command]
fn save_scan_region(app: AppHandle, x: u32, y: u32, w: u32, h: u32) -> Result<(), String> {
    // Don't trust the frontend: a zero/tiny region would make the crop empty and
    // detection silently fail. Require a sane minimum.
    const MIN: u32 = 8;
    if w < MIN || h < MIN {
        return Err(format!(
            "scan region too small ({w}x{h}); draw a larger box"
        ));
    }
    let path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("config.json");
    let mut cfg = scanner_core::config::Config::load(&path);
    cfg.scan_region = Some([x, y, w, h]);
    cfg.save(&path).map_err(|e| e.to_string())?;
    log::info!("calibrated scan region: [{x}, {y}, {w}, {h}]");
    // The frontend warns the user about this; log it too so a flaky-detection
    // report is easy to diagnose (the RS text is below the readable floor even at
    // max upscale — issue #110).
    if h < scanner_core::preprocess::MIN_READABLE_REGION_HEIGHT {
        log::warn!(
            "scan region is only {h}px tall (< {}px) — RS text may be too small for reliable OCR; \
             suggest a tighter box or higher in-game HUD/render scale",
            scanner_core::preprocess::MIN_READABLE_REGION_HEIGHT
        );
    }
    Ok(())
}

/// Exit the whole app. Uses process::exit so it always terminates regardless of
/// open windows or the in-process scan thread.
#[tauri::command]
fn quit() {
    log::info!("quit requested");
    std::process::exit(0);
}

fn config_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("config.json"))
}

/// Return the current runtime config for the settings UI.
#[tauri::command]
fn get_config(app: AppHandle) -> Result<scanner_core::config::Config, String> {
    Ok(scanner_core::config::Config::load(&config_path(&app)?))
}

/// The tunable subset the settings UI can change (snake_case to match serde, so
/// the JS side passes the same field names — no camelCase ambiguity).
#[derive(serde::Deserialize)]
struct SettingsUpdate {
    scan_interval_secs: f64,
    min_consecutive_frames: u32,
    upscale: u32,
    clahe_clip_limit: f64,
    /// Selected mining body (or null = no location filtering). Sent by the UI.
    #[serde(default)]
    mining_location: Option<String>,
}

/// Update the tunable detection settings, clamped to sane ranges, preserving the
/// scan region. The scan loop hot-reloads config each cycle, so changes apply live.
#[tauri::command]
fn set_config(app: AppHandle, update: SettingsUpdate) -> Result<(), String> {
    let path = config_path(&app)?;
    let mut cfg = scanner_core::config::Config::load(&path);
    cfg.scan_interval_secs = update.scan_interval_secs.clamp(0.3, 5.0);
    cfg.min_consecutive_frames = update.min_consecutive_frames.clamp(1, 6);
    cfg.upscale = update.upscale.clamp(1, 6);
    cfg.clahe_clip_limit = update.clahe_clip_limit.clamp(0.0, 8.0);
    cfg.mining_location = update.mining_location.filter(|s| !s.is_empty());
    cfg.save(&path).map_err(|e| e.to_string())?;
    log::info!(
        "settings updated: interval={:.2}s frames={} upscale={} clahe={:.1} location={:?}",
        cfg.scan_interval_secs,
        cfg.min_consecutive_frames,
        cfg.upscale,
        cfg.clahe_clip_limit,
        cfg.mining_location
    );
    Ok(())
}

#[derive(serde::Serialize)]
struct MiningLocation {
    system: String,
    body: String,
}

/// Distinct mining bodies for the settings location picker, sorted by (system, body).
#[tauri::command]
fn get_mining_locations() -> Vec<MiningLocation> {
    let mut set: std::collections::BTreeSet<(String, String)> = std::collections::BTreeSet::new();
    for ore in scanner_core::signatures::load_signatures() {
        for loc in ore.locations {
            if !loc.body.is_empty() {
                set.insert((loc.system, loc.body));
            }
        }
    }
    set.into_iter()
        .map(|(system, body)| MiningLocation { system, body })
        .collect()
}

/// Directory for the log file: `%APPDATA%\com.scorescanner.app\logs` (matches the
/// Tauri app_config_dir on Windows), so logging works even when the app is
/// installed read-only under Program Files. Falls back to next to the exe.
fn log_dir() -> Option<std::path::PathBuf> {
    if let Ok(appdata) = std::env::var("APPDATA") {
        return Some(
            std::path::PathBuf::from(appdata)
                .join("com.scorescanner.app")
                .join("logs"),
        );
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join("logs")))
}

/// Sensitive substrings to scrub from every log line, paired with their
/// placeholder. The log records the config path on startup (and paths in other
/// lines), which on Windows embeds the user's name — mild PII in a file people
/// attach to bug reports. Longest first so the full home path is redacted before
/// the bare username it contains.
fn redaction_needles() -> Vec<(String, &'static str)> {
    let mut needles: Vec<(String, &'static str)> = Vec::new();
    if let Ok(p) = std::env::var("USERPROFILE") {
        if !p.is_empty() {
            needles.push((p, "%USERPROFILE%"));
        }
    }
    if let Ok(h) = std::env::var("HOME") {
        if !h.is_empty() {
            needles.push((h, "$HOME"));
        }
    }
    if let Ok(u) = std::env::var("USERNAME").or_else(|_| std::env::var("USER")) {
        if !u.is_empty() {
            needles.push((u, "<user>"));
        }
    }
    needles.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    needles
}

/// A `Write` wrapper that redacts sensitive substrings from each line before it
/// reaches the log file. Best-effort: it redacts per write call (simplelog
/// writes a record at a time), which covers the path lines we care about.
struct RedactingWriter<W: std::io::Write> {
    inner: W,
    needles: Vec<(String, &'static str)>,
}

impl<W: std::io::Write> RedactingWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            needles: redaction_needles(),
        }
    }
}

impl<W: std::io::Write> std::io::Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.needles.is_empty() {
            return self.inner.write(buf);
        }
        match std::str::from_utf8(buf) {
            Ok(s) => {
                let mut red = s.to_owned();
                for (needle, repl) in &self.needles {
                    if red.contains(needle.as_str()) {
                        red = red.replace(needle.as_str(), repl);
                    }
                }
                self.inner.write_all(red.as_bytes())?;
                // Report the whole input as consumed; the redacted form may
                // differ in length but the caller's buffer is fully handled.
                Ok(buf.len())
            }
            // Non-UTF-8 (shouldn't happen for log text) passes through untouched.
            Err(_) => self.inner.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Max log size before it's rolled at startup. The log only writes on state
/// changes (not per frame), so a single roll to `scanner.log.1` bounds disk use
/// at ~2× this without a per-write rotation check on the hot path.
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

/// Log level from the `SC_ORE_LOG` env var (trace/debug/info/warn/error/off),
/// defaulting to Info. Lets a user capture detection-level detail (the OCR line
/// dumps logged at debug) for a bug report without a custom build.
fn log_level() -> simplelog::LevelFilter {
    use simplelog::LevelFilter;
    match std::env::var("SC_ORE_LOG")
        .ok()
        .as_deref()
        .map(|s| s.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("trace") => LevelFilter::Trace,
        Some("debug") => LevelFilter::Debug,
        Some("warn") => LevelFilter::Warn,
        Some("error") => LevelFilter::Error,
        Some("off") => LevelFilter::Off,
        _ => LevelFilter::Info,
    }
}

/// Roll `scanner.log` to `scanner.log.1` (replacing any previous roll) once it
/// exceeds `MAX_LOG_BYTES`, so an append-only log can't grow without bound over
/// months of sessions. Best-effort: any failure leaves the current log in place.
fn rotate_log_if_large(path: &std::path::Path) {
    rotate_if_over(path, MAX_LOG_BYTES);
}

/// Rotation core, parameterised on the size cap so it's testable without writing
/// a multi-MB file.
fn rotate_if_over(path: &std::path::Path, max_bytes: u64) {
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > max_bytes {
            let _ = std::fs::rename(path, path.with_extension("log.1"));
        }
    }
}

/// Route panics to the log file. Release builds have no console
/// (`windows_subsystem = "windows"`), so without this a panic on the main thread
/// — or one caught by the scan loop's guard — leaves nothing in scanner.log. The
/// message goes through the same `RedactingWriter`, so any path in it is scrubbed.
/// Chains to the previous hook (stderr in debug builds).
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown location".to_string());
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("<non-string panic payload>");
        log::error!("PANIC at {location}: {msg}");
        previous(info);
    }));
}

/// Log to `<app config>/logs/scanner.log`. Falls back silently to no file
/// logging if the path can't be created. Log lines are scrubbed of the user's
/// home path / username (see `RedactingWriter`).
fn init_logging() {
    let log_path = log_dir().map(|dir| {
        let _ = std::fs::create_dir_all(&dir);
        dir.join("scanner.log")
    });

    if let Some(path) = log_path {
        rotate_log_if_large(&path);
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = simplelog::WriteLogger::init(
                log_level(),
                simplelog::Config::default(),
                RedactingWriter::new(file),
            );
        }
    }
}

fn main() {
    init_logging();
    install_panic_hook();
    log::info!("SC Ore Scanner v{} starting.", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        // Single-instance: two overlays would both run the scan loop and fight
        // over screen capture. Registered FIRST (per the plugin's guidance). When
        // a second launch is attempted, this callback runs in the EXISTING
        // instance — focus its window — and the second process exits.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        // Remember only the overlay's POSITION across launches (not size), so the
        // window dimensions always come from the config — otherwise a stored size
        // would override height changes shipped in updates. State lives in
        // <app config>/.window-state.json.
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::POSITION)
                .build(),
        )
        // Belt-and-suspenders: if the main overlay window is ever destroyed (e.g.
        // the quit fallback closes it), exit the whole app.
        .on_window_event(|window, event| {
            if matches!(event, WindowEvent::Destroyed) && window.label() == "main" {
                window.app_handle().exit(0);
            }
        })
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                // On first run (no saved window state yet) pin the overlay to the
                // top-right of its monitor at any resolution. After that, the
                // window-state plugin restores wherever the user last moved it.
                let has_saved_state = app
                    .path()
                    .app_config_dir()
                    .map(|dir| dir.join(".window-state.json").exists())
                    .unwrap_or(false);

                if !has_saved_state {
                    if let (Ok(Some(monitor)), Ok(win_size)) =
                        (window.current_monitor(), window.outer_size())
                    {
                        let m_pos = monitor.position();
                        let m_size = monitor.size();
                        let margin: i32 = 20;
                        let x = m_pos.x + (m_size.width as i32) - (win_size.width as i32) - margin;
                        let y = m_pos.y + margin;
                        let _ = window.set_position(PhysicalPosition::new(x, y));
                    }
                }
            }

            // Start the Rust scan loop (capture -> detect -> emit "scan-result").
            scan::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_calibration,
            save_scan_region,
            get_config,
            set_config,
            get_mining_locations,
            quit
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{rotate_if_over, RedactingWriter};
    use std::io::Write;

    #[test]
    fn rotates_only_when_over_the_cap() {
        let dir = std::env::temp_dir().join(format!("sc-ore-rot-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let log = dir.join("scanner.log");
        let rolled = log.with_extension("log.1");
        let _ = std::fs::remove_file(&rolled);
        std::fs::write(&log, b"0123456789").unwrap(); // 10 bytes

        // Under the cap: left in place, no roll.
        rotate_if_over(&log, 100);
        assert!(log.exists(), "small log is not rotated");
        assert!(!rolled.exists());

        // Over the cap: moved aside to scanner.log.1.
        rotate_if_over(&log, 5);
        assert!(!log.exists(), "oversized log is moved aside");
        assert!(rolled.exists(), "rolled to scanner.log.1");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn redacts_home_path_and_username() {
        let mut w = RedactingWriter {
            inner: Vec::<u8>::new(),
            // Longest first, as redaction_needles() orders them.
            needles: vec![
                ("C:\\Users\\alice".to_string(), "%USERPROFILE%"),
                ("alice".to_string(), "<user>"),
            ],
        };
        write!(
            w,
            "Scan loop started; config at C:\\Users\\alice\\AppData\\Roaming\\app\\config.json (alice)"
        )
        .unwrap();

        let out = String::from_utf8(w.inner).unwrap();
        assert!(!out.contains("alice"), "username must be scrubbed: {out}");
        assert!(out.contains("%USERPROFILE%\\AppData\\Roaming\\app\\config.json"));
        assert!(out.contains("(<user>)"));
    }

    #[test]
    fn no_needles_passes_through() {
        let mut w = RedactingWriter {
            inner: Vec::<u8>::new(),
            needles: Vec::new(),
        };
        write!(w, "nothing to redact here").unwrap();
        assert_eq!(
            String::from_utf8(w.inner).unwrap(),
            "nothing to redact here"
        );
    }
}
