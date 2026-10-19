//! Commands for PDF to Images. The window renders each page with pdf.js and
//! sends the picture here; these commands only write it down.

use std::path::PathBuf;
use std::sync::Mutex;

use pdf_core::ImageExport;
use tauri::ipc::{InvokeBody, Request};
use tauri::State;

use super::pages::SavedFile;
use super::{canonical, Allowed};

struct Session {
    export: ImageExport,
    /// `<document name>_p`, the start of every picture name.
    prefix: String,
    /// Digits in the page number, so names sort: `p001`, or `p0001` past 999 pages.
    width: usize,
}

/// The export in progress, if any. Only one runs at a time.
#[derive(Default)]
pub struct ImageExports(Mutex<Option<Session>>);

/// Starts an export of pages of `source` into the folder `target`, or into the
/// ZIP file `target` when `zip` is set. `page_count` sizes the number padding.
#[tauri::command]
pub async fn export_images_begin(
    source: PathBuf,
    target: PathBuf,
    zip: bool,
    page_count: u32,
    allowed: State<'_, Allowed>,
    exports: State<'_, ImageExports>,
) -> Result<(), String> {
    let source = allowed.check_opened(&source)?;
    let export = if zip {
        match target.extension().and_then(|e| e.to_str()) {
            Some(e) if e.eq_ignore_ascii_case("zip") => {}
            _ => return Err("Please choose a file ending in .zip.".into()),
        }
        ImageExport::to_zip(&target, &source)
    } else {
        ImageExport::to_folder(&canonical(&target)?)
    }
    .map_err(|e| e.to_string())?;
    let stem = source
        .file_stem()
        .map_or_else(|| "document".into(), |s| s.to_string_lossy().into_owned());
    let session = Session {
        export,
        prefix: format!("{stem}_p"),
        width: page_count.to_string().len().max(3),
    };
    if let Some(old) = exports.0.lock().unwrap().replace(session) {
        old.export.cancel();
    }
    Ok(())
}

/// Adds one page picture. The body is the raw PNG or JPEG bytes; the headers
/// `page` (1-based) and `ext` (`png` or `jpg`) say which page and format.
#[tauri::command]
pub async fn export_images_add(
    request: Request<'_>,
    exports: State<'_, ImageExports>,
) -> Result<(), String> {
    let InvokeBody::Raw(data) = request.body() else {
        return Err("The page picture was not received.".into());
    };
    let header = |name: &str| {
        request
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    };
    let page: u32 = header("page")
        .and_then(|p| p.parse().ok())
        .ok_or("The page number is missing.")?;
    let ext = match header("ext").as_deref() {
        Some("png") => "png",
        Some("jpg") => "jpg",
        _ => return Err("Pictures can be saved as PNG or JPG.".into()),
    };
    let mut guard = exports.0.lock().unwrap();
    let session = guard.as_mut().ok_or("No export is in progress.")?;
    let name = format!("{}{:0w$}.{ext}", session.prefix, page, w = session.width);
    session.export.add(&name, data).map_err(|e| e.to_string())
}

/// Completes the export; returns the ZIP file, or every picture.
#[tauri::command]
pub async fn export_images_finish(
    allowed: State<'_, Allowed>,
    exports: State<'_, ImageExports>,
) -> Result<Vec<SavedFile>, String> {
    let session = exports
        .0
        .lock()
        .unwrap()
        .take()
        .ok_or("No export is in progress.")?;
    session
        .export
        .finish()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|w| {
            let path = canonical(&w.path)?;
            allowed.allow_saved(path.clone());
            Ok(SavedFile {
                path: path.to_string_lossy().into_owned(),
                size: w.size,
            })
        })
        .collect()
}

/// Abandons the export and removes what it wrote.
#[tauri::command]
pub async fn export_images_cancel(exports: State<'_, ImageExports>) -> Result<(), String> {
    if let Some(session) = exports.0.lock().unwrap().take() {
        session.export.cancel();
    }
    Ok(())
}
