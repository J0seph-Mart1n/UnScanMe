//! Pure-Rust document image preprocessing: crop, 90°/arbitrary rotate, and auto-deskew.

use anyhow::{bail, Context, Result};
use image::{DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage};
use std::io::Cursor;

fn encode_png(img: &DynamicImage) -> Result<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png)?;
    Ok(out.into_inner())
}

#[cfg(test)]
pub fn image_dimensions(bytes: &[u8]) -> Result<(u32, u32)> {
    let img = image::load_from_memory(bytes).context("unsupported or corrupt image")?;
    Ok(img.dimensions())
}

pub fn rotate_90_cw(bytes: &[u8]) -> Result<Vec<u8>> {
    let img = image::load_from_memory(bytes).context("unsupported or corrupt image")?;
    encode_png(&img.rotate90())
}

pub fn rotate_90_ccw(bytes: &[u8]) -> Result<Vec<u8>> {
    let img = image::load_from_memory(bytes).context("unsupported or corrupt image")?;
    encode_png(&img.rotate270())
}

/// Crop an image using normalized coordinates `(x, y, w, h)` in `[0.0, 1.0]`.
pub fn crop_normalized(bytes: &[u8], x: f32, y: f32, w: f32, h: f32) -> Result<Vec<u8>> {
    if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() {
        bail!("invalid crop coordinates");
    }
    let img = image::load_from_memory(bytes).context("unsupported or corrupt image")?;
    let (iw, ih) = img.dimensions();
    if iw == 0 || ih == 0 {
        bail!("cannot crop an empty image");
    }

    let nx = x.clamp(0.0, 1.0);
    let ny = y.clamp(0.0, 1.0);
    let nw = w.clamp(0.0, 1.0 - nx);
    let nh = h.clamp(0.0, 1.0 - ny);

    let px = ((nx * iw as f32).round() as u32).min(iw - 1);
    let py = ((ny * ih as f32).round() as u32).min(ih - 1);
    let pw = ((nw * iw as f32).round() as u32).max(1).min(iw - px);
    let ph = ((nh * ih as f32).round() as u32).max(1).min(ih - py);

    let cropped = img.crop_imm(px, py, pw, ph);
    encode_png(&cropped)
}

/// Rotate an image clockwise by `degrees` using bilinear interpolation and white background fill.
/// Expands the canvas just enough to avoid clipping document corners.
pub fn rotate_angle(bytes: &[u8], degrees: f32) -> Result<Vec<u8>> {
    if !degrees.is_finite() {
        bail!("invalid rotation angle");
    }
    let img = image::load_from_memory(bytes).context("unsupported or corrupt image")?;
    if degrees.abs() < 0.05 {
        return encode_png(&img);
    }

    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let theta = degrees.to_radians();
    let cos_t = theta.cos();
    let sin_t = theta.sin();

    // Compute expanded bounding box for rotated rectangle.
    let new_w = ((w as f32 * cos_t.abs()) + (h as f32 * sin_t.abs())).ceil().max(1.0) as u32;
    let new_h = ((w as f32 * sin_t.abs()) + (h as f32 * cos_t.abs())).ceil().max(1.0) as u32;

    let src_cx = (w as f32 - 1.0) * 0.5;
    let src_cy = (h as f32 - 1.0) * 0.5;
    let dst_cx = (new_w as f32 - 1.0) * 0.5;
    let dst_cy = (new_h as f32 - 1.0) * 0.5;

    let mut out = RgbaImage::from_pixel(new_w, new_h, Rgba([255, 255, 255, 255]));

    for dy in 0..new_h {
        let ry = dy as f32 - dst_cy;
        for dx in 0..new_w {
            let rx = dx as f32 - dst_cx;
            // Inverse rotation (clockwise screen coords: +theta rotates clockwise)
            let sx = rx * cos_t + ry * sin_t + src_cx;
            let sy = -rx * sin_t + ry * cos_t + src_cy;

            if sx < 0.0 || sy < 0.0 || sx > (w - 1) as f32 || sy > (h - 1) as f32 {
                continue;
            }

            let x0 = sx.floor() as u32;
            let y0 = sy.floor() as u32;
            let x1 = (x0 + 1).min(w - 1);
            let y1 = (y0 + 1).min(h - 1);
            let fx = sx - x0 as f32;
            let fy = sy - y0 as f32;

            let p00 = rgba.get_pixel(x0, y0).0;
            let p10 = rgba.get_pixel(x1, y0).0;
            let p01 = rgba.get_pixel(x0, y1).0;
            let p11 = rgba.get_pixel(x1, y1).0;

            let mut px = [255u8; 4];
            for c in 0..4 {
                let top = p00[c] as f32 * (1.0 - fx) + p10[c] as f32 * fx;
                let bot = p01[c] as f32 * (1.0 - fx) + p11[c] as f32 * fx;
                px[c] = (top * (1.0 - fy) + bot * fy).round().clamp(0.0, 255.0) as u8;
            }
            out.put_pixel(dx, dy, Rgba(px));
        }
    }

    encode_png(&DynamicImage::ImageRgba8(out))
}

