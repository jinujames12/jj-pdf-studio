//! Integration tests for Merge, Split, Extract and Rotate: build PDFs on disk,
//! run the operation, reload the result and check it.

mod common;

use std::path::PathBuf;

use common::{add_outline, bookmark_titles, make_pdf, markers, write_pdf, Tmp};
use lopdf::Document;
use pdf_core::{
    inspect, merge_files, plan_rotation, save_pages, split_to_folder, Error, MergeInput, PageOp,
    RotateAction, SplitMode, Target,
};

fn input(path: PathBuf) -> MergeInput {
    MergeInput { path, pages: None }
}

fn page(page: u32, rotate: i32) -> PageOp {
    PageOp::Page { page, rotate }
}

// Merge -----------------------------------------------------------------------

#[test]
fn merge_keeps_file_and_page_order_and_adds_a_bookmark_per_file() {
    let t = Tmp::new("merge-basic");
    let (a, b, c) = (t.path("alpha.pdf"), t.path("beta.pdf"), t.path("gamma.pdf"));
    write_pdf(&a, "A", 2);
    write_pdf(&b, "B", 3);
    write_pdf(&c, "C", 1);
    let out = t.path("out.pdf");

    merge_files(&[input(b), input(a), input(c)], &out, true).unwrap();
    let doc = Document::load(&out).unwrap();
    assert_eq!(markers(&doc), ["B-1", "B-2", "B-3", "A-1", "A-2", "C-1"]);
    let titles: Vec<_> = bookmark_titles(&doc).into_iter().map(|(_, t)| t).collect();
    assert_eq!(titles, ["beta", "alpha", "gamma"]);
}

#[test]
fn merge_without_bookmarks_writes_no_outline() {
    let t = Tmp::new("merge-nomarks");
    let (a, b) = (t.path("a.pdf"), t.path("b.pdf"));
    write_pdf(&a, "A", 1);
    write_pdf(&b, "B", 1);
    let out = t.path("out.pdf");

    merge_files(&[input(a), input(b)], &out, false).unwrap();
    let doc = Document::load(&out).unwrap();
    assert!(!doc.catalog().unwrap().has(b"Outlines"));
}

#[test]
fn merge_nests_each_files_own_bookmarks_under_its_file_bookmark() {
    let t = Tmp::new("merge-outline");
    let (a, b) = (t.path("a.pdf"), t.path("b.pdf"));
    let mut doc = make_pdf("A", 2);
    add_outline(&mut doc, "A");
    doc.save(&a).unwrap();
    write_pdf(&b, "B", 1);
    let out = t.path("out.pdf");

    merge_files(&[input(a), input(b)], &out, true).unwrap();
    let merged = Document::load(&out).unwrap();
    assert_eq!(
        bookmark_titles(&merged),
        [
            (0, "a".to_string()),
            (1, "A chapter 1".to_string()),
            (1, "A chapter 2".to_string()),
            (0, "b".to_string()),
        ]
    );
}

#[test]
fn merge_takes_chosen_pages_across_files_with_rotation() {
    let t = Tmp::new("merge-pages");
    let (a, b) = (t.path("a.pdf"), t.path("b.pdf"));
    write_pdf(&a, "A", 3);
    write_pdf(&b, "B", 3);
    let out = t.path("out.pdf");

    let inputs = [
        MergeInput {
            path: b,
            pages: Some(vec![page(3, 0), page(1, 90)]),
        },
        MergeInput {
            path: a,
            pages: Some(vec![page(2, 0), page(2, 0)]),
        },
    ];
    merge_files(&inputs, &out, true).unwrap();
    let doc = Document::load(&out).unwrap();
    assert_eq!(markers(&doc), ["B-3", "B-1", "A-2", "A-2"]);
    let rotations: Vec<_> = inspect(&out)
        .unwrap()
        .pages
        .iter()
        .map(|p| p.rotate)
        .collect();
    assert_eq!(rotations, [0, 90, 0, 0]);
    // Pages that were dropped are gone from the file, not just hidden.
    let bytes = std::fs::read(&out).unwrap();
    assert!(!bytes
        .windows(3)
        .any(|w| w == b"A-1" || w == b"A-3" || w == b"B-2"));
}

