//! Command for the Merge tool.

use std::path::PathBuf;

use pdf_core::MergeInput;
use tauri::State;

use super::pages::{require_pdf_extension, SavedFile};
use super::{canonical, Allowed};

/// Merges the listed files (or chosen pages of them) into a new PDF.
#[tauri::command]
pub async fn merge_pdfs(
    mut inputs: Vec<MergeInput>,
    output: PathBuf,
    bookmarks: bool,
    allowed: State<'_, Allowed>,
) -> Result<SavedFile, String> {
    for input in &mut inputs {
        input.path = allowed.check_opened(&input.path)?;
    }
    require_pdf_extension(&output)?;
    let out = output.clone();
    let size = tauri::async_runtime::spawn_blocking(move || {
        pdf_core::merge_files(&inputs, &out, bookmarks)
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
