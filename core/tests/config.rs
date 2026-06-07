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

/// The exact flow the `save_scan_region` Tauri command runs: load an existing
/// config that has the user's tuning but no region yet, set ONLY the region,
/// save, and reload. Calibrating a region must not clobber the other settings —
/// this is the regression #44 calls out (the round-trip is in `Config`, the
/// command just wraps it with the AppHandle's config path).
#[test]
fn save_scan_region_preserves_existing_settings() {
    let path =
        std::env::temp_dir().join(format!("sc_ore_cfg_setregion_{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);

    // Seed: tuned settings, not-yet-calibrated (the post-first-run state).
    let seed = Config {
        scan_region: None,
        scan_interval_secs: 1.25,
        upscale: 5,
        min_consecutive_frames: 4,
        clahe_clip_limit: 3.0,
        clahe_grid: [16, 16],
        mining_location: Some("Aaron Halo".to_string()),
    };
    seed.save(&path).expect("seed save");

    // The command's body: load -> set only the region -> save.
    let mut cfg = Config::load(&path);
    cfg.scan_region = Some([100, 200, 300, 150]);
    cfg.save(&path).expect("save region");

    // Reload: region applied, every other setting untouched.
    let loaded = Config::load(&path);
    let _ = std::fs::remove_file(&path);

    assert_eq!(loaded.scan_region, Some([100, 200, 300, 150]), "region set");
    assert_eq!(loaded.scan_interval_secs, 1.25);
    assert_eq!(loaded.upscale, 5);
    assert_eq!(loaded.min_consecutive_frames, 4);
    assert_eq!(loaded.clahe_clip_limit, 3.0);
    assert_eq!(loaded.clahe_grid, [16, 16]);
    assert_eq!(loaded.mining_location.as_deref(), Some("Aaron Halo"));
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
