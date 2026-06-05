//! OCR via the pure-Rust `ocrs` engine (rten backend — no Python, no native
//! ONNX runtime). Loads the detection/recognition models from a directory.

use std::path::Path;

use anyhow::Result;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;

pub struct Ocr {
    engine: OcrEngine,
}

impl Ocr {
    /// Load `text-detection.rten` and `text-recognition.rten` from `models_dir`.
    pub fn new(models_dir: &Path) -> Result<Self> {
        let detection = Model::load_file(models_dir.join("text-detection.rten"))?;
        let recognition = Model::load_file(models_dir.join("text-recognition.rten"))?;
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
