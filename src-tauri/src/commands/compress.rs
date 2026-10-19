//! Command for the Compress tool.

use std::path::PathBuf;

use pdf_core::{CompressOptions, CompressReport};
use serde::Serialize;
use tauri::State;

use super::pages::require_pdf_extension;
use super::{canonical, Allowed};

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
    allowed: State<'_, Allowed>,
) -> Result<Compressed, String> {
    let source = allowed.check_opened(&source)?;
    require_pdf_extension(&output)?;
    let out = output.clone();
    let report = tauri::async_runtime::spawn_blocking(move || {
        pdf_core::compress_file(&source, &out, &options)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
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
