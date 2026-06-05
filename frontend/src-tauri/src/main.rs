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
    let win = match WebviewWindowBuilder::new(
        &app,
        "calibrate",
        WebviewUrl::App("index.html".into()),
    )
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

    // Cover the PRIMARY monitor — the same one capture_primary() scans — so the
    // dragged box maps 1:1 to the capture, regardless of which monitor the
    // overlay window happens to live on.
    if let Ok(Some(m)) = win.primary_monitor() {
        let p = m.position();
        let s = m.size();
        log::info!(
            "calibration overlay -> primary monitor ({}, {}) {}x{}",
            p.x, p.y, s.width, s.height
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
        return Err(format!("scan region too small ({w}x{h}); draw a larger box"));
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
    Ok(())
}

/// Exit the whole app. Uses process::exit so it always terminates regardless of
/// open windows or the in-process scan thread.
#[tauri::command]
fn quit() {
    log::info!("quit requested");
    std::process::exit(0);
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

/// Log to `<app config>/logs/scanner.log`. Falls back silently to no file
/// logging if the path can't be created.
fn init_logging() {
    let log_path = log_dir().map(|dir| {
        let _ = std::fs::create_dir_all(&dir);
        dir.join("scanner.log")
    });

    if let Some(path) = log_path {
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = simplelog::WriteLogger::init(
                simplelog::LevelFilter::Info,
                simplelog::Config::default(),
                file,
            );
        }
    }
}

fn main() {
    init_logging();
    log::info!("SC Ore Scanner v{} starting.", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
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
        .invoke_handler(tauri::generate_handler![open_calibration, save_scan_region, quit])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
