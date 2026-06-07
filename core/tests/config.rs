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
        mining_location: Some("Cellin".to_string()),
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
    assert_eq!(loaded.mining_location.as_deref(), Some("Cellin"));
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

/// A hand-edited config with out-of-range values must still detect: every
/// tunable is clamped on load. The headline case is `min_consecutive_frames`
/// above the debouncer's history window — left unclamped, detection would
/// silently never confirm.
#[test]
fn load_clamps_out_of_range_values() {
    let json = r#"{
        "scan_region": [10, 20, 30, 40],
        "scan_interval_secs": 999.0,
        "upscale": 64,
        "min_consecutive_frames": 50,
        "clahe_clip_limit": 100.0,
        "clahe_grid": [0, 0]
    }"#;
    let path = std::env::temp_dir().join(format!("sc_ore_cfg_clamp_{}.json", std::process::id()));
    std::fs::write(&path, json).expect("write");
    let cfg = Config::load(&path);
    let _ = std::fs::remove_file(&path);

    assert_eq!(cfg.scan_interval_secs, 5.0);
    assert_eq!(cfg.upscale, 6);
    assert_eq!(
        cfg.min_consecutive_frames, 6,
        "must clamp below the debouncer history window"
    );
    assert_eq!(cfg.clahe_clip_limit, 8.0);
    assert_eq!(cfg.clahe_grid, [1, 1]);
    assert_eq!(cfg.scan_region, Some([10, 20, 30, 40]));
}

/// Non-finite floats and a degenerate scan region are normalized, not propagated.
#[test]
fn load_normalizes_nan_and_degenerate_region() {
    let json = r#"{
        "scan_region": [0, 0, 0, 100],
        "scan_interval_secs": 0.0,
        "min_consecutive_frames": 0,
        "upscale": 0
    }"#;
    let path = std::env::temp_dir().join(format!("sc_ore_cfg_norm_{}.json", std::process::id()));
    std::fs::write(&path, json).expect("write");
    let cfg = Config::load(&path);
    let _ = std::fs::remove_file(&path);

    assert_eq!(cfg.scan_region, None, "zero-dimension region is dropped");
    assert_eq!(cfg.scan_interval_secs, 0.3);
    assert_eq!(cfg.min_consecutive_frames, 1);
    assert_eq!(cfg.upscale, 1);
}
