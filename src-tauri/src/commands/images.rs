//! Commands for the Images to PDF tool.

use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use pdf_core::{ImageInput, ImageOptions};
use serde::Serialize;
use tauri::ipc::Response;
use tauri::{AppHandle, State};

use super::pages::{require_pdf_extension, SavedFile};
use super::{canonical, Allowed};
use crate::jobs::{self, Jobs};

const IMAGE_EXTENSIONS: [&str; 9] = [
    "jpg", "jpeg", "png", "bmp", "gif", "tif", "tiff", "webp", "jfif",
];

#[derive(Serialize)]
pub struct OpenedImage {
    /// Size in pixels as displayed (EXIF orientation applied).
    width: u32,
    height: u32,
    /// File size in bytes.
    size: u64,
    /// Last modified time, milliseconds since the Unix epoch (0 if unknown).
    modified: u64,
}

fn require_image_extension(path: &std::path::Path) -> Result<(), String> {
    match path.extension().and_then(|e| e.to_str()) {
        Some(e) if IMAGE_EXTENSIONS.iter().any(|x| e.eq_ignore_ascii_case(x)) => Ok(()),
        _ => Err("Please choose a JPG, PNG, BMP, GIF, TIFF or WebP image.".into()),
    }
}

/// Validates an image the user picked and returns what the list needs to show.
#[tauri::command]
pub async fn open_image(path: PathBuf, allowed: State<'_, Allowed>) -> Result<OpenedImage, String> {
    require_image_extension(&path)?;
    let path = canonical(&path)?;
    let p = path.clone();
    let opened = tauri::async_runtime::spawn_blocking(move || {
        let meta = pdf_core::probe_image(&p).map_err(|e| e.to_string())?;
        let file = std::fs::metadata(&p).map_err(|e| format!("Could not read the file: {e}"))?;
        let modified = file
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis() as u64);
        Ok::<_, String>(OpenedImage {
            width: meta.width,
            height: meta.height,
            size: file.len(),
            modified,
        })
    })
    .await
    .map_err(|e| e.to_string())??;
    allowed.allow_opened(path);
    Ok(opened)
}

/// A small preview (JPEG, or PNG with transparency) with orientation applied.
#[tauri::command]
pub async fn image_thumbnail(
    path: PathBuf,
    max_edge: u32,
    allowed: State<'_, Allowed>,
) -> Result<Response, String> {
    let path = allowed.check_opened(&path)?;
    let edge = max_edge.clamp(32, 640);
    let bytes =
        tauri::async_runtime::spawn_blocking(move || pdf_core::image_thumbnail(&path, edge))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
    Ok(Response::new(bytes))
}

fn check_inputs(inputs: &mut [ImageInput], allowed: &Allowed) -> Result<(), String> {
    for input in inputs {
        input.path = allowed.check_opened(&input.path)?;
    }
    Ok(())
}

/// Converts the images into one PDF.
#[tauri::command]
pub async fn images_to_pdf(
    mut inputs: Vec<ImageInput>,
    output: PathBuf,
    options: ImageOptions,
    app: AppHandle,
    jobs: State<'_, Jobs>,
    job_id: String,
    allowed: State<'_, Allowed>,
) -> Result<SavedFile, String> {
    check_inputs(&mut inputs, &allowed)?;
    require_pdf_extension(&output)?;
    let out = output.clone();
    let size = jobs::run(&app, &jobs, job_id, move |p| {
        pdf_core::images_to_pdf_with(&inputs, &out, &options, p)
    })
    .await?;
    let output = canonical(&output)?;
    allowed.allow_saved(output.clone());
    Ok(SavedFile {
        path: output.to_string_lossy().into_owned(),
        size,
    })
}

/// Converts each image into its own PDF inside `out_dir`.
#[tauri::command]
pub async fn images_to_folder(
    mut inputs: Vec<ImageInput>,
    out_dir: PathBuf,
    options: ImageOptions,
    app: AppHandle,
    jobs: State<'_, Jobs>,
    job_id: String,
    allowed: State<'_, Allowed>,
) -> Result<Vec<SavedFile>, String> {
    check_inputs(&mut inputs, &allowed)?;
    let out_dir = canonical(&out_dir)?;
    if !out_dir.is_dir() {
        return Err("Choose a folder to save the files in.".into());
    }
    let dir = out_dir.clone();
    let written = jobs::run(&app, &jobs, job_id, move |p| {
        pdf_core::images_to_folder_with(&inputs, &dir, &options, p)
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
