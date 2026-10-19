//! Tauri commands: thin wrappers that validate input and call `pdf-core`.

pub mod compress;
pub mod images;
pub mod merge;
pub mod pages;
pub mod resize;
pub mod rotate;
pub mod split;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Paths the user has handed to the app (picked in a dialog, dropped on the
/// window) or that the app wrote itself. Commands refuse anything else, so the
/// frontend cannot be tricked into reading or launching arbitrary files.
#[derive(Default)]
pub struct Allowed {
    opened: Mutex<HashSet<PathBuf>>,
    saved: Mutex<HashSet<PathBuf>>,
}

fn canonical(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|_| "That file could not be found.".to_string())
}

impl Allowed {
    fn allow_opened(&self, path: PathBuf) {
        self.opened.lock().unwrap().insert(path);
    }
    fn allow_saved(&self, path: PathBuf) {
        self.saved.lock().unwrap().insert(path);
    }
    fn check_opened(&self, path: &Path) -> Result<PathBuf, String> {
        let p = canonical(path)?;
        if self.opened.lock().unwrap().contains(&p) {
            Ok(p)
        } else {
            Err("That file has not been opened in the app.".into())
        }
    }
    fn check_saved(&self, path: &Path) -> Result<PathBuf, String> {
        let p = canonical(path)?;
        if self.saved.lock().unwrap().contains(&p) {
            Ok(p)
        } else {
            Err("That file was not saved by the app.".into())
        }
    }
}
