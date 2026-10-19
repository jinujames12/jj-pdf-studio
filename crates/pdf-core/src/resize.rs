//! Resize pages (SPEC 5.6).
//!
//! The page's own content streams are never edited. Each resized page gets a
//! small stream before them (`q`, optional clip, `s 0 0 s tx ty cm`) and one
//! after them (`Q`), a new `/MediaBox`, and its annotations are moved by the
//! same scale and offset. The page keeps its `/Rotate`.

use crate::progress::{step, Progress, Silent};
use std::collections::HashSet;
use std::path::Path;

use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream};
use serde::Deserialize;

use crate::assemble::{same_file, write_atomic};
use crate::document::{self, effective_rotate, inherited, page_box, PageInfo};
use crate::error::{Error, Result};
use crate::paper::{margin_pt, PageOrientation, Paper};
use crate::rotate::Target;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResizeMode {
    /// Scale uniformly to fit inside the target; never distorts or crops.
    #[default]
    Fit,
    /// Scale uniformly to cover the target; the overflow is cropped.
    Fill,
    /// Change the page size only; the content keeps its size.
    Size,
}

/// Where unscaled content sits on the new page (only used by `Size` mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    #[default]
    Center,
    TopLeft,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ResizeOptions {
    pub paper: Paper,
    #[serde(default)]
    pub orientation: PageOrientation,
    #[serde(default)]
    pub mode: ResizeMode,
    #[serde(default)]
    pub anchor: Anchor,
    #[serde(default)]
    pub margin_mm: f64,
    pub target: Target,
}

/// New on-screen size `(width, height)` in points for each page the options
/// touch, `None` for pages they leave alone. Pages are the document's pages in
/// order; `PageInfo` is their size before rotation.
pub fn plan_resize(pages: &[PageInfo], opts: &ResizeOptions) -> Result<Vec<Option<(f64, f64)>>> {
    let base = opts.paper.portrait_pt()?;
    let margin = margin_pt(opts.margin_mm)?;
    if base.0 <= 2.0 * margin || base.1 <= 2.0 * margin {
        return Err(Error::BadMargin);
    }
    let hit = opts.target.matches(pages.len() as u32)?;
    Ok(pages
        .iter()
        .zip(hit)
        .map(|(page, hit)| {
            hit.then(|| {
                let sideways = page.rotate % 180 != 0;
                let (w, h) = if sideways {
                    (page.height, page.width)
                } else {
                    (page.width, page.height)
                };
                opts.orientation.orient(base, w > h)
            })
        })
        .collect())
}

/// Visual point `(x, y)` on a page turned by `rotate` degrees, back to the
/// page's own unrotated coordinates. `(uw, uh)` is the unrotated page size.
fn unrotate(rotate: i32, (uw, uh): (f64, f64), (x, y): (f64, f64)) -> (f64, f64) {
    match rotate {
        90 => (uw - y, x),
        180 => (uw - x, uh - y),
        270 => (y, uh - x),
        _ => (x, y),
    }
}

/// The scale and offset applied to one page's content.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Placement {
    scale: f64,
    tx: f64,
    ty: f64,
    /// New unrotated page size.
    width: f64,
    height: f64,
    /// Rectangle (x, y, w, h) to crop to, in the new unrotated space.
    clip: Option<(f64, f64, f64, f64)>,
}

