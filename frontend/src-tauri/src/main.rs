// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;

use base64::Engine;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_window_state::StateFlags;

mod scan;

/// A screenshot of the monitor the scan loop captures, handed to the calibration
/// window so the user draws the region on exactly what gets scanned. Captured
/// *before* the calibration window opens so the window isn't in the shot.
#[derive(Clone, serde::Serialize)]
struct CaptureData {
    width: u32,
    height: u32,
    #[serde(rename = "dataUrl")]
    data_url: String,
}

struct CaptureState(Mutex<Option<CaptureData>>);

fn grab_capture() -> Result<CaptureData, String> {
    let img = scan::capture_primary().map_err(|e| e.to_string())?;
    let (width, height) = (img.width(), img.height());
    let mut png: Vec<u8> = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
    Ok(CaptureData {
        width,
        height,
        data_url: format!("data:image/png;base64,{b64}"),
    })
}

/// Open the calibration window: a normal, movable, decorated window showing a
/// screenshot of the scanned monitor. This avoids the transparent-fullscreen
/// pitfalls (wrong monitor, can't move, blank) — the screenshot *is* the primary
/// monitor regardless of where the window sits.
#[tauri::command]
fn open_calibration(app: AppHandle) -> Result<(), String> {
    log::info!("open_calibration requested");

    // Capture first, then store it, then open the window (so it isn't captured).
    let capture = grab_capture()?;
    log::info!("captured {}x{} for calibration", capture.width, capture.height);
    *app.state::<CaptureState>().0.lock().unwrap() = Some(capture);

    if let Some(w) = app.get_webview_window("calibrate") {
        let _ = w.show();
        let _ = w.set_focus();
        // Tell it to reload the (fresh) capture.
        let _ = w.emit("recapture", ());
        return Ok(());
    }

    WebviewWindowBuilder::new(&app, "calibrate", WebviewUrl::App("index.html".into()))
        .title("Calibrate scan region — draw a box around the RS number")
        .inner_size(1280.0, 820.0)
        .min_inner_size(640.0, 480.0)
        .center()
        .resizable(true)
        .focused(true)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Return the stored screenshot for the calibration window to display.
#[tauri::command]
fn get_capture(state: State<CaptureState>) -> Result<CaptureData, String> {
    state
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no capture available".to_string())
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

/// Exit the whole app. More reliable than closing the window from JS (the in-process
/// scan thread keeps running otherwise, and a drag-region header can swallow clicks).
#[tauri::command]
fn quit(app: AppHandle) {
    log::info!("quit requested");
    app.exit(0);
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
        .manage(CaptureState(Mutex::new(None)))
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
        .invoke_handler(tauri::generate_handler![
            open_calibration,
            get_capture,
            save_scan_region,
            quit
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