/// Estimate the clockwise rotation angle (in `[-15.0, +15.0]` degrees) needed to
/// level horizontal text lines in a scanned/photographed document image.
pub fn estimate_skew_angle(bytes: &[u8]) -> Result<f32> {
    let img = image::load_from_memory(bytes).context("unsupported or corrupt image")?;
    let small = if img.width().max(img.height()) > 360 {
        img.resize(360, 360, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let gray = small.to_luma8();
    let (w, h) = gray.dimensions();
    if w < 16 || h < 16 {
        return Ok(0.0);
    }

    // Extract foreground ink points using adaptive darkness threshold relative to mean luminance.
    let mut sum_luma = 0u64;
    for p in gray.pixels() {
        sum_luma += p.0[0] as u64;
    }
    let mean_luma = (sum_luma as f32) / ((w * h) as f32);
    let threshold = (mean_luma - 28.0).clamp(40.0, 210.0);

    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;
    let mut ink_points: Vec<(f32, f32, f32)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let v = gray.get_pixel(x, y).0[0] as f32;
            if v < threshold {
                let weight = (threshold - v).max(1.0);
                ink_points.push((x as f32 - cx, y as f32 - cy, weight));
            }
        }
    }

    if ink_points.len() < 20 {
        return Ok(0.0);
    }

    let diag = ((w * w + h * h) as f32).sqrt().ceil() as usize;
    let bins = diag + 4;
    let half_bins = (bins / 2) as f32;

    let score_angle = |deg: f32| -> f64 {
        let rad = deg.to_radians();
        let sin_t = rad.sin();
        let cos_t = rad.cos();
        let mut hist = vec![0.0f64; bins];
        for &(px, py, weight) in &ink_points {
            // Clockwise rotation y' = x*sin(t) + y*cos(t)
            let ry = px * sin_t + py * cos_t + half_bins;
            let idx = ry.round() as isize;
            if idx >= 0 && (idx as usize) < bins {
                hist[idx as usize] += weight as f64;
            }
        }
        // Maximize sum of squared row energies (sharp horizontal bands -> higher peak energy)
        hist.iter().map(|&v| v * v).sum()
    };

    let baseline_score = score_angle(0.0);
    let mut best_angle = 0.0f32;
    let mut best_score = baseline_score;

    // Coarse search: -15.0 to +15.0 in 0.5° steps
    let mut deg = -15.0f32;
    while deg <= 15.01 {
        let s = score_angle(deg);
        if s > best_score {
            best_score = s;
            best_angle = deg;
        }
        deg += 0.5;
    }

    // Fine search around best_angle: +/- 0.6° in 0.1° steps
    let start = (best_angle - 0.6).max(-15.0);
    let end = (best_angle + 0.6).min(15.0);
    let mut fine = start;
    while fine <= end + 0.001 {
        let s = score_angle(fine);
        if s > best_score {
            best_score = s;
            best_angle = fine;
        }
        fine += 0.1;
    }

    // Only report a correction if it meaningfully improves alignment over 0°
    if best_angle.abs() < 0.25 || best_score <= baseline_score * 1.015 {
        return Ok(0.0);
    }

    Ok((best_angle * 10.0).round() / 10.0)
}

