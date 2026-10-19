//! Benchmark for Compress on generated documents.
//!
//! Run with `cargo run --release -p pdf-core --example bench_compress`.
//! The documents are synthetic stand-ins for three common kinds of PDF (a
//! scan, an office export with photos, a text-only file), so the numbers show
//! how each preset behaves, not what any particular real file will do.

use std::path::Path;
use std::time::Instant;

use image::codecs::jpeg::JpegEncoder;
use image::ExtendedColorType;
use lopdf::{dictionary, Document, Object, Stream};
use pdf_core::{compress_file, CompressOptions, Preset};

struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0 >> 8
    }
}

/// Paper-coloured page with sensor noise and dark text-like bars.
fn scan_pixels(w: u32, h: u32, seed: u32) -> Vec<u8> {
    let mut rng = Rng(seed);
    let mut px = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        let line = (y / 40) % 3 != 2 && y % 40 > 8 && y % 40 < 30 && y > h / 12 && y < h * 11 / 12;
        for x in 0..w {
            let ink = line && (x / 23 + y / 40) % 7 != 0 && x > w / 10 && x < w * 9 / 10;
            let base: i32 = if ink { 40 } else { 228 };
            let grain = (rng.next() % 17) as i32 - 8;
            let shade = (base + grain).clamp(0, 255) as u8;
            px.extend_from_slice(&[shade, shade.saturating_sub(4), shade.saturating_sub(12)]);
        }
    }
    px
}

/// Smooth colourful picture with some texture.
fn photo_pixels(w: u32, h: u32, seed: u32) -> Vec<u8> {
    let mut rng = Rng(seed);
    let mut px = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 / w as f32;
            let fy = y as f32 / h as f32;
            let r = 128.0 + 100.0 * (fx * 6.0).sin() * (fy * 3.0).cos();
            let g = 128.0 + 100.0 * (fy * 5.0 + fx).sin();
            let b = 128.0 + 100.0 * ((fx + fy) * 4.0).cos();
            let n = (rng.next() % 21) as f32 - 10.0;
            px.extend_from_slice(&[
                (r + n).clamp(0.0, 255.0) as u8,
                (g + n).clamp(0.0, 255.0) as u8,
                (b + n).clamp(0.0, 255.0) as u8,
            ]);
        }
    }
    px
}

fn jpeg(px: &[u8], w: u32, h: u32, quality: u8) -> Vec<u8> {
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, quality)
        .encode(px, w, h, ExtendedColorType::Rgb8)
        .unwrap();
    out
}

fn image_dict(w: u32, h: u32) -> lopdf::Dictionary {
    dictionary! {
        "Type" => "XObject", "Subtype" => "Image",
        "Width" => i64::from(w), "Height" => i64::from(h),
        "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
    }
}

/// An image and the size in points it is drawn at.
type Placed = (Stream, f64, f64);

/// Builds a document from pages given as `(text content, images)`; the images
/// are stacked down from the top of the page.
fn build(pages: Vec<(String, Vec<Placed>)>, flate_content: bool) -> Document {
    let mut doc = Document::with_version("1.4");
    let pages_id = doc.new_object_id();
    let font = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut kids = Vec::new();
    for (text, images) in pages {
        let mut content = String::new();
        let mut xobjects = lopdf::Dictionary::new();
        let mut y = 760.0;
        for (i, (img, w, h)) in images.into_iter().enumerate() {
            let id = doc.add_object(img);
            let name = format!("Im{i}");
            content += &format!("q {w} 0 0 {h} 36 {} cm /{name} Do Q\n", y - h);
            xobjects.set(name, id);
            y -= h + 12.0;
        }
        content += &text;
        let mut stream = Stream::new(dictionary! {}, content.into_bytes());
        if flate_content {
            stream.compress().unwrap();
        }
        let content_id = doc.add_object(stream);
        let page = dictionary! {
            "Type" => "Page", "Parent" => pages_id, "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => dictionary! {
                "Font" => dictionary! { "F1" => font },
                "XObject" => xobjects,
            },
        };
        kids.push(Object::Reference(doc.add_object(page)));
    }
    let count = kids.len() as i64;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    doc
}

