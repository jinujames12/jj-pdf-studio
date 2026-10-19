//! Compress (SPEC 5.7).
//!
//! Every preset starts with a lossless pass: junk such as page thumbnails is
//! dropped, uncompressed streams are Flate-compressed, byte-identical streams
//! are merged and the file is written with object streams. `Balanced` and
//! `Strong` then re-encode large images as JPEG at a lower resolution.
//!
//! Images are only touched when their use on the page is understood: the
//! content streams are scanned for `cm` and `Do` operators to learn how large
//! each image is drawn, and an image drawn in several places is judged by its
//! largest use. Anything unusual (masks, palettes, CMYK, JBIG2, ...) passes
//! through unchanged. A recompressed image or the whole file is only kept when
//! it is actually smaller.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType, ImageFormat};
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::assemble::{same_file, write_bytes_atomic};
use crate::document::{self, inherited};
use crate::error::{Error, Result};

/// Content streams larger than this (after decoding) are not scanned.
const MAX_CONTENT: usize = 128 * 1024 * 1024;
/// Images with more pixels than this are left alone.
const MAX_PIXELS: u64 = 120_000_000;
/// Images below this many pixels are not worth touching.
const MIN_PIXELS: u64 = 4_096;
/// How deep nested form XObjects are followed when measuring images.
const MAX_FORM_DEPTH: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    /// Structure only; nothing visibly changes.
    Lossless,
    /// Images above 200 DPI go to 150 DPI, JPEG quality 75.
    #[default]
    Balanced,
    /// Images above 150 DPI go to 100 DPI, JPEG quality 60.
    Strong,
}

/// How a preset treats images: those drawn above `above_dpi` are brought down
/// to `target_dpi` and stored as JPEG at `quality`.
#[derive(Debug, Clone, Copy)]
struct ImageRule {
    above_dpi: f64,
    target_dpi: f64,
    quality: u8,
}

