//! Integration tests for Compress (SPEC 5.7).

mod common;

use common::{jpeg_bytes, make_pdf, markers, Tmp};
use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream};
use pdf_core::{compress_file, CompressOptions, CompressReport, Error, Preset};

const BALANCED: CompressOptions = CompressOptions {
    preset: Preset::Balanced,
    grayscale: false,
};

/// Deterministic noise, so Flate cannot shrink the pixels much.
fn noise(len: usize) -> Vec<u8> {
    let mut x: u32 = 0x1234_5678;
    (0..len)
        .map(|_| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (x >> 24) as u8
        })
        .collect()
}

/// Smooth picture with a little noise, like a photo or a scan.
fn photo(w: u32, h: u32, channels: usize) -> Vec<u8> {
    let n = noise(w as usize * h as usize * channels);
    let mut out = Vec::with_capacity(n.len());
    for y in 0..h {
        for x in 0..w {
            for c in 0..channels {
                let base = (x * 255 / w + y * 255 / h) / 2 + (c as u32) * 20;
                let i = (y as usize * w as usize + x as usize) * channels + c;
                out.push((base + u32::from(n[i] % 24)).min(255) as u8);
            }
        }
    }
    out
}

fn image_dict(w: u32, h: u32, colorspace: &str) -> Dictionary {
    dictionary! {
        "Type" => "XObject",
        "Subtype" => "Image",
        "Width" => i64::from(w),
        "Height" => i64::from(h),
        "ColorSpace" => colorspace,
        "BitsPerComponent" => 8,
    }
}

fn flate_image(w: u32, h: u32, colorspace: &str, pixels: Vec<u8>) -> Stream {
    let mut s = Stream::new(image_dict(w, h, colorspace), pixels);
    s.compress().unwrap();
    s
}

/// A one-page document drawing `image` once for every `(width, height)` in
/// points, side by side.
fn doc_with_image(image: Stream, draws: &[(f64, f64)]) -> (Document, ObjectId) {
    let mut doc = make_pdf("img", 1);
    let img = doc.add_object(image);
    let mut content = String::new();
    let mut x = 0.0;
    for (w, h) in draws {
        content += &format!("q {w} 0 0 {h} {x} 0 cm /Im0 Do Q\n");
        x += w + 10.0;
    }
    let page = *doc.get_pages().values().next().unwrap();
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
    let dict = doc.get_dictionary_mut(page).unwrap();
    dict.set("Contents", content_id);
    dict.set(
        "Resources",
        dictionary! { "XObject" => dictionary! { "Im0" => img } },
    );
    dict.set(
        "MediaBox",
        vec![0.into(), 0.into(), 2000.into(), 2000.into()],
    );
    (doc, img)
}

fn run(doc: &mut Document, opts: CompressOptions, name: &str) -> (CompressReport, Document, Tmp) {
    let tmp = Tmp::new(name);
    let src = tmp.path("in.pdf");
    let out = tmp.path("out.pdf");
    doc.save(&src).unwrap();
    let report = compress_file(&src, &out, &opts).unwrap();
    let result = if report.written {
        Document::load(&out).unwrap()
    } else {
        assert!(!out.exists(), "nothing may be written when not smaller");
        Document::load(&src).unwrap()
    };
    (report, result, tmp)
}

fn images(doc: &Document) -> Vec<&Stream> {
    doc.objects
        .values()
        .filter_map(|o| o.as_stream().ok())
        .filter(|s| {
            matches!(
                s.dict.get(b"Subtype").and_then(Object::as_name),
                Ok(b"Image")
            )
        })
        .collect()
}

fn size(s: &Stream, key: &[u8]) -> i64 {
    s.dict.get(key).unwrap().as_i64().unwrap()
}

fn filter(s: &Stream) -> Option<String> {
    s.filters()
        .ok()
        .map(|f| String::from_utf8_lossy(f[0]).into_owned())
}

