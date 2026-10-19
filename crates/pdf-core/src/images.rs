//! Images to PDF (SPEC 5.8).
//!
//! JPEGs are embedded as they are (`DCTDecode`, no re-encoding). Everything
//! else is decoded and stored as Flate-compressed pixels, with transparency in
//! an `/SMask`. EXIF orientation and the user's rotation are never baked into
//! pixels: they become part of the matrix that places the image on the page.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use image::{ColorType, DynamicImage, ExtendedColorType, ImageDecoder, ImageFormat, ImageReader};
use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream};
use serde::{Deserialize, Serialize};

use crate::assemble::write_atomic;
use crate::error::{Error, Result};
use crate::paper::{margin_pt, PageOrientation, Paper, MAX_SIDE_PT};
use crate::split::{unique_path, Written};

/// Pixels per inch assumed when an image does not say.
const DEFAULT_DPI: f64 = 96.0;
/// Long edge of images saved with `Quality::Reduce`.
const REDUCED_LONG_EDGE: u32 = 2000;
const REDUCED_JPEG_QUALITY: u8 = 80;

/// One image to place, with the user's extra turn.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ImageInput {
    pub path: PathBuf,
    /// Clockwise turn on top of the EXIF orientation, a multiple of 90.
    #[serde(default)]
    pub rotate: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    /// Scale to fit inside the margins; nothing is cut off.
    #[default]
    Inside,
    /// Scale to cover the area inside the margins; the overflow is cropped.
    Fill,
    /// Natural size at the image's DPI, centred (cropped if larger than the page).
    Original,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    #[default]
    Keep,
    /// JPEG quality 80, long edge at most 2000 px.
    Reduce,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ImageOptions {
    /// `None` makes each page exactly as large as its image (plus margins).
    pub page: Option<Paper>,
    #[serde(default)]
    pub orientation: PageOrientation,
    #[serde(default)]
    pub margin_mm: f64,
    #[serde(default)]
    pub fit: Fit,
    #[serde(default)]
    pub quality: Quality,
}

/// What the UI needs to know about an image once it has been added.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ImageMeta {
    /// Size in pixels as displayed, i.e. after EXIF orientation.
    pub width: u32,
    pub height: u32,
}

// Reading ------------------------------------------------------------------

fn display_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(|| "image".into(), |n| n.to_string_lossy().into_owned())
}

fn sniff(bytes: &[u8], name: &str) -> Result<ImageFormat> {
    match image::guess_format(bytes) {
        Ok(
            f @ (ImageFormat::Jpeg
            | ImageFormat::Png
            | ImageFormat::Bmp
            | ImageFormat::Gif
            | ImageFormat::Tiff
            | ImageFormat::WebP),
        ) => Ok(f),
        _ => Err(Error::UnsupportedImage(name.into())),
    }
}

/// Opens just the header: stored size and EXIF orientation (1 to 8).
fn header(bytes: &[u8], format: ImageFormat, name: &str) -> Result<(u32, u32, u8)> {
    let bad = |_| Error::BadImage(name.into());
    let mut decoder = ImageReader::with_format(Cursor::new(bytes), format)
        .into_decoder()
        .map_err(bad)?;
    let (w, h) = decoder.dimensions();
    let exif = decoder.orientation().map_or(1, |o| o.to_exif());
    Ok((w, h, exif))
}

/// Size of an image as the user will see it (EXIF orientation applied).
pub fn probe(path: &Path) -> Result<ImageMeta> {
    let name = display_name(path);
    let bytes = std::fs::read(path)?;
    let format = sniff(&bytes, &name)?;
    let (w, h, exif) = header(&bytes, format, &name)?;
    Ok(if exif >= 5 {
        ImageMeta {
            width: h,
            height: w,
        }
    } else {
        ImageMeta {
            width: w,
            height: h,
        }
    })
}

