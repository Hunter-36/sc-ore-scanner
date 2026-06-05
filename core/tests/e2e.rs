//! End-to-end OCR accuracy test over real capture fixtures — the Rust port of
//! the v1 manifest-driven Python e2e. Crops each fixture to the calibrated scan
//! region, runs the real embedded OCR + resolver, and asserts the expected top
//! ore. Add cases by dropping a PNG in tests/fixtures/ and adding an assertion.
//!
//! ocrs is deterministic, so one run per fixture suffices (v1 ran 10x because
//! RapidOCR had run-to-run variance).

use std::path::Path;

use scanner_core::{ocr::Ocr, pipeline::detect_ores, resolver::Resolver};

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
}
