//! Progress reporting and cancellation of long operations.

mod common;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use common::{write_pdf, Tmp};
use pdf_core::{
    merge_files_with, save_pages_with, split_to_folder_with, Error, MergeInput, PageOp, Progress,
    SplitMode,
};

/// Counts reports and cancels after `cancel_after` of them (never if `None`).
struct Probe {
    reports: AtomicU32,
    cancel_after: Option<u32>,
    cancelled: AtomicBool,
}

impl Probe {
    fn new(cancel_after: Option<u32>) -> Self {
        Probe {
            reports: AtomicU32::new(0),
            cancel_after,
            cancelled: AtomicBool::new(false),
        }
    }
}

impl Progress for Probe {
    fn report(&self, _done: u32, _total: u32, _message: &str) {
        let n = self.reports.fetch_add(1, Ordering::Relaxed) + 1;
        if self.cancel_after == Some(n) {
            self.cancelled.store(true, Ordering::Relaxed);
        }
    }
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

#[test]
fn split_reports_each_file() {
    let t = Tmp::new("progress-split");
    let src = t.path("in.pdf");
    write_pdf(&src, "S", 4);
    let probe = Probe::new(None);
    let out = t.path("out");
    std::fs::create_dir(&out).unwrap();
    let written = split_to_folder_with(&src, &out, &SplitMode::Every { n: 1 }, &probe).unwrap();
    assert_eq!(written.len(), 4);
    assert_eq!(probe.reports.load(Ordering::Relaxed), 4);
}

#[test]
fn cancelled_split_removes_the_files_it_wrote() {
    let t = Tmp::new("progress-split-cancel");
    let src = t.path("in.pdf");
    write_pdf(&src, "S", 5);
    let out = t.path("out");
    std::fs::create_dir(&out).unwrap();
    let probe = Probe::new(Some(2));
    let err = split_to_folder_with(&src, &out, &SplitMode::Every { n: 1 }, &probe).unwrap_err();
    assert!(matches!(err, Error::Cancelled));
    assert_eq!(std::fs::read_dir(&out).unwrap().count(), 0);
}

#[test]
fn cancelled_save_leaves_no_output() {
    let t = Tmp::new("progress-save-cancel");
    let src = t.path("in.pdf");
    write_pdf(&src, "S", 2);
    let out = t.path("out.pdf");
    let ops = [PageOp::Page { page: 1, rotate: 0 }];
    let err = save_pages_with(&src, &out, &ops, &Probe::new(Some(1))).unwrap_err();
    assert!(matches!(err, Error::Cancelled));
    assert!(!out.exists());
}

#[test]
fn cancelled_merge_leaves_no_output() {
    let t = Tmp::new("progress-merge-cancel");
    let (a, b) = (t.path("a.pdf"), t.path("b.pdf"));
    write_pdf(&a, "A", 1);
    write_pdf(&b, "B", 1);
    let out = t.path("out.pdf");
    let inputs = [a, b].map(|path| MergeInput { path, pages: None });
    let err = merge_files_with(&inputs, &out, true, &Probe::new(Some(1))).unwrap_err();
    assert!(matches!(err, Error::Cancelled));
    assert!(!out.exists());
}
