//! Commands for the Split and Extract tools.

use std::path::PathBuf;

use pdf_core::SplitMode;
use tauri::{AppHandle, State};

use super::pages::SavedFile;
use super::{canonical, Allowed};
use crate::jobs::{self, Jobs};

/// Validates page-range text for the live feedback in the UI.
#[tauri::command]
pub fn parse_page_ranges(text: String, page_count: u32) -> Result<Vec<(u32, u32)>, String> {
    pdf_core::parse_ranges(&text, page_count).map_err(|e| e.to_string())
}

/// Splits a document into several files inside `out_dir`.
#[tauri::command]
pub async fn split_pdf(
    source: PathBuf,
    out_dir: PathBuf,
    mode: SplitMode,
    app: AppHandle,
    jobs: State<'_, Jobs>,
    job_id: String,
    allowed: State<'_, Allowed>,
) -> Result<Vec<SavedFile>, String> {
    let source = allowed.check_opened(&source)?;
    let out_dir = canonical(&out_dir)?;
    if !out_dir.is_dir() {
        return Err("Choose a folder to save the files in.".into());
    }
    let dir = out_dir.clone();
    let written = jobs::run(&app, &jobs, job_id, move |p| {
        pdf_core::split_to_folder_with(&source, &dir, &mode, p)
    })
    .await?;
    written
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