/// Automatically detect skew and rotate the image to straighten it.
/// Returns `(processed_png_bytes, applied_degrees)`.
pub fn deskew(bytes: &[u8]) -> Result<(Vec<u8>, f32)> {
    let angle = estimate_skew_angle(bytes)?;
    if angle.abs() < 0.25 {
        let img = image::load_from_memory(bytes).context("unsupported or corrupt image")?;
        return Ok((encode_png(&img)?, 0.0));
    }
    let rotated = rotate_angle(bytes, angle)?;
    Ok((rotated, angle))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_lined_test_image(w: u32, h: u32) -> Vec<u8> {
        let mut img = RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255]));
        // Draw crisp horizontal dark text-like stripes
        for row_idx in 0..8 {
            let y_base = 24 + row_idx * 22;
            for dy in 0..5 {
                let y = y_base + dy;
                if y < h {
                    for x in 20..(w - 20) {
                        if (x / 7) % 5 != 0 {
                            img.put_pixel(x, y, Rgba([15, 15, 15, 255]));
                        }
                    }
                }
            }
        }
        encode_png(&DynamicImage::ImageRgba8(img)).unwrap()
    }

    #[test]
    fn rotate_90_swaps_dimensions() {
        let mut img = RgbaImage::from_pixel(120, 80, Rgba([255, 255, 255, 255]));
        img.put_pixel(10, 10, Rgba([0, 0, 0, 255]));
        let png = encode_png(&DynamicImage::ImageRgba8(img)).unwrap();

        let cw = rotate_90_cw(&png).unwrap();
        assert_eq!(image_dimensions(&cw).unwrap(), (80, 120));

        let ccw = rotate_90_ccw(&cw).unwrap();
        assert_eq!(image_dimensions(&ccw).unwrap(), (120, 80));
    }

    #[test]
    fn crop_normalized_extracts_exact_region_and_clamps() {
        let mut img = RgbaImage::from_pixel(100, 100, Rgba([255, 255, 255, 255]));
        for y in 25..75 {
            for x in 20..60 {
                img.put_pixel(x, y, Rgba([16, 163, 74, 255]));
            }
        }
        let png = encode_png(&DynamicImage::ImageRgba8(img)).unwrap();
        let cropped = crop_normalized(&png, 0.2, 0.25, 0.4, 0.5).unwrap();
        assert_eq!(image_dimensions(&cropped).unwrap(), (40, 50));

        let decoded = image::load_from_memory(&cropped).unwrap().to_rgba8();
        assert_eq!(decoded.get_pixel(5, 5).0, [16, 163, 74, 255]);

        // Clamping beyond 1.0 shouldn't panic
        let clamped = crop_normalized(&png, 0.8, 0.8, 0.5, 0.5).unwrap();
        assert_eq!(image_dimensions(&clamped).unwrap(), (20, 20));
    }

    #[test]
    fn deskew_recovers_skewed_document_angle() {
        let straight = make_lined_test_image(240, 220);
        // Already straight -> ~0.0°
        assert_eq!(estimate_skew_angle(&straight).unwrap(), 0.0);

        // Rotate clockwise by 6.0 degrees; deskew should suggest ~ -6.0 degrees
        let skewed = rotate_angle(&straight, 6.0).unwrap();
        let correction = estimate_skew_angle(&skewed).unwrap();
        assert!(
            (correction - (-6.0)).abs() <= 0.6,
            "expected correction near -6.0, got {correction}"
        );

        let (fixed, applied) = deskew(&skewed).unwrap();
        assert!((applied - (-6.0)).abs() <= 0.6);
        // After deskewing, remaining skew should be near zero
        let residual = estimate_skew_angle(&fixed).unwrap();
        assert!(residual.abs() <= 0.6, "expected residual near 0, got {residual}");
    }

    #[test]
    fn rejects_invalid_bytes_and_non_finite_params() {
        assert!(rotate_90_cw(b"garbage").is_err());
        assert!(crop_normalized(b"garbage", 0.0, 0.0, 0.5, 0.5).is_err());
        let straight = make_lined_test_image(80, 80);
        assert!(crop_normalized(&straight, f32::NAN, 0.0, 0.5, 0.5).is_err());
        assert!(rotate_angle(&straight, f32::INFINITY).is_err());
    }
}
