//! Resolver tests — mirror the v1 Python resolver unit tests.

use scanner_core::Resolver;

#[test]
fn signatures_loaded() {
    let r = Resolver::new();
    assert_eq!(r.signatures().len(), 41);
    assert!(r
        .signatures()
        .iter()
        .any(|o| o.id == "beryl" && o.base_rs == 3540));
    // SC 4.8 roster (issue #22): Janalite added, Felinite removed.
    assert!(r
        .signatures()
        .iter()
        .any(|o| o.id == "janalite" && o.base_rs == 3000));
    assert!(!r.signatures().iter().any(|o| o.id == "felinite"));
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
        (3840, "Aslarite", 1), // 4.7 ore (clustered near Laranite 3825)
        (4700, "C-Type Asteroid", 1),
        (4900, "E-Type Asteroid", 1),
    ];
    for (rs, name, qty) in cases {
        let matches = r.resolve(rs, 1.0);
        let top = matches
            .first()
            .unwrap_or_else(|| panic!("expected a match for {rs}"));
        assert_eq!(top.ore.name, name, "rs {rs} ore");
        assert_eq!(top.quantity, qty, "rs {rs} qty");
        assert!(
            (top.confidence - 1.0).abs() < 1e-9,
            "rs {rs} should be exact (conf 1.0)"
        );
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

#[test]
fn near_neighbor_disambiguation() {
    // Beryl 3540 / Taranite 3555 / Borase 3570 sit 15 RS apart; the error margin
    // (~35) overlaps all three, so each exact reading must still win over its
    // fuzzy neighbours on the confidence penalty.
    let r = Resolver::new();
    for (rs, name) in [(3540, "Beryl"), (3555, "Taranite"), (3570, "Borase")] {
        let top = r
            .resolve(rs, 1.0)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("expected a match for {rs}"));
        assert_eq!(top.ore.name, name, "rs {rs} should resolve to {name}");
        assert_eq!(top.quantity, 1);
        assert_eq!(top.error_margin, 0, "rs {rs} is exact");
    }
}

#[test]
fn fps_gems_are_flat_3000() {
    // SC 4.8 (issue #22): all FPS hand-mineable gems share a flat 3000 signature,
    // so the RS number gives quantity, not which gem.
    let r = Resolver::new();
    let fps: Vec<_> = r
        .signatures()
        .iter()
        .filter(|o| o.context.iter().any(|c| c == "fps"))
        .collect();
    assert_eq!(fps.len(), 4, "expected 4 FPS gems");
    assert!(fps.iter().all(|o| o.base_rs == 3000), "all FPS gems base 3000");
    for name in ["Hadanite", "Dolivine", "Aphorite", "Janalite"] {
        assert!(fps.iter().any(|o| o.name == name), "FPS gem {name} present");
    }
}

#[test]
fn ground_vehicle_deposits_are_4000() {
    let r = Resolver::new();
    let gv: Vec<_> = r
        .signatures()
        .iter()
        .filter(|o| o.context.iter().any(|c| c == "vehicle"))
        .collect();
    assert_eq!(gv.len(), 3, "expected 3 ground-vehicle deposits");
    assert!(gv.iter().all(|o| o.base_rs == 4000));
    for name in ["Beradom", "Feynmaline", "Glacosite"] {
        assert!(gv.iter().any(|o| o.name == name), "ROC deposit {name} present");
    }
}

#[test]
fn fps_signature_yields_quantity_for_every_gem() {
    // An FPS read of 9000 is 3x of *every* gem (base 3000) — ambiguous by design,
    // so all four must surface at qty 3 (the overlay shows them as alternatives).
    let r = Resolver::new();
    let m = r.resolve(9000, 1.0);
    let top = m.first().expect("a match for 9000");
    assert_eq!(top.quantity, 3);
    for name in ["Hadanite", "Dolivine", "Aphorite", "Janalite"] {
        assert!(
            m.iter().any(|x| x.ore.name == name && x.quantity == 3),
            "9000 should match {name} x3"
        );
    }
}

#[test]
fn vehicle_4000_collides_with_itype_asteroid() {
    // 4000 is shared by the I-Type asteroid and all three ROC deposits — the exact
    // collision called out in issue #22. All must surface (no mode filtering).
    let r = Resolver::new();
    let m = r.resolve(4000, 1.0);
    for name in ["I-Type Asteroid", "Beradom", "Feynmaline", "Glacosite"] {
        assert!(
            m.iter().any(|x| x.ore.name == name && x.quantity == 1),
            "4000 should match {name} x1"
        );
    }
}

#[test]
fn config_matches_v1_defaults() {
    let cfg = Resolver::new().config().clone();
    assert_eq!(cfg.valid_rs_min, 100);
    assert_eq!(cfg.valid_rs_max, 200_000);
    assert_eq!(cfg.min_quantity, 1);
    assert_eq!(cfg.max_quantity, 10);
}
