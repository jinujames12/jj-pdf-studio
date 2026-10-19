//! Loading documents and reading page geometry.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::Serialize;

use crate::error::{Error, Result};

/// US-Letter fallback when a page has no usable box.
const FALLBACK_BOX: [f64; 4] = [0.0, 0.0, 612.0, 792.0];

/// Page-tree attributes that must be copied onto a page before it is moved to
/// a different `/Parent`, otherwise it would lose what it inherited.
pub(crate) const INHERITABLE: [&[u8]; 4] = [b"MediaBox", b"CropBox", b"Resources", b"Rotate"];

/// Geometry of one page, enough for the UI to lay out placeholders and to size
/// new blank pages next to it. Sizes are in PDF points, before rotation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, serde::Deserialize)]
pub struct PageInfo {
    pub width: f64,
    pub height: f64,
    /// Effective `/Rotate` (inherited, normalised to 0, 90, 180 or 270).
    pub rotate: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DocInfo {
    pub pages: Vec<PageInfo>,
}

/// Passwords the user has typed this session, by canonical path, so every tool
/// can open a protected file after it was unlocked once. Memory only.
fn passwords() -> &'static Mutex<HashMap<PathBuf, String>> {
    static PASSWORDS: OnceLock<Mutex<HashMap<PathBuf, String>>> = OnceLock::new();
    PASSWORDS.get_or_init(Mutex::default)
}

fn password_key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn remembered_password(path: &Path) -> Option<String> {
    passwords()
        .lock()
        .unwrap()
        .get(&password_key(path))
        .cloned()
}

/// Checks `password` against `path` and, when it opens the file, remembers it
/// so `load` (and so every tool) can open the file from now on. Files that need
/// no password succeed without remembering anything.
pub fn unlock_for_session(path: &Path, password: &str) -> Result<()> {
    load_with(path, Some(password))?;
    passwords()
        .lock()
        .unwrap()
        .insert(password_key(path), password.to_owned());
    Ok(())
}

/// Loads a PDF, using the password remembered by `unlock_for_session` if any.
/// Files that need a password nobody has supplied give `Error::Encrypted`;
/// parse failures give a friendly "damaged" error.
pub fn load(path: &Path) -> Result<Document> {
    let remembered = remembered_password(path);
    match load_with(path, remembered.as_deref()) {
        // The file changed under us; forget the stale password and ask again.
        Err(Error::WrongPassword) => {
            passwords().lock().unwrap().remove(&password_key(path));
            Err(Error::Encrypted)
        }
        other => other,
    }
}

/// Like `load`, with an explicit password (`None` for files that open freely).
/// The returned document is decrypted and carries no `/Encrypt` entry.
pub fn load_with(path: &Path, password: Option<&str>) -> Result<Document> {
    let loaded = match password {
        Some(pw) if !pw.is_empty() => {
            refuse_legacy_owner_password(path, pw)?;
            Document::load_with_password(path, pw)
        }
        Some(_) => Document::load(path),
        None => Document::load(path),
    };
    match loaded {
        Ok(doc) => {
            if doc.is_encrypted() {
                return Err(Error::Encrypted);
            }
            if doc.get_pages().is_empty() {
                return Err(Error::Damaged);
            }
            Ok(doc)
        }
        Err(lopdf::Error::IO(e)) => Err(Error::Io(e)),
        Err(lopdf::Error::InvalidPassword)
        | Err(lopdf::Error::Decryption(lopdf::encryption::DecryptionError::IncorrectPassword)) => {
            Err(Error::WrongPassword)
        }
        Err(_) => {
            // Tell password-protected files apart from broken ones by looking
            // for the /Encrypt marker.
            let bytes = std::fs::read(path)?;
            if bytes.windows(8).any(|w| w == b"/Encrypt") {
                Err(Error::Encrypted)
            } else {
                Err(Error::Damaged)
            }
        }
    }
}

/// lopdf accepts the owner password of RC4 and AES-128 files (revisions 2 to 4)
/// but then derives the file key from it as if it were the open password, which
/// silently produces garbage. Refuse that case with a clear message instead.
/// Revision 6 files decrypt correctly with either password.
fn refuse_legacy_owner_password(path: &Path, password: &str) -> Result<()> {
    let Ok(doc) = Document::load(path) else {
        return Ok(()); // the real load reports the problem
    };
    if !doc.is_encrypted() {
        return Ok(());
    }
    let revision = doc
        .get_encrypted()
        .and_then(|d| d.get(b"R"))
        .and_then(Object::as_i64)
        .unwrap_or(6);
    if revision <= 4 && doc.authenticate_user_password(password).is_err() {
        return Err(if doc.authenticate_owner_password(password).is_ok() {
            Error::OwnerPasswordOnly
        } else {
            Error::WrongPassword
        });
    }
    Ok(())
}

