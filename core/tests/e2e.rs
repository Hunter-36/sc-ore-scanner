//! End-to-end OCR accuracy test over real capture fixtures — the Rust port of
//! the v1 manifest-driven Python e2e. Crops each fixture to the calibrated scan
//! region, runs the real embedded OCR + resolver, and asserts the expected top
//! ore. Add cases by dropping a PNG in tests/fixtures/ and adding an assertion.
//!
//! ocrs is deterministic, so one run per fixture suffices (v1 ran 10x because
//! RapidOCR had run-to-run variance).

use std::path::Path;

use image::imageops::FilterType;
use scanner_core::{
    ocr::Ocr,
    pipeline::{detect_ores, recognize_rs_numbers_from_processed, resolve_and_aggregate},
    preprocess::{auto_scale, crop_and_upscale},
    resolver::Resolver,
};

// The scan region within the fixtures (the in-game calibration equivalent).
const REGION: Option<[u32; 4]> = Some([193, 122, 109, 48]);
const SCALE: u32 = 4;

fn top_ore(file: &str, ocr: &Ocr, resolver: &Resolver) -> Option<(String, i64)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(file);
    let img = image::open(&path)
        .unwrap_or_else(|e| panic!("open {}: {e}", path.display()))
        .into_rgb8();
    let agg = detect_ores(&img, REGION, SCALE, ocr, resolver).expect("detect_ores");
    agg.values()
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
        .map(|m| (m.ore.name.clone(), m.quantity))
}

/// The candidate sets (each a sorted `(ore name, quantity)` list) for every card
/// detected in a fixture. For an ambiguous signature one card carries several
/// candidates the RS can't tell apart — assert on the SET, since which one is the
/// "primary" is a confidence tie and not meaningful to pin down.
///
/// `region` is per-fixture: the original 482×283 captures crop to the calibrated
/// window, while the tighter ambiguous screenshots are already framed on the sig
/// and so read at the full frame (`None`).
fn candidate_sets(
    file: &str,
    region: Option<[u32; 4]>,
    ocr: &Ocr,
    resolver: &Resolver,
) -> Vec<Vec<(String, i64)>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(file);
    let img = image::open(&path)
        .unwrap_or_else(|e| panic!("open {}: {e}", path.display()))
        .into_rgb8();
    let agg = detect_ores(&img, region, SCALE, ocr, resolver).expect("detect_ores");
    agg.values()
        .map(|d| {
            let mut set: Vec<(String, i64)> = d
                .candidates
                .iter()
                .map(|c| (c.ore.name.clone(), c.quantity))
                .collect();
            set.sort();
            set
        })
        .collect()
}

#[test]
fn ocr_accuracy_on_real_captures() {
    let ocr = Ocr::new().expect("OCR engine init");
    let resolver = Resolver::new();

    // RS 10,620 = 3 x Beryl (3540), with comma + location pin + particles.
    assert_eq!(
        top_ore(
            "sc_mining_scan_rs_10620_some_particles.png",
            &ocr,
            &resolver
        ),
        Some(("Beryl".to_string(), 3)),
    );

    // RS 7,080 = 2 x Beryl (3540); an UNKNOWN/18.8km marker must not be detected.
    assert_eq!(
        top_ore(
            "sc_mining_scan_rs_7080_some_particles_and_other_marker_not_rs.png",
            &ocr,
            &resolver
        ),
        Some(("Beryl".to_string(), 2)),
    );

    // No RS signature on screen -> nothing resolves.
    assert_eq!(top_ore("sc_mining_scan_no_rs.png", &ocr, &resolver), None);

    // RS 14,160 = 4 x Beryl (3540) AND 3 x S-Type Asteroid (4720), both exact.
    // The reading is ambiguous; the card must keep BOTH interpretations. These
    // tight crops are framed on the sig, so they read at the full frame.
    assert!(
        candidate_sets(
            "sc_mining_scan_rs_14160_ambiguous_beryl_or_s_type.png",
            None,
            &ocr,
            &resolver
        )
        .contains(&vec![
            ("Beryl".to_string(), 4),
            ("S-Type Asteroid".to_string(), 3),
        ]),
        "14,160 should resolve to the Beryl-or-S-Type ambiguous set",
    );

    // RS 19,200 = 6 x Savrilium (3200) AND 5 x Aslarite (3840), both exact.
    assert!(
        candidate_sets(
            "sc_mining_scan_rs_19200_ambiguous_savrilium_or_aslarite.png",
            None,
            &ocr,
            &resolver
        )
        .contains(&vec![
            ("Aslarite".to_string(), 5),
            ("Savrilium".to_string(), 6),
        ]),
        "19,200 should resolve to the Savrilium-or-Aslarite ambiguous set",
    );
}

/// Issue #110: ultrawide + high FOV renders the RS readout small in *screen pixels*,
/// so a fixed ×4 upscale feeds `ocrs` too few pixels for a reliable read. We have no
/// real ultrawide captures yet (a community ask — see CONTRIBUTING "Adding test
/// captures"), so this reproduces the failure faithfully by downscaling a known-good
/// 16:9 capture: halving the linear size shrinks the RS text in pixels exactly the way
/// a higher-resolution / higher-FOV HUD does. The adaptive upscale must recover the
/// read that a fixed ×4 loses — and would have FAILED before the fix, since the
/// pipeline upscaled by a fixed factor.
#[test]
fn adaptive_upscale_recovers_small_ultrawide_text() {
    let ocr = Ocr::new().expect("OCR engine init");
    let resolver = Resolver::new();

    // Beryl×3 reads cleanly at native size (region [193,122,109,48], ×4). Halve the
    // whole frame to simulate a HUD that renders smaller in pixels.
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join("sc_mining_scan_rs_10620_some_particles.png");
    let full = image::open(&path)
        .unwrap_or_else(|e| panic!("open {}: {e}", path.display()))
        .into_rgb8();
    let (fw, fh) = full.dimensions();
    let small = image::imageops::resize(&full, fw / 2, fh / 2, FilterType::Lanczos3);
    // The calibrated region, scaled to the downscaled frame: ~24px tall — small
    // enough that a fixed ×4 (→96px) leaves the text sub-readable.
    let region = [96u32, 61, 54, 24];
    let beryl3 = Some(("Beryl".to_string(), 3));

    let best = |agg: std::collections::HashMap<String, scanner_core::pipeline::Detection>| {
        agg.into_values()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .map(|m| (m.ore.name.clone(), m.quantity))
    };

    // The fix only matters because this region is in the "too small for ×4" regime:
    // auto_scale must pick a larger factor than the base.
    assert!(
        auto_scale(region[3], 4) > 4,
        "a {}px region should upscale beyond the ×4 base",
        region[3]
    );

    // Reproduction of the bug: the old fixed ×4 pipeline loses this signature.
    let fixed = crop_and_upscale(&small, Some(region), 4);
    let fixed_numbers = recognize_rs_numbers_from_processed(&fixed, &ocr, &resolver).expect("ocr");
    assert_ne!(
        best(resolve_and_aggregate(&fixed_numbers, &resolver, None)),
        beryl3,
        "fixed ×4 should fail on the shrunken text — this is the failure #110 reports"
    );

    // The fix: detect_ores upscales by auto_scale, recovering the read.
    assert_eq!(
        best(detect_ores(&small, Some(region), 4, &ocr, &resolver).expect("detect_ores")),
        beryl3,
        "adaptive upscale recovers the read a fixed ×4 loses",
    );
}