/// A small preview, JPEG (or PNG when there is transparency), orientation applied.
pub fn thumbnail(path: &Path, max_edge: u32) -> Result<Vec<u8>> {
    let name = display_name(path);
    let bytes = std::fs::read(path)?;
    let format = sniff(&bytes, &name)?;
    let bad = |_| Error::BadImage(name.clone());
    let mut decoder = ImageReader::with_format(Cursor::new(&bytes), format)
        .into_decoder()
        .map_err(bad)?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder).map_err(bad)?;
    img.apply_orientation(orientation);
    let small = img.thumbnail(max_edge, max_edge);
    let mut out = Vec::new();
    if small.color().has_alpha() {
        small
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .map_err(bad)?;
    } else {
        let rgb = small.into_rgb8();
        let (w, h) = rgb.dimensions();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 80)
            .encode(rgb.as_raw(), w, h, ExtendedColorType::Rgb8)
            .map_err(bad)?;
    }
    Ok(out)
}

/// What a JPEG header says, found without decoding the picture.
struct JpegInfo {
    /// Start-of-frame type: 0 baseline, 1 extended sequential, 2 progressive.
    /// Other frame types cannot be used as `DCTDecode`.
    frame: u8,
    precision: u8,
    components: u8,
    dpi: Option<f64>,
}

impl JpegInfo {
    /// True if the file can go into the PDF byte for byte.
    fn embeddable(&self) -> bool {
        self.frame <= 2 && self.precision == 8 && matches!(self.components, 1 | 3)
    }
}

