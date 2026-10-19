//! Page numbers (SPEC 5.12) and watermarks (SPEC 5.13).
//!
//! Both draw an extra content stream on each chosen page. The page's own
//! streams are never edited: they are wrapped in `q` / `Q` so whatever state
//! they leave behind cannot disturb the added drawing.
//!
//! The added drawing is described in *visual* coordinates (origin at the
//! bottom-left of the page as it is displayed). A single `cm` at the start of
//! the stream maps that onto the page's own space, taking the page box origin
//! and `/Rotate` into account, so rotated, cropped and mixed-size pages all
//! come out where the user expects.

use crate::progress::{step, Progress, Silent};
use std::path::{Path, PathBuf};

use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream};
use serde::Deserialize;

use crate::assemble::{assemble, same_file, write_atomic, PageOp};
use crate::document::{self, effective_rotate, inherited, page_box};
use crate::error::{Error, Result};
use crate::images::{add_image_object, prepare, unit_matrix, Quality};
use crate::paper::MM;
use crate::ranges::parse_pages;
use crate::resize::content_refs;

/// Affine map `[a, b, c, d, e, f]`: `x' = a x + c y + e`, `y' = b x + d y + f`.
type Matrix = [f64; 6];

/// Most copies of a tiled watermark on one page; more means a stamp too small
/// for the page, which would only make the file huge.
const MAX_TILES: usize = 4000;

/// A number for a content stream. Six decimals: a scale applied to a large
/// stamp (an image hundreds of units wide) would show rounding at four.
fn num(v: f64) -> String {
    let s = format!("{v:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() { "0" } else { s }.to_string()
}

// Text ---------------------------------------------------------------------

/// Helvetica advance widths (1/1000 em) for the ASCII characters 32 to 126.
const HELVETICA: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278,
    278, // space to /
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, // 0 to 9
    278, 278, 584, 584, 584, 556, 1015, // : to @
    667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, // A to M
    722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, // N to Z
    278, 278, 278, 469, 556, 333, // [ to `
    556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, // a to m
    556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, // n to z
    334, 260, 334, 584, // { to ~
];

/// Cap height and descender of Helvetica, in em.
const CAP_HEIGHT: f64 = 0.718;
const ASCENT: f64 = 0.718;
const DESCENT: f64 = 0.207;

/// A string ready to draw with Helvetica and `/WinAnsiEncoding`.
struct Text {
    bytes: Vec<u8>,
    /// Advance width in 1/1000 em.
    width: u32,
}

impl Text {
    fn new(s: &str) -> Result<Text> {
        let mut bytes = Vec::with_capacity(s.len());
        let mut width = 0u32;
        for c in s.chars() {
            let (byte, w) = match c {
                ' '..='~' => (c as u8, HELVETICA[c as usize - 32]),
                // The printable Latin-1 half maps straight onto WinAnsi; its
                // glyph widths are close enough to 556 for alignment.
                '\u{A0}'..='\u{FF}' => (c as u32 as u8, 556),
                '€' => (0x80, 556),
                '…' => (0x85, 1000),
                '‘' | '’' => (if c == '‘' { 0x91 } else { 0x92 }, 222),
                '“' | '”' => (if c == '“' { 0x93 } else { 0x94 }, 333),
                '•' => (0x95, 350),
                '–' => (0x96, 556),
                '—' => (0x97, 1000),
                '™' => (0x99, 1000),
                _ => return Err(Error::UnsupportedText(c)),
            };
            bytes.push(byte);
            width += u32::from(w);
        }
        Ok(Text { bytes, width })
    }

    /// Width in points at `size`.
    fn points(&self, size: f64) -> f64 {
        f64::from(self.width) * size / 1000.0
    }

    /// The `( ... )` literal with everything unusual escaped.
    fn literal(&self) -> String {
        let mut out = String::from("(");
        for &b in &self.bytes {
            match b {
                b'(' | b')' | b'\\' => {
                    out.push('\\');
                    out.push(b as char);
                }
                0x20..=0x7E => out.push(b as char),
                _ => out += &format!("\\{b:03o}"),
            }
        }
        out.push(')');
        out
    }
}

