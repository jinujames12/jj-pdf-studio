//! Command for the Merge tool.

use std::path::PathBuf;

use pdf_core::MergeInput;
use tauri::{AppHandle, State};

use super::pages::{require_pdf_extension, SavedFile};
use super::{canonical, Allowed};
use crate::jobs::{self, Jobs};

/// Merges the listed files (or chosen pages of them) into a new PDF.
#[tauri::command]
pub async fn merge_pdfs(
    mut inputs: Vec<MergeInput>,
    output: PathBuf,
    bookmarks: bool,
    app: AppHandle,
    jobs: State<'_, Jobs>,
    job_id: String,
    allowed: State<'_, Allowed>,
) -> Result<SavedFile, String> {
    for input in &mut inputs {
        input.path = allowed.check_opened(&input.path)?;
    }
    require_pdf_extension(&output)?;
    let out = output.clone();
    let size = jobs::run(&app, &jobs, job_id, move |p| {
        pdf_core::merge_files_with(&inputs, &out, bookmarks, p)
    })
    .await?;
    let output = canonical(&output)?;
    allowed.allow_saved(output.clone());
    Ok(SavedFile {
        path: output.to_string_lossy().into_owned(),
        size,
    })
}
