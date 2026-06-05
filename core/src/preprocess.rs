//! Image preprocessing for OCR, faithful to the v1 Python pipeline
//! (`backend/src/ocr/ocr_engine.py` `preprocess_image`):
//!   crop region -> upscale (Lanczos) -> grayscale -> CLAHE contrast boost.
//!
//! Contrast-based, NOT threshold-based — aggressive thresholding destroyed digit
//! strokes in v1 (documented gotcha). CLAHE lifts the teal HUD digits off the
//! dark, particle-flecked background.

use image::{imageops::FilterType, GrayImage, Luma, RgbImage};

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

/// Full OCR preprocessing: crop -> upscale -> grayscale -> CLAHE -> back to RGB
/// (ocrs wants RGB bytes). Mirrors the Python `preprocess_image`.
pub fn preprocess_for_ocr(
    img: &RgbImage,
    region: Option<[u32; 4]>,
    scale: u32,
    clahe_clip_limit: f64,
    clahe_grid: [u32; 2],
) -> RgbImage {
    let upscaled = crop_and_upscale(img, region, scale);

    // CLAHE is opt-in (clip > 0). The ocrs engine reads the raw upscaled HUD text
    // well; CLAHE at v1's clip=2.0 actually regressed ocrs detection, so it's off
    // by default and can be tuned (it's mainly useful for dark/low-contrast frames).
    if clahe_clip_limit <= 0.0 {
        return upscaled;
    }

    let gray = image::DynamicImage::ImageRgb8(upscaled).into_luma8();
    let eq = clahe(
        &gray,
        clahe_grid[0].max(1),
        clahe_grid[1].max(1),
        clahe_clip_limit as f32,
    );

    let (w, h) = eq.dimensions();
    let mut rgb = RgbImage::new(w, h);
    for (x, y, p) in eq.enumerate_pixels() {
        let v = p[0];
        rgb.put_pixel(x, y, image::Rgb([v, v, v]));
    }
    rgb
}

/// Contrast Limited Adaptive Histogram Equalization, matching OpenCV's CLAHE
/// closely enough for OCR: per-tile clipped-histogram equalization with bilinear
/// interpolation between tile mappings.
pub fn clahe(gray: &GrayImage, tiles_x: u32, tiles_y: u32, clip_limit: f32) -> GrayImage {
    let (w, h) = gray.dimensions();
    if w == 0 || h == 0 {
        return gray.clone();
    }
    let tw = w.div_ceil(tiles_x);
    let th = h.div_ceil(tiles_y);

    // Build a 256-entry mapping LUT for each tile.
    let mut luts = vec![[0u8; 256]; (tiles_x * tiles_y) as usize];
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let x0 = tx * tw;
            let y0 = ty * th;
            let x1 = (x0 + tw).min(w);
            let y1 = (y0 + th).min(h);

            let mut hist = [0u32; 256];
            let mut count = 0u32;
            for yy in y0..y1 {
                for xx in x0..x1 {
                    hist[gray.get_pixel(xx, yy)[0] as usize] += 1;
                    count += 1;
                }
            }
            let lut = &mut luts[(ty * tiles_x + tx) as usize];
            if count == 0 {
                for (i, v) in lut.iter_mut().enumerate() {
                    *v = i as u8;
                }
                continue;
            }

            // Clip histogram and redistribute the excess uniformly (OpenCV-style).
            let clip = ((clip_limit * count as f32 / 256.0).max(1.0)) as u32;
            let mut excess = 0u32;
            for c in hist.iter_mut() {
                if *c > clip {
                    excess += *c - clip;
                    *c = clip;
                }
            }
            let inc = excess / 256;
            let rem = (excess % 256) as usize;
            for (i, c) in hist.iter_mut().enumerate() {
                *c += inc;
                if i < rem {
                    *c += 1;
                }
            }

            // CDF -> LUT scaled to [0, 255].
            let mut cdf = 0u32;
            for i in 0..256 {
                cdf += hist[i];
                lut[i] = ((cdf as f32 / count as f32) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
        }
    }

    // Bilinearly interpolate each pixel's value between the surrounding tiles.
    let clamp_x = |t: i64| t.clamp(0, (tiles_x - 1) as i64) as u32;
    let clamp_y = |t: i64| t.clamp(0, (tiles_y - 1) as i64) as u32;
    let mut out = GrayImage::new(w, h);
    for yy in 0..h {
        for xx in 0..w {
            let v = gray.get_pixel(xx, yy)[0] as usize;
            // Tile-space coordinates, centred on tile centres.
            let fx = (xx as f32 + 0.5) / tw as f32 - 0.5;
            let fy = (yy as f32 + 0.5) / th as f32 - 0.5;
            let tx0 = fx.floor();
            let ty0 = fy.floor();
            let dx = fx - tx0;
            let dy = fy - ty0;
            let (tx0i, ty0i) = (tx0 as i64, ty0 as i64);

            let l = |txi: i64, tyi: i64| {
                luts[(clamp_y(tyi) * tiles_x + clamp_x(txi)) as usize][v] as f32
            };
            let top = l(tx0i, ty0i) * (1.0 - dx) + l(tx0i + 1, ty0i) * dx;
            let bot = l(tx0i, ty0i + 1) * (1.0 - dx) + l(tx0i + 1, ty0i + 1) * dx;
            let val = (top * (1.0 - dy) + bot * dy).round().clamp(0.0, 255.0) as u8;
            out.put_pixel(xx, yy, Luma([val]));
        }
    }
    out
}
