//! Helpers for the Images to PDF and Resize tests.
#![allow(dead_code)]

use image::{Rgb, RgbImage, Rgba, RgbaImage};
use lopdf::Document;

use super::make_pdf;

/// Like `make_pdf`, but page `i` has the media box `sizes[i-1]` (width, height).
pub fn make_sized_pdf(tag: &str, sizes: &[(i64, i64)]) -> Document {
    let mut doc = make_pdf(tag, sizes.len() as u32);
    for (&id, &(w, h)) in doc.get_pages().values().zip(sizes) {
        doc.get_dictionary_mut(id)
            .unwrap()
            .set("MediaBox", vec![0.into(), 0.into(), w.into(), h.into()]);
    }
    doc
}

fn as_f64(o: &lopdf::Object) -> f64 {
    o.as_float()
        .map_or_else(|_| o.as_i64().unwrap() as f64, f64::from)
}

/// The media box of every page, as `[x0, y0, x1, y1]`.
pub fn media_boxes(doc: &Document) -> Vec<[f64; 4]> {
    doc.get_pages()
        .values()
        .map(|&id| {
            let page = doc.get_dictionary(id).unwrap();
            let arr = page.get(b"MediaBox").unwrap().as_array().unwrap();
            let mut out = [0.0; 4];
            for (slot, o) in out.iter_mut().zip(arr) {
                *slot = as_f64(o);
            }
            out
        })
        .collect()
}

/// The six numbers before the first `cm` operator of a page's content.
pub fn first_cm(doc: &Document, page: lopdf::ObjectId) -> [f64; 6] {
    let text = String::from_utf8_lossy(&doc.get_page_content(page)).into_owned();
    let before = &text[..text.find("cm").expect("no cm operator")];
    let nums: Vec<f64> = before
        .split_whitespace()
        .rev()
        .take(6)
        .map(|t| t.parse().unwrap())
        .collect();
    [nums[5], nums[4], nums[3], nums[2], nums[1], nums[0]]
}

pub fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

pub fn gradient(w: u32, h: u32) -> RgbImage {
    RgbImage::from_fn(w, h, |x, y| {
        Rgb([(x * 255 / w) as u8, (y * 255 / h) as u8, 90])
    })
}

/// A JPEG of a gradient; `dpi` writes a JFIF density.
pub fn jpeg_bytes(w: u32, h: u32, dpi: Option<u16>) -> Vec<u8> {
    let img = gradient(w, h);
    let mut out = Vec::new();
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90);
    if let Some(d) = dpi {
        enc.set_pixel_density(image::codecs::jpeg::PixelDensity::dpi(d));
    }
    enc.encode(img.as_raw(), w, h, image::ExtendedColorType::Rgb8)
        .unwrap();
    out
}

/// Inserts an EXIF block with the given orientation right after the JPEG's SOI.
pub fn with_exif_orientation(jpeg: &[u8], orientation: u16) -> Vec<u8> {
    let mut tiff = b"MM\0*\0\0\0\x08".to_vec();
    // One IFD entry: tag 0x0112 (Orientation), type SHORT, count 1.
    tiff.extend([0, 1, 0x01, 0x12, 0, 3, 0, 0, 0, 1]);
    tiff.extend(orientation.to_be_bytes());
    tiff.extend([0, 0, 0, 0, 0, 0]); // value padding, then no next IFD
    let mut body = b"Exif\0\0".to_vec();
    body.extend(tiff);
    let mut out = vec![0xFF, 0xD8, 0xFF, 0xE1];
    out.extend(((body.len() + 2) as u16).to_be_bytes());
    out.extend(body);
    out.extend(&jpeg[2..]);
    out
}

/// RGBA picture that is transparent on its left half.
pub fn half_transparent(w: u32, h: u32) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, _| {
        Rgba([200, 30, 30, if x < w / 2 { 0 } else { 255 }])
    })
}
