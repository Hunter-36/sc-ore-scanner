//! Image preprocessing for OCR: crop to the calibrated scan region and upscale.
//!
//! Mirrors the Python pipeline (crop -> upscale). RapidOCR/ocrs read the bright
//! HUD digits well without heavy thresholding, so this stays light.

use image::{imageops::FilterType, RgbImage};

/// Crop to a `[x, y, width, height]` scan region (None = whole image), then
/// upscale by `scale` (Lanczos) so the small HUD text is large enough for OCR.
pub fn crop_and_upscale(img: &RgbImage, region: Option<[u32; 4]>, scale: u32) -> RgbImage {
    let cropped = match region {
        Some([x, y, w, h]) => image::imageops::crop_imm(img, x, y, w, h).to_image(),
        None => img.clone(),
    };
    if scale > 1 {
        let (w, h) = cropped.dimensions();
        image::imageops::resize(&cropped, w * scale, h * scale, FilterType::Lanczos3)
    } else {
        cropped
    }
}
