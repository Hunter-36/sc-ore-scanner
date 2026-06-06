//! Image preprocessing for OCR, faithful to the v1 Python `preprocess_image`:
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

/// Convert a `[x, y, w, h]` region of an RGBA frame buffer straight to an RGB
/// image, copying **only the region's pixels** (not the whole frame). `region` is
/// clamped to the frame bounds. This keeps screen capture cheap: a 4K grab is
/// ~33 MB, but the scan region is a few thousand pixels.
pub fn crop_rgba_to_rgb(raw: &[u8], full_w: u32, full_h: u32, region: [u32; 4]) -> RgbImage {
    let [rx, ry, rw, rh] = region;
    let x0 = rx.min(full_w);
    let y0 = ry.min(full_h);
    let w = rw.min(full_w - x0);
    let h = rh.min(full_h - y0);
    if w == 0 || h == 0 {
        return RgbImage::new(1, 1);
    }
    let mut rgb = RgbImage::new(w, h);
    for yy in 0..h {
        let row = ((y0 + yy) as usize * full_w as usize + x0 as usize) * 4;
        for xx in 0..w {
            let o = row + xx as usize * 4;
            rgb.put_pixel(xx, yy, image::Rgb([raw[o], raw[o + 1], raw[o + 2]]));
        }
    }
    rgb
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

#[cfg(test)]
mod tests {
    use super::{clahe, crop_and_upscale, crop_rgba_to_rgb};
    use image::{GrayImage, Luma, Rgb, RgbImage};

    #[test]
    fn crops_region_from_rgba_buffer() {
        // 3x2 RGBA frame; pixel R = (y*3 + x) * 10, G=1, B=2, A=255.
        let (full_w, full_h) = (3u32, 2u32);
        let mut raw = Vec::new();
        for i in 0..(full_w * full_h) {
            raw.extend_from_slice(&[(i as u8) * 10, 1, 2, 255]);
        }
        let img = crop_rgba_to_rgb(&raw, full_w, full_h, [1, 0, 2, 2]);
        assert_eq!(img.dimensions(), (2, 2));
        assert_eq!(img.get_pixel(0, 0).0, [10, 1, 2]); // idx 1
        assert_eq!(img.get_pixel(1, 0).0, [20, 1, 2]); // idx 2
        assert_eq!(img.get_pixel(0, 1).0, [40, 1, 2]); // idx 4
        assert_eq!(img.get_pixel(1, 1).0, [50, 1, 2]); // idx 5
    }

    #[test]
    fn clamps_out_of_bounds_region() {
        let raw = vec![9u8, 9, 9, 255]; // 1x1
        let img = crop_rgba_to_rgb(&raw, 1, 1, [0, 0, 100, 100]);
        assert_eq!(img.dimensions(), (1, 1));
        assert_eq!(img.get_pixel(0, 0).0, [9, 9, 9]);
    }

    /// A 4×4 image where each pixel's red channel encodes its index, so crops are
    /// verifiable pixel-for-pixel.
    fn indexed_rgb() -> RgbImage {
        RgbImage::from_fn(4, 4, |x, y| Rgb([(y * 4 + x) as u8, 1, 2]))
    }

    #[test]
    fn crop_and_upscale_crops_exactly_at_scale_1() {
        let img = indexed_rgb();
        // Region [1,0,2,2], no upscale → exact 2×2 crop of the original pixels.
        let out = crop_and_upscale(&img, Some([1, 0, 2, 2]), 1);
        assert_eq!(out.dimensions(), (2, 2));
        assert_eq!(out.get_pixel(0, 0).0, [1, 1, 2]); // original (1,0)
        assert_eq!(out.get_pixel(1, 0).0, [2, 1, 2]); // original (2,0)
        assert_eq!(out.get_pixel(0, 1).0, [5, 1, 2]); // original (1,1)
        assert_eq!(out.get_pixel(1, 1).0, [6, 1, 2]); // original (2,1)
    }

    #[test]
    fn crop_and_upscale_multiplies_dimensions() {
        let img = indexed_rgb();
        // Region 2×2 upscaled ×4 → 8×8 (Lanczos interpolates values; only the
        // dimensions are asserted).
        let out = crop_and_upscale(&img, Some([0, 0, 2, 2]), 4);
        assert_eq!(out.dimensions(), (8, 8));
    }

    #[test]
    fn crop_and_upscale_none_region_scale_1_is_identity() {
        let img = indexed_rgb();
        let out = crop_and_upscale(&img, None, 1);
        assert_eq!(out.dimensions(), (4, 4));
        assert_eq!(out.as_raw(), img.as_raw());
    }

    #[test]
    fn clahe_preserves_dimensions_and_is_deterministic() {
        // Horizontal gradient, 8×8.
        let g = GrayImage::from_fn(8, 8, |x, _| Luma([(x * 32) as u8]));
        let a = clahe(&g, 2, 2, 2.0);
        let b = clahe(&g, 2, 2, 2.0);
        assert_eq!(a.dimensions(), (8, 8));
        assert_eq!(a.as_raw(), b.as_raw(), "clahe must be deterministic");
    }

    #[test]
    fn clahe_empty_image_is_noop() {
        let g = GrayImage::new(0, 0);
        let out = clahe(&g, 8, 8, 2.0);
        assert_eq!(out.dimensions(), (0, 0));
    }

    #[test]
    fn clahe_spreads_a_low_contrast_image() {
        // Values clustered in a narrow band [100, 116) — CLAHE should widen the
        // spread (max-min) rather than leave it unchanged.
        let g = GrayImage::from_fn(8, 8, |x, y| Luma([100 + (((y * 8 + x) % 16) as u8)]));
        let out = clahe(&g, 1, 1, 40.0);
        let span = |img: &GrayImage| {
            let vals: Vec<u8> = img.pixels().map(|p| p[0]).collect();
            vals.iter().max().unwrap() - vals.iter().min().unwrap()
        };
        assert!(
            span(&out) >= span(&g),
            "CLAHE should not reduce contrast (in {} -> out {})",
            span(&g),
            span(&out)
        );
    }
}
