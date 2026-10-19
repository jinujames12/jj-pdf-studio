//! Unlock (remove a password and restrictions) and Protect (add AES-256).
//!
//! Decryption of RC4 40/128-bit, AES-128 and AES-256 (revision 6) files is done
//! by lopdf when a document is loaded; this module decides what to tell the
//! user, strips the encryption on save, and writes new revision 6 files.

use crate::progress::{step, Progress, Silent};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use lopdf::content::Content;
use lopdf::encryption::crypt_filters::{Aes256CryptFilter, CryptFilter};
use lopdf::{
    Dictionary, Document, EncryptionState, EncryptionVersion, Object, ObjectId, Permissions,
};
use rand::RngExt as _;
use serde::{Deserialize, Serialize};

use crate::assemble::{same_file, write_atomic};
use crate::document::{self, effective_rotate, inherited, page_box};
use crate::error::{Error, Result};

/// Which kind of protection a file has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Protection {
    /// Not encrypted at all.
    None,
    /// Opens without a password, but is encrypted and may block actions.
    RestrictionsOnly,
    /// Cannot be opened without the user (open) password.
    OpenPassword,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProtectionInfo {
    pub protection: Protection,
    /// "RC4 40-bit", "RC4 128-bit", "AES-128" or "AES-256"; `None` when the
    /// file is not encrypted or its settings could not be read.
    pub algorithm: Option<String>,
    /// Plain-language names of the actions the file blocks (restrictions-only
    /// files; empty when the permissions are unknown).
    pub blocked: Vec<String>,
}

/// Looks at a file without using any remembered password and says how it is
/// protected. Never needs a password.
pub fn inspect_protection(path: &Path) -> Result<ProtectionInfo> {
    let doc = match Document::load(path) {
        Ok(doc) => doc,
        Err(lopdf::Error::IO(e)) => return Err(Error::Io(e)),
        Err(_) => {
            let bytes = std::fs::read(path)?;
            return if bytes.windows(8).any(|w| w == b"/Encrypt") {
                Ok(ProtectionInfo {
                    protection: Protection::OpenPassword,
                    algorithm: None,
                    blocked: Vec::new(),
                })
            } else {
                Err(Error::Damaged)
            };
        }
    };
    if let Some(state) = doc.encryption_state.as_ref() {
        // Opened with the empty password: encrypted, but readable by anyone.
        return Ok(ProtectionInfo {
            protection: Protection::RestrictionsOnly,
            algorithm: Some(state_label(state)),
            blocked: blocked_actions(state.permissions()),
        });
    }
    if let Ok(dict) = doc.get_encrypted() {
        return Ok(ProtectionInfo {
            protection: Protection::OpenPassword,
            algorithm: algorithm_of(dict),
            blocked: Vec::new(),
        });
    }
    Ok(ProtectionInfo {
        protection: Protection::None,
        algorithm: None,
        blocked: Vec::new(),
    })
}

fn state_label(state: &EncryptionState) -> String {
    label(
        state.version(),
        state.key_length(),
        state.get_stream_filter().method(),
    )
}

fn label(version: i64, key_length: Option<usize>, method: &[u8]) -> String {
    match method {
        b"AESV3" => "AES-256".into(),
        b"AESV2" => "AES-128".into(),
        _ => match (version, key_length) {
            (1, _) | (_, None) => "RC4 40-bit".into(),
            (_, Some(bits)) => format!("RC4 {bits}-bit"),
        },
    }
}

/// Reads the algorithm out of an `/Encrypt` dictionary that could not be used.
fn algorithm_of(dict: &Dictionary) -> Option<String> {
    let version = dict.get(b"V").and_then(Object::as_i64).ok()?;
    let length = dict
        .get(b"Length")
        .and_then(Object::as_i64)
        .ok()
        .map(|n| n as usize);
    let method = dict
        .get(b"StmF")
        .and_then(Object::as_name)
        .ok()
        .and_then(|name| {
            let cf = dict.get(b"CF").and_then(Object::as_dict).ok()?;
            let filter = cf.get(name).and_then(Object::as_dict).ok()?;
            filter.get(b"CFM").and_then(Object::as_name).ok()
        })
        .unwrap_or(b"V2");
    // Crypt-filter based files (version 4 and up) are 128-bit unless AES-256.
    let length = if version >= 4 { Some(128) } else { length };
    Some(label(version, length, method))
}

