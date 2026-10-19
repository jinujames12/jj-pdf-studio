//! Page numbers and watermarks on rotated, cropped and mixed-size pages.
//!
//! The tests do not trust the generated numbers: they read the drawing back,
//! run the points through the same matrices a viewer would (the stamp's own
//! `cm`s, then the page's `/Rotate` and box), and check where the stamp ends up
//! on the *displayed* page.

mod common;

use common::{jpeg_bytes, make_sized_pdf, Tmp};
use lopdf::{Dictionary, Document, Object, ObjectId};
use pdf_core::{
    stamp_file, stamp_preview, Corner, Error, Layer, PageNumberOptions, Place, Stamping,
    WatermarkKind, WatermarkOptions,
};

type Matrix = [f64; 6];

fn apply(m: Matrix, (x, y): (f64, f64)) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

fn num(o: &Object) -> f64 {
    o.as_float()
        .map_or_else(|_| o.as_i64().unwrap() as f64, f64::from)
}

fn page_ids(doc: &Document) -> Vec<ObjectId> {
    doc.get_pages().values().copied().collect()
}

/// Decoded content streams of a page, one entry per stream.
fn streams(doc: &Document, page: ObjectId) -> Vec<String> {
    let refs = match doc.get_dictionary(page).unwrap().get(b"Contents").unwrap() {
        Object::Array(a) => a.clone(),
        o => vec![o.clone()],
    };
    refs.iter()
        .map(|r| {
            let id = r.as_reference().unwrap();
            let s = doc.get_object(id).unwrap().as_stream().unwrap();
            String::from_utf8_lossy(
                &s.decompressed_content()
                    .unwrap_or_else(|_| s.content.clone()),
            )
            .into_owned()
        })
        .collect()
}

/// The six numbers on every line ending in `cm`.
fn matrices(drawing: &str) -> Vec<Matrix> {
    drawing
        .lines()
        .filter(|l| l.ends_with(" cm"))
        .map(|l| {
            let v: Vec<f64> = l
                .split_whitespace()
                .take(6)
                .map(|t| t.parse().unwrap())
                .collect();
            [v[0], v[1], v[2], v[3], v[4], v[5]]
        })
        .collect()
}

/// Displayed size and the displayed position of a point in page space.
struct Shown {
    width: f64,
    height: f64,
    rotate: i32,
    origin: (f64, f64),
}

impl Shown {
    fn of(doc: &Document, page: ObjectId) -> Shown {
        let d = doc.get_dictionary(page).unwrap();
        let b: Vec<f64> = d
            .get(b"CropBox")
            .or_else(|_| d.get(b"MediaBox"))
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(num)
            .collect();
        let rotate = d.get(b"Rotate").map_or(0, |o| num(o) as i32);
        let (uw, uh) = (b[2] - b[0], b[3] - b[1]);
        let (width, height) = if rotate % 180 == 0 {
            (uw, uh)
        } else {
            (uh, uw)
        };
        Shown {
            width,
            height,
            rotate,
            origin: (b[0], b[1]),
        }
    }

    /// Page-space point -> displayed point (the inverse of what /Rotate does).
    fn visual(&self, (x, y): (f64, f64)) -> (f64, f64) {
        let (x, y) = (x - self.origin.0, y - self.origin.1);
        let (uw, uh) = if self.rotate % 180 == 0 {
            (self.width, self.height)
        } else {
            (self.height, self.width)
        };
        match self.rotate {
            90 => (y, uw - x),
            180 => (uw - x, uh - y),
            270 => (uh - y, x),
            _ => (x, y),
        }
    }
}

/// Mixed sizes; page 2 is turned a quarter, page 3 half, page 4 three quarters
/// and page 5 has a CropBox that does not start at the origin.
fn awkward() -> Document {
    let mut doc = make_sized_pdf(
        "P",
        &[(612, 792), (842, 1191), (960, 540), (300, 500), (600, 800)],
    );
    let ids = page_ids(&doc);
    for (id, turn) in ids.iter().zip([0, 90, 180, 270, 0]) {
        if turn != 0 {
            doc.get_dictionary_mut(*id).unwrap().set("Rotate", turn);
        }
    }
    doc.get_dictionary_mut(ids[4]).unwrap().set(
        "CropBox",
        vec![50.into(), 100.into(), 450.into(), 700.into()],
    );
    doc
}