/// `#rrggbb` as the three operands of `rg`.
fn color_operands(hex: &str) -> Result<String> {
    let bad = || Error::BadOption("The colour must look like #1A2B3C.");
    let h = hex.strip_prefix('#').ok_or_else(bad)?;
    if h.len() != 6 || !h.is_ascii() {
        return Err(bad());
    }
    let channel = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).map_err(|_| bad());
    Ok(format!(
        "{} {} {}",
        num(f64::from(channel(0)?) / 255.0),
        num(f64::from(channel(2)?) / 255.0),
        num(f64::from(channel(4)?) / 255.0)
    ))
}

fn check_font_size(size: f64) -> Result<()> {
    if size.is_finite() && (1.0..=500.0).contains(&size) {
        Ok(())
    } else {
        Err(Error::BadOption("The font size must be between 1 and 500."))
    }
}

fn check_margin(margin_mm: f64) -> Result<f64> {
    if margin_mm.is_finite() && (0.0..=100.0).contains(&margin_mm) {
        Ok(margin_mm * MM)
    } else {
        Err(Error::BadOption("The margin must be between 0 and 100 mm."))
    }
}

// Page geometry --------------------------------------------------------------

/// Where the displayed page sits: its size as shown, and the matrix from
/// displayed coordinates to the page's own coordinates.
struct View {
    width: f64,
    height: f64,
    to_page: Matrix,
}

fn view_of(doc: &Document, page: ObjectId) -> View {
    let b = page_box(doc, page);
    let (uw, uh) = (b[2] - b[0], b[3] - b[1]);
    let (x0, y0) = (b[0], b[1]);
    match effective_rotate(doc, page) {
        90 => View {
            width: uh,
            height: uw,
            to_page: [0.0, 1.0, -1.0, 0.0, uw + x0, y0],
        },
        180 => View {
            width: uw,
            height: uh,
            to_page: [-1.0, 0.0, 0.0, -1.0, uw + x0, uh + y0],
        },
        270 => View {
            width: uh,
            height: uw,
            to_page: [0.0, -1.0, 1.0, 0.0, x0, uh + y0],
        },
        _ => View {
            width: uw,
            height: uh,
            to_page: [1.0, 0.0, 0.0, 1.0, x0, y0],
        },
    }
}

fn cm(m: Matrix) -> String {
    format!(
        "{} {} {} {} {} {} cm\n",
        num(m[0]),
        num(m[1]),
        num(m[2]),
        num(m[3]),
        num(m[4]),
        num(m[5])
    )
}

/// The page numbers (1-based) a `pages` text selects; `None` or blank means all.
fn selected_pages(pages: &Option<String>, count: u32) -> Result<Vec<u32>> {
    match pages.as_deref().map(str::trim) {
        None | Some("") => Ok((1..=count).collect()),
        Some(text) => parse_pages(text, count),
    }
}

// Page edits -----------------------------------------------------------------

/// A dictionary-valued entry as an owned dictionary (following a reference).
fn dict_of(doc: &Document, o: &Object) -> Option<Dictionary> {
    match o {
        Object::Dictionary(d) => Some(d.clone()),
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    }
}

/// Registers `object` under a fresh name starting with `prefix` in the
/// `category` (`Font`, `XObject` or `ExtGState`) of the page's resources, and
/// returns the name. The page gets its own copy of the resource dictionary, so
/// resources shared with other pages or inherited from the page tree are never
/// touched, and names already in use (say by an earlier stamp) are never reused.
fn add_resource(
    doc: &mut Document,
    page: ObjectId,
    category: &str,
    prefix: &str,
    object: ObjectId,
) -> Result<String> {
    let mut resources = inherited(doc, page, b"Resources")
        .and_then(|o| dict_of(doc, &o))
        .unwrap_or_default();
    let mut entries = resources
        .get(category.as_bytes())
        .ok()
        .and_then(|o| dict_of(doc, o))
        .unwrap_or_default();
    let name = (1..)
        .map(|n| format!("{prefix}{n}"))
        .find(|n| !entries.has(n.as_bytes()))
        .expect("an unused name exists");
    entries.set(name.as_bytes().to_vec(), object);
    resources.set(category, entries);
    doc.get_dictionary_mut(page)?.set("Resources", resources);
    Ok(name)
}

