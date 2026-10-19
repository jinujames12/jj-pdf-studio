//! Command for the Rotate tool. Saving reuses `save_pages`.

use std::path::PathBuf;

use pdf_core::RotateAction;
use tauri::State;

use super::Allowed;

/// Applies one rotate action on top of the turns already chosen and returns
/// the new turns, one per page. Nothing is written.
#[tauri::command]
pub async fn plan_rotation(
    source: PathBuf,
    current: Vec<i32>,
    action: RotateAction,
    allowed: State<'_, Allowed>,
) -> Result<Vec<i32>, String> {
    let source = allowed.check_opened(&source)?;
    tauri::async_runtime::spawn_blocking(move || {
        let info = pdf_core::inspect(&source)?;
        pdf_core::plan_rotation(&info.pages, &current, &action)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}
