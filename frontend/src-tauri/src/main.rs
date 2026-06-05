// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_window_state::StateFlags;

mod scan;

/// Open the full-screen calibration overlay (drag-select the scan region). Re-uses
/// the existing window if already open. Covers the primary monitor so the dragged
/// rectangle maps 1:1 to what the scan loop captures.
#[tauri::command]
fn open_calibration(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("calibrate") {
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }

    // Prefer the monitor the overlay is on (reliable), then the primary monitor.
    let monitor = app
        .get_webview_window("main")
        .and_then(|w| w.current_monitor().ok().flatten());

    let win = WebviewWindowBuilder::new(&app, "calibrate", WebviewUrl::App("index.html".into()))
        .title("Calibrate scan region")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(true)
        .build()
        .map_err(|e| e.to_string())?;

    match monitor.or_else(|| win.primary_monitor().ok().flatten()) {
        Some(m) => {
            let p = m.position();
            let s = m.size();
            let _ = win.set_position(PhysicalPosition::new(p.x, p.y));
            let _ = win.set_size(PhysicalSize::new(s.width, s.height));
        }
        // No monitor info — at least fill the work area so it's never a tiny
        // blank window the user can't escape.
        None => {
            let _ = win.maximize();
        }
    }
    let _ = win.show();
    let _ = win.set_focus();
    Ok(())
}

/// Persist a calibrated scan region [x, y, w, h] (physical px) into config.json,
/// preserving other settings. The scan loop re-reads config each cycle, so the
/// new region takes effect without a restart.
#[tauri::command]
fn save_scan_region(app: AppHandle, x: u32, y: u32, w: u32, h: u32) -> Result<(), String> {
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

/// Log to `logs/scanner.log` next to the exe (matching the v1 layout). Falls
/// back silently to no file logging if the path can't be created.
fn init_logging() {
    let log_path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join("logs")))
        .map(|dir| {
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
        .invoke_handler(tauri::generate_handler![open_calibration, save_scan_region])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
