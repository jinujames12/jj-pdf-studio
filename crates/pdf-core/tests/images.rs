//! Integration tests for Images to PDF: write images to disk, convert, reload.

mod common;

use std::path::{Path, PathBuf};

use common::{
    approx, first_cm, gradient, half_transparent, jpeg_bytes, media_boxes, with_exif_orientation,
    Tmp,
};
use image::{DynamicImage, GrayImage, ImageFormat};
use lopdf::{Document, Object, ObjectId, Stream};
use pdf_core::{
    images_to_folder, images_to_pdf, probe_image, Error, Fit, ImageInput, ImageOptions,
    PageOrientation, Paper, Quality,
};

fn opts(page: Option<Paper>) -> ImageOptions {
    ImageOptions {
        page,
        orientation: PageOrientation::Auto,
        margin_mm: 0.0,
        fit: Fit::Inside,
        quality: Quality::Keep,
    }
}

fn input(path: &Path) -> ImageInput {
    ImageInput {
        path: path.to_path_buf(),
        rotate: 0,
    }
}

fn first_page(doc: &Document) -> ObjectId {
    *doc.get_pages().values().next().unwrap()
}

/// The image XObject drawn on a page.
fn page_image(doc: &Document, page: ObjectId) -> &Stream {
    let (res, _) = doc.get_page_resources(page).unwrap();
    let xobjects = res.unwrap().get(b"XObject").unwrap().as_dict().unwrap();
    let id = xobjects.get(b"Im0").unwrap().as_reference().unwrap();
    doc.get_object(id).unwrap().as_stream().unwrap()
}

fn name_of(o: &Object) -> String {
    String::from_utf8_lossy(o.as_name().unwrap()).into_owned()
}

fn convert(images: &[PathBuf], options: &ImageOptions, out: &Path) -> Document {
    let inputs: Vec<_> = images.iter().map(|p| input(p)).collect();
    images_to_pdf(&inputs, out, options).unwrap();
    Document::load(out).unwrap()
}

#[test]
fn jpeg_is_embedded_byte_for_byte() {
    let t = Tmp::new("img-jpeg");
    let bytes = jpeg_bytes(120, 80, None);
    let src = t.path("photo.jpg");
    std::fs::write(&src, &bytes).unwrap();

    let doc = convert(&[src], &opts(Some(Paper::A4)), &t.path("out.pdf"));
    let img = page_image(&doc, first_page(&doc));
    assert_eq!(name_of(img.dict.get(b"Filter").unwrap()), "DCTDecode");
    assert_eq!(name_of(img.dict.get(b"ColorSpace").unwrap()), "DeviceRGB");
    assert_eq!(img.content, bytes, "the JPEG must not be re-encoded");
    assert_eq!(img.dict.get(b"Width").unwrap().as_i64().unwrap(), 120);
}

#[test]
fn a_wide_picture_gets_a_landscape_a4_page_filled_edge_to_edge() {
    let t = Tmp::new("img-a4");
    let src = t.path("wide.jpg");
    std::fs::write(&src, jpeg_bytes(200, 100, None)).unwrap();

    let doc = convert(&[src], &opts(Some(Paper::A4)), &t.path("out.pdf"));
    let b = media_boxes(&doc)[0];
    assert!(approx(b[2], 841.89) && approx(b[3], 595.28), "{b:?}");
    // Width-limited: spans the page width, centred vertically.
    let cm = first_cm(&doc, first_page(&doc));
    assert!(approx(cm[0], 841.89) && approx(cm[3], 420.945), "{cm:?}");
    assert!(approx(cm[5], (595.28 - 420.945) / 2.0), "{cm:?}");
}

#[test]
fn orientation_and_margin_are_applied() {
    let t = Tmp::new("img-orient");
    let src = t.path("wide.jpg");
    std::fs::write(&src, jpeg_bytes(200, 100, None)).unwrap();

    let mut o = opts(Some(Paper::Letter));
    o.orientation = PageOrientation::Portrait;
    o.margin_mm = 10.0;
    let doc = convert(std::slice::from_ref(&src), &o, &t.path("out.pdf"));
    let b = media_boxes(&doc)[0];
    assert!(approx(b[2], 612.0) && approx(b[3], 792.0), "{b:?}");
    let m = 10.0 * 72.0 / 25.4;
    let cm = first_cm(&doc, first_page(&doc));
    assert!(approx(cm[0], 612.0 - 2.0 * m), "{cm:?}");
    assert!(approx(cm[4], m), "{cm:?}");
}

