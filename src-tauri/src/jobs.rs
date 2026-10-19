//! Background jobs: progress events and cancellation.
//!
//! A command that can take a while gets a `job_id` from the window, runs its
//! work on a blocking thread and passes a [`JobCtx`] to `pdf-core`. The context
//! sends `progress` events to the window and reports the Cancel button's flag.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use pdf_core::Progress;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

/// The cancel flags of the jobs that are running now, by job id.
#[derive(Default)]
pub struct Jobs(Mutex<HashMap<String, Arc<AtomicBool>>>);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent<'a> {
    job_id: &'a str,
    done: u32,
    total: u32,
    message: &'a str,
}

pub struct JobCtx {
    app: AppHandle,
    id: String,
    cancel: Arc<AtomicBool>,
}

impl Progress for JobCtx {
    fn report(&self, done: u32, total: u32, message: &str) {
        let _ = self.app.emit(
            "progress",
            ProgressEvent {
                job_id: &self.id,
                done,
                total,
                message,
            },
        );
    }

    fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

/// Runs `work` on a blocking thread as job `job_id`. Errors come back as the
/// plain-language text shown to the user ("Cancelled." after Cancel).
pub async fn run<T, F>(app: &AppHandle, jobs: &Jobs, job_id: String, work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&JobCtx) -> pdf_core::Result<T> + Send + 'static,
{
    let cancel = Arc::new(AtomicBool::new(false));
    jobs.0
        .lock()
        .unwrap()
        .insert(job_id.clone(), Arc::clone(&cancel));
    let ctx = JobCtx {
        app: app.clone(),
        id: job_id.clone(),
        cancel,
    };
    let joined = tauri::async_runtime::spawn_blocking(move || work(&ctx)).await;
    jobs.0.lock().unwrap().remove(&job_id);
    joined
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Asks a running job to stop at its next checkpoint.
#[tauri::command]
pub fn cancel_job(job_id: String, jobs: State<'_, Jobs>) {
    if let Some(flag) = jobs.0.lock().unwrap().get(&job_id) {
        flag.store(true, Ordering::Relaxed);
    }
}
