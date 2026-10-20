//! Document properties (SPEC 5.14): read and edit the `/Info` entries, or strip
//! all metadata before sharing.

use crate::progress::{step, Progress, Silent};
use std::path::Path;

use lopdf::{Dictionary, Document, Object};
use serde::{Deserialize, Serialize};

use crate::assemble::{same_file, write_atomic};
use crate::document::{self, PageInfo};
use crate::error::{Error, Result};
use crate::security::{inspect_protection, ProtectionInfo};

/// The `/Info` entries the user can edit.
const EDITABLE: [(&str, &[u8]); 5] = [
    ("title", b"Title"),
    ("author", b"Author"),
    ("subject", b"Subject"),
    ("keywords", b"Keywords"),
    ("creator", b"Creator"),
];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Properties {
    pub title: String,
    pub author: String,
    pub subject: String,
    pub keywords: String,
    pub creator: String,
    pub producer: String,
    /// "2024-05-17 09:30" style, empty when the file has none.
    pub created: String,
    pub modified: String,
    /// "1.7" and so on, as the file header says.
    pub pdf_version: String,
    pub pages: Vec<PageInfo>,
    pub size: u64,
    pub protection: ProtectionInfo,
    /// True when the file carries an XMP metadata stream.
    pub has_xmp: bool,
}

/// What to save. Empty text clears an entry. With `remove_all` the text fields
/// are ignored and nothing is kept.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct PropertiesEdit {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub keywords: String,
    #[serde(default)]
    pub creator: String,
    #[serde(default)]
    pub remove_all: bool,
}

impl PropertiesEdit {
    fn value(&self, field: &str) -> &str {
        match field {
            "title" => &self.title,
            "author" => &self.author,
            "subject" => &self.subject,
            "keywords" => &self.keywords,
            _ => &self.creator,
        }
    }
}

/// A PDF text string as Rust text: UTF-16BE or UTF-8 with a byte order mark,
/// otherwise PDFDocEncoding, which matches Latin-1 for everything usual.
fn decode(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = rest
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&c| u16::from_be_bytes(c))
            .collect();
        String::from_utf16_lossy(&units)
    } else if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(rest).into_owned()
    } else {
        bytes.iter().map(|&b| char::from(b)).collect()
    }
}

/// Plain ASCII is stored as is; anything else as UTF-16BE with a byte order mark.
fn encode(text: &str) -> Object {
    if text.is_ascii() {
        Object::string_literal(text)
    } else {
        let mut bytes = vec![0xFE, 0xFF];
        for unit in text.encode_utf16() {
            bytes.extend(unit.to_be_bytes());
        }
        Object::String(bytes, lopdf::StringFormat::Hexadecimal)
    }
}

