//! Merge several PDFs (or chosen pages of them) into one document.
//!
//! Each input is first cut down to the wanted pages with `assemble`, which also
//! resolves inherited page attributes. Its objects are then renumbered so they
//! cannot collide with the other inputs and grafted under one new page tree.

use crate::progress::{step, Progress, Silent};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use lopdf::{dictionary, Document, Object, ObjectId, StringFormat};
use serde::Deserialize;

use crate::assemble::{assemble, same_file, write_atomic, PageOp};
use crate::document;
use crate::error::{Error, Result};

/// One file to merge.
#[derive(Debug, Clone, Deserialize)]
pub struct MergeInput {
    pub path: PathBuf,
    /// The pages to take, in order. `None` takes every page as it is.
    pub pages: Option<Vec<PageOp>>,
}

/// A source file's own bookmarks, to be nested under the file's bookmark.
struct Outline {
    first: ObjectId,
    last: ObjectId,
    count: i64,
}

struct Part {
    title: String,
    first_page: ObjectId,
    outline: Option<Outline>,
}

/// Cap on a bookmark chain, so a corrupt cyclic `/Next` link cannot hang us.
const MAX_SIBLINGS: usize = 100_000;

pub fn merge(inputs: &[MergeInput], bookmarks: bool) -> Result<Document> {
    merge_with(inputs, bookmarks, &Silent)
}

fn merge_with(inputs: &[MergeInput], bookmarks: bool, progress: &dyn Progress) -> Result<Document> {
    if inputs.is_empty() {
        return Err(Error::NoInputs);
    }
    let mut next_id = 1;
    let mut objects: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut pages: Vec<ObjectId> = Vec::new();
    let mut parts: Vec<Part> = Vec::new();

    for (i, input) in inputs.iter().enumerate() {
        step(
            progress,
            i as u32,
            inputs.len() as u32,
            &format!("Reading file {} of {}", i + 1, inputs.len()),
        )?;
        let mut doc = document::load(&input.path)?;
        let ops = match &input.pages {
            Some(ops) => ops.clone(),
            None => (1..=doc.get_pages().len() as u32)
                .map(|page| PageOp::Page { page, rotate: 0 })
                .collect(),
        };
        assemble(&mut doc, &ops)?;
        doc.renumber_objects_with(next_id);
        next_id = doc.max_id + 1;

        // The old catalog and page-tree root are replaced by new ones below.
        let catalog_id = doc.trailer.get(b"Root")?.as_reference()?;
        let old_root = doc.catalog()?.get(b"Pages")?.as_reference()?;
        let outline = bookmarks.then(|| outline_of(&doc)).flatten();
        let own_pages: Vec<ObjectId> = doc.get_pages().values().copied().collect();
        parts.push(Part {
            title: title_of(&input.path),
            first_page: own_pages[0],
            outline,
        });
        pages.extend(own_pages);

        for (id, object) in doc.objects {
            let is_xref_or_objstm = matches!(&object, Object::Stream(s)
                if matches!(s.dict.get(b"Type").and_then(Object::as_name), Ok(b"XRef" | b"ObjStm")));
            if id != catalog_id && id != old_root && !is_xref_or_objstm {
                objects.insert(id, object);
            }
        }
    }

    let mut out = Document::with_version("1.5");
    out.objects = objects;
    out.max_id = next_id - 1;

    let pages_id = out.new_object_id();
    for &page in &pages {
        out.get_dictionary_mut(page)?.set("Parent", pages_id);
    }
    out.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => pages.iter().map(|&p| Object::Reference(p)).collect::<Vec<_>>(),
            "Count" => pages.len() as i64,
        }),
    );
    let mut catalog = dictionary! { "Type" => "Catalog", "Pages" => pages_id };
    if bookmarks {
        let outlines_id = add_bookmarks(&mut out, &parts)?;
        catalog.set("Outlines", outlines_id);
        catalog.set("PageMode", "UseOutlines");
    }
    let catalog_id = out.add_object(catalog);
    out.trailer.set("Root", catalog_id);
    out.prune_objects();
    Ok(out)
}

