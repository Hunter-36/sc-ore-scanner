// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Manager, PhysicalPosition};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Pin the overlay to the top-right of its monitor, at any resolution
            // (the config x/y are only a 1920-wide fallback).
            if let Some(window) = app.get_webview_window("main") {
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
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