#[test]
fn twenty_files_merge_into_one_document() {
    let t = Tmp::new("merge-20");
    let mut inputs = Vec::new();
    let mut expected = Vec::new();
    for i in 0..20 {
        let path = t.path(&format!("f{i:02}.pdf"));
        write_pdf(&path, &format!("F{i}"), 3);
        expected.extend((1..=3).map(|p| format!("F{i}-{p}")));
        inputs.push(input(path));
    }
    let out = t.path("merged.pdf");

    merge_files(&inputs, &out, true).unwrap();
    let doc = Document::load(&out).unwrap();
    assert_eq!(doc.get_pages().len(), 60);
    assert_eq!(markers(&doc), expected);
    assert_eq!(bookmark_titles(&doc).len(), 20);
}

#[test]
fn merge_refuses_to_overwrite_an_input_and_to_run_with_nothing() {
    let t = Tmp::new("merge-protect");
    let (a, b) = (t.path("a.pdf"), t.path("b.pdf"));
    write_pdf(&a, "A", 1);
    write_pdf(&b, "B", 1);
    let before = std::fs::read(&b).unwrap();

    let result = merge_files(&[input(a), input(b.clone())], &b, true);
    assert!(matches!(result, Err(Error::OutputIsSource(_))));
    assert_eq!(std::fs::read(&b).unwrap(), before);
    assert!(matches!(
        merge_files(&[], &t.path("x.pdf"), true),
        Err(Error::NoInputs)
    ));
}

#[test]
fn merge_reports_a_damaged_input_and_writes_nothing() {
    let t = Tmp::new("merge-bad");
    let (a, junk, out) = (t.path("a.pdf"), t.path("junk.pdf"), t.path("out.pdf"));
    write_pdf(&a, "A", 1);
    std::fs::write(&junk, b"nope").unwrap();

    let result = merge_files(&[input(a), input(junk)], &out, true);
    assert!(matches!(result, Err(Error::Damaged)));
    assert!(!out.exists());
}

// Split and Extract -----------------------------------------------------------