fn run(doc: &mut Document, t: &Tmp, s: &Stamping) -> Document {
    let (src, out) = (t.path("in.pdf"), t.path("out.pdf"));
    doc.save(&src).unwrap();
    stamp_file(&src, &out, s).unwrap();
    Document::load(&out).unwrap()
}

fn numbers(position: Corner) -> PageNumberOptions {
    PageNumberOptions {
        position,
        margin_mm: 10.0,
        font_size: 10.0,
        color: "#000000".into(),
        format: "{n}".into(),
        start: 1,
        pages: None,
    }
}

fn mm(v: f64) -> f64 {
    v * 72.0 / 25.4
}

/// Where the text origin of a page-number drawing lands on the displayed page.
fn text_origin(doc: &Document, page: ObjectId) -> (f64, f64) {
    let drawing = streams(doc, page).pop().unwrap();
    let to_page = matrices(&drawing)[0];
    let tm = drawing.lines().find(|l| l.ends_with(" Tm")).unwrap();
    let v: Vec<f64> = tm
        .split_whitespace()
        .take(6)
        .map(|t| t.parse().unwrap())
        .collect();
    Shown::of(doc, page).visual(apply(to_page, (v[4], v[5])))
}

#[test]
fn bottom_centre_numbers_sit_where_expected_on_every_kind_of_page() {
    let t = Tmp::new("pn-centre");
    let out = run(
        &mut awkward(),
        &t,
        &Stamping::PageNumbers(numbers(Corner::BottomCenter)),
    );
    let width_of_digit = 556.0 * 10.0 / 1000.0;
    for (i, &id) in page_ids(&out).iter().enumerate() {
        let shown = Shown::of(&out, id);
        let (x, y) = text_origin(&out, id);
        // Single digits for pages 1 to 5, all one digit wide.
        assert!(
            (x - (shown.width - width_of_digit) / 2.0).abs() < 0.01,
            "page {}: x = {x}, expected centred on {}",
            i + 1,
            shown.width
        );
        assert!(
            (y - (mm(10.0) + 0.207 * 10.0)).abs() < 0.01,
            "page {}: y = {y}",
            i + 1
        );
    }
}

#[test]
fn every_corner_lands_in_the_right_corner_of_the_displayed_page() {
    let t = Tmp::new("pn-corners");
    for (corner, right, top) in [
        (Corner::TopLeft, false, true),
        (Corner::TopRight, true, true),
        (Corner::BottomLeft, false, false),
        (Corner::BottomRight, true, false),
    ] {
        let out = run(&mut awkward(), &t, &Stamping::PageNumbers(numbers(corner)));
        for &id in &page_ids(&out) {
            let shown = Shown::of(&out, id);
            let (x, y) = text_origin(&out, id);
            assert_eq!(x > shown.width / 2.0, right, "{corner:?}: x = {x}");
            assert_eq!(y > shown.height / 2.0, top, "{corner:?}: y = {y}");
        }
    }
}

#[test]
fn text_is_drawn_upright_on_the_displayed_page() {
    // The text direction (1, 0) in stamp space must still point right on screen.
    let t = Tmp::new("pn-upright");
    let out = run(
        &mut awkward(),
        &t,
        &Stamping::PageNumbers(numbers(Corner::BottomRight)),
    );
    for &id in &page_ids(&out) {
        let drawing = streams(&out, id).pop().unwrap();
        let m = matrices(&drawing)[0];
        let shown = Shown::of(&out, id);
        let o = shown.visual(apply(m, (0.0, 0.0)));
        let right = shown.visual(apply(m, (10.0, 0.0)));
        let up = shown.visual(apply(m, (0.0, 10.0)));
        assert!((right.0 - o.0 - 10.0).abs() < 1e-6 && (right.1 - o.1).abs() < 1e-6);
        assert!((up.0 - o.0).abs() < 1e-6 && (up.1 - o.1 - 10.0).abs() < 1e-6);
    }
}

