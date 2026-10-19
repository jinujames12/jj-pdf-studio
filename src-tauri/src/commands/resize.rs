//! Commands for the Resize tool.

use std::path::PathBuf;

use pdf_core::ResizeOptions;
use tauri::State;

use super::pages::{require_pdf_extension, SavedFile};
use super::{canonical, Allowed};

/// New on-screen size `[width, height]` in points of every page the options
/// touch (`null` for pages left alone). Nothing is written.
#[tauri::command]
pub async fn plan_resize(
    source: PathBuf,
    options: ResizeOptions,
    allowed: State<'_, Allowed>,
) -> Result<Vec<Option<(f64, f64)>>, String> {
    let source = allowed.check_opened(&source)?;
    tauri::async_runtime::spawn_blocking(move || {
        let info = pdf_core::inspect(&source)?;
        pdf_core::plan_resize(&info.pages, &options)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// Writes a resized copy of the document; never overwrites the source.
#[tauri::command]
pub async fn resize_pdf(
    source: PathBuf,
    output: PathBuf,
    options: ResizeOptions,
    allowed: State<'_, Allowed>,
) -> Result<SavedFile, String> {
    let source = allowed.check_opened(&source)?;
    require_pdf_extension(&output)?;
    let out = output.clone();
    let size = tauri::async_runtime::spawn_blocking(move || {
        pdf_core::resize_file(&source, &out, &options)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    let output = canonical(&output)?;
    allowed.allow_saved(output.clone());
    Ok(SavedFile {
        path: output.to_string_lossy().into_owned(),
        size,
    })
}