fn text_of(doc: &Document, info: &Dictionary, key: &[u8]) -> String {
    match info.get(key) {
        Ok(Object::String(b, _)) => decode(b),
        Ok(Object::Reference(id)) => match doc.get_object(*id) {
            Ok(Object::String(b, _)) => decode(b),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

/// `D:20240517093000+05'30'` as `2024-05-17 09:30`; unparseable text is shown as is.
fn pretty_date(raw: &str) -> String {
    let digits = raw.strip_prefix("D:").unwrap_or(raw);
    let n = |from: usize, to: usize| {
        digits
            .get(from..to)
            .filter(|s| s.bytes().all(|b| b.is_ascii_digit()))
    };
    match (n(0, 4), n(4, 6), n(6, 8)) {
        (Some(y), Some(m), Some(d)) => match (n(8, 10), n(10, 12)) {
            (Some(h), Some(min)) => format!("{y}-{m}-{d} {h}:{min}"),
            _ => format!("{y}-{m}-{d}"),
        },
        (Some(y), Some(m), None) => format!("{y}-{m}"),
        (Some(y), None, None) => y.to_string(),
        _ => raw.to_string(),
    }
}

fn info_dictionary(doc: &Document) -> Option<Dictionary> {
    match doc.trailer.get(b"Info").ok()? {
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        Object::Dictionary(d) => Some(d.clone()),
        _ => None,
    }
}

fn has_xmp(doc: &Document) -> bool {
    doc.catalog().is_ok_and(|c| c.has(b"Metadata"))
}

/// Reads the properties shown in the Properties tool.
pub fn read_properties(path: &Path) -> Result<Properties> {
    let doc = document::load(path)?;
    let info = info_dictionary(&doc).unwrap_or_default();
    let text = |key: &[u8]| text_of(&doc, &info, key);
    let pages = doc
        .get_pages()
        .values()
        .map(|&id| document::page_info(&doc, id))
        .collect();
    Ok(Properties {
        title: text(b"Title"),
        author: text(b"Author"),
        subject: text(b"Subject"),
        keywords: text(b"Keywords"),
        creator: text(b"Creator"),
        producer: text(b"Producer"),
        created: pretty_date(&text(b"CreationDate")),
        modified: pretty_date(&text(b"ModDate")),
        pdf_version: doc.version.clone(),
        pages,
        size: std::fs::metadata(path)?.len(),
        protection: inspect_protection(path)?,
        has_xmp: has_xmp(&doc),
    })
}

/// Applies `edit` to the document in memory.
pub fn edit_properties(doc: &mut Document, edit: &PropertiesEdit) -> Result<()> {
    if edit.remove_all {
        doc.trailer.remove(b"Info");
        if let Ok(catalog) = doc.catalog_mut() {
            catalog.remove(b"Metadata");
        }
        // Pages and other objects can carry their own XMP stream.
        let ids: Vec<_> = doc.get_pages().values().copied().collect();
        for id in ids {
            if let Ok(page) = doc.get_dictionary_mut(id) {
                page.remove(b"Metadata");
                page.remove(b"PieceInfo");
            }
        }
        doc.prune_objects();
        return Ok(());
    }

    let mut info = info_dictionary(doc).unwrap_or_default();
    for (field, key) in EDITABLE {
        let value = edit.value(field).trim();
        if value.is_empty() {
            info.remove(key);
        } else {
            info.set(key, encode(value));
        }
    }
    match doc.trailer.get(b"Info").ok().cloned() {
        // Nothing left to say: no Info dictionary at all.
        _ if info.is_empty() => {
            doc.trailer.remove(b"Info");
        }
        Some(Object::Reference(id)) => {
            doc.objects.insert(id, Object::Dictionary(info));
        }
        _ => {
            let id = doc.add_object(info);
            doc.trailer.set("Info", id);
        }
    }
    // The XMP stream would still show the old title and author in some
    // viewers, so it goes when the Info entries are rewritten.
    if let Ok(catalog) = doc.catalog_mut() {
        catalog.remove(b"Metadata");
    }
    doc.prune_objects();
    Ok(())
}

/// Loads `source`, applies `edit` and writes the result to `output`, which must
/// not be the source file. Returns the size of the written file in bytes.
pub fn properties_file(source: &Path, output: &Path, edit: &PropertiesEdit) -> Result<u64> {
    properties_file_with(source, output, edit, &Silent)
}

/// Like [`properties_file`], reporting progress and stopping if cancelled.
pub fn properties_file_with(
    source: &Path,
    output: &Path,
    edit: &PropertiesEdit,
    progress: &dyn Progress,
) -> Result<u64> {
    if same_file(source, output) {
        return Err(Error::OutputIsSource(output.to_path_buf()));
    }
    step(progress, 0, 3, "Reading the document")?;
    let mut doc = document::load(source)?;
    step(progress, 1, 3, "Updating properties")?;
    edit_properties(&mut doc, edit)?;
    step(progress, 2, 3, "Saving")?;
    write_atomic(&mut doc, output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trips_through_both_encodings() {
        for s in ["Plain title", "Café ünï", "日本語"] {
            let Object::String(bytes, _) = encode(s) else {
                panic!("not a string");
            };
            assert_eq!(decode(&bytes), s);
        }
        assert_eq!(decode(b"Caf\xe9"), "Café");
    }

    #[test]
    fn dates_are_made_readable() {
        assert_eq!(pretty_date("D:20240517093000+05'30'"), "2024-05-17 09:30");
        assert_eq!(pretty_date("D:20240517"), "2024-05-17");
        assert_eq!(pretty_date(""), "");
        assert_eq!(pretty_date("last Tuesday"), "last Tuesday");
    }
}
