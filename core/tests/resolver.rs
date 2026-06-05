//! Resolver tests — mirror backend/tests/unit/test_resolver.py.

use scanner_core::Resolver;

#[test]
fn signatures_loaded() {
    let r = Resolver::new();
    assert_eq!(r.signatures().len(), 38);
    assert!(r.signatures().iter().any(|o| o.id == "beryl" && o.base_rs == 3540));
}

#[test]
fn exact_division_top_match() {
    let r = Resolver::new();
    let cases = [
        (3170, "Quantainium", 1),
        (10620, "Beryl", 3),
        (7080, "Beryl", 2),
        (17140, "Aluminium", 4),
        (3540, "Beryl", 1),
        (6340, "Quantainium", 2),
        (3840, "Aslarite", 1),     // 4.7 ore (clustered near Laranite 3825)
        (4700, "C-Type Asteroid", 1),
        (4900, "E-Type Asteroid", 1),
    ];
    for (rs, name, qty) in cases {
        let matches = r.resolve(rs, 1.0);
        let top = matches.first().unwrap_or_else(|| panic!("expected a match for {rs}"));
        assert_eq!(top.ore.name, name, "rs {rs} ore");
        assert_eq!(top.quantity, qty, "rs {rs} qty");
        assert!((top.confidence - 1.0).abs() < 1e-9, "rs {rs} should be exact (conf 1.0)");
        assert_eq!(top.error_margin, 0, "rs {rs} exact -> no error");
    }
}

#[test]
fn matches_sorted_by_confidence_desc() {
    let r = Resolver::new();
    let m = r.resolve(10620, 1.0);
    let confs: Vec<f64> = m.iter().map(|x| x.confidence).collect();
    let mut sorted = confs.clone();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
    assert_eq!(confs, sorted);
}

#[test]
fn confidence_scales_with_ocr_confidence() {
    let r = Resolver::new();
    let full = r.resolve(3540, 1.0)[0].confidence;
    let half = r.resolve(3540, 0.5)[0].confidence;
    assert!((half - full * 0.5).abs() < 1e-9);
}

#[test]
fn ocr_correction_split() {
    // 33170 -> 3 x 3170 (Quantainium) via the split path.
    let r = Resolver::new();
    let m = r.resolve(33170, 1.0);
    assert!(m.iter().any(|x| x.ore.name == "Quantainium"));
}

#[test]
fn ocr_correction_extra_digit() {
    // 105620 -> drop a digit -> 10620 -> 3 x Beryl.
    let r = Resolver::new();
    let m = r.resolve(105620, 1.0);
    assert!(m.iter().any(|x| x.ore.name == "Beryl"));
}

#[test]
fn aggregate_keeps_highest_confidence() {
    let r = Resolver::new();
    let m = r.resolve(10620, 1.0);
    let agg = r.aggregate(&m);
    assert!(agg.contains_key("beryl"));
    assert_eq!(agg["beryl"].quantity, 3);
}

#[test]
fn quantity_out_of_range_no_beryl() {
    // 3540 * 11 exceeds max_quantity and isn't a clean multiple of another base.
    let r = Resolver::new();
    let m = r.resolve(3540 * 11, 1.0);
    assert!(m.iter().all(|x| x.ore.name != "Beryl"));
}
