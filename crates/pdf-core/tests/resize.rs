//! Integration tests for Resize: mixed-size documents, selections, annotations.

mod common;

use common::{approx, first_cm, make_sized_pdf, markers, media_boxes, Tmp};
use lopdf::{dictionary, Document, Object};
use pdf_core::{
    resize_file, Anchor, Error, PageOrientation, Paper, ResizeMode, ResizeOptions, Target,
};

fn opts(paper: Paper, mode: ResizeMode) -> ResizeOptions {
    ResizeOptions {
        paper,
        orientation: PageOrientation::Auto,
        mode,
        anchor: Anchor::Center,
        margin_mm: 0.0,
        target: Target::All,
    }
}

const A4: (f64, f64) = (595.276, 841.89);

/// Saves `doc`, resizes it and loads the result.
fn run(doc: &mut Document, t: &Tmp, o: &ResizeOptions) -> Document {
    let (src, out) = (t.path("in.pdf"), t.path("out.pdf"));
    doc.save(&src).unwrap();
    resize_file(&src, &out, o).unwrap();
    Document::load(&out).unwrap()
}

fn page_ids(doc: &Document) -> Vec<lopdf::ObjectId> {
    doc.get_pages().values().copied().collect()
}

fn rect(a: i64, b: i64, c: i64, d: i64) -> Object {
    vec![a.into(), b.into(), c.into(), d.into()].into()
}

#[test]
fn a_mixed_size_document_ends_up_uniform_and_keeps_its_content() {
    let t = Tmp::new("rs-mixed");
    // Letter, A3 portrait, a wide slide, a tiny card.
    let mut doc = make_sized_pdf("P", &[(612, 792), (842, 1191), (960, 540), (100, 150)]);
    let out = run(&mut doc, &t, &opts(Paper::A4, ResizeMode::Fit));

    for (i, b) in media_boxes(&out).iter().enumerate() {
        // Auto follows each page's own orientation: only the slide is landscape.
        let (w, h) = if i == 2 { (A4.1, A4.0) } else { A4 };
        assert!(approx(b[0], 0.0) && approx(b[1], 0.0), "{b:?}");
        assert!(approx(b[2], w) && approx(b[3], h), "page {}: {b:?}", i + 1);
    }
    // Original drawing instructions survive, in order.
    assert_eq!(markers(&out), ["P-1", "P-2", "P-3", "P-4"]);
    // Scaling is uniform and nothing is cut: the 2:3 card is height-limited on A4.
    let cm = first_cm(&out, page_ids(&out)[3]);
    assert!(approx(cm[0], cm[3]), "uniform scale: {cm:?}");
    assert!(approx(cm[0], A4.1 / 150.0), "{cm:?}");
}

#[test]
fn original_streams_are_untouched_and_wrapped_in_q() {
    let t = Tmp::new("rs-wrap");
    let mut doc = make_sized_pdf("P", &[(200, 300)]);
    let before = doc.get_page_content(page_ids(&doc)[0]);
    let out = run(&mut doc, &t, &opts(Paper::A4, ResizeMode::Fit));
    let page = page_ids(&out)[0];

    let contents = out
        .get_dictionary(page)
        .unwrap()
        .get(b"Contents")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(contents.len(), 3, "head, original, tail");
    let original = out
        .get_object(contents[1].as_reference().unwrap())
        .unwrap()
        .as_stream()
        .unwrap();
    assert_eq!(original.content, before.trim_ascii_end());
    let text = String::from_utf8_lossy(&out.get_page_content(page)).into_owned();
    assert!(text.starts_with("q\n") && text.trim_end().ends_with('Q'));
}