#[test]
fn fill_crops_and_original_keeps_natural_size() {
    let t = Tmp::new("img-fit");
    let src = t.path("a.jpg");
    std::fs::write(&src, jpeg_bytes(200, 100, Some(96))).unwrap();

    let mut o = opts(Some(Paper::A4));
    o.orientation = PageOrientation::Portrait;
    o.fit = Fit::Fill;
    let doc = convert(std::slice::from_ref(&src), &o, &t.path("fill.pdf"));
    let page = first_page(&doc);
    let text = String::from_utf8_lossy(&doc.get_page_content(page)).into_owned();
    assert!(text.contains("re W n"), "fill must clip: {text}");
    let cm = first_cm(&doc, page);
    assert!(cm[0] > 595.28, "image is wider than the page: {cm:?}");
    assert!(approx(cm[3], 841.89), "{cm:?}");

    o.fit = Fit::Original;
    let doc = convert(std::slice::from_ref(&src), &o, &t.path("orig.pdf"));
    let cm = first_cm(&doc, first_page(&doc));
    assert!(
        approx(cm[0], 150.0) && approx(cm[3], 75.0),
        "200x100 px at 96 dpi: {cm:?}"
    );
}

#[test]
fn fit_to_image_uses_the_embedded_dpi() {
    let t = Tmp::new("img-dpi");
    let (hi, plain) = (t.path("hi.jpg"), t.path("plain.png"));
    std::fs::write(&hi, jpeg_bytes(300, 150, Some(300))).unwrap();
    gradient(96, 48).save(&plain).unwrap();

    let doc = convert(&[hi, plain], &opts(None), &t.path("out.pdf"));
    let boxes = media_boxes(&doc);
    assert!(
        approx(boxes[0][2], 72.0) && approx(boxes[0][3], 36.0),
        "300 px at 300 dpi: {boxes:?}"
    );
    assert!(
        approx(boxes[1][2], 72.0) && approx(boxes[1][3], 36.0),
        "96 px at the 96 dpi default: {boxes:?}"
    );
}

#[test]
fn exif_orientation_and_user_rotation_turn_the_picture() {
    let t = Tmp::new("img-exif");
    let src = t.path("phone.jpg");
    // Stored 200x100, EXIF 6: shown 100x200 (portrait).
    std::fs::write(
        &src,
        with_exif_orientation(&jpeg_bytes(200, 100, Some(96)), 6),
    )
    .unwrap();

    let meta = probe_image(&src).unwrap();
    assert_eq!((meta.width, meta.height), (100, 200));

    let doc = convert(std::slice::from_ref(&src), &opts(None), &t.path("a.pdf"));
    let b = media_boxes(&doc)[0];
    assert!(approx(b[2], 75.0) && approx(b[3], 150.0), "{b:?}");
    // Rotated 90 clockwise, as the matrix for orientation 6 says.
    let cm = first_cm(&doc, first_page(&doc));
    assert!(approx(cm[0], 0.0) && approx(cm[3], 0.0), "{cm:?}");
    assert!(approx(cm[2], 75.0) && approx(cm[1], -150.0), "{cm:?}");

    // A further user turn of 90 makes it landscape again.
    let mut turned = input(&src);
    turned.rotate = 90;
    images_to_pdf(&[turned], &t.path("b.pdf"), &opts(None)).unwrap();
    let b = media_boxes(&Document::load(t.path("b.pdf")).unwrap())[0];
    assert!(approx(b[2], 150.0) && approx(b[3], 75.0), "{b:?}");
}

#[test]
fn png_transparency_becomes_a_soft_mask_and_opaque_alpha_is_dropped() {
    let t = Tmp::new("img-alpha");
    let (clear, solid, gray) = (t.path("clear.png"), t.path("solid.png"), t.path("gray.png"));
    half_transparent(8, 8).save(&clear).unwrap();
    let mut opaque = half_transparent(8, 8);
    opaque.pixels_mut().for_each(|p| p.0[3] = 255);
    opaque.save(&solid).unwrap();
    GrayImage::from_fn(8, 8, |x, _| image::Luma([x as u8 * 30]))
        .save(&gray)
        .unwrap();

    let doc = convert(&[clear, solid, gray], &opts(None), &t.path("out.pdf"));
    let pages: Vec<_> = doc.get_pages().values().copied().collect();

    let first = page_image(&doc, pages[0]);
    assert_eq!(name_of(first.dict.get(b"Filter").unwrap()), "FlateDecode");
    let mask = first.dict.get(b"SMask").unwrap().as_reference().unwrap();
    let mask = doc.get_object(mask).unwrap().as_stream().unwrap();
    let alpha = mask.decompressed_content().unwrap();
    assert_eq!(alpha.len(), 64);
    assert_eq!((alpha[0], alpha[7]), (0, 255));
    assert_eq!(first.decompressed_content().unwrap().len(), 8 * 8 * 3);

    assert!(!page_image(&doc, pages[1]).dict.has(b"SMask"));
    let g = page_image(&doc, pages[2]);
    assert_eq!(name_of(g.dict.get(b"ColorSpace").unwrap()), "DeviceGray");
    assert_eq!(g.decompressed_content().unwrap().len(), 64);
}

#[test]
fn every_supported_format_converts() {
    let t = Tmp::new("img-formats");
    let img = DynamicImage::ImageRgb8(gradient(40, 30));
    let mut paths = Vec::new();
    for (name, format) in [
        ("a.bmp", ImageFormat::Bmp),
        ("b.gif", ImageFormat::Gif),
        ("c.tiff", ImageFormat::Tiff),
        ("d.webp", ImageFormat::WebP),
        ("e.png", ImageFormat::Png),
    ] {
        let p = t.path(name);
        img.save_with_format(&p, format).unwrap();
        paths.push(p);
    }
    let doc = convert(&paths, &opts(Some(Paper::A4)), &t.path("out.pdf"));
    assert_eq!(doc.get_pages().len(), 5);
}

