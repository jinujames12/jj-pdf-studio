//! Commands for the Pages tool: open, preview bytes, save, and follow-ups.

use std::path::PathBuf;
use std::process::Command;

use pdf_core::{DocInfo, PageOp};
use serde::Serialize;
use tauri::ipc::Response;
use tauri::{AppHandle, State};

use super::{canonical, Allowed};
use crate::jobs::{self, Jobs};

#[derive(Serialize)]
pub struct SavedFile {
    pub path: String,
    pub size: u64,
}

pub(super) fn require_pdf_extension(path: &std::path::Path) -> Result<(), String> {
    match path.extension().and_then(|e| e.to_str()) {
        Some(e) if e.eq_ignore_ascii_case("pdf") => Ok(()),
        _ => Err("Please choose a file ending in .pdf.".into()),
    }
}

/// The error `open_pdf` returns for a file that needs a password nobody has
/// typed yet. The frontend recognises it and asks, so it is never shown.
pub const NEEDS_PASSWORD: &str = "PASSWORD_REQUIRED";

/// Validates a PDF the user picked and returns its page geometry.
#[tauri::command]
pub async fn open_pdf(path: PathBuf, allowed: State<'_, Allowed>) -> Result<DocInfo, String> {
    require_pdf_extension(&path)?;
    let path = canonical(&path)?;
    let p = path.clone();
    let info = tauri::async_runtime::spawn_blocking(move || pdf_core::inspect(&p))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| match e {
            pdf_core::Error::Encrypted => NEEDS_PASSWORD.to_string(),
            e => e.to_string(),
        })?;
    allowed.allow_opened(path);
    Ok(info)
}

/// Bytes of an opened PDF for pdf.js (binary IPC, no JSON encoding). Files the
/// user unlocked with a password are decrypted in memory first.
#[tauri::command]
pub async fn read_pdf(path: PathBuf, allowed: State<'_, Allowed>) -> Result<Response, String> {
    let path = allowed.check_opened(&path)?;
    let bytes = tauri::async_runtime::spawn_blocking(move || pdf_core::viewable_bytes(&path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(Response::new(bytes))
}

/// Writes a new PDF made from `ops`; never overwrites the source.
#[tauri::command]
pub async fn save_pages(
    source: PathBuf,
    output: PathBuf,
    ops: Vec<PageOp>,
    app: AppHandle,
    jobs: State<'_, Jobs>,
    job_id: String,
    allowed: State<'_, Allowed>,
) -> Result<SavedFile, String> {
    let source = allowed.check_opened(&source)?;
    require_pdf_extension(&output)?;
    let out = output.clone();
    let size = jobs::run(&app, &jobs, job_id, move |p| {
        pdf_core::save_pages_with(&source, &out, &ops, p)
    })
    .await?;
    let output = canonical(&output)?;
    allowed.allow_saved(output.clone());
    Ok(SavedFile {
        path: output.to_string_lossy().into_owned(),
        size,
    })
}

/// Opens a file the app saved in the default PDF viewer.
#[tauri::command]
pub fn open_saved(path: PathBuf, allowed: State<'_, Allowed>) -> Result<(), String> {
    let path = allowed.check_saved(&path)?;
    // `explorer <file>` opens it with the default handler without a shell.
    Command::new("explorer")
        .arg(path)
        .spawn()
        .map(drop)
        .map_err(|e| format!("Could not open the file: {e}"))
}

/// Shows a file the app saved in Explorer, selected.
#[tauri::command]
pub fn reveal_saved(path: PathBuf, allowed: State<'_, Allowed>) -> Result<(), String> {
    let path = allowed.check_saved(&path)?;
    let mut arg = std::ffi::OsString::from("/select,");
    arg.push(path);
    Command::new("explorer")
        .arg(arg)
        .spawn()
        .map(drop)
        .map_err(|e| format!("Could not open the folder: {e}"))
}

/// The only web addresses the app opens, by name. The frontend never passes a
/// URL, so it cannot be made to launch an arbitrary address.
const LINKS: [(&str, &str); 2] = [
    ("repository", "https://github.com/jjglobal/jj-pdf-studio"),
    ("issues", "https://github.com/jjglobal/jj-pdf-studio/issues"),
];

/// Opens one of the project's pages in the default browser.
#[tauri::command]
pub fn open_link(name: String) -> Result<(), String> {
    let (_, url) = LINKS
        .iter()
        .find(|(n, _)| *n == name)
        .ok_or("Unknown link.")?;
    Command::new("explorer")
        .arg(url)
        .spawn()
        .map(drop)
        .map_err(|e| format!("Could not open the browser: {e}"))
}
