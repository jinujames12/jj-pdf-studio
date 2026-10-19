//! Commands for Unlock, Protect and the password prompt.

use std::path::PathBuf;

use pdf_core::{CopyHints, ProtectOptions, ProtectionInfo};
use serde::Serialize;
use tauri::{AppHandle, State};

use super::pages::{require_pdf_extension, SavedFile};
use super::{canonical, Allowed};
use crate::jobs::{self, Jobs};

/// Checks a password typed in the prompt and, when it opens the file, keeps it
/// for this session so every tool can open the file. The password is never
/// written to disk. A wrong password gives a message to show in the prompt.
#[tauri::command]
pub async fn unlock_session(path: PathBuf, password: String) -> Result<(), String> {
    require_pdf_extension(&path)?;
    let path = canonical(&path)?;
    tauri::async_runtime::spawn_blocking(move || pdf_core::unlock_for_session(&path, &password))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// How the opened file is protected, for the Unlock screen.
#[tauri::command]
pub async fn protection_info(
    source: PathBuf,
    allowed: State<'_, Allowed>,
) -> Result<ProtectionInfo, String> {
    let source = allowed.check_opened(&source)?;
    tauri::async_runtime::spawn_blocking(move || pdf_core::inspect_protection(&source))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct UnlockResult {
    pub saved: SavedFile,
    pub protection: ProtectionInfo,
    pub hints: CopyHints,
}

/// Saves an unprotected copy of the opened file; never overwrites the source.
#[tauri::command]
pub async fn unlock_pdf(
    source: PathBuf,
    output: PathBuf,
    app: AppHandle,
    jobs: State<'_, Jobs>,
    job_id: String,
    allowed: State<'_, Allowed>,
) -> Result<UnlockResult, String> {
    let source = allowed.check_opened(&source)?;
    require_pdf_extension(&output)?;
    let out = output.clone();
    let report = jobs::run(&app, &jobs, job_id, move |p| {
        pdf_core::unlock_file_with(&source, &out, None, p)
    })
    .await?;
    let output = canonical(&output)?;
    allowed.allow_saved(output.clone());
    Ok(UnlockResult {
        saved: SavedFile {
            path: output.to_string_lossy().into_owned(),
            size: report.size,
        },
        protection: report.protection,
        hints: report.hints,
    })
}

/// Saves an AES-256 protected copy of the opened file; never overwrites the source.
#[tauri::command]
pub async fn protect_pdf(
    source: PathBuf,
    output: PathBuf,
    options: ProtectOptions,
    app: AppHandle,
    jobs: State<'_, Jobs>,
    job_id: String,
    allowed: State<'_, Allowed>,
) -> Result<SavedFile, String> {
    let source = allowed.check_opened(&source)?;
    require_pdf_extension(&output)?;
    let out = output.clone();
    let size = jobs::run(&app, &jobs, job_id, move |p| {
        pdf_core::protect_file_with(&source, &out, &options, p)
    })
    .await?;
    let output = canonical(&output)?;
    allowed.allow_saved(output.clone());
    Ok(SavedFile {
        path: output.to_string_lossy().into_owned(),
        size,
    })
}