/// Adds `drawing` to the page, over or under its own content.
fn add_drawing(doc: &mut Document, page: ObjectId, drawing: String, under: bool) -> Result<()> {
    let drawing =
        Object::Reference(doc.add_object(Stream::new(dictionary! {}, drawing.into_bytes())));
    let mut contents = Vec::new();
    if under {
        contents.push(drawing);
        contents.extend(content_refs(doc, page));
    } else {
        let open = doc.add_object(Stream::new(dictionary! {}, b"q\n".to_vec()));
        let close = doc.add_object(Stream::new(dictionary! {}, b"\nQ\n".to_vec()));
        contents.push(Object::Reference(open));
        contents.extend(content_refs(doc, page));
        contents.push(Object::Reference(close));
        contents.push(drawing);
    }
    doc.get_dictionary_mut(page)?.set("Contents", contents);
    Ok(())
}

fn helvetica(doc: &mut Document) -> ObjectId {
    doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    })
}

// Page numbers ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Corner {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PageNumberOptions {
    pub position: Corner,
    pub margin_mm: f64,
    pub font_size: f64,
    /// `#rrggbb`.
    pub color: String,
    /// Text with `{n}` (this page's number) and `{total}` (the last number).
    pub format: String,
    /// Number given to the first numbered page.
    pub start: i64,
    /// Page-range text such as `2-end`; blank numbers every page.
    #[serde(default)]
    pub pages: Option<String>,
}

/// The text drawn on each numbered page, for pages `numbered` in order.
fn page_number_labels(opts: &PageNumberOptions, numbered: usize) -> Vec<String> {
    let last = opts.start + numbered as i64 - 1;
    (0..numbered)
        .map(|i| {
            opts.format
                .replace("{n}", &(opts.start + i as i64).to_string())
                .replace("{total}", &last.to_string())
        })
        .collect()
}

pub fn add_page_numbers(doc: &mut Document, opts: &PageNumberOptions) -> Result<()> {
    check_font_size(opts.font_size)?;
    let margin = check_margin(opts.margin_mm)?;
    let color = color_operands(&opts.color)?;
    if opts.format.trim().is_empty() {
        return Err(Error::BadOption("Enter the text to put on each page."));
    }
    if opts.start.unsigned_abs() > 1_000_000_000 {
        return Err(Error::BadOption("The start number is too large."));
    }
    let ids: Vec<ObjectId> = doc.get_pages().values().copied().collect();
    let chosen = selected_pages(&opts.pages, ids.len() as u32)?;
    let labels = page_number_labels(opts, chosen.len());
    // Check every label before touching the document.
    let texts = labels
        .iter()
        .map(|l| Text::new(l))
        .collect::<Result<Vec<_>>>()?;

    let font = helvetica(doc);
    for (text, &n) in texts.iter().zip(&chosen) {
        let page = ids[n as usize - 1];
        let view = view_of(doc, page);
        let size = opts.font_size;
        let width = text.points(size);
        let x = match opts.position {
            Corner::TopLeft | Corner::BottomLeft => margin,
            Corner::TopCenter | Corner::BottomCenter => (view.width - width) / 2.0,
            Corner::TopRight | Corner::BottomRight => view.width - margin - width,
        };
        // The margin is measured to the glyphs, not to the baseline.
        let y = match opts.position {
            Corner::BottomLeft | Corner::BottomCenter | Corner::BottomRight => {
                margin + DESCENT * size
            }
            _ => view.height - margin - ASCENT * size,
        };
        let name = add_resource(doc, page, "Font", "JJF", font)?;
        let drawing = format!(
            "q\n{}{color} rg\nBT\n/{name} {} Tf\n1 0 0 1 {} {} Tm\n{} Tj\nET\nQ\n",
            cm(view.to_page),
            num(size),
            num(x),
            num(y),
            text.literal()
        );
        add_drawing(doc, page, drawing, false)?;
    }
    Ok(())
}