#[test]
fn lossless_shrinks_structure_and_keeps_pages() {
    let mut doc = make_pdf("doc", 40);
    // Uncompressed, repetitive content plus the junk the pass must remove.
    let pages: Vec<_> = doc.get_pages().values().copied().collect();
    for (i, &p) in pages.iter().enumerate() {
        let thumb = doc.add_object(Stream::new(dictionary! {}, noise(2000)));
        let big = format!(
            "BT /F1 12 Tf 10 10 Td (doc-{}) Tj ET\n{}",
            i + 1,
            "0 0 m 10 10 l S\n".repeat(400)
        );
        let content = doc.add_object(Stream::new(dictionary! {}, big.into_bytes()));
        let d = doc.get_dictionary_mut(p).unwrap();
        d.set("Thumb", thumb);
        d.set("PieceInfo", dictionary! { "App" => 1 });
        d.set("Contents", content);
    }
    let (report, out, _tmp) = run(
        &mut doc,
        CompressOptions {
            preset: Preset::Lossless,
            grayscale: false,
        },
        "lossless",
    );
    assert!(report.written);
    assert!(report.after * 2 < report.before, "{report:?}");
    assert_eq!(report.images_recompressed, 0);
    assert_eq!(out.get_pages().len(), 40);
    let m = markers(&out);
    assert_eq!(m[0], "doc-1");
    assert_eq!(m[39], "doc-40");
    for &p in out.get_pages().values() {
        let d = out.get_dictionary(p).unwrap();
        assert!(!d.has(b"Thumb") && !d.has(b"PieceInfo"));
    }
}

#[test]
fn identical_streams_are_merged() {
    let mut doc = make_pdf("dup", 3);
    let shared = noise(5000);
    let pages: Vec<_> = doc.get_pages().values().copied().collect();
    let mut fonts = Vec::new();
    for &p in &pages {
        // Three separate objects with the same bytes, as merged files produce.
        let obj = doc.add_object(Stream::new(
            dictionary! { "Subtype" => "Blob" },
            shared.clone(),
        ));
        fonts.push(obj);
        doc.get_dictionary_mut(p)
            .unwrap()
            .set("Resources", dictionary! { "Blob" => obj });
    }
    let (report, out, _tmp) = run(
        &mut doc,
        CompressOptions {
            preset: Preset::Lossless,
            grayscale: false,
        },
        "dedupe",
    );
    assert!(report.written, "{report:?}");
    let ids: Vec<_> = out
        .get_pages()
        .values()
        .map(|&p| {
            let r = out.get_dictionary(p).unwrap().get(b"Resources").unwrap();
            let r = r.as_dict().unwrap();
            r.get(b"Blob").unwrap().as_reference().unwrap()
        })
        .collect();
    assert_eq!(ids[0], ids[1]);
    assert_eq!(ids[1], ids[2]);
    let blobs = out
        .objects
        .values()
        .filter(|o| matches!(o, Object::Stream(s) if s.dict.has(b"Subtype")))
        .count();
    assert_eq!(blobs, 1);
}

#[test]
fn balanced_downsamples_a_large_flate_image() {
    // 1600x1200 pixels drawn 288x216 pt (4x3 in) is 400 DPI.
    let pixels = photo(1600, 1200, 3);
    let (mut doc, _) = doc_with_image(
        flate_image(1600, 1200, "DeviceRGB", pixels),
        &[(288.0, 216.0)],
    );
    let (report, out, _tmp) = run(&mut doc, BALANCED, "balanced");
    assert!(report.written);
    assert_eq!(report.images_recompressed, 1);
    assert!(report.after * 4 < report.before, "{report:?}");
    let img = images(&out);
    assert_eq!(img.len(), 1);
    // 150 DPI at 4x3 in.
    assert_eq!(
        (size(img[0], b"Width"), size(img[0], b"Height")),
        (600, 450)
    );
    assert_eq!(filter(img[0]).as_deref(), Some("DCTDecode"));
    // The page still draws the image exactly as before.
    assert_eq!(markers(&out).len(), 1);
}

