//! Config persistence — the load -> mutate -> save -> load round-trip that the
//! save_scan_region Tauri command relies on (preserve other fields).

use scanner_core::config::Config;

#[test]
fn round_trip_preserves_all_fields() {
    let cfg = Config {
        scan_region: Some([10, 20, 30, 40]),
        scan_interval_secs: 1.5,
        upscale: 3,
        min_consecutive_frames: 2,
        clahe_clip_limit: 2.0,
        clahe_grid: [4, 4],
    };

    let path = std::env::temp_dir().join(format!("sc_ore_cfg_{}.json", std::process::id()));
    cfg.save(&path).expect("save");
    let loaded = Config::load(&path);
    let _ = std::fs::remove_file(&path);

    assert_eq!(loaded.scan_region, Some([10, 20, 30, 40]));
    assert_eq!(loaded.scan_interval_secs, 1.5);
    assert_eq!(loaded.upscale, 3);
    assert_eq!(loaded.min_consecutive_frames, 2);
    assert_eq!(loaded.clahe_clip_limit, 2.0);
    assert_eq!(loaded.clahe_grid, [4, 4]);
}

#[test]
fn load_missing_file_is_defaults() {
    let path = std::env::temp_dir().join("sc_ore_cfg_does_not_exist_xyz.json");
    let _ = std::fs::remove_file(&path);
    let cfg = Config::load(&path);
    assert_eq!(cfg.scan_region, None);
    assert_eq!(cfg.scan_interval_secs, 0.75);
    assert_eq!(cfg.min_consecutive_frames, 3);
}