#[test]
fn right_aligned_text_ends_at_the_margin() {
    let t = Tmp::new("pn-right");
    let mut o = numbers(Corner::BottomRight);
    o.format = "Page {n} of {total}".into();
    let out = run(&mut awkward(), &t, &Stamping::PageNumbers(o));
    let text_width = [667, 556, 556, 556, 278, 556, 278, 556, 278, 278, 556]
        .iter()
        .sum::<i32>() as f64
        * 10.0
        / 1000.0;
    for &id in &page_ids(&out) {
        let shown = Shown::of(&out, id);
        let (x, _) = text_origin(&out, id);
        assert!((x + text_width - (shown.width - mm(10.0))).abs() < 0.01);
    }
    let drawing = streams(&out, page_ids(&out)[2]).pop().unwrap();
    assert!(drawing.contains("(Page 3 of 5) Tj"), "{drawing}");
}

#[test]
fn start_number_and_page_range_skip_the_cover() {
    let t = Tmp::new("pn-range");
    let mut o = numbers(Corner::BottomCenter);
    o.pages = Some("2-end".into());
    o.format = "{n}/{total}".into();
    let mut doc = awkward();
    let before: Vec<usize> = page_ids(&doc)
        .iter()
        .map(|&id| streams(&doc, id).len())
        .collect();
    let out = run(&mut doc, &t, &Stamping::PageNumbers(o));
    let ids = page_ids(&out);
    // The cover keeps its single stream; the others gained q, Q and the number.
    assert_eq!(streams(&out, ids[0]).len(), before[0]);
    for (n, &id) in ids.iter().enumerate().skip(1) {
        let s = streams(&out, id);
        assert_eq!(s.len(), before[n] + 3);
        assert!(
            s.last().unwrap().contains(&format!("({}/{}) Tj", n, 4)),
            "{s:?}"
        );
    }
}

#[test]
fn the_original_content_and_resources_survive() {
    let t = Tmp::new("pn-keep");
    let mut doc = awkward();
    // Fonts inherited from the page tree must still be there on the page.
    let pages_root = doc
        .catalog()
        .unwrap()
        .get(b"Pages")
        .unwrap()
        .as_reference()
        .unwrap();
    let font = doc.add_object(
        lopdf::dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Times-Roman" },
    );
    doc.get_dictionary_mut(pages_root).unwrap().set(
        "Resources",
        lopdf::dictionary! { "Font" => lopdf::dictionary! { "F1" => font } },
    );
    let out = run(
        &mut doc,
        &t,
        &Stamping::PageNumbers(numbers(Corner::TopLeft)),
    );
    for &id in &page_ids(&out) {
        let content = String::from_utf8_lossy(&out.get_page_content(id)).into_owned();
        assert!(
            content.contains("-1) Tj") || content.contains("P-"),
            "{content}"
        );
        let res = out
            .get_dictionary(id)
            .unwrap()
            .get(b"Resources")
            .unwrap()
            .as_dict()
            .unwrap();
        let fonts: &Dictionary = res.get(b"Font").unwrap().as_dict().unwrap();
        assert!(fonts.has(b"F1"), "inherited font lost");
        assert!(fonts.has(b"JJF1"), "stamp font missing");
    }
    // Stamping twice never reuses a name, so the first numbers stay intact.
    let again = run(
        &mut Document::load(t.path("out.pdf")).unwrap(),
        &t,
        &Stamping::PageNumbers(numbers(Corner::TopRight)),
    );
    let id = page_ids(&again)[0];
    let res = again
        .get_dictionary(id)
        .unwrap()
        .get(b"Resources")
        .unwrap()
        .as_dict()
        .unwrap();
    let fonts = res.get(b"Font").unwrap().as_dict().unwrap();
    assert!(fonts.has(b"JJF1") && fonts.has(b"JJF2"));
}

#[test]
fn bad_input_is_refused_in_plain_language() {
    let t = Tmp::new("pn-bad");
    let (src, out) = (t.path("in.pdf"), t.path("out.pdf"));
    awkward().save(&src).unwrap();
    let try_it = |f: &dyn Fn(&mut PageNumberOptions)| {
        let mut o = numbers(Corner::BottomCenter);
        f(&mut o);
        stamp_file(&src, &out, &Stamping::PageNumbers(o))
    };
    assert!(matches!(
        try_it(&|o| o.format = "第{n}页".into()),
        Err(Error::UnsupportedText(_))
    ));
    assert!(matches!(
        try_it(&|o| o.color = "blue".into()),
        Err(Error::BadOption(_))
    ));
    assert!(matches!(
        try_it(&|o| o.font_size = 0.0),
        Err(Error::BadOption(_))
    ));
    assert!(matches!(
        try_it(&|o| o.pages = Some("9".into())),
        Err(Error::PageOutOfRange(9))
    ));
    assert!(matches!(
        try_it(&|o| o.format = "  ".into()),
        Err(Error::BadOption(_))
    ));
    assert!(!out.exists(), "nothing is written on failure");
    // The source itself is never the output.
    assert!(matches!(
        stamp_file(&src, &src, &Stamping::PageNumbers(numbers(Corner::TopLeft))),
        Err(Error::OutputIsSource(_))
    ));
}

