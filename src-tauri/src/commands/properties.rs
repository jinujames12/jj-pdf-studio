//! Commands for the Properties tool.

use std::path::PathBuf;

use pdf_core::{Properties, PropertiesEdit};
use tauri::{AppHandle, State};

use super::pages::{require_pdf_extension, SavedFile};
use super::{canonical, Allowed};
use crate::jobs::{self, Jobs};

/// The document's metadata and facts, for the Properties screen.
#[tauri::command]
pub async fn read_properties(
    source: PathBuf,
    allowed: State<'_, Allowed>,
) -> Result<Properties, String> {
    let source = allowed.check_opened(&source)?;
    tauri::async_runtime::spawn_blocking(move || pdf_core::read_properties(&source))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Writes a copy with the edited (or removed) metadata; never overwrites the source.
#[tauri::command]
pub async fn save_properties(
    source: PathBuf,
    output: PathBuf,
    edit: PropertiesEdit,
    app: AppHandle,
    jobs: State<'_, Jobs>,
    job_id: String,
    allowed: State<'_, Allowed>,
) -> Result<SavedFile, String> {
    let source = allowed.check_opened(&source)?;
    require_pdf_extension(&output)?;
    let out = output.clone();
    let size = jobs::run(&app, &jobs, job_id, move |p| {
        pdf_core::properties_file_with(&source, &out, &edit, p)
    })
    .await?;
    let output = canonical(&output)?;
    allowed.allow_saved(output.clone());
    Ok(SavedFile {
        path: output.to_string_lossy().into_owned(),
        size,
    })
}