/// Works out the placement of the box `src` (`[x0, y0, x1, y1]`) of a page
/// turned by `rotate`, on a new page of visual size `target`.
fn place(
    src: [f64; 4],
    rotate: i32,
    target: (f64, f64),
    opts: &ResizeOptions,
    margin: f64,
) -> Placement {
    let (sw, sh) = (src[2] - src[0], src[3] - src[1]);
    let sideways = rotate % 180 != 0;
    let (vw, vh) = if sideways { (sh, sw) } else { (sw, sh) };
    let (tw, th) = target;
    let (cw, ch) = (tw - 2.0 * margin, th - 2.0 * margin);
    let scale = match opts.mode {
        ResizeMode::Fit => (cw / vw).min(ch / vh),
        ResizeMode::Fill => (cw / vw).max(ch / vh),
        ResizeMode::Size => 1.0,
    };
    let (fw, fh) = (vw * scale, vh * scale);
    // Lower-left corner of the scaled content in the visual page.
    let (ox, oy) = if opts.mode == ResizeMode::Size && opts.anchor == Anchor::TopLeft {
        (margin, th - margin - fh)
    } else {
        (margin + (cw - fw) / 2.0, margin + (ch - fh) / 2.0)
    };
    let (uw, uh) = if sideways { (th, tw) } else { (tw, th) };
    let a = unrotate(rotate, (uw, uh), (ox, oy));
    let b = unrotate(rotate, (uw, uh), (ox + fw, oy + fh));
    let (ux, uy) = (a.0.min(b.0), a.1.min(b.1));
    Placement {
        scale,
        tx: ux - src[0] * scale,
        ty: uy - src[1] * scale,
        width: uw,
        height: uh,
        // Margins are the same on every side, rotated or not.
        clip: (opts.mode == ResizeMode::Fill && margin > 0.0).then_some((
            margin,
            margin,
            uw - 2.0 * margin,
            uh - 2.0 * margin,
        )),
    }
}

fn num(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() { "0" } else { s }.to_string()
}

fn number(o: &Object) -> Option<f64> {
    match o {
        Object::Integer(i) => Some(*i as f64),
        Object::Real(r) => Some(f64::from(*r)),
        _ => None,
    }
}

/// Moves every x,y pair in `o` (a flat list of coordinates, or lists of them,
/// as in `/Rect`, `/QuadPoints`, `/L` and `/InkList`).
fn map_points(o: &mut Object, p: &Placement) {
    let Object::Array(items) = o else { return };
    if items.iter().all(|i| matches!(i, Object::Array(_))) {
        items.iter_mut().for_each(|i| map_points(i, p));
        return;
    }
    let values: Option<Vec<f64>> = items.iter().map(number).collect();
    let Some(values) = values.filter(|v| v.len() % 2 == 0) else {
        return;
    };
    *items = values
        .chunks(2)
        .flat_map(|xy| {
            [
                Object::Real((xy[0] * p.scale + p.tx) as f32),
                Object::Real((xy[1] * p.scale + p.ty) as f32),
            ]
        })
        .collect();
}

fn move_annotation(annot: &mut Dictionary, p: &Placement) {
    for key in [
        &b"Rect"[..],
        b"QuadPoints",
        b"L",
        b"CL",
        b"Vertices",
        b"InkList",
    ] {
        if let Ok(o) = annot.get_mut(key) {
            map_points(o, p);
        }
    }
    // A moved /Rect must stay normalised (lower-left first).
    if let Ok(Object::Array(r)) = annot.get_mut(b"Rect") {
        if let [Some(x0), Some(y0), Some(x1), Some(y1)] =
            [0, 1, 2, 3].map(|i| r.get(i).and_then(number))
        {
            *r = vec![
                Object::Real(x0.min(x1) as f32),
                Object::Real(y0.min(y1) as f32),
                Object::Real(x0.max(x1) as f32),
                Object::Real(y0.max(y1) as f32),
            ];
        }
    }
    // Rectangle differences (square and circle annotations) scale with the page.
    if let Ok(Object::Array(rd)) = annot.get_mut(b"RD") {
        for v in rd.iter_mut() {
            if let Some(n) = number(v) {
                *v = Object::Real((n * p.scale) as f32);
            }
        }
    }
}