fn scan_jpeg(b: &[u8]) -> Option<JpegInfo> {
    if !b.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut dpi = None;
    let mut i = 2;
    while i + 4 <= b.len() {
        if b[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = b[i + 1];
        if marker == 0xFF {
            i += 1; // fill byte
            continue;
        }
        if marker == 0x01 || (0xD0..=0xD9).contains(&marker) {
            i += 2; // markers without a length
            continue;
        }
        let len = usize::from(u16::from_be_bytes([b[i + 2], b[i + 3]]));
        let body = b.get(i + 4..i + 2 + len)?;
        match marker {
            0xE0 if body.starts_with(b"JFIF\0") && body.len() >= 12 => {
                let x = f64::from(u16::from_be_bytes([body[8], body[9]]));
                let y = f64::from(u16::from_be_bytes([body[10], body[11]]));
                let per_inch = match body[7] {
                    1 => 1.0,
                    2 => 2.54, // dots per centimetre
                    _ => 0.0,  // 0 only gives an aspect ratio
                };
                dpi = (per_inch > 0.0 && x > 0.0 && (x - y).abs() <= x * 0.01)
                    .then_some(x * per_inch);
            }
            0xC0..=0xCF if !matches!(marker, 0xC4 | 0xC8 | 0xCC) && body.len() >= 6 => {
                return Some(JpegInfo {
                    frame: marker - 0xC0,
                    precision: body[0],
                    components: body[5],
                    dpi,
                });
            }
            0xDA => return None, // picture data before any frame header
            _ => {}
        }
        i += 2 + len;
    }
    None
}

/// True if the JPEG's segments lead to an end-of-image marker, i.e. the file
/// was not cut off. Data after the marker (some phones append a video) is fine.
fn jpeg_is_complete(b: &[u8]) -> bool {
    let mut i = 2;
    while i + 1 < b.len() {
        if b[i] != 0xFF {
            i += 1; // picture data
            continue;
        }
        match b[i + 1] {
            0xD9 => return true,
            0xFF => i += 1, // fill byte before a marker
            // Stuffed zero in picture data, restart and standalone markers.
            0x00 | 0x01 | 0xD0..=0xD8 => i += 2,
            _ => match b.get(i + 2..i + 4) {
                Some(len) => i += 2 + usize::from(u16::from_be_bytes([len[0], len[1]])),
                None => return false,
            },
        }
    }
    false
}

/// Pixel density from a PNG `pHYs` chunk, if it is in metres.
fn png_dpi(b: &[u8]) -> Option<f64> {
    let mut i = 8;
    while i + 12 <= b.len() {
        let len = u32::from_be_bytes(b[i..i + 4].try_into().ok()?) as usize;
        match &b[i + 4..i + 8] {
            b"pHYs" if len == 9 => {
                let d = b.get(i + 8..i + 17)?;
                let x = f64::from(u32::from_be_bytes(d[0..4].try_into().ok()?));
                let y = f64::from(u32::from_be_bytes(d[4..8].try_into().ok()?));
                return (d[8] == 1 && x > 0.0 && (x - y).abs() <= x * 0.01).then_some(x * 0.0254);
            }
            b"IDAT" => return None,
            _ => {}
        }
        i += 12 + len;
    }
    None
}

enum Pixels {
    /// The original JPEG file, to be embedded unchanged.
    Jpeg { data: Vec<u8>, gray: bool },
    Raster {
        gray: bool,
        data: Vec<u8>,
        alpha: Option<Vec<u8>>,
    },
}

struct Prepared {
    /// Stored size in pixels (the picture before any orientation).
    width: u32,
    height: u32,
    /// EXIF orientation, 1 to 8.
    exif: u8,
    dpi: f64,
    pixels: Pixels,
}

fn plausible_dpi(dpi: Option<f64>) -> f64 {
    dpi.filter(|d| (10.0..=2400.0).contains(d))
        .unwrap_or(DEFAULT_DPI)
}

fn prepare(path: &Path, quality: Quality) -> Result<Prepared> {
    let name = display_name(path);
    let bad = |_| Error::BadImage(name.clone());
    let bytes = std::fs::read(path)?;
    let format = sniff(&bytes, &name)?;
    let mut decoder = ImageReader::with_format(Cursor::new(&bytes), format)
        .into_decoder()
        .map_err(bad)?;
    let (width, height) = decoder.dimensions();
    let exif = decoder.orientation().map_or(1, |o| o.to_exif());

    let jpeg = (format == ImageFormat::Jpeg)
        .then(|| scan_jpeg(&bytes))
        .flatten();
    let dpi = plausible_dpi(match format {
        ImageFormat::Jpeg => jpeg.as_ref().and_then(|j| j.dpi),
        ImageFormat::Png => png_dpi(&bytes),
        _ => None,
    });
    if format == ImageFormat::Jpeg && !jpeg_is_complete(&bytes) {
        // The decoder would quietly draw what it has; better to say so.
        return Err(Error::BadImage(name.clone()));
    }
    let direct = jpeg.filter(JpegInfo::embeddable).map(|j| Pixels::Jpeg {
        data: bytes.clone(),
        gray: j.components == 1,
    });
    if quality == Quality::Keep {
        if let Some(pixels) = direct {
            return Ok(Prepared {
                width,
                height,
                exif,
                dpi,
                pixels,
            });
        }
    }

    let mut img = DynamicImage::from_decoder(decoder).map_err(bad)?;
    let gray = matches!(
        img.color(),
        ColorType::L8 | ColorType::L16 | ColorType::La8 | ColorType::La16
    );
    let mut dpi = dpi;
    let mut resized = false;
    if quality == Quality::Reduce && width.max(height) > REDUCED_LONG_EDGE {
        let k = f64::from(REDUCED_LONG_EDGE) / f64::from(width.max(height));
        let nw = ((f64::from(width) * k).round() as u32).max(1);
        let nh = ((f64::from(height) * k).round() as u32).max(1);
        img = img.resize_exact(nw, nh, FilterType::Lanczos3);
        // Fewer pixels for the same printed size.
        dpi *= k;
        resized = true;
    }
    let (w, h) = (img.width(), img.height());

    // Split off transparency; a fully opaque alpha channel is dropped.
    let (data, alpha) = match (gray, img.color().has_alpha()) {
        (true, false) => (img.into_luma8().into_raw(), None),
        (false, false) => (img.into_rgb8().into_raw(), None),
        (true, true) => split_alpha(img.into_luma_alpha8().into_raw(), 2),
        (false, true) => split_alpha(img.into_rgba8().into_raw(), 4),
    };

    let pixels = if quality == Quality::Reduce && alpha.is_none() {
        let mut out = Vec::new();
        let color = if gray {
            ExtendedColorType::L8
        } else {
            ExtendedColorType::Rgb8
        };
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, REDUCED_JPEG_QUALITY)
            .encode(&data, w, h, color)
            .map_err(|_| Error::BadImage(name.clone()))?;
        match direct {
            // Never make a JPEG bigger by "reducing" it.
            Some(original) if !resized && out.len() >= bytes.len() => original,
            _ => Pixels::Jpeg { data: out, gray },
        }
    } else {
        Pixels::Raster { gray, data, alpha }
    };
    Ok(Prepared {
        width: w,
        height: h,
        exif,
        dpi,
        pixels,
    })
}