fn blocked_actions(p: Permissions) -> Vec<String> {
    [
        (Permissions::PRINTABLE, "printing"),
        (Permissions::COPYABLE, "copying text and images"),
        (Permissions::MODIFIABLE, "editing"),
        (Permissions::ANNOTABLE, "adding comments"),
    ]
    .into_iter()
    .filter(|(flag, _)| !p.contains(*flag))
    .map(|(_, name)| name.to_string())
    .collect()
}

/// Why text may still not be copyable after unlocking.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CopyHints {
    /// 1-based pages that are just a full-page picture (no text to copy).
    pub scanned_pages: Vec<u32>,
    /// Fonts that would copy as garbled characters (no `/ToUnicode` map).
    pub fonts_without_unicode: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UnlockReport {
    pub size: u64,
    /// What the file had before it was unlocked.
    pub protection: ProtectionInfo,
    pub hints: CopyHints,
}

/// Saves an unprotected copy of `source` to `output`. `password` is the open
/// password for files that need one; files that open freely (restrictions
/// only) need none. Fails with `NotProtected` when there is nothing to remove.
pub fn unlock_file(source: &Path, output: &Path, password: Option<&str>) -> Result<UnlockReport> {
    unlock_file_with(source, output, password, &Silent)
}

/// Like [`unlock_file`], reporting progress and stopping if cancelled.
pub fn unlock_file_with(
    source: &Path,
    output: &Path,
    password: Option<&str>,
    progress: &dyn Progress,
) -> Result<UnlockReport> {
    if same_file(source, output) {
        return Err(Error::OutputIsSource(output.to_path_buf()));
    }
    step(progress, 0, 3, "Reading the document")?;
    let protection = inspect_protection(source)?;
    if protection.protection == Protection::None {
        return Err(Error::NotProtected);
    }
    let mut doc = match password {
        Some(pw) => document::load_with(source, Some(pw))?,
        None => document::load(source)?,
    };
    if !doc.was_encrypted() {
        return Err(Error::NotProtected);
    }
    step(progress, 1, 3, "Removing protection")?;
    let algorithm = doc.encryption_state.as_ref().map(state_label);
    // Drop the unused /Encrypt dictionary so no trace of it remains.
    if let Some(id) = doc
        .encryption_state
        .as_ref()
        .and_then(|s| s.encrypt_object_id())
    {
        doc.objects.remove(&id);
    }
    doc.trailer.remove(b"Encrypt");
    doc.encryption_state = None;
    let hints = copy_hints(&doc);
    step(progress, 2, 3, "Saving")?;
    let size = write_atomic(&mut doc, output)?;
    Ok(UnlockReport {
        size,
        protection: ProtectionInfo {
            algorithm: algorithm.or(protection.algorithm),
            ..protection
        },
        hints,
    })
}

// --- Why copying may still fail ------------------------------------------------

/// A page counts as scanned when its content has no text-showing operator and
/// one image covers nearly all of it.
const FULL_PAGE_COVERAGE: f64 = 0.9;

fn copy_hints(doc: &Document) -> CopyHints {
    let scanned_pages = doc
        .get_pages()
        .into_iter()
        .filter(|&(_, id)| is_scanned_page(doc, id))
        .map(|(n, _)| n)
        .collect();
    CopyHints {
        scanned_pages,
        fonts_without_unicode: fonts_without_unicode(doc),
    }
}

type Matrix = [f64; 6];