#[test]
fn reduce_limits_the_long_edge_and_re_encodes() {
    let t = Tmp::new("img-reduce");
    let big = t.path("big.png");
    gradient(2100, 1050).save(&big).unwrap();

    let keep = convert(std::slice::from_ref(&big), &opts(None), &t.path("keep.pdf"));
    let mut o = opts(None);
    o.quality = Quality::Reduce;
    let reduced = convert(std::slice::from_ref(&big), &o, &t.path("reduced.pdf"));

    let img = page_image(&reduced, first_page(&reduced));
    assert_eq!(name_of(img.dict.get(b"Filter").unwrap()), "DCTDecode");
    assert_eq!(img.dict.get(b"Width").unwrap().as_i64().unwrap(), 2000);
    assert_eq!(img.dict.get(b"Height").unwrap().as_i64().unwrap(), 1000);
    assert!(
        std::fs::metadata(t.path("reduced.pdf")).unwrap().len()
            < std::fs::metadata(t.path("keep.pdf")).unwrap().len()
    );
    // Fewer pixels, same printed size.
    let (a, b) = (media_boxes(&keep)[0], media_boxes(&reduced)[0]);
    assert!(approx(a[2], b[2]) && approx(a[3], b[3]), "{a:?} vs {b:?}");
}

#[test]
fn reduce_never_makes_a_small_jpeg_bigger() {
    let t = Tmp::new("img-reduce-small");
    let src = t.path("tiny.jpg");
    // Already quality 30: re-encoding at 80 would grow it.
    let img = gradient(64, 64);
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 30)
        .encode(img.as_raw(), 64, 64, image::ExtendedColorType::Rgb8)
        .unwrap();
    std::fs::write(&src, &bytes).unwrap();

    let mut o = opts(None);
    o.quality = Quality::Reduce;
    let doc = convert(&[src], &o, &t.path("out.pdf"));
    assert_eq!(page_image(&doc, first_page(&doc)).content, bytes);
}

#[test]
fn one_pdf_per_image_never_overwrites() {
    let t = Tmp::new("img-folder");
    let (a, b) = (t.path("one.jpg"), t.path("two.png"));
    std::fs::write(&a, jpeg_bytes(30, 20, None)).unwrap();
    gradient(30, 20).save(&b).unwrap();
    let out = t.path("out");
    std::fs::create_dir(&out).unwrap();
    std::fs::write(out.join("one.pdf"), b"keep me").unwrap();

    let written = images_to_folder(&[input(&a), input(&b)], &out, &opts(Some(Paper::A4))).unwrap();
    let names: Vec<_> = written
        .iter()
        .map(|w| w.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["one (2).pdf", "two.pdf"]);
    assert_eq!(std::fs::read(out.join("one.pdf")).unwrap(), b"keep me");
    for w in &written {
        assert_eq!(Document::load(&w.path).unwrap().get_pages().len(), 1);
    }
}

#[test]
fn bad_input_is_reported_in_plain_language_and_leaves_nothing_behind() {
    let t = Tmp::new("img-bad");
    let good = t.path("good.jpg");
    std::fs::write(&good, jpeg_bytes(30, 20, None)).unwrap();
    let text = t.path("notes.jpg");
    std::fs::write(&text, b"this is not a picture").unwrap();
    let cut = t.path("cut.jpg");
    let full = jpeg_bytes(300, 200, None);
    std::fs::write(&cut, &full[..full.len() / 3]).unwrap();
    let out = t.path("out.pdf");

    assert!(matches!(
        images_to_pdf(&[input(&good), input(&text)], &out, &opts(None)),
        Err(Error::UnsupportedImage(n)) if n == "notes.jpg"
    ));
    assert!(matches!(
        images_to_pdf(&[input(&cut)], &out, &opts(None)),
        Err(Error::BadImage(_))
    ));
    assert!(!out.exists());

    assert!(matches!(
        images_to_pdf(&[], &out, &opts(None)),
        Err(Error::NoImages)
    ));
    let mut o = opts(Some(Paper::A4));
    o.margin_mm = 200.0;
    assert!(matches!(
        images_to_pdf(&[input(&good)], &out, &o),
        Err(Error::BadMargin)
    ));
    o.margin_mm = -1.0;
    assert!(matches!(
        images_to_pdf(&[input(&good)], &out, &o),
        Err(Error::BadMargin)
    ));
    let mut turned = input(&good);
    turned.rotate = 45;
    assert!(matches!(
        images_to_pdf(&[turned], &out, &opts(None)),
        Err(Error::BadRotation(45))
    ));

    // Per-image mode removes the files it already wrote when a later one fails.
    let dir = t.path("dir");
    std::fs::create_dir(&dir).unwrap();
    assert!(images_to_folder(&[input(&good), input(&text)], &dir, &opts(None)).is_err());
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
}
