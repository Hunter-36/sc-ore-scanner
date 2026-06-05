//! OCR via the pure-Rust `ocrs` engine (rten backend — no Python, no native
//! ONNX runtime). The detection/recognition models are embedded into the binary
//! at build time (see build.rs), so the app ships as one self-contained exe.

use anyhow::Result;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;

// Fetched by build.rs into OUT_DIR, baked into the binary here.
static DETECTION_MODEL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/text-detection.rten"));
static RECOGNITION_MODEL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/text-recognition.rten"));

pub struct Ocr {
    engine: OcrEngine,
}

impl Ocr {
    /// Build the OCR engine from the embedded models.
    pub fn new() -> Result<Self> {
        let detection = Model::load_static_slice(DETECTION_MODEL)?;
        let recognition = Model::load_static_slice(RECOGNITION_MODEL)?;
        let engine = OcrEngine::new(OcrEngineParams {
            detection_model: Some(detection),
            recognition_model: Some(recognition),
            ..Default::default()
        })?;
        Ok(Self { engine })
    }

    /// Recognize text in an image, returning the non-empty text lines.
    pub fn recognize_lines(&self, img: &image::RgbImage) -> Result<Vec<String>> {
        let source = ImageSource::from_bytes(img.as_raw(), img.dimensions())?;
        let input = self.engine.prepare_input(source)?;
        let text = self.engine.get_text(&input)?;
        Ok(text
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect())
    }
}