impl Preset {
    fn image_rule(self) -> Option<ImageRule> {
        match self {
            Preset::Lossless => None,
            Preset::Balanced => Some(ImageRule {
                above_dpi: 200.0,
                target_dpi: 150.0,
                quality: 75,
            }),
            Preset::Strong => Some(ImageRule {
                above_dpi: 150.0,
                target_dpi: 100.0,
                quality: 60,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub struct CompressOptions {
    #[serde(default)]
    pub preset: Preset,
    /// Also turn colour images into grayscale (`Strong` only).
    #[serde(default)]
    pub grayscale: bool,
}

/// What a compression run did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CompressReport {
    /// Size of the source file in bytes.
    pub before: u64,
    /// Size of the compressed result. Only a real file when `written`.
    pub after: u64,
    /// False when the result was not smaller: nothing was written and the
    /// original should simply be kept.
    pub written: bool,
    /// Images the tool measured and could in principle recompress.
    pub images_found: u32,
    /// Images that were replaced by a smaller version.
    pub images_recompressed: u32,
}

/// Compresses `source` into `output`, which must not be the source file.
/// Nothing is written when the result would not be smaller than the source.
pub fn compress_file(
    source: &Path,
    output: &Path,
    opts: &CompressOptions,
) -> Result<CompressReport> {
    if same_file(source, output) {
        return Err(Error::OutputIsSource(output.to_path_buf()));
    }
    let before = std::fs::metadata(source)?.len();
    let mut doc = document::load(source)?;
    let stats = compress(&mut doc, opts);
    let bytes = serialize(&mut doc)?;
    let after = bytes.len() as u64;
    let written = after < before;
    if written {
        write_bytes_atomic(&bytes, output)?;
    }
    Ok(CompressReport {
        before,
        after,
        written,
        images_found: stats.found,
        images_recompressed: stats.recompressed,
    })
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Stats {
    found: u32,
    recompressed: u32,
}

/// Runs the whole pipeline on `doc` in memory.
fn compress(doc: &mut Document, opts: &CompressOptions) -> Stats {
    strip_junk(doc);
    let stats = match opts.preset.image_rule() {
        Some(rule) => recompress_images(doc, &rule, opts.grayscale),
        None => Stats::default(),
    };
    deflate_streams(doc);
    merge_duplicates(doc);
    prune(doc);
    stats
}

// -- Lossless pass ---------------------------------------------------------

/// Removes page thumbnails and application-private data, which viewers
/// regenerate or ignore.
fn strip_junk(doc: &mut Document) {
    let pages: Vec<ObjectId> = doc.get_pages().values().copied().collect();
    for id in pages {
        if let Ok(page) = doc.get_dictionary_mut(id) {
            page.remove(b"Thumb");
            page.remove(b"PieceInfo");
        }
    }
    if let Ok(catalog) = doc.catalog_mut() {
        catalog.remove(b"PieceInfo");
    }
}

/// Flate-compresses every stream that has no filter yet. XMP metadata stays
/// readable by tools that scan for it, as the PDF specification recommends.
fn deflate_streams(doc: &mut Document) {
    for object in doc.objects.values_mut() {
        if let Object::Stream(s) = object {
            let skip = matches!(
                s.dict.get(b"Type").and_then(Object::as_name),
                Ok(b"Metadata")
            );
            if s.allows_compression && !skip {
                // A stream that cannot be compressed is simply left as it is.
                let _ = s.compress();
            }
        }
    }
}

/// Points every reference to a byte-identical stream at one copy. Repeated
/// passes catch streams that only became identical once the streams they
/// refer to (a soft mask, say) were merged.
fn merge_duplicates(doc: &mut Document) {
    for _ in 0..4 {
        let mut buckets: HashMap<u64, Vec<(ObjectId, String)>> = HashMap::new();
        let mut remap: HashMap<ObjectId, ObjectId> = HashMap::new();
        for (&id, object) in &doc.objects {
            let Object::Stream(s) = object else { continue };
            if matches!(
                s.dict.get(b"Type").and_then(Object::as_name),
                Ok(b"XRef" | b"ObjStm")
            ) {
                continue;
            }
            // `Length` is left out: it only mirrors the content.
            let mut dict = s.dict.clone();
            dict.remove(b"Length");
            let sig = format!("{dict:?}");
            let key = hash_of(&sig, &s.content);
            let bucket = buckets.entry(key).or_default();
            let twin = bucket.iter().find(|(other, other_sig)| {
                *other_sig == sig
                    && matches!(&doc.objects[other], Object::Stream(o) if o.content == s.content)
            });
            match twin {
                Some(&(keep, _)) => {
                    remap.insert(id, keep);
                }
                None => bucket.push((id, sig)),
            }
        }
        if remap.is_empty() {
            return;
        }
        let mut redirect = |id: &mut ObjectId| {
            if let Some(&to) = remap.get(id) {
                *id = to;
            }
        };
        for object in doc.objects.values_mut() {
            visit_refs(object, &mut redirect);
        }
        for (_, value) in doc.trailer.iter_mut() {
            visit_refs(value, &mut redirect);
        }
        for id in remap.keys() {
            doc.objects.remove(id);
        }
    }
}

fn hash_of(sig: &str, content: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    sig.hash(&mut h);
    content.hash(&mut h);
    h.finish()
}

/// Calls `f` on every object reference inside `o`.
fn visit_refs(o: &mut Object, f: &mut dyn FnMut(&mut ObjectId)) {
    match o {
        Object::Reference(id) => f(id),
        Object::Array(items) => items.iter_mut().for_each(|i| visit_refs(i, f)),
        Object::Dictionary(d) => d.iter_mut().for_each(|(_, v)| visit_refs(v, f)),
        Object::Stream(s) => s.dict.iter_mut().for_each(|(_, v)| visit_refs(v, f)),
        _ => {}
    }
}

fn collect_refs(o: &Object, out: &mut Vec<ObjectId>) {
    match o {
        Object::Reference(id) => out.push(*id),
        Object::Array(items) => items.iter().for_each(|i| collect_refs(i, out)),
        Object::Dictionary(d) => d.iter().for_each(|(_, v)| collect_refs(v, out)),
        Object::Stream(s) => s.dict.iter().for_each(|(_, v)| collect_refs(v, out)),
        _ => {}
    }
}

/// Drops every object that cannot be reached from the trailer. (lopdf's own
/// `prune_objects` is quadratic, which hurts on files with many objects.)
fn prune(doc: &mut Document) {
    let mut live: HashSet<ObjectId> = HashSet::new();
    let mut todo = Vec::new();
    doc.trailer
        .iter()
        .for_each(|(_, v)| collect_refs(v, &mut todo));
    while let Some(id) = todo.pop() {
        if live.insert(id) {
            if let Some(object) = doc.objects.get(&id) {
                collect_refs(object, &mut todo);
            }
        }
    }
    doc.objects.retain(|id, _| live.contains(id));
}

/// Writes `doc` with object streams and a cross-reference stream, checking the
/// result by reading it back. Falls back to the classic layout if that fails.
fn serialize(doc: &mut Document) -> Result<Vec<u8>> {
    let pages = doc.get_pages().len();
    // Saving in the modern layout edits the trailer; keep it for the fallback.
    let trailer = doc.trailer.clone();
    let max_id = doc.max_id;
    let mut modern = Vec::new();
    if doc.save_modern(&mut modern).is_ok() {
        let sound = Document::load_mem(&modern)
            .map(|d| !d.is_encrypted() && d.get_pages().len() == pages)
            .unwrap_or(false);
        if sound {
            return Ok(modern);
        }
    }
    doc.trailer = trailer;
    doc.max_id = max_id;
    let mut plain = Vec::new();
    doc.save_to(&mut plain)?;
    Ok(plain)
}

// -- Image recompression ---------------------------------------------------

type Matrix = [f64; 6];
const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `a` applied first, then `b` (PDF row-vector convention).
fn mul(a: &Matrix, b: &Matrix) -> Matrix {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

fn deref<'a>(doc: &'a Document, o: &'a Object) -> &'a Object {
    match o {
        Object::Reference(id) => doc.objects.get(id).unwrap_or(o),
        _ => o,
    }
}

fn dict_of(o: &Object) -> Option<&Dictionary> {
    match o {
        Object::Dictionary(d) => Some(d),
        Object::Stream(s) => Some(&s.dict),
        _ => None,
    }
}

fn number(o: &Object) -> Option<f64> {
    match o {
        Object::Integer(i) => Some(*i as f64),
        Object::Real(r) => Some(f64::from(*r)),
        _ => None,
    }
}

fn matrix_of(items: &[Object]) -> Option<Matrix> {
    let v: Vec<f64> = items.iter().map(number).collect::<Option<_>>()?;
    v.try_into().ok()
}

fn dim(doc: &Document, dict: &Dictionary, key: &[u8]) -> Option<u32> {
    let v = number(deref(doc, dict.get(key).ok()?))?;
    (v >= 1.0 && v <= f64::from(u32::MAX)).then_some(v as u32)
}

/// How large each image is drawn, as the lowest resolution (pixels per inch)
/// at which any of its uses shows it. Only images drawn through `Do` on a page
/// (directly or inside a form) are listed.
struct Scan<'a> {
    doc: &'a Document,
    dpi: HashMap<ObjectId, f64>,
    forms: HashMap<ObjectId, Option<Rc<Vec<Operation>>>>,
}

impl<'a> Scan<'a> {
    fn new(doc: &'a Document) -> Self {
        Scan {
            doc,
            dpi: HashMap::new(),
            forms: HashMap::new(),
        }
    }

    fn pages(&mut self) {
        let doc = self.doc;
        for &page in doc.get_pages().values() {
            let Some(res) = inherited(doc, page, b"Resources") else {
                continue;
            };
            let Some(res) = dict_of(deref(doc, &res)) else {
                continue;
            };
            let Ok(bytes) = doc.get_page_content_with_limit(page, MAX_CONTENT) else {
                continue;
            };
            // A page whose content cannot be read keeps its images as they are.
            let Ok(content) = Content::decode(&bytes) else {
                continue;
            };
            self.run(&content.operations, res, IDENTITY, &mut Vec::new());
        }
    }

    fn run(
        &mut self,
        ops: &[Operation],
        res: &Dictionary,
        start: Matrix,
        active: &mut Vec<ObjectId>,
    ) {
        let doc = self.doc;
        let mut ctm = start;
        let mut saved: Vec<Matrix> = Vec::new();
        for op in ops {
            match op.operator.as_str() {
                "q" => saved.push(ctm),
                "Q" => {
                    if let Some(m) = saved.pop() {
                        ctm = m;
                    }
                }
                "cm" => {
                    if let Some(m) = matrix_of(&op.operands) {
                        ctm = mul(&m, &ctm);
                    }
                }
                "Do" => {
                    let Some(Object::Name(name)) = op.operands.first() else {
                        continue;
                    };
                    let id = res
                        .get(b"XObject")
                        .ok()
                        .and_then(|x| dict_of(deref(doc, x)))
                        .and_then(|x| x.get(name).ok())
                        .and_then(|o| o.as_reference().ok());
                    if let Some(id) = id {
                        self.draw(id, &ctm, res, active);
                    }
                }
                _ => {}
            }
        }
    }

    fn draw(&mut self, id: ObjectId, ctm: &Matrix, res: &Dictionary, active: &mut Vec<ObjectId>) {
        let doc = self.doc;
        let Some(Object::Stream(s)) = doc.objects.get(&id) else {
            return;
        };
        match s.dict.get(b"Subtype").and_then(Object::as_name) {
            Ok(b"Image") => {
                let (Some(w), Some(h)) =
                    (dim(doc, &s.dict, b"Width"), dim(doc, &s.dict, b"Height"))
                else {
                    return;
                };
                // The unit square maps to a parallelogram; its side lengths
                // are the drawn width and height in points.
                let (wpt, hpt) = (ctm[0].hypot(ctm[1]), ctm[2].hypot(ctm[3]));
                if wpt < 1.0 || hpt < 1.0 {
                    return; // hidden or degenerate use
                }
                let dpi = (f64::from(w) * 72.0 / wpt).min(f64::from(h) * 72.0 / hpt);
                self.dpi
                    .entry(id)
                    .and_modify(|d| *d = d.min(dpi))
                    .or_insert(dpi);
            }
            Ok(b"Form") => {
                if active.len() >= MAX_FORM_DEPTH || active.contains(&id) {
                    return;
                }
                let own = s
                    .dict
                    .get(b"Matrix")
                    .ok()
                    .and_then(|m| deref(doc, m).as_array().ok())
                    .and_then(|a| matrix_of(a))
                    .unwrap_or(IDENTITY);
                let form_res = s
                    .dict
                    .get(b"Resources")
                    .ok()
                    .and_then(|r| dict_of(deref(doc, r)))
                    .unwrap_or(res);
                let Some(ops) = self.form_ops(id, s) else {
                    return;
                };
                active.push(id);
                self.run(&ops, form_res, mul(&own, ctm), active);
                active.pop();
            }
            _ => {}
        }
    }

    fn form_ops(&mut self, id: ObjectId, s: &Stream) -> Option<Rc<Vec<Operation>>> {
        self.forms
            .entry(id)
            .or_insert_with(|| {
                let bytes = s.get_plain_content_with_limit(MAX_CONTENT).ok()?;
                Some(Rc::new(Content::decode(&bytes).ok()?.operations))
            })
            .clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    /// A JPEG file, used as it is.
    Jpeg,
    /// Raw samples, with or without Flate compression.
    Samples,
}

/// A soft mask that shrinks along with its image.
struct MaskJob {
    id: ObjectId,
    stream: Stream,
    width: u32,
    height: u32,
}

/// Everything a worker thread needs to recompress one image.
struct Job {
    id: ObjectId,
    stream: Stream,
    source: Source,
    width: u32,
    height: u32,
    channels: u8,
    new_width: u32,
    new_height: u32,
    gray: bool,
    quality: u8,
    mask: Option<MaskJob>,
}

struct Done {
    id: ObjectId,
    jpeg: Vec<u8>,
    width: u32,
    height: u32,
    gray: bool,
    mask: Option<(ObjectId, Stream)>,
}

/// Number of colour channels if the image uses a plain RGB or gray space
/// (also ICC profiles with 1 or 3 components, which are kept as they are).
fn channels_of(doc: &Document, dict: &Dictionary) -> Option<u8> {
    match deref(doc, dict.get(b"ColorSpace").ok()?) {
        Object::Name(n) if n == b"DeviceRGB" => Some(3),
        Object::Name(n) if n == b"DeviceGray" => Some(1),
        Object::Array(a) if a.len() == 2 && a[0].as_name().ok() == Some(b"ICCBased") => {
            let profile = deref(doc, &a[1]).as_stream().ok()?;
            match profile.dict.get(b"N").and_then(Object::as_i64) {
                Ok(1) => Some(1),
                Ok(3) => Some(3),
                _ => None,
            }
        }
        _ => None,
    }
}

fn source_of(s: &Stream) -> Option<Source> {
    if s.dict.get(b"Filter").is_err() {
        return Some(Source::Samples);
    }
    match s.filters().ok()?.as_slice() {
        // A DCT transform override would change how the JPEG decodes.
        [b"DCTDecode"] if s.dict.get(b"DecodeParms").is_err() => Some(Source::Jpeg),
        [b"FlateDecode"] => Some(Source::Samples),
        _ => None,
    }
}

/// True for the keys that make an image too special to touch.
fn has_special_keys(dict: &Dictionary) -> bool {
    [&b"Decode"[..], b"Mask", b"SMaskInData", b"Matte"]
        .iter()
        .any(|k| dict.has(k))
        || matches!(dict.get(b"ImageMask"), Ok(Object::Boolean(true)))
}

fn plan_mask(doc: &Document, dict: &Dictionary) -> Option<MaskJob> {
    let Object::Reference(id) = *dict.get(b"SMask").ok()? else {
        return None;
    };
    let stream = doc.objects.get(&id)?.as_stream().ok()?;
    let d = &stream.dict;
    let plain = matches!(d.get(b"ColorSpace"), Ok(Object::Name(n)) if n == b"DeviceGray")
        && dim(doc, d, b"BitsPerComponent") == Some(8)
        && !has_special_keys(d)
        && !d.has(b"SMask");
    (plain && source_of(stream) == Some(Source::Samples)).then(|| MaskJob {
        id,
        width: dim(doc, d, b"Width").unwrap_or(0),
        height: dim(doc, d, b"Height").unwrap_or(0),
        stream: stream.clone(),
    })
}

fn plan_job(
    doc: &Document,
    id: ObjectId,
    dpi: f64,
    rule: &ImageRule,
    grayscale: bool,
    masks_taken: &mut HashSet<ObjectId>,
) -> Option<Job> {
    let stream = doc.objects.get(&id)?.as_stream().ok()?;
    let d = &stream.dict;
    if !matches!(d.get(b"Subtype").and_then(Object::as_name), Ok(b"Image"))
        || has_special_keys(d)
        || dim(doc, d, b"BitsPerComponent") != Some(8)
    {
        return None;
    }
    let (width, height) = (dim(doc, d, b"Width")?, dim(doc, d, b"Height")?);
    let pixels = u64::from(width) * u64::from(height);
    if !(MIN_PIXELS..=MAX_PIXELS).contains(&pixels) {
        return None;
    }
    let channels = channels_of(doc, d)?;
    let source = source_of(stream)?;
    let shrink = dpi > rule.above_dpi;
    let gray = grayscale && channels == 3;
    if !shrink && !gray {
        return None;
    }
    let factor = if shrink { rule.target_dpi / dpi } else { 1.0 };
    let scaled = |n: u32| ((f64::from(n) * factor).round() as u32).max(1);
    let (new_width, new_height) = (scaled(width), scaled(height));
    // A soft mask is shared at most once, and only follows a real shrink.
    let mask = (new_width < width)
        .then(|| plan_mask(doc, d))
        .flatten()
        .filter(|m| m.width > 0 && m.height > 0 && masks_taken.insert(m.id));
    Some(Job {
        id,
        stream: stream.clone(),
        source,
        width,
        height,
        channels,
        new_width,
        new_height,
        gray,
        quality: rule.quality,
        mask,
    })
}

/// Decodes the image, shrinks it and encodes it as JPEG. `None` when anything
/// is unexpected or the result would not be smaller than the original.
fn recompress(job: &Job) -> Option<Done> {
    let (w, h, ch) = (job.width, job.height, usize::from(job.channels));
    let expected = w as usize * h as usize * ch;
    let decoded = match job.source {
        Source::Jpeg => {
            let img =
                image::load_from_memory_with_format(&job.stream.content, ImageFormat::Jpeg).ok()?;
            match (&img, job.channels) {
                (DynamicImage::ImageRgb8(_), 3) | (DynamicImage::ImageLuma8(_), 1) => img,
                _ => return None,
            }
        }
        Source::Samples => {
            let raw = job.stream.get_plain_content_with_limit(expected).ok()?;
            if raw.len() != expected {
                return None;
            }
            if ch == 3 {
                DynamicImage::ImageRgb8(image::RgbImage::from_raw(w, h, raw)?)
            } else {
                DynamicImage::ImageLuma8(image::GrayImage::from_raw(w, h, raw)?)
            }
        }
    };
    if decoded.width() != w || decoded.height() != h {
        return None;
    }
    let resized = if (job.new_width, job.new_height) == (w, h) {
        decoded
    } else {
        decoded.resize_exact(job.new_width, job.new_height, FilterType::Lanczos3)
    };
    let mut jpeg = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, job.quality);
    let encoded = if job.gray || job.channels == 1 {
        let img = resized.to_luma8();
        encoder.encode(
            img.as_raw(),
            img.width(),
            img.height(),
            ExtendedColorType::L8,
        )
    } else {
        let img = resized.to_rgb8();
        encoder.encode(
            img.as_raw(),
            img.width(),
            img.height(),
            ExtendedColorType::Rgb8,
        )
    };
    encoded.ok()?;
    if jpeg.len() >= job.stream.content.len() {
        return None;
    }
    let mask = job
        .mask
        .as_ref()
        .and_then(|m| shrink_mask(m, job.new_width, job.width));
    Some(Done {
        id: job.id,
        jpeg,
        width: job.new_width,
        height: job.new_height,
        gray: job.gray,
        mask,
    })
}

/// Scales a soft mask by the same ratio as its image, keeping it lossless.
fn shrink_mask(m: &MaskJob, new_image_width: u32, image_width: u32) -> Option<(ObjectId, Stream)> {
    let ratio = f64::from(new_image_width) / f64::from(image_width);
    let scaled = |n: u32| ((f64::from(n) * ratio).round() as u32).max(1);
    let (nw, nh) = (scaled(m.width), scaled(m.height));
    let expected = m.width as usize * m.height as usize;
    let raw = m.stream.get_plain_content_with_limit(expected).ok()?;
    let gray = image::GrayImage::from_raw(m.width, m.height, raw)?;
    let small = image::imageops::resize(&gray, nw, nh, FilterType::Lanczos3);
    let mut dict = m.stream.dict.clone();
    dict.set("Width", i64::from(nw));
    dict.set("Height", i64::from(nh));
    dict.remove(b"Filter");
    dict.remove(b"DecodeParms");
    let mut stream = Stream::new(dict, small.into_raw());
    stream.compress().ok()?;
    (stream.content.len() < m.stream.content.len()).then_some((m.id, stream))
}

fn recompress_images(doc: &mut Document, rule: &ImageRule, grayscale: bool) -> Stats {
    let mut scan = Scan::new(doc);
    scan.pages();
    let measured = scan.dpi;

    // Images used as masks of other images are not drawn by themselves and are
    // handled together with their owner.
    let mut owned_masks: HashSet<ObjectId> = HashSet::new();
    for object in doc.objects.values() {
        if let Some(d) = dict_of(object) {
            for key in [&b"SMask"[..], b"Mask"] {
                if let Ok(Object::Reference(id)) = d.get(key) {
                    owned_masks.insert(*id);
                }
            }
        }
    }

    let mut ids: Vec<ObjectId> = measured
        .keys()
        .copied()
        .filter(|id| !owned_masks.contains(id))
        .collect();
    ids.sort_unstable();
    let mut masks_taken = HashSet::new();
    let jobs: Vec<Job> = ids
        .iter()
        .filter_map(|&id| plan_job(doc, id, measured[&id], rule, grayscale, &mut masks_taken))
        .collect();
    let found = jobs.len() as u32;

    // Decoding, resizing and encoding is the slow part; do it in parallel.
    let done: Vec<Done> = jobs.par_iter().filter_map(recompress).collect();
    let recompressed = done.len() as u32;
    for d in done {
        if let Some(Object::Stream(s)) = doc.objects.get_mut(&d.id) {
            s.dict.set("Width", i64::from(d.width));
            s.dict.set("Height", i64::from(d.height));
            s.dict.set("BitsPerComponent", 8_i64);
            if d.gray {
                s.dict.set("ColorSpace", "DeviceGray");
            }
            s.dict.set("Filter", "DCTDecode");
            s.dict.remove(b"DecodeParms");
            s.set_content(d.jpeg);
        }
        if let Some((id, stream)) = d.mask {
            doc.objects.insert(id, Object::Stream(stream));
        }
    }
    Stats {
        found,
        recompressed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrices_apply_the_inner_one_first() {
        let scale = [2.0, 0.0, 0.0, 2.0, 0.0, 0.0];
        let shift = [1.0, 0.0, 0.0, 1.0, 10.0, 20.0];
        // Scale, then shift: the shift is not scaled.
        assert_eq!(mul(&scale, &shift), [2.0, 0.0, 0.0, 2.0, 10.0, 20.0]);
        // Shift, then scale: the shift is doubled.
        assert_eq!(mul(&shift, &scale), [2.0, 0.0, 0.0, 2.0, 20.0, 40.0]);
    }

    #[test]
    fn presets_follow_the_spec_table() {
        assert!(Preset::Lossless.image_rule().is_none());
        let b = Preset::Balanced.image_rule().unwrap();
        assert_eq!((b.above_dpi, b.target_dpi, b.quality), (200.0, 150.0, 75));
        let s = Preset::Strong.image_rule().unwrap();
        assert_eq!(s.target_dpi, 100.0);
        assert_eq!(s.quality, 60);
    }
}