/// Merges `inputs` and writes the result to `output`, which must not be one of
/// the inputs. Returns the size of the written file in bytes.
pub fn merge_files(inputs: &[MergeInput], output: &Path, bookmarks: bool) -> Result<u64> {
    merge_files_with(inputs, output, bookmarks, &Silent)
}

/// Like [`merge_files`], reporting progress and stopping if cancelled.
pub fn merge_files_with(
    inputs: &[MergeInput],
    output: &Path,
    bookmarks: bool,
    progress: &dyn Progress,
) -> Result<u64> {
    if let Some(input) = inputs.iter().find(|i| same_file(&i.path, output)) {
        return Err(Error::OutputIsSource(input.path.clone()));
    }
    let mut doc = merge_with(inputs, bookmarks, progress)?;
    step(progress, inputs.len() as u32, inputs.len() as u32, "Saving")?;
    write_atomic(&mut doc, output)
}

/// File name without folder and extension, used as the bookmark title.
fn title_of(path: &Path) -> String {
    path.file_stem()
        .map_or_else(|| "Document".into(), |s| s.to_string_lossy().into_owned())
}

/// PDF text string: plain bytes for ASCII, UTF-16BE with a byte-order mark
/// otherwise, so non-Latin file names show correctly in a viewer.
fn text_string(s: &str) -> Object {
    let bytes = if s.is_ascii() {
        s.as_bytes().to_vec()
    } else {
        let mut b = vec![0xFE, 0xFF];
        b.extend(s.encode_utf16().flat_map(u16::to_be_bytes));
        b
    };
    Object::String(bytes, StringFormat::Literal)
}

/// The top level of the source's own bookmarks, if it still has any (`assemble`
/// drops them when it removed pages, because they could point at deleted ones).
fn outline_of(doc: &Document) -> Option<Outline> {
    let root = doc
        .catalog()
        .ok()?
        .get(b"Outlines")
        .ok()?
        .as_reference()
        .ok()?;
    let dict = doc.get_dictionary(root).ok()?;
    Some(Outline {
        first: dict.get(b"First").ok()?.as_reference().ok()?,
        last: dict.get(b"Last").ok()?.as_reference().ok()?,
        count: dict
            .get(b"Count")
            .ok()
            .and_then(|c| c.as_i64().ok())
            .unwrap_or(0),
    })
}

/// Builds the outline tree (one bookmark per file, source bookmarks below it)
/// and returns the id of the outline root.
fn add_bookmarks(doc: &mut Document, parts: &[Part]) -> Result<ObjectId> {
    let root = doc.new_object_id();
    let ids: Vec<ObjectId> = parts.iter().map(|_| doc.new_object_id()).collect();
    for (i, part) in parts.iter().enumerate() {
        let mut item = dictionary! {
            "Title" => text_string(&part.title),
            "Parent" => root,
            "Dest" => vec![Object::Reference(part.first_page), Object::Name(b"Fit".to_vec())],
        };
        if i > 0 {
            item.set("Prev", ids[i - 1]);
        }
        if let Some(&next) = ids.get(i + 1) {
            item.set("Next", next);
        }
        let mut visible = 0;
        if let Some(outline) = &part.outline {
            item.set("First", outline.first);
            item.set("Last", outline.last);
            // Point the source's top-level bookmarks at their new parent.
            let mut child = Some(outline.first);
            for _ in 0..MAX_SIBLINGS {
                let Some(id) = child else { break };
                let Ok(dict) = doc.get_dictionary_mut(id) else {
                    break;
                };
                dict.set("Parent", ids[i]);
                child = dict.get(b"Next").ok().and_then(|n| n.as_reference().ok());
            }
            visible = outline.count.max(0);
        }
        if visible > 0 {
            item.set("Count", visible);
        }
        doc.objects.insert(ids[i], Object::Dictionary(item));
    }
    doc.objects.insert(
        root,
        Object::Dictionary(dictionary! {
            "Type" => "Outlines",
            "First" => ids[0],
            "Last" => ids[ids.len() - 1],
            "Count" => parts.len() as i64,
        }),
    );
    Ok(root)
}
