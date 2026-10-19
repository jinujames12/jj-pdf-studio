//! Command for the Compress tool.

use std::path::PathBuf;

use pdf_core::{CompressOptions, CompressReport};
use serde::Serialize;
use tauri::{AppHandle, State};

use super::pages::require_pdf_extension;
use super::{canonical, Allowed};
use crate::jobs::{self, Jobs};

/// Outcome of a compression run. `path` is set only when a smaller file was
/// written; otherwise the original should simply be kept.
#[derive(Serialize)]
pub struct Compressed {
    pub path: Option<String>,
    #[serde(flatten)]
    pub report: CompressReport,
}

/// Writes a compressed copy of the document. Never overwrites the source, and
/// writes nothing at all when the result would not be smaller.
#[tauri::command]
pub async fn compress_pdf(
    source: PathBuf,
    output: PathBuf,
    options: CompressOptions,
    app: AppHandle,
    jobs: State<'_, Jobs>,
    job_id: String,
    allowed: State<'_, Allowed>,
) -> Result<Compressed, String> {
    let source = allowed.check_opened(&source)?;
    require_pdf_extension(&output)?;
    let out = output.clone();
    let report = jobs::run(&app, &jobs, job_id, move |p| {
        pdf_core::compress_file_with(&source, &out, &options, p)
    })
    .await?;
    if !report.written {
        return Ok(Compressed { path: None, report });
    }
    let output = canonical(&output)?;
    allowed.allow_saved(output.clone());
    Ok(Compressed {
        path: Some(output.to_string_lossy().into_owned()),
        report,
    })
}
