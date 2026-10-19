//! Helpers shared by the integration tests of the multi-file tools.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use lopdf::{dictionary, Document, Object, Stream};

mod media;
#[allow(unused_imports)]
pub use media::*;

/// Scratch directory that is removed on drop.
pub struct Tmp(pub PathBuf);

impl Tmp {
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("jjpdf-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
    pub fn path(&self, file: &str) -> PathBuf {
        self.0.join(file)
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Writes an `n`-page PDF to `path`. Page `i` draws the marker `<tag>-<i>` so
/// tests can tell pages apart after merging and reordering.
pub fn write_pdf(path: &Path, tag: &str, n: u32) {
    let mut doc = make_pdf(tag, n);
    doc.save(path).unwrap();
}

pub fn make_pdf(tag: &str, n: u32) -> Document {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut kids = Vec::new();
    for i in 1..=n {
        let content = doc.add_object(Stream::new(
            dictionary! {},
            format!("BT /F1 12 Tf 10 10 Td ({tag}-{i}) Tj ET").into_bytes(),
        ));
        let page = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 300.into()],
        };
        kids.push(Object::Reference(doc.add_object(page)));
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => i64::from(n),
        }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    doc
}

/// Adds a flat outline with one bookmark per page, titled `<tag> chapter <i>`.
pub fn add_outline(doc: &mut Document, tag: &str) {
    let pages: Vec<_> = doc.get_pages().values().copied().collect();
    let root = doc.new_object_id();
    let ids: Vec<_> = pages.iter().map(|_| doc.new_object_id()).collect();
    for (i, &page) in pages.iter().enumerate() {
        let mut item = dictionary! {
            "Title" => Object::string_literal(format!("{tag} chapter {}", i + 1)),
            "Parent" => root,
            "Dest" => vec![Object::Reference(page), Object::Name(b"Fit".to_vec())],
        };
        if i > 0 {
            item.set("Prev", ids[i - 1]);
        }
        if i + 1 < ids.len() {
            item.set("Next", ids[i + 1]);
        }
        doc.objects.insert(ids[i], Object::Dictionary(item));
    }
    doc.objects.insert(
        root,
        Object::Dictionary(dictionary! {
            "Type" => "Outlines",
            "First" => ids[0],
            "Last" => ids[ids.len() - 1],
            "Count" => ids.len() as i64,
        }),
    );
    let catalog = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
    doc.get_dictionary_mut(catalog)
        .unwrap()
        .set("Outlines", root);
}

/// Marker text of each page, in document order.
pub fn markers(doc: &Document) -> Vec<String> {
    doc.get_pages()
        .values()
        .map(|&id| {
            let text = String::from_utf8_lossy(&doc.get_page_content(id)).into_owned();
            let start = text.find('(').map_or(0, |p| p + 1);
            text[start..].split(')').next().unwrap().to_string()
        })
        .collect()
}

/// Titles of the bookmarks below `parent`, depth first, as `(depth, title)`.
pub fn bookmark_titles(doc: &Document) -> Vec<(usize, String)> {
    fn text(o: &Object) -> String {
        match o {
            Object::String(b, _) if b.starts_with(&[0xFE, 0xFF]) => {
                let units: Vec<u16> = b[2..]
                    .chunks(2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16_lossy(&units)
            }
            Object::String(b, _) => String::from_utf8_lossy(b).into_owned(),
            _ => String::new(),
        }
    }
    fn walk(
        doc: &Document,
        first: Option<lopdf::ObjectId>,
        depth: usize,
        out: &mut Vec<(usize, String)>,
    ) {
        let mut cur = first;
        while let Some(id) = cur {
            let d = doc.get_dictionary(id).unwrap();
            out.push((depth, text(d.get(b"Title").unwrap())));
            let child = d.get(b"First").ok().and_then(|f| f.as_reference().ok());
            walk(doc, child, depth + 1, out);
            cur = d.get(b"Next").ok().and_then(|n| n.as_reference().ok());
        }
    }
    let mut out = Vec::new();
    let Ok(root) = doc.catalog().unwrap().get(b"Outlines") else {
        return out;
    };
    let root = doc.get_dictionary(root.as_reference().unwrap()).unwrap();
    let first = root.get(b"First").ok().and_then(|f| f.as_reference().ok());
    walk(doc, first, 0, &mut out);
    out
}