/// Splits interleaved pixels (`stride` bytes each, alpha last) into colour and alpha.
fn split_alpha(raw: Vec<u8>, stride: usize) -> (Vec<u8>, Option<Vec<u8>>) {
    let mut color = Vec::with_capacity(raw.len() / stride * (stride - 1));
    let mut alpha = Vec::with_capacity(raw.len() / stride);
    for px in raw.chunks_exact(stride) {
        color.extend_from_slice(&px[..stride - 1]);
        alpha.push(px[stride - 1]);
    }
    let opaque = alpha.iter().all(|&a| a == 255);
    (color, (!opaque).then_some(alpha))
}

// Placement ----------------------------------------------------------------

/// Affine map `[a, b, c, d, e, f]`: `x' = a x + c y + e`, `y' = b x + d y + f`.
type Matrix = [f64; 6];

const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `second` after `first`.
fn then(first: Matrix, second: Matrix) -> Matrix {
    let [a1, b1, c1, d1, e1, f1] = first;
    let [a2, b2, c2, d2, e2, f2] = second;
    [
        a2 * a1 + c2 * b1,
        b2 * a1 + d2 * b1,
        a2 * c1 + c2 * d1,
        b2 * c1 + d2 * d1,
        a2 * e1 + c2 * f1 + e2,
        b2 * e1 + d2 * f1 + f2,
    ]
}

/// Map from the stored picture's unit square to the displayed unit square
/// (both with y pointing up): first the EXIF orientation, then `quarter_turns`
/// clockwise quarter turns.
fn unit_matrix(exif: u8, quarter_turns: i32) -> Matrix {
    let orient: Matrix = match exif {
        2 => [-1.0, 0.0, 0.0, 1.0, 1.0, 0.0],
        3 => [-1.0, 0.0, 0.0, -1.0, 1.0, 1.0],
        4 => [1.0, 0.0, 0.0, -1.0, 0.0, 1.0],
        5 => [0.0, -1.0, -1.0, 0.0, 1.0, 1.0],
        6 => [0.0, -1.0, 1.0, 0.0, 0.0, 1.0],
        7 => [0.0, 1.0, 1.0, 0.0, 0.0, 0.0],
        8 => [0.0, 1.0, -1.0, 0.0, 1.0, 0.0],
        _ => IDENTITY,
    };
    // One clockwise quarter turn: (x, y) -> (y, 1 - x).
    let quarter: Matrix = [0.0, -1.0, 1.0, 0.0, 0.0, 1.0];
    (0..quarter_turns.rem_euclid(4)).fold(orient, |m, _| then(m, quarter))
}

fn num(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() { "0" } else { s }.to_string()
}

/// Where and how large one image goes on its page, in points.
struct Layout {
    page: (f64, f64),
    /// Origin and size of the image rectangle.
    rect: (f64, f64, f64, f64),
    /// Area to clip to, if the image can spill over it.
    clip: Option<(f64, f64, f64, f64)>,
}

