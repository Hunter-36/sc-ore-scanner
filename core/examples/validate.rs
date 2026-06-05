//! Local accuracy check for the Rust detection pipeline against the real
//! capture fixtures. Run with the ocrs models dir:
//!   cargo run --example validate --release
//! (Points at the spike models + the repo's test_images by default.)

use std::path::{Path, PathBuf};

use scanner_core::{ocr::Ocr, pipeline::detect_ores, resolver::Resolver};

fn main() -> anyhow::Result<()> {
    // Models dir: first CLI arg, else ./models. (ocrs .rten models aren't committed.)
    let models: PathBuf = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("models"));
    let img_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../backend/tests/test_images");

    let ocr = Ocr::new(&models)?;
    let resolver = Resolver::new();
    let region = Some([193u32, 122, 109, 48]);

    let cases: [(&str, Option<(&str, i64)>); 3] = [
        ("sc_mining_scan_rs_10620_some_particles.png", Some(("Beryl", 3))),
        ("sc_mining_scan_rs_7080_some_particles_and_other_marker_not_rs.png", Some(("Beryl", 2))),
        ("sc_mining_scan_no_rs.png", None),
    ];

    let mut all_pass = true;
    for (file, expect) in cases {
        let img = image::open(img_dir.join(file))?.into_rgb8();
        let agg = detect_ores(&img, region, 4, &ocr, &resolver)?;
        let top = agg
            .values()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap());
        let got = top.map(|m| (m.ore.name.as_str(), m.quantity));
        let ok = match expect {
            Some((n, q)) => got == Some((n, q)),
            None => agg.is_empty(),
        };
        all_pass &= ok;
        let ores: Vec<String> = agg
            .values()
            .map(|m| format!("{}x {}", m.quantity, m.ore.name))
            .collect();
        println!(
            "{:<55} expect={:?} got={:?} ores={:?} {}",
            file,
            expect,
            got,
            ores,
            if ok { "PASS" } else { "FAIL" }
        );
    }

    println!("\n{}", if all_pass { "ALL PASS" } else { "SOME FAILED" });
    Ok(())
}
