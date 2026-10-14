//! Comparing two renderings of the same figure (fig2dev's and RustFig's),
//! for the reference tests.

#![allow(dead_code)]
//!
//! Exact pixel equality is not the goal: the two renderers differ in
//! anti-aliasing, fonts and by a pixel here and there. Both images are
//! trimmed to their ink, scaled to a common size, converted to grey and
//! blurred slightly, then compared. The scores catch real differences
//! (a missing arrowhead, a wrong fill, a shifted text) and regressions.

use image::{GrayImage, ImageReader, Limits, Luma, Rgb, RgbImage};
use std::io::Cursor;

/// Pixels lighter than this count as background when trimming.
const INK: u8 = 245;
/// Longest side of the common comparison size.
const MAX_SIDE: u32 = 1200;
/// Largest image accepted, per side.
const MAX_DECODE: u32 = 20_000;
/// A blurred pixel differing by more than this (of 255) is a strong difference.
const STRONG: u8 = 64;

/// How different two renderings are; 0 means identical.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Score {
    /// Mean absolute grey difference, 0–1.
    pub mean: f64,
    /// Share of pixels that differ strongly, 0–1.
    pub strong: f64,
}

/// The comparison's score and images for review.
pub struct Comparison {
    pub score: Score,
    /// The reference, trimmed and scaled to the common size.
    pub reference: GrayImage,
    /// The candidate, trimmed and scaled to the common size.
    pub candidate: GrayImage,
    /// Ink only in the reference is red, only in the candidate blue, in both grey.
    pub diff: RgbImage,
}

/// Compares two encoded images (PNG or any format the `image` crate reads).
pub fn compare(reference: &[u8], candidate: &[u8]) -> Result<Comparison, String> {
    let a = trim(&decode(reference).map_err(|e| format!("reference: {e}"))?);
    let b = trim(&decode(candidate).map_err(|e| format!("candidate: {e}"))?);
    let (w, h) = common_size(&a, &b);
    let a = image::imageops::resize(&a, w, h, image::imageops::FilterType::Triangle);
    let b = image::imageops::resize(&b, w, h, image::imageops::FilterType::Triangle);
    let ba = image::imageops::blur(&a, 1.0);
    let bb = image::imageops::blur(&b, 1.0);
    let mut sum = 0u64;
    let mut strong = 0u64;
    for (pa, pb) in ba.pixels().zip(bb.pixels()) {
        let d = pa.0[0].abs_diff(pb.0[0]);
        sum += u64::from(d);
        if d > STRONG {
            strong += 1;
        }
    }
    let n = u64::from(w) * u64::from(h);
    let score = if n == 0 {
        Score { mean: 0.0, strong: 0.0 }
    } else {
        Score {
            mean: sum as f64 / (n as f64 * 255.0),
            strong: strong as f64 / n as f64,
        }
    };
    let diff = RgbImage::from_fn(w, h, |x, y| {
        let ia = a.get_pixel(x, y).0[0] < 128;
        let ib = b.get_pixel(x, y).0[0] < 128;
        match (ia, ib) {
            (true, true) => Rgb([150, 150, 150]),
            (true, false) => Rgb([220, 30, 30]),
            (false, true) => Rgb([30, 60, 220]),
            (false, false) => Rgb([255, 255, 255]),
        }
    });
    Ok(Comparison {
        score,
        reference: a,
        candidate: b,
        diff,
    })
}

/// Decodes an image and flattens it onto white, in grey.
fn decode(bytes: &[u8]) -> Result<GrayImage, String> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DECODE);
    limits.max_image_height = Some(MAX_DECODE);
    reader.limits(limits);
    let rgba = reader.decode().map_err(|e| e.to_string())?.to_rgba8();
    Ok(GrayImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, al] = rgba.get_pixel(x, y).0;
        let grey = (u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) / 1000;
        let a = u32::from(al);
        let v = (grey * a + 255 * (255 - a)) / 255;
        Luma([u8::try_from(v).unwrap_or(255)])
    }))
}

/// Crops to the bounding box of the ink, keeping a 1-pixel image when blank.
fn trim(img: &GrayImage) -> GrayImage {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if p.0[0] < INK {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    if x0 > x1 || y0 > y1 {
        return GrayImage::from_pixel(1, 1, Luma([255]));
    }
    image::imageops::crop_imm(img, x0, y0, x1 - x0 + 1, y1 - y0 + 1).to_image()
}

/// The reference's aspect ratio, at the larger of the two sizes, capped.
fn common_size(a: &GrayImage, b: &GrayImage) -> (u32, u32) {
    let (aw, ah) = (f64::from(a.width().max(1)), f64::from(a.height().max(1)));
    let longest = f64::from(
        a.width()
            .max(a.height())
            .max(b.width())
            .max(b.height())
            .clamp(1, MAX_SIDE),
    );
    let s = longest / aw.max(ah);
    let w = (aw * s).round().clamp(1.0, f64::from(MAX_SIDE));
    let h = (ah * s).round().clamp(1.0, f64::from(MAX_SIDE));
    (w as u32, h as u32)
}

/// Encodes a grey or colour image as PNG, for review output.
pub fn png<I>(img: &I) -> Result<Vec<u8>, String>
where
    I: image::GenericImageView,
    image::DynamicImage: From<I>,
    I: Clone,
{
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::from(img.clone())
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(img: &GrayImage) -> Vec<u8> {
        png(img).unwrap()
    }

    fn square(size: u32, at: u32, side: u32) -> GrayImage {
        GrayImage::from_fn(size, size, |x, y| {
            let inside = (at..at + side).contains(&x) && (at..at + side).contains(&y);
            Luma([if inside { 0 } else { 255 }])
        })
    }

    #[test]
    fn identical_images_score_zero() {
        let a = encode(&square(50, 10, 20));
        let c = compare(&a, &a).unwrap();
        assert_eq!(c.score.mean, 0.0);
        assert_eq!(c.score.strong, 0.0);
    }

    #[test]
    fn position_and_scale_do_not_matter() {
        let a = encode(&square(50, 10, 20));
        let b = encode(&square(200, 70, 80));
        let c = compare(&a, &b).unwrap();
        assert!(c.score.mean < 0.02, "{:?}", c.score);
    }

    #[test]
    fn different_shapes_score_high() {
        let a = encode(&square(50, 10, 20));
        let b = encode(&GrayImage::from_fn(50, 50, |x, y| {
            Luma([if x + y < 50 { 0 } else { 255 }])
        }));
        let c = compare(&a, &b).unwrap();
        assert!(c.score.strong > 0.2, "{:?}", c.score);
        assert_eq!(c.diff.dimensions(), c.reference.dimensions());
    }

    #[test]
    fn blank_and_garbage_inputs() {
        let blank = encode(&GrayImage::from_pixel(10, 10, Luma([255])));
        assert_eq!(compare(&blank, &blank).unwrap().score.mean, 0.0);
        assert!(compare(b"not an image", &blank).is_err());
    }
}