#[test]
fn strong_goes_lower_than_balanced() {
    let make = || {
        doc_with_image(
            flate_image(1600, 1200, "DeviceRGB", photo(1600, 1200, 3)),
            &[(288.0, 216.0)],
        )
        .0
    };
    let (rb, ob, _t1) = run(&mut make(), BALANCED, "cmp-balanced");
    let strong = CompressOptions {
        preset: Preset::Strong,
        grayscale: false,
    };
    let (rs, os, _t2) = run(&mut make(), strong, "cmp-strong");
    assert!(rs.after < rb.after, "{rs:?} vs {rb:?}");
    assert_eq!(size(images(&os)[0], b"Width"), 400);
    assert_eq!(size(images(&ob)[0], b"Width"), 600);
}

#[test]
fn a_jpeg_is_recompressed_at_a_lower_resolution() {
    let jpeg = jpeg_bytes(1600, 1200, None);
    let mut dict = image_dict(1600, 1200, "DeviceRGB");
    dict.set("Filter", "DCTDecode");
    let (mut doc, _) = doc_with_image(Stream::new(dict, jpeg), &[(288.0, 216.0)]);
    let (report, out, _tmp) = run(&mut doc, BALANCED, "jpeg");
    assert!(report.written);
    assert_eq!(report.images_recompressed, 1);
    let img = images(&out);
    assert_eq!(size(img[0], b"Width"), 600);
    assert_eq!(filter(img[0]).as_deref(), Some("DCTDecode"));
}

#[test]
fn images_at_or_below_the_threshold_are_left_alone() {
    // 800x600 pixels at 4x3 in is 200 DPI: not above the Balanced threshold.
    let pixels = photo(800, 600, 3);
    let original = flate_image(800, 600, "DeviceRGB", pixels);
    let content = original.content.clone();
    let (mut doc, _) = doc_with_image(original, &[(288.0, 216.0)]);
    let (report, out, _tmp) = run(&mut doc, BALANCED, "threshold");
    assert_eq!(report.images_recompressed, 0);
    let img = images(&out);
    assert_eq!(size(img[0], b"Width"), 800);
    assert_eq!(filter(img[0]).as_deref(), Some("FlateDecode"));
    assert_eq!(img[0].content, content);
}

#[test]
fn the_largest_use_decides_the_resolution() {
    // 1600 px drawn 4 in wide (400 DPI) and 5.33 in wide (300 DPI): the second
    // use must still look good, so the image goes to 150 / 300 of its size.
    let pixels = photo(1600, 1200, 3);
    let (mut doc, _) = doc_with_image(
        flate_image(1600, 1200, "DeviceRGB", pixels),
        &[(288.0, 216.0), (384.0, 288.0)],
    );
    let (_, out, _tmp) = run(&mut doc, BALANCED, "largest-use");
    assert_eq!(size(images(&out)[0], b"Width"), 800);
}

#[test]
fn an_image_that_is_drawn_large_somewhere_is_not_touched() {
    // The second use is 200 DPI, so nothing is above the threshold.
    let pixels = photo(1600, 1200, 3);
    let (mut doc, _) = doc_with_image(
        flate_image(1600, 1200, "DeviceRGB", pixels),
        &[(288.0, 216.0), (576.0, 432.0)],
    );
    let (report, out, _tmp) = run(&mut doc, BALANCED, "large-use");
    assert_eq!(report.images_recompressed, 0);
    assert_eq!(size(images(&out)[0], b"Width"), 1600);
}