fn names(written: &[pdf_core::Written]) -> Vec<String> {
    written
        .iter()
        .map(|w| w.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect()
}

fn part_markers(w: &pdf_core::Written) -> Vec<String> {
    markers(&Document::load(&w.path).unwrap())
}

#[test]
fn split_every_n_pages() {
    let t = Tmp::new("split-every");
    let src = t.path("book.pdf");
    write_pdf(&src, "P", 5);
    let out_dir = t.path("out");
    std::fs::create_dir(&out_dir).unwrap();

    let written = split_to_folder(&src, &out_dir, &SplitMode::Every { n: 2 }).unwrap();
    assert_eq!(
        names(&written),
        ["book_part01.pdf", "book_part02.pdf", "book_part03.pdf"]
    );
    assert_eq!(part_markers(&written[0]), ["P-1", "P-2"]);
    assert_eq!(part_markers(&written[2]), ["P-5"]);
    assert!(written.iter().all(|w| w.size > 0));
}

#[test]
fn split_by_ranges_names_files_by_page() {
    let t = Tmp::new("split-ranges");
    let src = t.path("doc.pdf");
    write_pdf(&src, "P", 9);
    let mode = SplitMode::Ranges {
        text: "1-3, 4-8, 9-end".into(),
    };

    let written = split_to_folder(&src, &t.0, &mode).unwrap();
    assert_eq!(
        names(&written),
        ["doc_p1-3.pdf", "doc_p4-8.pdf", "doc_p9.pdf"]
    );
    assert_eq!(
        part_markers(&written[1]),
        ["P-4", "P-5", "P-6", "P-7", "P-8"]
    );
    assert_eq!(part_markers(&written[2]), ["P-9"]);
}

#[test]
fn split_at_markers_and_pruning_of_other_pages() {
    let t = Tmp::new("split-at");
    let src = t.path("doc.pdf");
    write_pdf(&src, "P", 6);
    let out_dir = t.path("o");
    std::fs::create_dir(&out_dir).unwrap();

    let mode = SplitMode::At { after: vec![2, 5] };
    let written = split_to_folder(&src, &out_dir, &mode).unwrap();
    assert_eq!(written.len(), 3);
    assert_eq!(part_markers(&written[1]), ["P-3", "P-4", "P-5"]);
    // Each part holds only its own pages.
    let bytes = std::fs::read(&written[0].path).unwrap();
    assert!(!bytes.windows(3).any(|w| w == b"P-3"));
}

#[test]
fn split_never_overwrites_existing_files() {
    let t = Tmp::new("split-unique");
    let src = t.path("doc.pdf");
    write_pdf(&src, "P", 2);
    let out_dir = t.path("o");
    std::fs::create_dir(&out_dir).unwrap();

    let first = split_to_folder(&src, &out_dir, &SplitMode::Every { n: 1 }).unwrap();
    let before = std::fs::read(&first[0].path).unwrap();
    let second = split_to_folder(&src, &out_dir, &SplitMode::Every { n: 1 }).unwrap();
    assert_eq!(names(&second), ["doc_part01 (2).pdf", "doc_part02 (2).pdf"]);
    assert_eq!(std::fs::read(&first[0].path).unwrap(), before);
}

#[test]
fn split_with_invalid_input_writes_nothing() {
    let t = Tmp::new("split-invalid");
    let src = t.path("doc.pdf");
    write_pdf(&src, "P", 3);
    let out_dir = t.path("o");
    std::fs::create_dir(&out_dir).unwrap();

    for mode in [
        SplitMode::Every { n: 0 },
        SplitMode::Ranges { text: "2-9".into() },
        SplitMode::At { after: vec![] },
    ] {
        assert!(split_to_folder(&src, &out_dir, &mode).is_err());
    }
    assert_eq!(std::fs::read_dir(&out_dir).unwrap().count(), 0);
}

#[test]
fn extract_selected_pages_into_one_file_or_one_file_per_page() {
    let t = Tmp::new("extract");
    let src = t.path("doc.pdf");
    write_pdf(&src, "P", 8);

    // One PDF with all selected pages: the same engine as the Pages tool.
    let one = t.path("picked.pdf");
    save_pages(
        &src,
        &one,
        &[page(2, 0), page(5, 0), page(6, 0), page(7, 0)],
    )
    .unwrap();
    assert_eq!(
        markers(&Document::load(&one).unwrap()),
        ["P-2", "P-5", "P-6", "P-7"]
    );

    // One PDF per selected page.
    let out_dir = t.path("each");
    std::fs::create_dir(&out_dir).unwrap();
    let mode = SplitMode::Pages {
        pages: vec![5, 2, 6],
    };
    let written = split_to_folder(&src, &out_dir, &mode).unwrap();
    assert_eq!(names(&written), ["doc_p2.pdf", "doc_p5.pdf", "doc_p6.pdf"]);
    assert_eq!(part_markers(&written[1]), ["P-5"]);
}

// Rotate ----------------------------------------------------------------------

#[test]
fn rotate_odd_pages_and_save_losslessly() {
    let t = Tmp::new("rotate");
    let src = t.path("doc.pdf");
    write_pdf(&src, "P", 4);
    let info = inspect(&src).unwrap();

    let action = RotateAction::Turn {
        target: Target::Odd,
        degrees: 90,
    };
    let turns = plan_rotation(&info.pages, &[], &action).unwrap();
    let ops: Vec<_> = turns
        .iter()
        .enumerate()
        .map(|(i, &r)| page(i as u32 + 1, r))
        .collect();
    let out = t.path("rotated.pdf");
    save_pages(&src, &out, &ops).unwrap();

    let rotations: Vec<_> = inspect(&out)
        .unwrap()
        .pages
        .iter()
        .map(|p| p.rotate)
        .collect();
    assert_eq!(rotations, [90, 0, 90, 0]);
    // Content is untouched: same pages in the same order.
    assert_eq!(
        markers(&Document::load(&out).unwrap()),
        ["P-1", "P-2", "P-3", "P-4"]
    );
}

#[test]
fn make_all_pages_landscape_turns_only_the_portrait_ones() {
    let t = Tmp::new("orient");
    let src = t.path("doc.pdf");
    write_pdf(&src, "P", 2); // both pages are 200 x 300 (portrait)
    let info = inspect(&src).unwrap();

    let sideways = plan_rotation(
        &info.pages,
        &[0, 90],
        &RotateAction::Orient {
            orientation: pdf_core::Orientation::Landscape,
        },
    )
    .unwrap();
    // Page 2 was already turned sideways, so only page 1 needs a turn.
    assert_eq!(sideways, [90, 90]);
}