#[test]
fn only_the_selected_pages_change() {
    let t = Tmp::new("rs-select");
    let mut doc = make_sized_pdf("P", &[(200, 300); 4]);
    let mut o = opts(Paper::A4, ResizeMode::Fit);
    o.target = Target::Pages {
        text: "2, 4".into(),
    };
    let out = run(&mut doc, &t, &o);

    let boxes = media_boxes(&out);
    assert!(approx(boxes[0][2], 200.0) && approx(boxes[2][3], 300.0));
    assert!(approx(boxes[1][2], A4.0) && approx(boxes[3][3], A4.1));
    let first = out.get_page_content(page_ids(&out)[0]);
    assert!(!String::from_utf8_lossy(&first).contains("cm"));
}

#[test]
fn fill_crops_and_size_only_keeps_the_scale() {
    let t = Tmp::new("rs-modes");
    let mut doc = make_sized_pdf("P", &[(200, 300)]);

    let mut fill = opts(Paper::A4, ResizeMode::Fill);
    fill.margin_mm = 10.0;
    let out = run(&mut doc, &t, &fill);
    let text = String::from_utf8_lossy(&out.get_page_content(page_ids(&out)[0])).into_owned();
    assert!(text.contains("re W n"), "fill with a margin clips: {text}");

    let out = run(&mut doc, &t, &opts(Paper::A4, ResizeMode::Size));
    let cm = first_cm(&out, page_ids(&out)[0]);
    assert!(approx(cm[0], 1.0) && approx(cm[3], 1.0), "{cm:?}");
    assert!(
        approx(cm[4], (A4.0 - 200.0) / 2.0) && approx(cm[5], (A4.1 - 300.0) / 2.0),
        "{cm:?}"
    );

    let mut top_left = opts(Paper::A4, ResizeMode::Size);
    top_left.anchor = Anchor::TopLeft;
    let out = run(&mut doc, &t, &top_left);
    let cm = first_cm(&out, page_ids(&out)[0]);
    assert!(approx(cm[4], 0.0) && approx(cm[5], A4.1 - 300.0), "{cm:?}");
}

#[test]
fn rotation_is_kept_and_the_page_is_sized_in_the_unrotated_space() {
    let t = Tmp::new("rs-rotate");
    let mut doc = make_sized_pdf("P", &[(300, 200)]);
    let id = page_ids(&doc)[0];
    doc.get_dictionary_mut(id).unwrap().set("Rotate", 90);
    run(&mut doc, &t, &opts(Paper::A4, ResizeMode::Fit));

    // Shown as 200x300 portrait, so the target is portrait A4; the stored box
    // is turned on its side to match the kept /Rotate.
    let b = media_boxes(&Document::load(t.path("out.pdf")).unwrap())[0];
    assert!(approx(b[2], A4.1) && approx(b[3], A4.0), "{b:?}");
    let info = pdf_core::inspect(&t.path("out.pdf")).unwrap();
    assert_eq!(info.pages[0].rotate, 90);
    assert!(approx(info.pages[0].width, A4.1));
}

#[test]
fn crop_boxes_are_removed_even_when_inherited() {
    let t = Tmp::new("rs-crop");
    let mut doc = make_sized_pdf("P", &[(200, 300), (200, 300)]);
    let ids = page_ids(&doc);
    // Page 1 has its own CropBox and TrimBox; page 2 inherits one from the tree.
    let d = doc.get_dictionary_mut(ids[0]).unwrap();
    d.set("CropBox", rect(10, 10, 150, 150));
    d.set("TrimBox", rect(10, 10, 150, 150));
    let root = d.get(b"Parent").unwrap().as_reference().unwrap();
    doc.get_dictionary_mut(root)
        .unwrap()
        .set("CropBox", rect(10, 10, 120, 120));

    let out = run(&mut doc, &t, &opts(Paper::A4, ResizeMode::Fit));
    let info = pdf_core::inspect(&t.path("out.pdf")).unwrap();
    for p in &info.pages {
        assert!(approx(p.width, A4.0) && approx(p.height, A4.1), "{p:?}");
    }
    let first = out.get_dictionary(page_ids(&out)[0]).unwrap();
    assert!(!first.has(b"TrimBox"));
}