fn layout(shown_px: (f64, f64), dpi: f64, opts: &ImageOptions, margin: f64) -> Result<Layout> {
    let natural = (shown_px.0 / dpi * 72.0, shown_px.1 / dpi * 72.0);
    let Some(paper) = opts.page else {
        // The page is the image plus its margins, kept below the viewer limit.
        let shrink =
            (MAX_SIDE_PT / (natural.0 + 2.0 * margin).max(natural.1 + 2.0 * margin)).min(1.0);
        let (iw, ih) = (natural.0 * shrink, natural.1 * shrink);
        let m = margin * shrink;
        return Ok(Layout {
            page: (iw + 2.0 * m, ih + 2.0 * m),
            rect: (m, m, iw, ih),
            clip: None,
        });
    };
    let wide = shown_px.0 > shown_px.1;
    let (pw, ph) = opts.orientation.orient(paper.portrait_pt()?, wide);
    let (cw, ch) = (pw - 2.0 * margin, ph - 2.0 * margin);
    if cw <= 1.0 || ch <= 1.0 {
        return Err(Error::BadMargin);
    }
    let (kx, ky) = (cw / natural.0, ch / natural.1);
    let scale = match opts.fit {
        Fit::Inside => kx.min(ky),
        Fit::Fill => kx.max(ky),
        Fit::Original => 1.0,
    };
    let (iw, ih) = (natural.0 * scale, natural.1 * scale);
    Ok(Layout {
        page: (pw, ph),
        rect: (margin + (cw - iw) / 2.0, margin + (ch - ih) / 2.0, iw, ih),
        clip: (opts.fit != Fit::Inside).then_some((margin, margin, cw, ch)),
    })
}

// Writing ------------------------------------------------------------------

fn add_image_object(doc: &mut Document, p: &Prepared) -> ObjectId {
    let color = |gray: bool| if gray { "DeviceGray" } else { "DeviceRGB" };
    let mut dict = dictionary! {
        "Type" => "XObject",
        "Subtype" => "Image",
        "Width" => i64::from(p.width),
        "Height" => i64::from(p.height),
        "BitsPerComponent" => 8,
    };
    match &p.pixels {
        Pixels::Jpeg { data, gray } => {
            dict.set("ColorSpace", color(*gray));
            dict.set("Filter", "DCTDecode");
            doc.add_object(Stream::new(dict, data.clone()).with_compression(false))
        }
        Pixels::Raster { gray, data, alpha } => {
            dict.set("ColorSpace", color(*gray));
            if let Some(alpha) = alpha {
                let mask = dictionary! {
                    "Type" => "XObject",
                    "Subtype" => "Image",
                    "Width" => i64::from(p.width),
                    "Height" => i64::from(p.height),
                    "ColorSpace" => "DeviceGray",
                    "BitsPerComponent" => 8,
                };
                let mask = doc.add_object(flate(mask, alpha.clone()));
                dict.set("SMask", mask);
            }
            doc.add_object(flate(dict, data.clone()))
        }
    }
}

fn flate(dict: Dictionary, data: Vec<u8>) -> Stream {
    let mut stream = Stream::new(dict, data);
    // A failed compression just leaves the stream uncompressed, which is valid.
    let _ = stream.compress();
    stream
}

fn add_page(
    doc: &mut Document,
    pages_root: ObjectId,
    p: &Prepared,
    rotate: i32,
    opts: &ImageOptions,
    margin: f64,
) -> Result<ObjectId> {
    let turns = rotate.rem_euclid(360) / 90;
    let swap = (p.exif >= 5) != (turns % 2 == 1);
    let shown = if swap {
        (f64::from(p.height), f64::from(p.width))
    } else {
        (f64::from(p.width), f64::from(p.height))
    };
    let lay = layout(shown, p.dpi, opts, margin)?;

    let (x, y, w, h) = lay.rect;
    let [a, b, c, d, e, f] = unit_matrix(p.exif, turns);
    let cm = [w * a, h * b, w * c, h * d, w * e + x, h * f + y];
    let mut content = String::from("q\n");
    if let Some((cx, cy, cw, ch)) = lay.clip {
        content += &format!("{} {} {} {} re W n\n", num(cx), num(cy), num(cw), num(ch));
    }
    let cm: Vec<String> = cm.iter().map(|v| num(*v)).collect();
    content += &format!("{} cm\n/Im0 Do\nQ\n", cm.join(" "));

    let image = add_image_object(doc, p);
    let contents = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
    Ok(doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_root,
        "MediaBox" => vec![
            0.into(),
            0.into(),
            Object::Real(lay.page.0 as f32),
            Object::Real(lay.page.1 as f32),
        ],
        "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => image } },
        "Contents" => contents,
    }))
}