// Watermark --------------------------------------------------------------------

fn text_mark(place: Place, rotation: f64) -> WatermarkOptions {
    WatermarkOptions {
        kind: WatermarkKind::Text {
            text: "DRAFT".into(),
            font_size: 60.0,
            color: "#C2353A".into(),
        },
        opacity: 0.3,
        rotation,
        place,
        layer: Layer::Over,
        margin_mm: 10.0,
        pages: None,
    }
}

/// Displayed positions of the form-space point `p` for every copy of the stamp
/// in the page's watermark drawing (`over` = last stream, else first).
fn copies(doc: &Document, page: ObjectId, over: bool, p: (f64, f64)) -> Vec<(f64, f64)> {
    let s = streams(doc, page);
    let drawing = if over { s.last() } else { s.first() }.unwrap();
    let ms = matrices(drawing);
    let shown = Shown::of(doc, page);
    ms[1..]
        .iter()
        .map(|&m| shown.visual(apply(ms[0], apply(m, p))))
        .collect()
}

#[test]
fn a_centred_text_watermark_is_centred_on_every_page() {
    let t = Tmp::new("wm-centre");
    for angle in [0.0, 45.0, 90.0] {
        let out = run(
            &mut awkward(),
            &t,
            &Stamping::Watermark(text_mark(Place::Center, angle)),
        );
        // "DRAFT" is 722+722+667+611+611 = 3333 wide; cap height is 0.718 em.
        let centre = (3.333 * 60.0 / 2.0, 0.718 * 60.0 / 2.0);
        for &id in &page_ids(&out) {
            let shown = Shown::of(&out, id);
            let at = copies(&out, id, true, centre);
            assert_eq!(at.len(), 1);
            assert!(
                (at[0].0 - shown.width / 2.0).abs() < 0.01
                    && (at[0].1 - shown.height / 2.0).abs() < 0.01,
                "angle {angle}: {:?} vs {}x{}",
                at[0],
                shown.width,
                shown.height
            );
        }
    }
}

#[test]
fn rotation_turns_counter_clockwise_on_the_displayed_page() {
    let t = Tmp::new("wm-angle");
    let out = run(
        &mut awkward(),
        &t,
        &Stamping::Watermark(text_mark(Place::Center, 45.0)),
    );
    for &id in &page_ids(&out) {
        // The text's baseline direction, as shown, should rise at 45 degrees.
        let a = copies(&out, id, true, (0.0, 0.0))[0];
        let b = copies(&out, id, true, (100.0, 0.0))[0];
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        assert!(dx > 0.0 && dy > 0.0 && (dx - dy).abs() < 1e-6, "{dx} {dy}");
    }
}

#[test]
fn watermark_has_opacity_and_goes_under_or_over_the_content() {
    let t = Tmp::new("wm-layer");
    let mut under = text_mark(Place::Center, 45.0);
    under.layer = Layer::Under;
    let out = run(&mut awkward(), &t, &Stamping::Watermark(under));
    for &id in &page_ids(&out) {
        let s = streams(&out, id);
        assert!(
            s[0].contains(" gs\n") && s[0].contains(" Do\n"),
            "under: stamp first"
        );
        assert!(s.last().unwrap().contains("P-"), "page content last");
        let res = out
            .get_dictionary(id)
            .unwrap()
            .get(b"Resources")
            .unwrap()
            .as_dict()
            .unwrap();
        let gs_id = res
            .get(b"ExtGState")
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"JJGS1")
            .unwrap()
            .as_reference()
            .unwrap();
        let gs = out.get_dictionary(gs_id).unwrap();
        assert!((num(gs.get(b"ca").unwrap()) - 0.3).abs() < 1e-6);
    }
    let out = run(
        &mut awkward(),
        &t,
        &Stamping::Watermark(text_mark(Place::Center, 45.0)),
    );
    let s = streams(&out, page_ids(&out)[0]);
    assert!(s.last().unwrap().contains(" Do\n"), "over: stamp last");
    assert_eq!(s.first().unwrap(), "q\n");
}

