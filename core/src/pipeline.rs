//! End-to-end detection: image -> crop/upscale -> OCR -> digit extraction ->
//! resolve (best ore per reading) -> aggregate. Mirrors the Python scanning loop.

use std::collections::HashMap;

use anyhow::Result;

use crate::ocr::Ocr;
use crate::preprocess::crop_and_upscale;
use crate::resolver::{OreMatch, Resolver};

/// Detect ores in a single frame. Returns the best match per ore id.
pub fn detect_ores(
    img: &image::RgbImage,
    region: Option<[u32; 4]>,
    scale: u32,
    ocr: &Ocr,
    resolver: &Resolver,
) -> Result<HashMap<String, OreMatch>> {
    let processed = crop_and_upscale(img, region, scale);
    let (pw, ph) = processed.dimensions();
    let lines = ocr.recognize_lines(&processed)?;
    log::info!("OCR crop {}x{} -> {} line(s): {:?}", pw, ph, lines.len(), lines);

    let cfg = resolver.config();
    let mut matches: Vec<OreMatch> = Vec::new();

    for line in lines {
        // Strip the thousands comma / pin glyph / stray punctuation.
        let digits: String = line.chars().filter(|c| c.is_ascii_digit()).collect();
        if !(3..=6).contains(&digits.len()) {
            continue;
        }
        let Ok(num) = digits.parse::<i64>() else {
            continue;
        };
        if num < cfg.valid_rs_min || num > cfg.valid_rs_max {
            log::info!("  candidate {num} out of RS range [{}, {}], skipped", cfg.valid_rs_min, cfg.valid_rs_max);
            continue;
        }
        // Best ore per reading (exact beats fuzzy), like the v1 scanning loop.
        if let Some(best) = resolver.resolve(num, 1.0).into_iter().next() {
            matches.push(best);
        }
    }

    Ok(resolver.aggregate(&matches))
}