fn check(inputs: &[ImageInput], opts: &ImageOptions) -> Result<f64> {
    if inputs.is_empty() {
        return Err(Error::NoImages);
    }
    for input in inputs {
        if input.rotate % 90 != 0 {
            return Err(Error::BadRotation(input.rotate));
        }
    }
    if let Some(paper) = opts.page {
        paper.portrait_pt()?;
    }
    margin_pt(opts.margin_mm)
}

/// Builds a document with one page per input image.
fn build(inputs: &[ImageInput], opts: &ImageOptions, margin: f64) -> Result<Document> {
    let mut doc = Document::with_version("1.5");
    let pages_root = doc.new_object_id();
    let mut kids = Vec::with_capacity(inputs.len());
    for input in inputs {
        // One image is decoded at a time, so memory stays at one picture.
        let prepared = prepare(&input.path, opts.quality)?;
        let page = add_page(&mut doc, pages_root, &prepared, input.rotate, opts, margin)?;
        kids.push(Object::Reference(page));
    }
    doc.objects.insert(
        pages_root,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => kids.len() as i64,
            "Kids" => kids,
        }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_root });
    doc.trailer.set("Root", catalog);
    Ok(doc)
}

/// Writes all images into one PDF at `output`. Returns its size in bytes.
pub fn images_to_pdf(inputs: &[ImageInput], output: &Path, opts: &ImageOptions) -> Result<u64> {
    let margin = check(inputs, opts)?;
    let mut doc = build(inputs, opts, margin)?;
    write_atomic(&mut doc, output)
}

