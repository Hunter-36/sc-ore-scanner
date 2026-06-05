//! End-to-end detection, faithful to the Python scanning loop:
//! image -> preprocess (crop/upscale/grayscale/CLAHE) -> OCR -> per-number
//! extraction -> RS range filter -> (debounce, in the caller) -> resolve
//! (best ore per reading) -> aggregate.

use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::ocr::Ocr;
use crate::preprocess::preprocess_for_ocr;
use crate::resolver::{OreMatch, Resolver};
use crate::signatures::OreSignature;

/// A resolved ore for a reading, plus any equally-likely alternative readings of
/// the same RS number (an ambiguous signature, e.g. 19,200 = 6× Savrilium = 5×
/// Aslarite). `alternatives` holds the other interpretations as display strings
/// like "5x Aslarite"; empty when the reading is unambiguous.
#[derive(Debug, Clone)]
pub struct Detection {
    pub ore: OreSignature,
    pub quantity: i64,
    pub detected_rs: i64,
    pub confidence: f64,
    pub alternatives: Vec<String>,
}

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
    log::debug!(
        "OCR crop {}x{} -> {} line(s): {:?}",
        pw,
        ph,
        lines.len(),
        lines
    );

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
                log::debug!(
                    "  candidate {num} out of RS range [{}, {}], skipped",
                    cfg.valid_rs_min,
                    cfg.valid_rs_max
                );
                continue;
            }
            numbers.push(num);
        }
    }
    Ok(numbers)
}

/// Resolve each RS number to its best ore and aggregate (best per ore id). When a
/// reading has several equally-confident interpretations (an ambiguous signature),
/// the extras are kept on `Detection::alternatives` instead of being silently
/// dropped, so the overlay can show "could be either".
pub fn resolve_and_aggregate(numbers: &[i64], resolver: &Resolver) -> HashMap<String, Detection> {
    let mut agg: HashMap<String, Detection> = HashMap::new();
    for &num in numbers {
        let matches = resolver.resolve(num, 1.0);
        let Some(best) = matches.first() else {
            continue;
        };
        let top = best.confidence;

        // Collect the tied-top interpretations (same confidence — i.e. an exact
        // collision). `matches` is sorted desc, so stop at the first lower one.
        // De-dup by (ore name, quantity) since correction paths can repeat a hit.
        let mut seen: HashSet<(String, i64)> = HashSet::new();
        let mut tied: Vec<&OreMatch> = Vec::new();
        for m in &matches {
            if (m.confidence - top).abs() > 1e-9 {
                break;
            }
            if seen.insert((m.ore.name.clone(), m.quantity)) {
                tied.push(m);
            }
        }

        let primary = tied[0];
        let alternatives: Vec<String> = tied[1..]
            .iter()
            .map(|m| format!("{}x {}", m.quantity, m.ore.name))
            .collect();
        let det = Detection {
            ore: primary.ore.clone(),
            quantity: primary.quantity,
            detected_rs: primary.detected_rs,
            confidence: primary.confidence,
            alternatives,
        };

        agg.entry(primary.ore.id.clone())
            .and_modify(|e| {
                if det.confidence > e.confidence {
                    *e = det.clone();
                }
            })
            .or_insert(det);
    }
    agg
}

/// Single-frame detection (no debouncing) — used by the validation tool/tests.
pub fn detect_ores(
    img: &image::RgbImage,
    region: Option<[u32; 4]>,
    scale: u32,
    ocr: &Ocr,
    resolver: &Resolver,
) -> Result<HashMap<String, Detection>> {
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

    #[test]
    fn ambiguous_signature_keeps_both_interpretations() {
        // 19,200 = 6 x Savrilium (3200) = 5 x Aslarite (3840) — both exact.
        let r = crate::resolver::Resolver::new();
        let agg = super::resolve_and_aggregate(&[19_200], &r);
        assert_eq!(agg.len(), 1, "ambiguous reading -> one primary card");
        let det = agg.values().next().unwrap();
        let mut all = det.alternatives.clone();
        all.push(format!("{}x {}", det.quantity, det.ore.name));
        all.sort();
        assert_eq!(
            all,
            vec!["5x Aslarite".to_string(), "6x Savrilium".to_string()]
        );
    }

    #[test]
    fn unambiguous_signature_has_no_alternatives() {
        // 10,620 = 3 x Beryl, no other exact interpretation.
        let r = crate::resolver::Resolver::new();
        let agg = super::resolve_and_aggregate(&[10_620], &r);
        let det = agg.values().next().unwrap();
        assert_eq!((det.ore.name.as_str(), det.quantity), ("Beryl", 3));
        assert!(det.alternatives.is_empty());
    }

    #[test]
    fn clustered_values_do_not_leak_fuzzy_neighbours() {
        // Beryl 3540 / Taranite 3555 / Borase 3570 sit within each other's fuzzy
        // margin (~35), but an exact reading must surface ONLY itself — no fuzzy
        // neighbour leaking as its own card or as an "alternative" (issue #11).
        let r = crate::resolver::Resolver::new();
        for (rs, name) in [(3540, "Beryl"), (3555, "Taranite"), (3570, "Borase")] {
            let agg = super::resolve_and_aggregate(&[rs], &r);
            assert_eq!(agg.len(), 1, "rs {rs} -> exactly one ore");
            let det = agg.values().next().unwrap();
            assert_eq!(det.ore.name, name, "rs {rs}");
            assert!(
                det.alternatives.is_empty(),
                "rs {rs} -> no fuzzy alternatives"
            );
        }
    }
}