/// Annotation objects of a page, as references, plus whether the list is a
/// direct array on the page (whose inline dictionaries are handled separately).
fn annotation_list(doc: &Document, page: ObjectId) -> Vec<Object> {
    let Ok(dict) = doc.get_dictionary(page) else {
        return Vec::new();
    };
    match dict.get(b"Annots") {
        Ok(Object::Array(a)) => a.clone(),
        Ok(Object::Reference(id)) => doc
            .get_object(*id)
            .and_then(Object::as_array)
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn move_annotations(
    doc: &mut Document,
    page: ObjectId,
    p: &Placement,
    seen: &mut HashSet<ObjectId>,
) {
    // Inline dictionaries live in the array itself, so rewrite it after.
    let mut list = annotation_list(doc, page);
    let mut inline_changed = false;
    for item in &mut list {
        match item {
            Object::Reference(id) => {
                if seen.insert(*id) {
                    if let Ok(d) = doc.get_dictionary_mut(*id) {
                        move_annotation(d, p);
                    }
                }
            }
            Object::Dictionary(d) => {
                move_annotation(d, p);
                inline_changed = true;
            }
            _ => {}
        }
    }
    if !inline_changed {
        return;
    }
    let Ok(dict) = doc.get_dictionary(page) else {
        return;
    };
    match dict.get(b"Annots") {
        Ok(Object::Reference(id)) => {
            let id = *id;
            if let Ok(Object::Array(a)) = doc.get_object_mut(id) {
                *a = list;
            }
        }
        _ => {
            if let Ok(d) = doc.get_dictionary_mut(page) {
                d.set("Annots", list);
            }
        }
    }
}

/// The page's content streams as a list of references.
pub(crate) fn content_refs(doc: &Document, page: ObjectId) -> Vec<Object> {
    let Ok(dict) = doc.get_dictionary(page) else {
        return Vec::new();
    };
    match dict.get(b"Contents") {
        Ok(Object::Array(a)) => a.clone(),
        Ok(Object::Reference(id)) => match doc.get_object(*id) {
            Ok(Object::Array(a)) => a.clone(),
            Ok(_) => vec![Object::Reference(*id)],
            Err(_) => Vec::new(),
        },
        _ => Vec::new(),
    }
}

fn resize_page(
    doc: &mut Document,
    page: ObjectId,
    target: (f64, f64),
    opts: &ResizeOptions,
    margin: f64,
    seen: &mut HashSet<ObjectId>,
) -> Result<()> {
    let src = page_box(doc, page);
    let rotate = effective_rotate(doc, page);
    let p = place(src, rotate, target, opts, margin);

    let mut head = String::from("q\n");
    if let Some((x, y, w, h)) = p.clip {
        head += &format!("{} {} {} {} re W n\n", num(x), num(y), num(w), num(h));
    }
    head += &format!(
        "{s} 0 0 {s} {} {} cm\n",
        num(p.tx),
        num(p.ty),
        s = num(p.scale)
    );
    let mut contents = vec![Object::Reference(
        doc.add_object(Stream::new(dictionary! {}, head.into_bytes())),
    )];
    contents.extend(content_refs(doc, page));
    contents.push(Object::Reference(
        doc.add_object(Stream::new(dictionary! {}, b"\nQ\n".to_vec())),
    ));

    move_annotations(doc, page, &p, seen);

    let media = vec![
        Object::Integer(0),
        Object::Integer(0),
        Object::Real(p.width as f32),
        Object::Real(p.height as f32),
    ];
    let dict = doc.get_dictionary_mut(page)?;
    dict.set("Contents", contents);
    dict.set("MediaBox", media.clone());
    for key in [&b"CropBox"[..], b"BleedBox", b"TrimBox", b"ArtBox"] {
        dict.remove(key);
    }
    // A CropBox inherited from the page tree would still crop the new page.
    if inherited(doc, page, b"CropBox").is_some() {
        doc.get_dictionary_mut(page)?.set("CropBox", media);
    }
    Ok(())
}

/// Resizes the pages selected by `opts` in place.
pub fn resize(doc: &mut Document, opts: &ResizeOptions) -> Result<()> {
    let ids: Vec<ObjectId> = doc.get_pages().values().copied().collect();
    let infos: Vec<PageInfo> = ids.iter().map(|&id| document::page_info(doc, id)).collect();
    let sizes = plan_resize(&infos, opts)?;
    let margin = margin_pt(opts.margin_mm)?;
    let mut seen = HashSet::new();
    for (id, size) in ids.into_iter().zip(sizes) {
        if let Some(target) = size {
            resize_page(doc, id, target, opts, margin, &mut seen)?;
        }
    }
    Ok(())
}

/// Loads `source`, resizes it and writes the result to `output`, which must
/// not be the source file. Returns the size of the written file in bytes.
pub fn resize_file(source: &Path, output: &Path, opts: &ResizeOptions) -> Result<u64> {
    resize_file_with(source, output, opts, &Silent)
}

/// Like [`resize_file`], reporting progress and stopping if cancelled.
pub fn resize_file_with(
    source: &Path,
    output: &Path,
    opts: &ResizeOptions,
    progress: &dyn Progress,
) -> Result<u64> {
    if same_file(source, output) {
        return Err(Error::OutputIsSource(output.to_path_buf()));
    }
    step(progress, 0, 3, "Reading the document")?;
    let mut doc = document::load(source)?;
    step(progress, 1, 3, "Resizing pages")?;
    resize(&mut doc, opts)?;
    step(progress, 2, 3, "Saving")?;
    write_atomic(&mut doc, output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(mode: ResizeMode) -> ResizeOptions {
        ResizeOptions {
            paper: Paper::Custom {
                width_mm: 100.0,
                height_mm: 200.0,
            },
            orientation: PageOrientation::Portrait,
            mode,
            anchor: Anchor::Center,
            margin_mm: 0.0,
            target: Target::All,
        }
    }

    const TARGET: (f64, f64) = (200.0, 400.0);

    /// Maps the corners of the page's box through the placement and returns
    /// the covered rectangle `[x0, y0, x1, y1]` (unrotated space).
    fn covered(src: [f64; 4], p: &Placement) -> [f64; 4] {
        [
            src[0] * p.scale + p.tx,
            src[1] * p.scale + p.ty,
            src[2] * p.scale + p.tx,
            src[3] * p.scale + p.ty,
        ]
    }

    #[test]
    fn fit_scales_uniformly_and_centres() {
        // A 400x200 landscape page onto a 200x400 portrait target.
        let src = [0.0, 0.0, 400.0, 200.0];
        let p = place(src, 0, TARGET, &opts(ResizeMode::Fit), 0.0);
        assert_eq!(p.scale, 0.5);
        assert_eq!((p.width, p.height), TARGET);
        assert_eq!(covered(src, &p), [0.0, 150.0, 200.0, 250.0]);
        assert!(p.clip.is_none());
    }

    #[test]
    fn fill_covers_the_target_and_clips_inside_margins() {
        let src = [0.0, 0.0, 400.0, 200.0];
        let mut o = opts(ResizeMode::Fill);
        let p = place(src, 0, TARGET, &o, 0.0);
        assert_eq!(p.scale, 2.0);
        let c = covered(src, &p);
        assert!(c[0] <= 0.0 && c[2] >= 200.0 && c[1] <= 0.0 && c[3] >= 400.0);
        o.margin_mm = 10.0;
        let m = 10.0;
        let p = place(src, 0, TARGET, &o, m);
        assert_eq!(p.clip, Some((m, m, 200.0 - 2.0 * m, 400.0 - 2.0 * m)));
    }

    #[test]
    fn size_only_keeps_scale_and_can_anchor_top_left() {
        let src = [0.0, 0.0, 100.0, 100.0];
        let p = place(src, 0, TARGET, &opts(ResizeMode::Size), 0.0);
        assert_eq!(p.scale, 1.0);
        assert_eq!(covered(src, &p), [50.0, 150.0, 150.0, 250.0]);
        let mut o = opts(ResizeMode::Size);
        o.anchor = Anchor::TopLeft;
        let p = place(src, 0, TARGET, &o, 0.0);
        assert_eq!(covered(src, &p), [0.0, 300.0, 100.0, 400.0]);
    }

    #[test]
    fn a_box_with_an_offset_origin_lands_inside_the_page() {
        let src = [100.0, 50.0, 300.0, 450.0]; // 200 x 400, not at the origin
        let p = place(src, 0, TARGET, &opts(ResizeMode::Fit), 0.0);
        assert_eq!(p.scale, 1.0);
        assert_eq!(covered(src, &p), [0.0, 0.0, 200.0, 400.0]);
    }

    #[test]
    fn rotated_pages_keep_their_rotation_and_fit_visually() {
        // Unrotated 400x200 with /Rotate 90 shows as 200x400: it already
        // matches the target, so nothing is scaled.
        let src = [0.0, 0.0, 400.0, 200.0];
        let p = place(src, 90, TARGET, &opts(ResizeMode::Fit), 0.0);
        assert_eq!(p.scale, 1.0);
        // The unrotated page is the target turned on its side.
        assert_eq!((p.width, p.height), (400.0, 200.0));
        assert_eq!(covered(src, &p), [0.0, 0.0, 400.0, 200.0]);
    }

    #[test]
    fn top_left_anchor_follows_rotation() {
        // With /Rotate 90 the visual top-left is the unrotated top-left too
        // (the page turns clockwise: its top-left corner is the visual top-right
        // of the unrotated page's bottom-left).
        let src = [0.0, 0.0, 50.0, 100.0];
        let mut o = opts(ResizeMode::Size);
        o.anchor = Anchor::TopLeft;
        let p = place(src, 90, TARGET, &o, 0.0);
        // Visual content: 100 wide, 50 tall at the top-left => visual box
        // x 0..100, y 350..400. Unrotating by 90 (x = uw - Y, y = X) with
        // uw = 400: x 0..50, y 0..100.
        assert_eq!(covered(src, &p), [0.0, 0.0, 50.0, 100.0]);
    }

    #[test]
    fn plan_applies_orientation_and_target_pages() {
        let pages = [
            PageInfo {
                width: 100.0,
                height: 200.0,
                rotate: 0,
            },
            PageInfo {
                width: 200.0,
                height: 100.0,
                rotate: 0,
            },
            PageInfo {
                width: 100.0,
                height: 200.0,
                rotate: 90,
            },
        ];
        let mut o = opts(ResizeMode::Fit);
        o.orientation = PageOrientation::Auto;
        o.paper = Paper::A4;
        let plan = plan_resize(&pages, &o).unwrap();
        let sizes: Vec<_> = plan.iter().map(|s| s.unwrap()).collect();
        assert!(sizes[0].0 < sizes[0].1);
        assert!(sizes[1].0 > sizes[1].1);
        assert!(sizes[2].0 > sizes[2].1, "turned page is shown landscape");
        o.target = Target::Pages { text: "2".into() };
        let plan = plan_resize(&pages, &o).unwrap();
        assert!(plan[0].is_none() && plan[1].is_some() && plan[2].is_none());
        o.margin_mm = 500.0;
        assert!(matches!(plan_resize(&pages, &o), Err(Error::BadMargin)));
    }

    #[test]
    fn points_and_rects_move_with_the_content() {
        let p = Placement {
            scale: 0.5,
            tx: 10.0,
            ty: 20.0,
            width: 0.0,
            height: 0.0,
            clip: None,
        };
        let mut annot = dictionary! {
            "Rect" => vec![100.into(), 200.into(), 50.into(), 40.into()],
            "QuadPoints" => vec![0.into(), 0.into(), 10.into(), 0.into(), 0.into(), 10.into(), 10.into(), 10.into()],
        };
        move_annotation(&mut annot, &p);
        let rect: Vec<f64> = annot
            .get(b"Rect")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|o| number(o).unwrap())
            .collect();
        assert_eq!(rect, [35.0, 40.0, 60.0, 120.0]);
        let quad = annot.get(b"QuadPoints").unwrap().as_array().unwrap();
        assert_eq!(number(&quad[2]), Some(15.0));
        assert_eq!(number(&quad[7]), Some(25.0));
    }
}