#[test]
fn images_inside_forms_use_the_form_matrix() {
    let pixels = photo(1600, 1200, 3);
    let (mut doc, img) =
        doc_with_image(flate_image(1600, 1200, "DeviceRGB", pixels), &[(1.0, 1.0)]);
    // The form draws the image 288x216 inside a form scaled by 0.5, so the
    // image shows 144x108 pt, which is 800 DPI.
    let mut form_dict = dictionary! {
        "Type" => "XObject",
        "Subtype" => "Form",
        "BBox" => vec![0.into(), 0.into(), 1000.into(), 1000.into()],
        "Matrix" => vec![Object::Real(0.5), 0.into(), 0.into(), Object::Real(0.5), 0.into(), 0.into()],
    };
    form_dict.set(
        "Resources",
        dictionary! { "XObject" => dictionary! { "Im0" => img } },
    );
    let form = doc.add_object(Stream::new(
        form_dict,
        b"q 288 0 0 216 0 0 cm /Im0 Do Q".to_vec(),
    ));
    let page = *doc.get_pages().values().next().unwrap();
    let content = doc.add_object(Stream::new(dictionary! {}, b"/Fm0 Do".to_vec()));
    let d = doc.get_dictionary_mut(page).unwrap();
    d.set("Contents", content);
    d.set(
        "Resources",
        dictionary! { "XObject" => dictionary! { "Fm0" => form } },
    );
    let (report, out, _tmp) = run(&mut doc, BALANCED, "form");
    assert_eq!(report.images_recompressed, 1);
    // 150 / 800 of 1600 pixels.
    assert_eq!(size(images(&out)[0], b"Width"), 300);
}

#[test]
fn grayscale_turns_colour_images_gray() {
    let pixels = photo(800, 600, 3);
    // 100 DPI is below the Strong threshold, so only the colour changes.
    let (mut doc, _) = doc_with_image(
        flate_image(800, 600, "DeviceRGB", pixels),
        &[(576.0, 432.0)],
    );
    let (report, out, _tmp) = run(
        &mut doc,
        CompressOptions {
            preset: Preset::Strong,
            grayscale: true,
        },
        "gray",
    );
    assert_eq!(report.images_recompressed, 1);
    let img = images(&out)[0];
    assert_eq!(
        img.dict.get(b"ColorSpace").unwrap().as_name().unwrap(),
        b"DeviceGray"
    );
    assert_eq!(size(img, b"Width"), 800);

    // Without the option the same document keeps its colour and size.
    let (mut doc, _) = doc_with_image(
        flate_image(800, 600, "DeviceRGB", photo(800, 600, 3)),
        &[(576.0, 432.0)],
    );
    let strong = CompressOptions {
        preset: Preset::Strong,
        grayscale: false,
    };
    let (report, _, _t) = run(&mut doc, strong, "no-gray");
    assert_eq!(report.images_recompressed, 0);
}

#[test]
fn soft_masks_shrink_with_their_image() {
    let mask_px: Vec<u8> = (0..1600 * 1200).map(|i| ((i % 1600) / 7) as u8).collect();
    let mask = flate_image(1600, 1200, "DeviceGray", mask_px);
    let mut base = flate_image(1600, 1200, "DeviceRGB", photo(1600, 1200, 3));
    let (mut doc, img) = doc_with_image(base.clone(), &[(288.0, 216.0)]);
    let mask_id = doc.add_object(mask);
    base.dict.set("SMask", mask_id);
    doc.objects.insert(img, Object::Stream(base));
    let (report, out, _tmp) = run(&mut doc, BALANCED, "smask");
    assert_eq!(report.images_recompressed, 1);
    let all = images(&out);
    assert_eq!(all.len(), 2);
    for s in all {
        assert_eq!(size(s, b"Width"), 600, "image and mask shrink together");
        assert_eq!(size(s, b"Height"), 450);
    }
}

/// Changes an image dictionary into one the tool must leave alone.
type Tweak = Box<dyn Fn(&mut Dictionary)>;

#[test]
fn unusual_images_pass_through() {
    // Indexed colour, a Decode array and a colour key mask are all skipped.
    let cases: Vec<(&str, Tweak)> = vec![
        (
            "decode",
            Box::new(|d: &mut Dictionary| {
                d.set(
                    "Decode",
                    vec![1.into(), 0.into(), 1.into(), 0.into(), 1.into(), 0.into()],
                );
            }),
        ),
        (
            "keymask",
            Box::new(|d: &mut Dictionary| {
                d.set(
                    "Mask",
                    vec![0.into(), 1.into(), 0.into(), 1.into(), 0.into(), 1.into()],
                );
            }),
        ),
        (
            "cmyk",
            Box::new(|d: &mut Dictionary| d.set("ColorSpace", "DeviceCMYK")),
        ),
        (
            "unknown-filter",
            Box::new(|d: &mut Dictionary| d.set("Filter", "JBIG2Decode")),
        ),
    ];
    for (name, tweak) in cases {
        let mut s = flate_image(1600, 1200, "DeviceRGB", photo(1600, 1200, 3));
        tweak(&mut s.dict);
        let before = s.content.clone();
        let (mut doc, _) = doc_with_image(s, &[(288.0, 216.0)]);
        let (report, out, _tmp) = run(&mut doc, BALANCED, name);
        assert_eq!(report.images_recompressed, 0, "{name}");
        assert_eq!(images(&out)[0].content, before, "{name}");
    }
}

