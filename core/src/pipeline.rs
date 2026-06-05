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
            // Best ore per reading (exact beats fuzzy), like the v1 scanning loop.
            if let Some(best) = resolver.resolve(num, 1.0).into_iter().next() {
                matches.push(best);
            }
        }
    }

    Ok(resolver.aggregate(&matches))
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
        assert_eq!(extract_numbers("0 7,080 18.8km"), ["0", "7080", "18", "8"]);
    }

    #[test]
    fn plain_and_comma_numbers() {
        assert_eq!(extract_numbers("10,620"), ["10620"]);
        assert_eq!(extract_numbers("880"), ["880"]);
        assert_eq!(extract_numbers("UNKNOWN"), Vec::<String>::new());
    }
}
