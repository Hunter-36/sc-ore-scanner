// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Manager, PhysicalPosition};
use tauri_plugin_window_state::StateFlags;

mod scan;

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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