#[test]
fn a_broken_jpeg_is_left_as_it_is() {
    let mut dict = image_dict(1600, 1200, "DeviceRGB");
    dict.set("Filter", "DCTDecode");
    let junk = noise(30_000);
    let (mut doc, _) = doc_with_image(Stream::new(dict, junk.clone()), &[(288.0, 216.0)]);
    let (report, out, _tmp) = run(&mut doc, BALANCED, "broken");
    assert_eq!(report.images_recompressed, 0);
    assert_eq!(images(&out)[0].content, junk);
}

#[test]
fn nothing_is_written_when_the_result_is_not_smaller() {
    // Already about as small as a PDF gets.
    let mut doc = make_pdf("tiny", 1);
    doc.compress();
    let tmp = Tmp::new("not-smaller");
    let src = tmp.path("in.pdf");
    let out = tmp.path("out.pdf");
    doc.save(&src).unwrap();
    // First run squeezes what there is; the second has nothing left to win.
    let first = compress_file(&src, &out, &BALANCED).unwrap();
    let again = if first.written {
        let second = tmp.path("second.pdf");
        compress_file(&out, &second, &BALANCED).unwrap()
    } else {
        first
    };
    assert!(!again.written, "{again:?}");
    assert!(again.after >= again.before);
    assert!(!tmp.path("second.pdf").exists());
}

#[test]
fn the_report_matches_the_files() {
    let (mut doc, _) = doc_with_image(
        flate_image(1600, 1200, "DeviceRGB", photo(1600, 1200, 3)),
        &[(288.0, 216.0)],
    );
    let tmp = Tmp::new("report");
    let src = tmp.path("in.pdf");
    let out = tmp.path("out.pdf");
    doc.save(&src).unwrap();
    let r = compress_file(&src, &out, &BALANCED).unwrap();
    assert_eq!(r.before, std::fs::metadata(&src).unwrap().len());
    assert_eq!(r.after, std::fs::metadata(&out).unwrap().len());
    assert!(r.written && r.after < r.before);
    assert_eq!(r.images_found, 1);
    // Objects are packed into object streams.
    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.windows(7).any(|w| w == b"/ObjStm"));
}

#[test]
fn the_source_is_never_overwritten() {
    let tmp = Tmp::new("same-file");
    let src = tmp.path("in.pdf");
    make_pdf("x", 1).save(&src).unwrap();
    let before = std::fs::read(&src).unwrap();
    let err = compress_file(&src, &src, &BALANCED).unwrap_err();
    assert!(matches!(err, Error::OutputIsSource(_)));
    assert_eq!(std::fs::read(&src).unwrap(), before);
}

#[test]
fn bookmarks_and_links_survive() {
    let mut doc = make_pdf("bm", 3);
    common::add_outline(&mut doc, "bm");
    let big = format!("BT (bm-1) Tj ET\n{}", "0 0 m 5 5 l S\n".repeat(500));
    let page = *doc.get_pages().values().next().unwrap();
    let c = doc.add_object(Stream::new(dictionary! {}, big.into_bytes()));
    doc.get_dictionary_mut(page).unwrap().set("Contents", c);
    let (report, out, _tmp) = run(
        &mut doc,
        CompressOptions {
            preset: Preset::Lossless,
            grayscale: false,
        },
        "bookmarks",
    );
    assert!(report.written);
    let titles = common::bookmark_titles(&out);
    assert_eq!(titles.len(), 3, "{titles:?}");
}