/// `m` applied first, then `n` (PDF row-vector convention).
fn multiply(m: &Matrix, n: &Matrix) -> Matrix {
    [
        m[0] * n[0] + m[1] * n[2],
        m[0] * n[1] + m[1] * n[3],
        m[2] * n[0] + m[3] * n[2],
        m[2] * n[1] + m[3] * n[3],
        m[4] * n[0] + m[5] * n[2] + n[4],
        m[4] * n[1] + m[5] * n[3] + n[5],
    ]
}

fn number(o: &Object) -> Option<f64> {
    match o {
        Object::Integer(i) => Some(*i as f64),
        Object::Real(r) => Some(f64::from(*r)),
        _ => None,
    }
}

fn dict_of<'a>(doc: &'a Document, o: &'a Object) -> Option<&'a Dictionary> {
    match o {
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        other => other.as_dict().ok(),
    }
}

fn is_scanned_page(doc: &Document, page: ObjectId) -> bool {
    let Ok(content) = Content::decode(&doc.get_page_content(page)) else {
        return false;
    };
    let resources = inherited(doc, page, b"Resources");
    let xobjects = resources
        .as_ref()
        .and_then(|r| dict_of(doc, r))
        .and_then(|r| r.get(b"XObject").ok())
        .and_then(|o| dict_of(doc, o));
    let [x0, y0, x1, y1] = page_box(doc, page);
    let (mut width, mut height) = (x1 - x0, y1 - y0);
    if effective_rotate(doc, page) % 180 == 90 {
        std::mem::swap(&mut width, &mut height);
    }

    let mut ctm: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut stack: Vec<Matrix> = Vec::new();
    let mut full_page_image = false;
    for op in &content.operations {
        match op.operator.as_str() {
            "Tj" | "TJ" | "'" | "\"" => return false,
            "q" => stack.push(ctm),
            "Q" => ctm = stack.pop().unwrap_or(ctm),
            "cm" => {
                let m: Vec<f64> = op.operands.iter().filter_map(number).collect();
                if let Ok(m) = <[f64; 6]>::try_from(m) {
                    ctm = multiply(&m, &ctm);
                }
            }
            "Do" => {
                let is_image = op
                    .operands
                    .first()
                    .and_then(|n| n.as_name().ok())
                    .and_then(|name| xobjects?.get(name).ok())
                    .and_then(|o| o.as_reference().ok())
                    .and_then(|id| doc.get_object(id).ok())
                    .and_then(|o| o.as_stream().ok())
                    .is_some_and(|s| {
                        s.dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(&b"Image"[..])
                    });
                if is_image
                    && (ctm[0].abs() + ctm[2].abs()) >= width * FULL_PAGE_COVERAGE
                    && (ctm[1].abs() + ctm[3].abs()) >= height * FULL_PAGE_COVERAGE
                {
                    full_page_image = true;
                }
            }
            _ => {}
        }
    }
    full_page_image
}

/// Counts fonts that would copy as garbled characters: composite fonts with no
/// `/ToUnicode`, and embedded simple fonts with neither `/ToUnicode` nor an
/// `/Encoding` to map their codes.
fn fonts_without_unicode(doc: &Document) -> usize {
    let mut seen = HashSet::new();
    doc.objects
        .iter()
        .filter(|(id, _)| seen.insert(**id))
        .filter_map(|(_, obj)| obj.as_dict().ok())
        .filter(|d| d.get(b"Type").and_then(Object::as_name).ok() == Some(&b"Font"[..]))
        .filter(|d| !d.has(b"ToUnicode"))
        .filter(|d| match d.get(b"Subtype").and_then(Object::as_name) {
            Ok(b"Type0") => true,
            Ok(b"Type1" | b"TrueType" | b"MMType1") => {
                let embedded = d
                    .get(b"FontDescriptor")
                    .ok()
                    .and_then(|o| dict_of(doc, o))
                    .is_some_and(|fd| {
                        fd.has(b"FontFile") || fd.has(b"FontFile2") || fd.has(b"FontFile3")
                    });
                embedded && !d.has(b"Encoding")
            }
            _ => false,
        })
        .count()
}

