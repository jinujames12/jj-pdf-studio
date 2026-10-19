//! Commands for Page numbers and Watermark.

use std::path::PathBuf;

use pdf_core::{Stamping, WatermarkKind};
use tauri::ipc::Response;
use tauri::{AppHandle, State};

use super::pages::{require_pdf_extension, SavedFile};
use super::{canonical, Allowed};
use crate::jobs::{self, Jobs};

/// Swaps a watermark picture's path for the canonical path of a file the user
/// picked; any other path is refused.
fn authorize(mut stamping: Stamping, allowed: &Allowed) -> Result<Stamping, String> {
    if let Stamping::Watermark(w) = &mut stamping {
        if let WatermarkKind::Image { path, .. } = &mut w.kind {
            *path = allowed.check_opened(path)?;
        }
    }
    Ok(stamping)
}

/// Writes a stamped copy of the document; never overwrites the source.
#[tauri::command]
pub async fn stamp_pdf(
    source: PathBuf,
    output: PathBuf,
    stamping: Stamping,
    app: AppHandle,
    jobs: State<'_, Jobs>,
    job_id: String,
    allowed: State<'_, Allowed>,
) -> Result<SavedFile, String> {
    let source = allowed.check_opened(&source)?;
    require_pdf_extension(&output)?;
    let stamping = authorize(stamping, &allowed)?;
    let out = output.clone();
    let size = jobs::run(&app, &jobs, job_id, move |p| {
        pdf_core::stamp_file_with(&source, &out, &stamping, p)
    })
    .await?;
    let output = canonical(&output)?;
    allowed.allow_saved(output.clone());
    Ok(SavedFile {
        path: output.to_string_lossy().into_owned(),
        size,
    })
}

/// A one-page PDF showing page `page` (1-based) as it would be saved, for the
/// live preview. Nothing is written to disk.
#[tauri::command]
pub async fn stamp_preview(
    source: PathBuf,
    stamping: Stamping,
    page: u32,
    allowed: State<'_, Allowed>,
) -> Result<Response, String> {
    let source = allowed.check_opened(&source)?;
    let stamping = authorize(stamping, &allowed)?;
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        pdf_core::stamp_preview(&source, &stamping, page)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    Ok(Response::new(bytes))
}
