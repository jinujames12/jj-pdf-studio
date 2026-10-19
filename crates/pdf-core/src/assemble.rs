//! Builds an output document from an operation list. This is the single code
//! path behind reorder, rotate, delete and insert-blank: the UI edits a list
//! and nothing touches the PDF until Save.

use std::collections::HashSet;
use std::path::Path;

use lopdf::{dictionary, Document, Object, Stream};
use serde::Deserialize;

use crate::document::{self, dict_with_inherited, effective_rotate};
use crate::error::{Error, Result};

/// One output page.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PageOp {
    /// A page of the source document.
    Page {
        /// 1-based page number in the source.
        page: u32,
        /// Rotation added to the page's own, a multiple of 90 (negative allowed).
        #[serde(default)]
        rotate: i32,
    },
    /// A new empty page; size in PDF points.
    Blank {
        width: f64,
        height: f64,
        #[serde(default)]
        rotate: i32,
    },
}

fn check_rotation(r: i32) -> Result<()> {
    if r % 90 == 0 {
        Ok(())
    } else {
        Err(Error::BadRotation(r))
    }
}

/// Applies `ops` to `doc` in place. The result contains exactly the listed
/// pages, in order. Source pages keep their object ids (so links between kept
/// pages stay valid); a page listed more than once is copied for the repeats.
pub fn assemble(doc: &mut Document, ops: &[PageOp]) -> Result<()> {
    if ops.is_empty() {
        return Err(Error::NoPages);
    }
    let source_pages = doc.get_pages();
    let pages_root = doc
        .catalog()?
        .get(b"Pages")?
        .as_reference()
        .map_err(|_| Error::Damaged)?;

    // Validate everything up front so a bad op cannot leave a half-built doc.
    for op in ops {
        match op {
            PageOp::Page { page, rotate } => {
                check_rotation(*rotate)?;
                if !source_pages.contains_key(page) {
                    return Err(Error::PageOutOfRange(*page));
                }
            }
            PageOp::Blank {
                width,
                height,
                rotate,
            } => {
                check_rotation(*rotate)?;
                if !(width.is_finite() && height.is_finite() && *width > 0.0 && *height > 0.0) {
                    return Err(Error::BadBlankSize);
                }
            }
        }
    }

    let mut used = HashSet::new();
    let mut kids: Vec<Object> = Vec::with_capacity(ops.len());
    for op in ops {
        let id = match op {
            PageOp::Page { page, rotate } => {
                let src = source_pages[page];
                // Resolve inherited attributes before mutating anything: the
                // page is about to get a new parent.
                let mut dict = dict_with_inherited(doc, src)?;
                let rot = (effective_rotate(doc, src) + rotate).rem_euclid(360);
                set_rotate(&mut dict, rot);
                dict.set("Parent", pages_root);
                if used.insert(src) {
                    doc.objects.insert(src, Object::Dictionary(dict));
                    src
                } else {
                    doc.add_object(dict)
                }
            }
            PageOp::Blank {
                width,
                height,
                rotate,
            } => {
                let contents = doc.add_object(Stream::new(dictionary! {}, Vec::new()));
                let mut dict = dictionary! {
                    "Type" => "Page",
                    "Parent" => pages_root,
                    "MediaBox" => vec![
                        0.into(),
                        0.into(),
                        Object::Real(*width as f32),
                        Object::Real(*height as f32),
                    ],
                    "Resources" => dictionary! {},
                    "Contents" => contents,
                };
                set_rotate(&mut dict, rotate.rem_euclid(360));
                doc.add_object(dict)
            }
        };
        kids.push(Object::Reference(id));
    }

    let pages_dict = doc.get_dictionary_mut(pages_root)?;
    pages_dict.set("Kids", kids);
    pages_dict.set("Count", ops.len() as i64);
    // Page attributes now live on each page; drop the root's copies so they
    // cannot leak onto blank pages.
    for key in document::INHERITABLE {
        pages_dict.remove(key);
    }

    if used.len() < source_pages.len() {
        // A bookmark or named destination may point at a removed page, which
        // would keep its content in the file. Drop them rather than leak it.
        let catalog = doc.catalog_mut()?;
        for key in [&b"Outlines"[..], b"Dests", b"Names"] {
            catalog.remove(key);
        }
    }
    doc.prune_objects();
    Ok(())
}

fn set_rotate(dict: &mut lopdf::Dictionary, deg: i32) {
    if deg == 0 {
        dict.remove(b"Rotate");
    } else {
        dict.set("Rotate", i64::from(deg));
    }
}

/// True when `output` is the same file as `source`. Compares the canonical
/// paths case-insensitively, since Windows paths are case-insensitive.
pub fn same_file(source: &Path, output: &Path) -> bool {
    let (Ok(a), Ok(b)) = (source.canonicalize(), output.canonicalize()) else {
        return false; // output does not exist yet, so it cannot be the source
    };
    a == b
        || a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
}

/// Loads `source`, applies `ops` and writes the result to `output`, which must
/// not be the source file. Returns the size of the written file in bytes.
pub fn save_pages(source: &Path, output: &Path, ops: &[PageOp]) -> Result<u64> {
    if same_file(source, output) {
        return Err(Error::OutputIsSource(output.to_path_buf()));
    }
    let mut doc = document::load(source)?;
    assemble(&mut doc, ops)?;
    write_atomic(&mut doc, output)
}

/// Saves `doc` to `output` through a temporary file beside it, so a failure
/// never leaves a truncated PDF at the chosen name. Returns the size in bytes.
pub(crate) fn write_atomic(doc: &mut Document, output: &Path) -> Result<u64> {
    write_via_temp(output, |tmp| doc.save(tmp).map(drop).map_err(Error::from))
}

/// Like `write_atomic`, for a file that is already serialised.
pub(crate) fn write_bytes_atomic(bytes: &[u8], output: &Path) -> Result<u64> {
    write_via_temp(output, |tmp| {
        std::fs::write(tmp, bytes).map_err(Error::from)
    })
}

fn write_via_temp(output: &Path, write: impl FnOnce(&Path) -> Result<()>) -> Result<u64> {
    let mut tmp = output.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    let result = write(&tmp).and_then(|()| std::fs::rename(&tmp, output).map_err(Error::from));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result?;
    Ok(std::fs::metadata(output)?.len())
}