/// The bytes of a PDF that any viewer can open: the file itself, or, when the
/// user unlocked it with a password this session, a decrypted copy made in
/// memory (nothing is written to disk).
pub fn viewable_bytes(path: &Path) -> Result<Vec<u8>> {
    if remembered_password(path).is_none() {
        return Ok(std::fs::read(path)?);
    }
    let mut doc = load(path)?;
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes)?;
    Ok(bytes)
}

/// Reads page count and per-page geometry.
pub fn inspect(path: &Path) -> Result<DocInfo> {
    let doc = load(path)?;
    let pages = doc
        .get_pages()
        .values()
        .map(|&id| page_info(&doc, id))
        .collect();
    Ok(DocInfo { pages })
}

pub(crate) fn page_info(doc: &Document, page: ObjectId) -> PageInfo {
    let b = page_box(doc, page);
    PageInfo {
        width: b[2] - b[0],
        height: b[3] - b[1],
        rotate: effective_rotate(doc, page),
    }
}

/// The visible box of a page as `[x0, y0, x1, y1]` with `x0 < x1` and
/// `y0 < y1`: the CropBox, else the MediaBox, else US Letter.
pub(crate) fn page_box(doc: &Document, page: ObjectId) -> [f64; 4] {
    let b = inherited(doc, page, b"CropBox")
        .and_then(|o| box_of(doc, &o))
        .or_else(|| inherited(doc, page, b"MediaBox").and_then(|o| box_of(doc, &o)))
        .unwrap_or(FALLBACK_BOX);
    [
        b[0].min(b[2]),
        b[1].min(b[3]),
        b[0].max(b[2]),
        b[1].max(b[3]),
    ]
}

pub(crate) fn effective_rotate(doc: &Document, page: ObjectId) -> i32 {
    let raw = inherited(doc, page, b"Rotate")
        .and_then(|o| num(doc, &o))
        .map_or(0, |v| v as i32);
    raw.rem_euclid(360) / 90 * 90
}

/// Looks `key` up on the page and then up its `/Parent` chain.
pub(crate) fn inherited(doc: &Document, page: ObjectId, key: &[u8]) -> Option<Object> {
    let mut id = page;
    // The depth cap protects against a malformed, cyclic /Parent chain.
    for _ in 0..64 {
        let dict = doc.get_dictionary(id).ok()?;
        if let Ok(v) = dict.get(key) {
            return Some(v.clone());
        }
        id = dict.get(b"Parent").ok()?.as_reference().ok()?;
    }
    None
}

fn resolve<'a>(doc: &'a Document, o: &'a Object) -> &'a Object {
    match o {
        Object::Reference(id) => doc.get_object(*id).unwrap_or(o),
        _ => o,
    }
}

fn num(doc: &Document, o: &Object) -> Option<f64> {
    match resolve(doc, o) {
        Object::Integer(i) => Some(*i as f64),
        Object::Real(r) => Some(f64::from(*r)),
        _ => None,
    }
}

fn box_of(doc: &Document, o: &Object) -> Option<[f64; 4]> {
    let arr = resolve(doc, o).as_array().ok()?;
    if arr.len() != 4 {
        return None;
    }
    let mut out = [0.0; 4];
    for (slot, item) in out.iter_mut().zip(arr) {
        *slot = num(doc, item)?;
    }
    // Zero-area boxes would produce invisible pages; treat them as missing.
    ((out[2] - out[0]).abs() > 0.0 && (out[3] - out[1]).abs() > 0.0).then_some(out)
}

/// The page's dictionary with inheritable attributes resolved onto it.
pub(crate) fn dict_with_inherited(doc: &Document, page: ObjectId) -> Result<Dictionary> {
    let mut dict = doc.get_dictionary(page)?.clone();
    for key in INHERITABLE {
        if !dict.has(key) {
            if let Some(v) = inherited(doc, page, key) {
                dict.set(key.to_vec(), v);
            }
        }
    }
    Ok(dict)
}