#[test]
fn tiled_watermarks_cover_each_page_whatever_its_size() {
    let t = Tmp::new("wm-tiled");
    let mut o = text_mark(Place::Tiled, 30.0);
    if let WatermarkKind::Text { font_size, .. } = &mut o.kind {
        *font_size = 24.0;
    }
    let out = run(&mut awkward(), &t, &Stamping::Watermark(o));
    let mut previous = 0;
    for &id in &page_ids(&out) {
        let shown = Shown::of(&out, id);
        let at = copies(&out, id, true, (0.0, 0.0));
        assert!(at.len() >= 6, "{} tiles", at.len());
        // Tiles reach all four quadrants of the displayed page.
        let quadrants: std::collections::HashSet<_> = at
            .iter()
            .map(|&(x, y)| (x > shown.width / 2.0, y > shown.height / 2.0))
            .collect();
        assert_eq!(
            quadrants.len(),
            4,
            "page of {}x{}",
            shown.width,
            shown.height
        );
        previous = previous.max(at.len());
    }
    assert!(previous > 20, "the biggest page gets many tiles");
}

#[test]
fn corner_watermarks_stay_inside_the_margins_when_turned() {
    let t = Tmp::new("wm-corner");
    for place in [
        Place::TopLeft,
        Place::TopRight,
        Place::BottomLeft,
        Place::BottomRight,
    ] {
        let out = run(
            &mut awkward(),
            &t,
            &Stamping::Watermark(text_mark(place, 30.0)),
        );
        for &id in &page_ids(&out) {
            let shown = Shown::of(&out, id);
            // All four corners of the stamp box must be inside the page.
            for p in [(0.0, 0.0), (199.98, 0.0), (0.0, 43.08), (199.98, 43.08)] {
                let (x, y) = copies(&out, id, true, p)[0];
                assert!(
                    x >= mm(10.0) - 0.5
                        && x <= shown.width - mm(10.0) + 0.5
                        && y >= mm(10.0) - 0.5
                        && y <= shown.height - mm(10.0) + 0.5,
                    "{place:?} {p:?} -> ({x}, {y}) on {}x{}",
                    shown.width,
                    shown.height
                );
            }
        }
    }
}

#[test]
fn an_image_stamp_is_sized_relative_to_each_page() {
    let t = Tmp::new("wm-image");
    let logo = t.path("logo.jpg");
    std::fs::write(&logo, jpeg_bytes(200, 100, None)).unwrap();
    let o = WatermarkOptions {
        kind: WatermarkKind::Image {
            path: logo,
            width_percent: 25.0,
        },
        opacity: 0.5,
        rotation: 0.0,
        place: Place::BottomRight,
        layer: Layer::Over,
        margin_mm: 5.0,
        pages: Some("1-4".into()),
    };
    let mut doc = awkward();
    let untouched = streams(&doc, page_ids(&doc)[4]).len();
    let out = run(&mut doc, &t, &Stamping::Watermark(o));
    for &id in &page_ids(&out)[..4] {
        let shown = Shown::of(&out, id);
        let left = copies(&out, id, true, (0.0, 0.0))[0];
        let right = copies(&out, id, true, (200.0, 0.0))[0];
        let top = copies(&out, id, true, (0.0, 100.0))[0];
        assert!(
            ((right.0 - left.0) - 0.25 * shown.width).abs() < 0.01,
            "width"
        );
        assert!(
            ((top.1 - left.1) - 0.125 * shown.width).abs() < 0.01,
            "aspect kept"
        );
        assert!(
            (right.0 - (shown.width - mm(5.0))).abs() < 0.01,
            "right margin: right={right:?} left={left:?} shown={}x{} rot={}",
            shown.width,
            shown.height,
            shown.rotate
        );
        assert!((left.1 - mm(5.0)).abs() < 0.01, "bottom margin");
    }
    assert_eq!(
        streams(&out, page_ids(&out)[4]).len(),
        untouched,
        "page 5 outside the range"
    );
}