// Watermark ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Place {
    Center,
    /// Repeated across the whole page.
    Tiled,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    #[default]
    Over,
    Under,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum WatermarkKind {
    Text {
        text: String,
        font_size: f64,
        /// `#rrggbb`.
        color: String,
    },
    Image {
        path: PathBuf,
        /// Width of the stamp as a percentage of the page width.
        width_percent: f64,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WatermarkOptions {
    #[serde(flatten)]
    pub kind: WatermarkKind,
    /// 0 (invisible) to 1 (solid).
    pub opacity: f64,
    /// Counter-clockwise degrees.
    pub rotation: f64,
    pub place: Place,
    #[serde(default)]
    pub layer: Layer,
    /// Distance from the page edge for corner positions.
    #[serde(default)]
    pub margin_mm: f64,
    #[serde(default)]
    pub pages: Option<String>,
}

/// A watermark drawn once as a Form XObject, ready to be placed on pages.
struct Stamp {
    form: ObjectId,
    /// Size of the visible stamp, in the form's own units.
    width: f64,
    height: f64,
    /// Where its lower-left corner is in form space.
    origin: (f64, f64),
    /// Image stamps are sized relative to each page; text stamps are not.
    relative_width: Option<f64>,
}

fn build_stamp(doc: &mut Document, kind: &WatermarkKind) -> Result<Stamp> {
    match kind {
        WatermarkKind::Text {
            text,
            font_size,
            color,
        } => {
            check_font_size(*font_size)?;
            if text.trim().is_empty() {
                return Err(Error::BadOption("Enter the watermark text."));
            }
            let t = Text::new(text)?;
            let color = color_operands(color)?;
            let font = helvetica(doc);
            let width = t.points(*font_size);
            let content = format!(
                "BT\n{color} rg\n/F1 {} Tf\n0 0 Td\n{} Tj\nET\n",
                num(*font_size),
                t.literal()
            );
            let dict = dictionary! {
                "Type" => "XObject",
                "Subtype" => "Form",
                "BBox" => vec![
                    Object::Real(-2.0),
                    Object::Real((-DESCENT * font_size - 2.0) as f32),
                    Object::Real((width + 2.0) as f32),
                    Object::Real((ASCENT * font_size + 2.0) as f32),
                ],
                "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
            };
            let form = doc.add_object(Stream::new(dict, content.into_bytes()));
            Ok(Stamp {
                form,
                width,
                height: CAP_HEIGHT * font_size,
                origin: (0.0, 0.0),
                relative_width: None,
            })
        }
        WatermarkKind::Image {
            path,
            width_percent,
        } => {
            if !(width_percent.is_finite() && (1.0..=100.0).contains(width_percent)) {
                return Err(Error::BadOption(
                    "The stamp width must be between 1% and 100% of the page width.",
                ));
            }
            let prepared = prepare(path, Quality::Keep)?;
            let swap = prepared.exif >= 5;
            let (pw, ph) = (f64::from(prepared.width), f64::from(prepared.height));
            let (w, h) = if swap { (ph, pw) } else { (pw, ph) };
            let image = add_image_object(doc, &prepared);
            // Unit square -> displayed picture, one point per pixel.
            let [a, b, c, d, e, f] = unit_matrix(prepared.exif, 0);
            let m = [w * a, h * b, w * c, h * d, w * e, h * f];
            let content = format!("q\n{}/Im0 Do\nQ\n", cm(m));
            let dict = dictionary! {
                "Type" => "XObject",
                "Subtype" => "Form",
                "BBox" => vec![0.into(), 0.into(), Object::Real(w as f32), Object::Real(h as f32)],
                "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => image } },
            };
            let form = doc.add_object(Stream::new(dict, content.into_bytes()));
            Ok(Stamp {
                form,
                width: w,
                height: h,
                origin: (0.0, 0.0),
                relative_width: Some(width_percent / 100.0),
            })
        }
    }
}

/// Centres (in displayed coordinates) of the stamp copies on one page.
fn placements(
    place: Place,
    (vw, vh): (f64, f64),
    (w, h): (f64, f64),
    angle: f64,
    margin: f64,
) -> Result<Vec<(f64, f64)>> {
    let (sin, cos) = angle.sin_cos();
    // Half the size of the box that encloses the turned stamp.
    let hx = (cos.abs() * w + sin.abs() * h) / 2.0;
    let hy = (sin.abs() * w + cos.abs() * h) / 2.0;
    Ok(match place {
        Place::Center => vec![(vw / 2.0, vh / 2.0)],
        Place::TopLeft => vec![(margin + hx, vh - margin - hy)],
        Place::TopRight => vec![(vw - margin - hx, vh - margin - hy)],
        Place::BottomLeft => vec![(margin + hx, margin + hy)],
        Place::BottomRight => vec![(vw - margin - hx, margin + hy)],
        Place::Tiled => {
            // A grid in the turned frame, staggered on alternate rows, kept
            // where it touches the page.
            let (px, py) = (1.4 * w + 20.0, 1.4 * h + 20.0 + 0.6 * h);
            let reach = vw.hypot(vh) / 2.0 + px.max(py);
            let (nx, ny) = ((reach / px).ceil() as i64, (reach / py).ceil() as i64);
            if (2 * nx + 1) * (2 * ny + 1) > 1_000_000 {
                return Err(Error::BadOption(
                    "The watermark is too small to tile on this page. Make it larger.",
                ));
            }
            let mut out = Vec::new();
            for j in -ny..=ny {
                let shift = if j.rem_euclid(2) == 0 { 0.0 } else { px / 2.0 };
                for i in -nx..=nx {
                    let (u, v) = (i as f64 * px + shift, j as f64 * py);
                    let (x, y) = (vw / 2.0 + u * cos - v * sin, vh / 2.0 + u * sin + v * cos);
                    if x + hx > 0.0 && x - hx < vw && y + hy > 0.0 && y - hy < vh {
                        out.push((x, y));
                    }
                }
            }
            if out.len() > MAX_TILES {
                return Err(Error::BadOption(
                    "The watermark is too small to tile on this page. Make it larger.",
                ));
            }
            out
        }
    })
}

pub fn add_watermark(doc: &mut Document, opts: &WatermarkOptions) -> Result<()> {
    if !(opts.opacity.is_finite() && (0.0..=1.0).contains(&opts.opacity)) {
        return Err(Error::BadOption("The opacity must be between 0 and 1."));
    }
    if !opts.rotation.is_finite() {
        return Err(Error::BadOption(
            "The rotation must be a number of degrees.",
        ));
    }
    let margin = check_margin(opts.margin_mm)?;
    let ids: Vec<ObjectId> = doc.get_pages().values().copied().collect();
    let chosen = selected_pages(&opts.pages, ids.len() as u32)?;
    let stamp = build_stamp(doc, &opts.kind)?;
    let state = doc.add_object(dictionary! {
        "Type" => "ExtGState",
        "ca" => Object::Real(opts.opacity as f32),
        "CA" => Object::Real(opts.opacity as f32),
    });
    let angle = opts.rotation.to_radians();
    let (sin, cos) = angle.sin_cos();
    let under = opts.layer == Layer::Under;

    for n in chosen {
        let page = ids[n as usize - 1];
        let view = view_of(doc, page);
        let scale = stamp
            .relative_width
            .map_or(1.0, |fraction| fraction * view.width / stamp.width);
        let (w, h) = (stamp.width * scale, stamp.height * scale);
        let centres = placements(opts.place, (view.width, view.height), (w, h), angle, margin)?;

        let form = add_resource(doc, page, "XObject", "JJWM", stamp.form)?;
        let gs = add_resource(doc, page, "ExtGState", "JJGS", state)?;
        let (cx0, cy0) = (
            stamp.origin.0 + stamp.width / 2.0,
            stamp.origin.1 + stamp.height / 2.0,
        );
        let mut drawing = format!("q\n{}/{gs} gs\n", cm(view.to_page));
        for (x, y) in centres {
            // Form space -> centre it, scale, turn, move to the spot.
            let m = [
                scale * cos,
                scale * sin,
                -scale * sin,
                scale * cos,
                x - scale * (cos * cx0 - sin * cy0),
                y - scale * (sin * cx0 + cos * cy0),
            ];
            drawing += &format!("q\n{}/{form} Do\nQ\n", cm(m));
        }
        drawing += "Q\n";
        add_drawing(doc, page, drawing, under)?;
    }
    Ok(())
}

// Files ----------------------------------------------------------------------

/// Either kind of stamp, as the UI sends it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "tool", rename_all = "snake_case")]
pub enum Stamping {
    PageNumbers(PageNumberOptions),
    Watermark(WatermarkOptions),
}

impl Stamping {
    pub fn apply(&self, doc: &mut Document) -> Result<()> {
        match self {
            Stamping::PageNumbers(o) => add_page_numbers(doc, o),
            Stamping::Watermark(o) => add_watermark(doc, o),
        }
    }
}

/// Loads `source`, stamps it and writes the result to `output`, which must not
/// be the source file. Returns the size of the written file in bytes.
pub fn stamp_file(source: &Path, output: &Path, stamping: &Stamping) -> Result<u64> {
    stamp_file_with(source, output, stamping, &Silent)
}

/// Like [`stamp_file`], reporting progress and stopping if cancelled.
pub fn stamp_file_with(
    source: &Path,
    output: &Path,
    stamping: &Stamping,
    progress: &dyn Progress,
) -> Result<u64> {
    if same_file(source, output) {
        return Err(Error::OutputIsSource(output.to_path_buf()));
    }
    step(progress, 0, 3, "Reading the document")?;
    let mut doc = document::load(source)?;
    step(progress, 1, 3, "Stamping pages")?;
    stamping.apply(&mut doc)?;
    step(progress, 2, 3, "Saving")?;
    write_atomic(&mut doc, output)
}

/// A one-page PDF (in memory) showing page `page` (1-based) as the stamped
/// document would have it, for the live preview.
pub fn stamp_preview(source: &Path, stamping: &Stamping, page: u32) -> Result<Vec<u8>> {
    let mut doc = document::load(source)?;
    let count = doc.get_pages().len() as u32;
    if page == 0 || page > count {
        return Err(Error::PageOutOfRange(page));
    }
    stamping.apply(&mut doc)?;
    assemble(&mut doc, &[PageOp::Page { page, rotate: 0 }])?;
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_widths_and_escapes() {
        let t = Text::new("Page 1").unwrap();
        assert_eq!(t.width, 667 + 556 + 556 + 556 + 278 + 556);
        assert_eq!(Text::new("a(b)\\").unwrap().literal(), "(a\\(b\\)\\\\)");
        assert_eq!(Text::new("é").unwrap().literal(), "(\\351)");
        assert!(matches!(
            Text::new("日本"),
            Err(Error::UnsupportedText('日'))
        ));
        assert!(matches!(
            Text::new("a\nb"),
            Err(Error::UnsupportedText('\n'))
        ));
    }

    #[test]
    fn colours_parse() {
        assert_eq!(color_operands("#FF0000").unwrap(), "1 0 0");
        assert!(color_operands("red").is_err());
        assert!(color_operands("#12345").is_err());
        assert!(color_operands("#ggg000").is_err());
    }

    #[test]
    fn labels_use_the_last_number_as_total() {
        let o = PageNumberOptions {
            position: Corner::BottomCenter,
            margin_mm: 10.0,
            font_size: 10.0,
            color: "#000000".into(),
            format: "Page {n} of {total}".into(),
            start: 1,
            pages: None,
        };
        assert_eq!(
            page_number_labels(&o, 3),
            ["Page 1 of 3", "Page 2 of 3", "Page 3 of 3"]
        );
        let o = PageNumberOptions { start: 5, ..o };
        assert_eq!(page_number_labels(&o, 2), ["Page 5 of 6", "Page 6 of 6"]);
    }

    #[test]
    fn corners_account_for_rotation_and_margin() {
        let at = |place, angle: f64| {
            placements(
                place,
                (600.0, 800.0),
                (100.0, 20.0),
                angle.to_radians(),
                10.0,
            )
            .unwrap()[0]
        };
        let (x, y) = at(Place::BottomLeft, 0.0);
        assert!((x - 60.0).abs() < 1e-9 && (y - 20.0).abs() < 1e-9);
        // Turned a quarter, the stamp is 20 wide and 100 tall.
        let (x, y) = at(Place::TopRight, 90.0);
        assert!((x - 580.0).abs() < 1e-6 && (y - 740.0).abs() < 1e-6);
    }

    #[test]
    fn tiles_stay_on_the_page_and_are_capped() {
        let tiles = placements(Place::Tiled, (600.0, 800.0), (100.0, 20.0), 0.0, 0.0).unwrap();
        assert!(tiles.len() > 10);
        assert!(tiles
            .iter()
            .all(|&(x, y)| x > -60.0 && x < 660.0 && y > -20.0 && y < 820.0));
        assert!(placements(Place::Tiled, (14400.0, 14400.0), (1.0, 1.0), 0.0, 0.0).is_err());
    }
}
