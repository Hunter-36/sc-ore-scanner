//! End-to-end detection, faithful to the Python scanning loop:
//! image -> preprocess (crop/upscale/grayscale/CLAHE) -> OCR -> per-number
//! extraction -> RS range filter -> (debounce, in the caller) -> resolve
//! (best ore per reading) -> aggregate.

use std::collections::HashMap;

use anyhow::Result;

use crate::ocr::Ocr;
use crate::preprocess::preprocess_for_ocr;
use crate::resolver::{OreMatch, Resolver};

const DEFAULT_CLAHE_CLIP: f64 = 0.0; // off — see preprocess_for_ocr
const DEFAULT_CLAHE_GRID: [u32; 2] = [8, 8];

/// OCR a single frame and return the valid RS numbers found (may repeat). The
/// caller debounces these before resolving, mirroring v1.
pub fn recognize_rs_numbers(
    img: &image::RgbImage,
    region: Option<[u32; 4]>,
    scale: u32,
    clahe_clip_limit: f64,
    clahe_grid: [u32; 2],
    ocr: &Ocr,
    resolver: &Resolver,
) -> Result<Vec<i64>> {
    let processed = preprocess_for_ocr(img, region, scale, clahe_clip_limit, clahe_grid);
    recognize_rs_numbers_from_processed(&processed, ocr, resolver)
}

/// As above but on an already-preprocessed image (so callers that also want to
/// save/inspect the processed frame don't preprocess twice).
pub fn recognize_rs_numbers_from_processed(
    processed: &image::RgbImage,
    ocr: &Ocr,
    resolver: &Resolver,
) -> Result<Vec<i64>> {
    let (pw, ph) = processed.dimensions();
    let lines = ocr.recognize_lines(processed)?;
    log::info!("OCR crop {}x{} -> {} line(s): {:?}", pw, ph, lines.len(), lines);

    let cfg = resolver.config();
    let mut numbers = Vec::new();
    for line in &lines {
        // Pull out each number token on the line separately, so an RS value that
        // shares a line with other HUD text (e.g. "0 7,080 18.8km") still yields
        // a clean candidate (7080) instead of a merged 8-digit blob.
        for digits in extract_numbers(line) {
            if !(3..=6).contains(&digits.len()) {
                continue;
            }
            let Ok(num) = digits.parse::<i64>() else {
                continue;
            };
            if num < cfg.valid_rs_min || num > cfg.valid_rs_max {
                log::info!(
                    "  candidate {num} out of RS range [{}, {}], skipped",
                    cfg.valid_rs_min, cfg.valid_rs_max
                );
                continue;
            }
            numbers.push(num);
        }
    }
    Ok(numbers)
}

/// Resolve each RS number to its best ore (exact beats fuzzy) and aggregate.
pub fn resolve_and_aggregate(numbers: &[i64], resolver: &Resolver) -> HashMap<String, OreMatch> {
    let mut matches: Vec<OreMatch> = Vec::new();
    for &num in numbers {
        if let Some(best) = resolver.resolve(num, 1.0).into_iter().next() {
            matches.push(best);
        }
    }
    resolver.aggregate(&matches)
}

/// Single-frame detection (no debouncing) — used by the validation tool/tests.
pub fn detect_ores(
    img: &image::RgbImage,
    region: Option<[u32; 4]>,
    scale: u32,
    ocr: &Ocr,
    resolver: &Resolver,
) -> Result<HashMap<String, OreMatch>> {
    let numbers = recognize_rs_numbers(
        img,
        region,
        scale,
        DEFAULT_CLAHE_CLIP,
        DEFAULT_CLAHE_GRID,
        ocr,
        resolver,
    )?;
    Ok(resolve_and_aggregate(&numbers, resolver))
}

/// Extract digit-run tokens from OCR text. Commas are treated as thousands
/// separators (kept inside a number); every other non-digit (space, '.', letters)
/// ends the current token. So "0 7,080 18.8km" -> ["0", "7080", "18", "8"].
fn extract_numbers(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            cur.push(c);
        } else if c == ',' {
            // thousands separator — skip, keep building the current number
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::extract_numbers;

    #[test]
    fn rs_value_separated_from_distance_marker() {
        // The real failing OCR line: RS value + distance on one line.
        assert_eq!(extract_numbers("0 7,080 18.8km"), ["0", "7080", "18", "8"]);
        assert_eq!(extract_numbers("\"97,080 18.8km"), ["97080", "18", "8"]);
    }

    #[test]
    fn plain_and_comma_numbers() {
        assert_eq!(extract_numbers("10,620"), ["10620"]);
        assert_eq!(extract_numbers("880"), ["880"]);
        assert_eq!(extract_numbers("UNKNOWN"), Vec::<String>::new());
    }
}