// --- Protect -----------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ProtectOptions {
    /// Needed to open the file. Must not be empty.
    pub open_password: String,
    /// Needed to lift the restrictions below. Required (and different from the
    /// open password) when any action is blocked.
    pub owner_password: Option<String>,
    pub allow_printing: bool,
    pub allow_copying: bool,
    pub allow_editing: bool,
}

impl ProtectOptions {
    fn restricts(&self) -> bool {
        !(self.allow_printing && self.allow_copying && self.allow_editing)
    }

    fn permissions(&self) -> Permissions {
        let mut p = Permissions::empty();
        if self.allow_printing {
            p |= Permissions::PRINTABLE | Permissions::PRINTABLE_IN_HIGH_QUALITY;
        }
        if self.allow_copying {
            p |= Permissions::COPYABLE;
        }
        if self.allow_editing {
            p |= Permissions::MODIFIABLE
                | Permissions::ANNOTABLE
                | Permissions::FILLABLE
                | Permissions::ASSEMBLABLE;
        }
        // Screen readers may always read the text (PDF 2.0 requires this bit).
        p | Permissions::COPYABLE_FOR_ACCESSIBILITY
    }
}

/// Encrypts `doc` in place with AES-256 (revision 6). The document must not be
/// encrypted already.
pub fn protect(doc: &mut Document, options: &ProtectOptions) -> Result<()> {
    if options.open_password.is_empty() {
        return Err(Error::NoPassword);
    }
    let given = options.owner_password.as_deref().filter(|p| !p.is_empty());
    let owner = match given {
        Some(o) if o != options.open_password => o,
        _ if options.restricts() => return Err(Error::BadOwnerPassword),
        // Nothing to restrict: the open password also serves as owner password.
        _ => options.open_password.as_str(),
    };

    let mut rng = rand::rng();
    let mut key = [0u8; 32];
    rng.fill(&mut key);
    // Encrypted files must carry a file identifier.
    if !doc.trailer.has(b"ID") {
        let mut id = [0u8; 16];
        rng.fill(&mut id);
        let id = Object::string_literal(id.to_vec());
        doc.trailer.set("ID", vec![id.clone(), id]);
    }

    let filter: Arc<dyn CryptFilter> = Arc::new(Aes256CryptFilter);
    let state = EncryptionState::try_from(EncryptionVersion::V5 {
        encrypt_metadata: true,
        crypt_filters: BTreeMap::from([(b"StdCF".to_vec(), filter)]),
        file_encryption_key: &key,
        stream_filter: b"StdCF".to_vec(),
        string_filter: b"StdCF".to_vec(),
        owner_password: owner,
        user_password: &options.open_password,
        permissions: options.permissions(),
    })?;
    doc.encrypt(&state)?;
    // Revision 6 is defined by PDF 2.0.
    doc.version = "2.0".into();
    Ok(())
}

/// Saves a copy of `source` protected with `options`. A source that is itself
/// password protected is opened with the password remembered for the session.
/// Returns the size of the written file.
pub fn protect_file(source: &Path, output: &Path, options: &ProtectOptions) -> Result<u64> {
    protect_file_with(source, output, options, &Silent)
}

/// Like [`protect_file`], reporting progress and stopping if cancelled.
pub fn protect_file_with(
    source: &Path,
    output: &Path,
    options: &ProtectOptions,
    progress: &dyn Progress,
) -> Result<u64> {
    if same_file(source, output) {
        return Err(Error::OutputIsSource(output.to_path_buf()));
    }
    step(progress, 0, 3, "Reading the document")?;
    let mut doc = document::load(source)?;
    step(progress, 1, 3, "Encrypting")?;
    protect(&mut doc, options)?;
    step(progress, 2, 3, "Saving")?;
    write_atomic(&mut doc, output)
}
