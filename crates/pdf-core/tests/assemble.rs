//! Round-trip tests for the Pages tool: build a PDF, apply an operation list,
//! save, reload and check the result.

use std::path::{Path, PathBuf};

use lopdf::{dictionary, Document, Object, Stream};
use pdf_core::{inspect, save_pages, Error, PageOp};

/// Scratch directory that is removed on drop.
struct Tmp(PathBuf);

impl Tmp {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("jjpdf-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
    fn path(&self, file: &str) -> PathBuf {
        self.0.join(file)
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Builds an `n`-page PDF. Page `i` draws the marker text "PAGE-i" so tests can
/// tell pages apart after reordering. When `inherit` is set, MediaBox and Rotate
/// live on the page-tree root instead of on each page.
fn make_pdf(n: u32, inherit: bool) -> Document {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut kids = Vec::new();
    for i in 1..=n {
        let content = doc.add_object(Stream::new(
            dictionary! {},
            format!("BT /F1 12 Tf 10 10 Td (PAGE-{i}) Tj ET").into_bytes(),
        ));
        let mut page = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content,
        };
        if !inherit {
            page.set("MediaBox", vec![0.into(), 0.into(), 200.into(), 300.into()]);
        }
        kids.push(Object::Reference(doc.add_object(page)));
    }
    let mut pages = dictionary! {
        "Type" => "Pages",
        "Kids" => kids,
        "Count" => i64::from(n),
        "Resources" => dictionary! {},
    };
    if inherit {
        pages.set("MediaBox", vec![0.into(), 0.into(), 400.into(), 500.into()]);
        pages.set("Rotate", 90);
    }
    doc.objects.insert(pages_id, Object::Dictionary(pages));
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    doc
}

fn write(doc: &mut Document, path: &Path) {
    doc.save(path).unwrap();
}

/// Marker text of each page, in document order.
fn markers(doc: &Document) -> Vec<String> {
    doc.get_pages()
        .values()
        .map(|&id| {
            let bytes = doc.get_page_content(id);
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let start = text.find("(PAGE-").map_or(0, |p| p + 1);
            text[start..].split(')').next().unwrap().to_string()
        })
        .collect()
}

fn page(page: u32, rotate: i32) -> PageOp {
    PageOp::Page { page, rotate }
}

fn run(src: &Path, out: &Path, ops: &[PageOp]) -> Document {
    save_pages(src, out, ops).unwrap();
    Document::load(out).unwrap()
}

#[test]
fn reorder_rotate_and_delete_round_trip() {
    let t = Tmp::new("reorder");
    let (src, out) = (t.path("in.pdf"), t.path("out.pdf"));
    write(&mut make_pdf(4, false), &src);

    let result = run(&src, &out, &[page(4, 0), page(2, 90), page(1, -90)]);
    assert_eq!(markers(&result), ["PAGE-4", "PAGE-2", "PAGE-1"]);
    let info = inspect(&out).unwrap();
    let rotations: Vec<_> = info.pages.iter().map(|p| p.rotate).collect();
    assert_eq!(rotations, [0, 90, 270]);
    assert_eq!(result.get_pages().len(), 3);
}

#[test]
fn deleted_page_content_is_removed_from_the_file() {
    let t = Tmp::new("prune");
    let (src, out) = (t.path("in.pdf"), t.path("out.pdf"));
    write(&mut make_pdf(3, false), &src);

    run(&src, &out, &[page(1, 0), page(3, 0)]);
    let bytes = std::fs::read(&out).unwrap();
    assert!(!bytes.windows(6).any(|w| w == b"PAGE-2"));
}

#[test]
fn rotation_accumulates_on_top_of_existing_rotate() {
    let t = Tmp::new("rotate");
    let (src, out) = (t.path("in.pdf"), t.path("out.pdf"));
    write(&mut make_pdf(1, false), &src);
    let mid = t.path("mid.pdf");
    run(&src, &mid, &[page(1, 90)]);
    let result = run(&mid, &out, &[page(1, 90)]);
    assert_eq!(inspect(&out).unwrap().pages[0].rotate, 180);
    // A full turn removes /Rotate entirely.
    let full = t.path("full.pdf");
    run(&out, &full, &[page(1, 180)]);
    assert_eq!(inspect(&full).unwrap().pages[0].rotate, 0);
    drop(result);
}

#[test]
fn inherited_size_and_rotation_survive_reordering() {
    let t = Tmp::new("inherit");
    let (src, out) = (t.path("in.pdf"), t.path("out.pdf"));
    write(&mut make_pdf(3, true), &src);

    run(&src, &out, &[page(3, 0), page(1, 90)]);
    let info = inspect(&out).unwrap();
    assert_eq!((info.pages[0].width, info.pages[0].height), (400.0, 500.0));
    assert_eq!(info.pages[0].rotate, 90);
    assert_eq!(info.pages[1].rotate, 180);
}

#[test]
fn blank_page_is_inserted_with_requested_size() {
    let t = Tmp::new("blank");
    let (src, out) = (t.path("in.pdf"), t.path("out.pdf"));
    write(&mut make_pdf(2, false), &src);

    let blank = PageOp::Blank {
        width: 595.0,
        height: 842.0,
        rotate: 90,
    };
    let result = run(&src, &out, &[page(1, 0), blank, page(2, 0)]);
    let info = inspect(&out).unwrap();
    assert_eq!(info.pages.len(), 3);
    assert_eq!((info.pages[1].width, info.pages[1].height), (595.0, 842.0));
    assert_eq!(info.pages[1].rotate, 90);
    assert_eq!(markers(&result)[0], "PAGE-1");
    assert_eq!(markers(&result)[2], "PAGE-2");
}

#[test]
fn a_page_listed_twice_is_copied() {
    let t = Tmp::new("dup");
    let (src, out) = (t.path("in.pdf"), t.path("out.pdf"));
    write(&mut make_pdf(2, false), &src);

    let result = run(&src, &out, &[page(2, 0), page(2, 90), page(1, 0)]);
    assert_eq!(markers(&result), ["PAGE-2", "PAGE-2", "PAGE-1"]);
    let rotations: Vec<_> = inspect(&out)
        .unwrap()
        .pages
        .iter()
        .map(|p| p.rotate)
        .collect();
    assert_eq!(rotations, [0, 90, 0]);
}

#[test]
fn outlines_are_kept_on_pure_reorder_but_dropped_when_pages_are_removed() {
    let t = Tmp::new("outline");
    let (src, keep, drop_) = (t.path("in.pdf"), t.path("keep.pdf"), t.path("drop.pdf"));
    let mut doc = make_pdf(2, false);
    let outlines = doc.add_object(dictionary! { "Type" => "Outlines", "Count" => 0 });
    let root = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
    doc.get_dictionary_mut(root)
        .unwrap()
        .set("Outlines", outlines);
    write(&mut doc, &src);

    let kept = run(&src, &keep, &[page(2, 0), page(1, 0)]);
    assert!(kept.catalog().unwrap().has(b"Outlines"));
    let dropped = run(&src, &drop_, &[page(1, 0)]);
    assert!(!dropped.catalog().unwrap().has(b"Outlines"));
}

#[test]
fn invalid_operation_lists_are_rejected_and_write_nothing() {
    let t = Tmp::new("invalid");
    let (src, out) = (t.path("in.pdf"), t.path("out.pdf"));
    write(&mut make_pdf(2, false), &src);

    assert!(matches!(save_pages(&src, &out, &[]), Err(Error::NoPages)));
    assert!(matches!(
        save_pages(&src, &out, &[page(3, 0)]),
        Err(Error::PageOutOfRange(3))
    ));
    assert!(matches!(
        save_pages(&src, &out, &[page(1, 45)]),
        Err(Error::BadRotation(45))
    ));
    let bad_blank = PageOp::Blank {
        width: 0.0,
        height: 10.0,
        rotate: 0,
    };
    assert!(matches!(
        save_pages(&src, &out, &[bad_blank]),
        Err(Error::BadBlankSize)
    ));
    assert!(!out.exists());
}

#[test]
fn saving_over_the_source_is_refused_and_the_original_is_untouched() {
    let t = Tmp::new("protect");
    let src = t.path("in.pdf");
    write(&mut make_pdf(2, false), &src);
    let before = std::fs::read(&src).unwrap();

    assert!(matches!(
        save_pages(&src, &src, &[page(1, 0)]),
        Err(Error::OutputIsSource(_))
    ));
    // Same file spelled differently (case differs, plus a `.` component).
    let alias = t.0.join(".").join("IN.PDF");
    assert!(matches!(
        save_pages(&src, &alias, &[page(1, 0)]),
        Err(Error::OutputIsSource(_))
    ));
    assert_eq!(std::fs::read(&src).unwrap(), before);
}

#[test]
fn damaged_and_encrypted_files_get_clear_errors() {
    let t = Tmp::new("bad");
    let junk = t.path("junk.pdf");
    std::fs::write(&junk, b"this is not a pdf").unwrap();
    assert!(matches!(inspect(&junk), Err(Error::Damaged)));

    let enc = t.path("enc.pdf");
    std::fs::write(&enc, b"%PDF-1.6\ntrailer\n<< /Encrypt 5 0 R >>\n").unwrap();
    assert!(matches!(inspect(&enc), Err(Error::Encrypted)));

    let out = t.path("out.pdf");
    assert!(matches!(
        save_pages(&junk, &out, &[page(1, 0)]),
        Err(Error::Damaged)
    ));
}