fn text_page(n: usize, lines: usize) -> String {
    let mut s = String::from("BT /F1 10 Tf 12 TL 36 750 Td\n");
    for l in 0..lines {
        s += &format!(
            "(Page {n} line {l}: the quick brown fox jumps over the lazy dog, again and again.) '\n"
        );
    }
    s + "ET\n"
}

/// An A4/Letter scan: one full-page JPEG per page at 300 DPI.
fn scan(pages: usize) -> Document {
    let (w, h) = (2550, 3300);
    let pages = (0..pages)
        .map(|i| {
            let px = scan_pixels(w, h, i as u32 + 1);
            let mut dict = image_dict(w, h);
            dict.set("Filter", "DCTDecode");
            let img = Stream::new(dict, jpeg(&px, w, h, 90));
            (String::new(), vec![(img, 612.0, 792.0)])
        })
        .collect();
    build(pages, true)
}

/// A report with text and two 300 DPI photos stored as Flate pixels per page.
fn office_with_photos(pages: usize) -> Document {
    let (w, h) = (1500, 1000);
    let pages = (0..pages)
        .map(|i| {
            let imgs = (0..2)
                .map(|k| {
                    let px = photo_pixels(w, h, (i * 2 + k) as u32 + 7);
                    let mut s = Stream::new(image_dict(w, h), px);
                    s.compress().unwrap();
                    (s, 360.0, 240.0)
                })
                .collect();
            (text_page(i + 1, 20), imgs)
        })
        .collect();
    build(pages, true)
}

/// A text-only document the way simple generators write it: no compression.
fn text_only(pages: usize, compressed: bool) -> Document {
    let pages = (0..pages)
        .map(|i| (text_page(i + 1, 60), Vec::new()))
        .collect();
    build(pages, compressed)
}

fn mb(n: u64) -> f64 {
    n as f64 / 1024.0 / 1024.0
}

fn main() {
    let dir = std::env::temp_dir().join("jjpdf-bench-compress");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cases: Vec<(&str, Document)> = vec![
        ("Scan, 8 pages, JPEG 300 DPI", scan(8)),
        (
            "Office report, 10 pages, 2 photos per page",
            office_with_photos(10),
        ),
        ("Text only, 100 pages, uncompressed", text_only(100, false)),
        ("Text only, 100 pages, already Flate", text_only(100, true)),
    ];

    println!("| Document | Original | Lossless | Balanced | Strong |");
    println!("| --- | --- | --- | --- | --- |");
    for (name, mut doc) in cases {
        let src = dir.join("in.pdf");
        doc.save(&src).unwrap();
        let mut row = format!(
            "| {name} | {:.2} MB |",
            mb(std::fs::metadata(&src).unwrap().len())
        );
        for preset in [Preset::Lossless, Preset::Balanced, Preset::Strong] {
            let out = dir.join("out.pdf");
            let _ = std::fs::remove_file(&out);
            let start = Instant::now();
            let r = compress_file(
                &src,
                &out,
                &CompressOptions {
                    preset,
                    grayscale: false,
                },
            )
            .unwrap();
            let secs = start.elapsed().as_secs_f64();
            check(&out, r.written);
            if r.written {
                let pct = 100.0 * (1.0 - r.after as f64 / r.before as f64);
                row += &format!(" {:.2} MB (-{pct:.0}%, {secs:.1}s) |", mb(r.after));
            } else {
                row += &format!(" kept original ({secs:.1}s) |");
            }
        }
        println!("{row}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The output must be a readable PDF whenever one was written.
fn check(out: &Path, written: bool) {
    if written {
        let d = Document::load(out).expect("output does not load");
        assert!(!d.get_pages().is_empty());
    } else {
        assert!(!out.exists());
    }
}
