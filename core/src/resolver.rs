//! Resolves a detected RS number to ore matches via division math + OCR-error
//! correction. A faithful port of the v1 Python resolver.

use std::collections::HashMap;

use crate::signatures::{load_signatures, OreSignature};

/// Signature-matching tunables (mirrors the Python SignatureConfig defaults).
#[derive(Debug, Clone)]
pub struct SignatureConfig {
    pub min_quantity: i64,
    pub max_quantity: i64,
    pub max_error_margin: f64,
    pub error_margin_percent: f64,
    pub valid_rs_min: i64,
    pub valid_rs_max: i64,
}

impl Default for SignatureConfig {
    fn default() -> Self {
        Self {
            min_quantity: 1,
            max_quantity: 10,
            max_error_margin: 50.0,
            error_margin_percent: 0.01,
            valid_rs_min: 100,
            valid_rs_max: 200_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OreMatch {
    pub ore: OreSignature,
    pub quantity: i64,
    pub detected_rs: i64,
    pub confidence: f64,
    pub error_margin: i64,
}

pub struct Resolver {
    signatures: Vec<OreSignature>,
    config: SignatureConfig,
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            signatures: load_signatures(),
            config: SignatureConfig::default(),
        }
    }

    pub fn signatures(&self) -> &[OreSignature] {
        &self.signatures
    }

    pub fn config(&self) -> &SignatureConfig {
        &self.config
    }

    /// Resolve a detected RS number into ore matches, sorted by confidence (desc).
    pub fn resolve(&self, detected_rs: i64, ocr_confidence: f64) -> Vec<OreMatch> {
        let mut matches = self.try_division_match(detected_rs, ocr_confidence);

        let digits = detected_rs.abs().to_string().len();
        if digits == 5 || digits == 6 {
            matches.extend(self.try_ocr_correction(detected_rs, ocr_confidence));
        }

        matches.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches
    }

    fn try_division_match(&self, detected_rs: i64, ocr_confidence: f64) -> Vec<OreMatch> {
        let c = &self.config;
        let mut out = Vec::new();
        for ore in &self.signatures {
            let base = ore.base_rs;
            if base <= 0 {
                continue;
            }
            let quantity = detected_rs as f64 / base as f64;
            if quantity < c.min_quantity as f64 || quantity > c.max_quantity as f64 {
                continue;
            }
            let remainder = detected_rs % base;
            let error_margin = c.max_error_margin.min(base as f64 * c.error_margin_percent);
            let within = remainder == 0
                || (remainder as f64) <= error_margin
                || ((base - remainder) as f64) <= error_margin;
            if !within {
                continue;
            }
            let final_quantity = quantity.round() as i64;
            let actual_error = (detected_rs - base * final_quantity).abs();
            let error_penalty = (1.0 - (actual_error as f64 / (base as f64 * 0.1))).clamp(0.0, 1.0);
            out.push(OreMatch {
                ore: ore.clone(),
                quantity: final_quantity,
                detected_rs,
                confidence: ocr_confidence * error_penalty,
                error_margin: actual_error,
            });
        }
        out
    }

    fn try_ocr_correction(&self, detected_rs: i64, ocr_confidence: f64) -> Vec<OreMatch> {
        let mut out = Vec::new();
        let s = detected_rs.to_string();
        let chars: Vec<char> = s.chars().collect();

        // Drop each digit in turn (e.g. 105620 -> 10620).
        for i in 0..chars.len() {
            let cand: String = chars
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, ch)| *ch)
                .collect();
            if cand.len() == 4 || cand.len() == 5 {
                if let Ok(cand_rs) = cand.parse::<i64>() {
                    out.extend(self.try_division_match(cand_rs, ocr_confidence * 0.8));
                }
            }
        }

        // Split a 5-digit read into quantity + 4-digit signature (e.g. 33170 -> 3 x 3170).
        if chars.len() == 5 {
            for split_pos in [1usize, 2usize] {
                let (qstr, sigstr) = s.split_at(split_pos);
                if sigstr.len() == 4 {
                    if let (Ok(quantity), Ok(base_rs)) =
                        (qstr.parse::<i64>(), sigstr.parse::<i64>())
                    {
                        if let Some(ore) = self.signatures.iter().find(|o| o.base_rs == base_rs) {
                            let c = &self.config;
                            if quantity >= c.min_quantity && quantity <= c.max_quantity {
                                out.push(OreMatch {
                                    ore: ore.clone(),
                                    quantity,
                                    detected_rs,
                                    confidence: ocr_confidence * 0.9,
                                    error_margin: 0,
                                });
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// Keep the highest-confidence match per ore id.
    pub fn aggregate(&self, matches: &[OreMatch]) -> HashMap<String, OreMatch> {
        let mut agg: HashMap<String, OreMatch> = HashMap::new();
        for m in matches {
            agg.entry(m.ore.id.clone())
                .and_modify(|e| {
                    if m.confidence > e.confidence {
                        *e = m.clone();
                    }
                })
                .or_insert_with(|| m.clone());
        }
        agg
    }
}