#[test]
fn annotations_move_with_the_content() {
    let t = Tmp::new("rs-annot");
    let mut doc = make_sized_pdf("P", &[(200, 300)]);
    let page = page_ids(&doc)[0];
    let link = doc.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Link", "Rect" => rect(10, 20, 110, 70),
    });
    let inline = dictionary! {
        "Type" => "Annot", "Subtype" => "Text", "Rect" => rect(0, 0, 20, 20),
    };
    doc.get_dictionary_mut(page).unwrap().set(
        "Annots",
        vec![Object::Reference(link), Object::Dictionary(inline)],
    );

    let out = run(&mut doc, &t, &opts(Paper::A4, ResizeMode::Fit));
    let page = page_ids(&out)[0];
    let cm = first_cm(&out, page);
    let (s, tx, ty) = (cm[0], cm[4], cm[5]);

    let annots = out
        .get_dictionary(page)
        .unwrap()
        .get(b"Annots")
        .unwrap()
        .as_array()
        .unwrap();
    let rect_of = |o: &Object| -> Vec<f64> {
        let d = match o {
            Object::Reference(id) => out.get_dictionary(*id).unwrap(),
            Object::Dictionary(d) => d,
            _ => panic!("unexpected annotation entry"),
        };
        d.get(b"Rect")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|n| f64::from(n.as_float().unwrap()))
            .collect()
    };
    let want = [10.0 * s + tx, 20.0 * s + ty, 110.0 * s + tx, 70.0 * s + ty];
    let r = rect_of(&annots[0]);
    assert!(r.iter().zip(want).all(|(g, w)| approx(*g, w)), "{r:?}");
    let r = rect_of(&annots[1]);
    assert!(approx(r[0], tx) && approx(r[3], 20.0 * s + ty), "{r:?}");
}

#[test]
fn an_annotation_shared_by_two_pages_is_moved_once() {
    let t = Tmp::new("rs-shared");
    let mut doc = make_sized_pdf("P", &[(200, 300), (200, 300)]);
    let shared = doc.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Link", "Rect" => rect(0, 0, 100, 100),
    });
    for id in page_ids(&doc) {
        doc.get_dictionary_mut(id)
            .unwrap()
            .set("Annots", vec![Object::Reference(shared)]);
    }
    let out = run(&mut doc, &t, &opts(Paper::A4, ResizeMode::Fit));
    let page = page_ids(&out)[0];
    let cm = first_cm(&out, page);
    let annots = out
        .get_dictionary(page)
        .unwrap()
        .get(b"Annots")
        .unwrap()
        .as_array()
        .unwrap();
    let d = out
        .get_dictionary(annots[0].as_reference().unwrap())
        .unwrap();
    let x1 = f64::from(
        d.get(b"Rect").unwrap().as_array().unwrap()[2]
            .as_float()
            .unwrap(),
    );
    assert!(approx(x1, 100.0 * cm[0] + cm[4]), "{x1}");
}

#[test]
fn invalid_requests_are_refused_and_the_source_is_safe() {
    let t = Tmp::new("rs-bad");
    let mut doc = make_sized_pdf("P", &[(200, 300)]);
    let (src, out) = (t.path("in.pdf"), t.path("out.pdf"));
    doc.save(&src).unwrap();
    let before = std::fs::read(&src).unwrap();

    let mut o = opts(Paper::A4, ResizeMode::Fit);
    assert!(matches!(
        resize_file(&src, &src, &o),
        Err(Error::OutputIsSource(_))
    ));
    assert_eq!(std::fs::read(&src).unwrap(), before);

    o.margin_mm = 150.0;
    assert!(matches!(resize_file(&src, &out, &o), Err(Error::BadMargin)));
    o.margin_mm = 0.0;
    o.paper = Paper::Custom {
        width_mm: 0.0,
        height_mm: 100.0,
    };
    assert!(matches!(
        resize_file(&src, &out, &o),
        Err(Error::BadPageSize)
    ));
    o.paper = Paper::A4;
    o.target = Target::Pages { text: "9".into() };
    assert!(resize_file(&src, &out, &o).is_err());
    assert!(!out.exists());
}