#[test]
fn bad_watermarks_are_refused() {
    let t = Tmp::new("wm-bad");
    let (src, out) = (t.path("in.pdf"), t.path("out.pdf"));
    awkward().save(&src).unwrap();
    let go = |f: &dyn Fn(&mut WatermarkOptions)| {
        let mut o = text_mark(Place::Center, 45.0);
        f(&mut o);
        stamp_file(&src, &out, &Stamping::Watermark(o))
    };
    assert!(matches!(go(&|o| o.opacity = 1.5), Err(Error::BadOption(_))));
    assert!(matches!(
        go(&|o| o.rotation = f64::NAN),
        Err(Error::BadOption(_))
    ));
    assert!(matches!(
        go(&|o| o.kind = WatermarkKind::Text {
            text: "".into(),
            font_size: 20.0,
            color: "#000000".into()
        }),
        Err(Error::BadOption(_))
    ));
    assert!(matches!(
        go(&|o| o.kind = WatermarkKind::Image {
            path: t.path("missing.png"),
            width_percent: 20.0
        }),
        Err(Error::Io(_))
    ));
    assert!(!out.exists());
}

#[test]
fn the_preview_is_a_single_stamped_page() {
    let t = Tmp::new("preview");
    let src = t.path("in.pdf");
    awkward().save(&src).unwrap();
    let s = Stamping::PageNumbers(numbers(Corner::BottomCenter));
    let bytes = stamp_preview(&src, &s, 2).unwrap();
    let doc = Document::load_mem(&bytes).unwrap();
    assert_eq!(doc.get_pages().len(), 1);
    let id = page_ids(&doc)[0];
    assert_eq!(Shown::of(&doc, id).rotate, 90, "keeps its rotation");
    assert!(streams(&doc, id).last().unwrap().contains("(2) Tj"));
    assert!(matches!(
        stamp_preview(&src, &s, 6),
        Err(Error::PageOutOfRange(6))
    ));
}

#[test]
fn stamping_json_from_the_ui_deserialises() {
    let json = r##"{"tool":"watermark","kind":"text","text":"X","font_size":40,"color":"#000000",
        "opacity":0.5,"rotation":45,"place":"top-left","layer":"under","margin_mm":5,"pages":null}"##;
    let s: Stamping = serde_json::from_str(json).unwrap();
    assert!(matches!(
        s,
        Stamping::Watermark(WatermarkOptions {
            place: Place::TopLeft,
            layer: Layer::Under,
            ..
        })
    ));
    let json = r##"{"tool":"page_numbers","position":"bottom-right","margin_mm":10,"font_size":10,
        "color":"#112233","format":"Page {n}","start":1}"##;
    assert!(matches!(
        serde_json::from_str::<Stamping>(json).unwrap(),
        Stamping::PageNumbers(_)
    ));
}

#[test]
fn a_tiny_tiled_stamp_on_a_huge_page_is_refused() {
    let t = Tmp::new("wm-huge");
    let (src, out) = (t.path("in.pdf"), t.path("out.pdf"));
    make_sized_pdf("P", &[(14400, 14400)]).save(&src).unwrap();
    let mut o = text_mark(Place::Tiled, 0.0);
    o.kind = WatermarkKind::Text {
        text: ".".into(),
        font_size: 1.0,
        color: "#000000".into(),
    };
    assert!(matches!(
        stamp_file(&src, &out, &Stamping::Watermark(o)),
        Err(Error::BadOption(_))
    ));
    assert!(!out.exists());
}

/// Writes stamped samples of the awkward document to `$JJ_SAMPLES`, for checking
/// with a real PDF reader: `JJ_SAMPLES=dir cargo test -p pdf-core --test stamp -- --ignored`.
#[test]
#[ignore]
fn write_samples() {
    let dir = std::path::PathBuf::from(std::env::var("JJ_SAMPLES").expect("set JJ_SAMPLES"));
    let src = dir.join("awkward.pdf");
    awkward().save(&src).unwrap();
    let mut o = numbers(Corner::BottomCenter);
    o.format = "Page {n} of {total}".into();
    stamp_file(&src, &dir.join("numbers.pdf"), &Stamping::PageNumbers(o)).unwrap();
    for (name, place) in [("wm_centre", Place::Center), ("wm_tiled", Place::Tiled)] {
        let mut w = text_mark(place, 45.0);
        w.layer = Layer::Under;
        stamp_file(
            &src,
            &dir.join(format!("{name}.pdf")),
            &Stamping::Watermark(w),
        )
        .unwrap();
    }
}