/// Writes one PDF per image into `out_dir`, named after the image. Files
/// already written by this call are removed if a later one fails.
pub fn images_to_folder(
    inputs: &[ImageInput],
    out_dir: &Path,
    opts: &ImageOptions,
) -> Result<Vec<Written>> {
    let margin = check(inputs, opts)?;
    let mut written: Vec<Written> = Vec::with_capacity(inputs.len());
    let result = (|| {
        for input in inputs {
            let mut doc = build(std::slice::from_ref(input), opts, margin)?;
            let stem = input
                .path
                .file_stem()
                .map_or_else(|| "image".into(), |s| s.to_string_lossy().into_owned());
            let path = unique_path(out_dir, &stem);
            let size = write_atomic(&mut doc, &path)?;
            written.push(Written { path, size });
        }
        Ok(())
    })();
    if let Err(e) = result {
        for w in &written {
            let _ = std::fs::remove_file(&w.path);
        }
        return Err(e);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, Luma, Rgb, RgbImage};

    /// Where a stored pixel lands in the displayed grid, using `unit_matrix`.
    fn mapped(
        m: Matrix,
        (col, row): (u32, u32),
        (w, h): (u32, u32),
        (dw, dh): (u32, u32),
    ) -> (u32, u32) {
        // Pixel centre in the stored unit square (y up).
        let xs = (f64::from(col) + 0.5) / f64::from(w);
        let ys = 1.0 - (f64::from(row) + 0.5) / f64::from(h);
        let x = m[0] * xs + m[2] * ys + m[4];
        let y = m[1] * xs + m[3] * ys + m[5];
        let c = (x * f64::from(dw)).floor() as u32;
        let r = ((1.0 - y) * f64::from(dh)).floor() as u32;
        (c, r)
    }

    #[test]
    fn matrix_agrees_with_the_image_crate_for_every_exif_orientation() {
        // 3x2 picture whose pixels are all different.
        let mut stored = GrayImage::new(3, 2);
        for (i, px) in stored.pixels_mut().enumerate() {
            *px = Luma([i as u8 * 10]);
        }
        for exif in 1..=8u8 {
            let mut shown = DynamicImage::ImageLuma8(stored.clone());
            shown.apply_orientation(image::metadata::Orientation::from_exif(exif).unwrap());
            let shown = shown.into_luma8();
            let m = unit_matrix(exif, 0);
            for (col, row, px) in stored.enumerate_pixels() {
                let at = mapped(m, (col, row), (3, 2), shown.dimensions());
                assert_eq!(
                    shown.get_pixel(at.0, at.1),
                    px,
                    "exif {exif}, stored pixel ({col},{row})"
                );
            }
        }
    }

    #[test]
    fn quarter_turns_rotate_clockwise() {
        // Top-left of the picture ends up top-right after one clockwise turn.
        let m = unit_matrix(1, 1);
        let (x, y) = (m[2] * 1.0 + m[4], m[3] * 1.0 + m[5]);
        assert!((x - 1.0).abs() < 1e-9 && (y - 1.0).abs() < 1e-9);
        assert_eq!(unit_matrix(1, 4), IDENTITY);
        assert_eq!(unit_matrix(1, -1), unit_matrix(1, 3));
    }

    #[test]
    fn jpeg_header_scan_reads_frame_and_density() {
        let img = RgbImage::from_pixel(8, 4, Rgb([10, 20, 30]));
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
            .encode(img.as_raw(), 8, 4, ExtendedColorType::Rgb8)
            .unwrap();
        let info = scan_jpeg(&jpeg).unwrap();
        assert!(info.embeddable());
        assert_eq!((info.frame, info.precision, info.components), (0, 8, 3));
        assert!(scan_jpeg(b"not a jpeg").is_none());
    }

    #[test]
    fn layouts_follow_the_fit_mode() {
        let a4 = ImageOptions {
            page: Some(Paper::A4),
            orientation: PageOrientation::Auto,
            margin_mm: 0.0,
            fit: Fit::Inside,
            quality: Quality::Keep,
        };
        // A wide picture gets a landscape page and fills its width.
        let l = layout((200.0, 100.0), 96.0, &a4, 0.0).unwrap();
        assert!(l.page.0 > l.page.1);
        assert!((l.rect.2 - l.page.0).abs() < 1e-6 && l.rect.3 < l.page.1);
        assert!(l.clip.is_none());
        // Fill covers the page and crops.
        let fill = ImageOptions {
            fit: Fit::Fill,
            ..a4.clone()
        };
        let l = layout((200.0, 100.0), 96.0, &fill, 0.0).unwrap();
        assert!((l.rect.3 - l.page.1).abs() < 1e-6 && l.rect.2 > l.page.0);
        assert!(l.clip.is_some());
        // Original keeps the natural size (96 px = 72 pt), centred.
        let orig = ImageOptions {
            fit: Fit::Original,
            ..a4.clone()
        };
        let l = layout((96.0, 96.0), 96.0, &orig, 0.0).unwrap();
        assert!((l.rect.2 - 72.0).abs() < 1e-9);
        assert!((l.rect.0 - (l.page.0 - 72.0) / 2.0).abs() < 1e-9);
        // A margin that eats the page is refused.
        assert!(matches!(
            layout((1.0, 1.0), 96.0, &a4, 500.0),
            Err(Error::BadMargin)
        ));
    }

    #[test]
    fn fit_to_image_uses_dpi_and_margin() {
        let opts = ImageOptions {
            page: None,
            orientation: PageOrientation::Auto,
            margin_mm: 0.0,
            fit: Fit::Inside,
            quality: Quality::Keep,
        };
        let l = layout((192.0, 96.0), 96.0, &opts, 10.0).unwrap();
        assert!((l.page.0 - (144.0 + 20.0)).abs() < 1e-9);
        assert!((l.page.1 - (72.0 + 20.0)).abs() < 1e-9);
        // Absurdly large pictures are shrunk to the viewer limit.
        let huge = layout((200_000.0, 1000.0), 96.0, &opts, 0.0).unwrap();
        assert!(huge.page.0 <= MAX_SIDE_PT + 1e-6);
    }

    #[test]
    fn numbers_are_written_compactly() {
        assert_eq!(num(1.0), "1");
        assert_eq!(num(0.5), "0.5");
        assert_eq!(num(-0.00001), "0");
        assert_eq!(num(12.34567), "12.3457");
    }
}
