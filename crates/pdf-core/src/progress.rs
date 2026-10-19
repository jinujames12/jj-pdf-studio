//! Progress reporting and cancellation for long operations.
//!
//! The `*_with` variants of the file operations take a `&dyn Progress`; the
//! plain versions use [`Silent`]. The Tauri shell implements the trait to send
//! progress events to the window and to stop a job when the user presses Cancel.

use crate::error::{Error, Result};

pub trait Progress: Sync {
    /// `done` of `total` steps are finished; `message` says what comes next.
    fn report(&self, _done: u32, _total: u32, _message: &str) {}
    /// True once the user asked to stop.
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Reports nothing and never cancels.
pub struct Silent;

impl Progress for Silent {}

/// Reports a step, or stops with [`Error::Cancelled`] if the user cancelled.
pub(crate) fn step(p: &dyn Progress, done: u32, total: u32, message: &str) -> Result<()> {
    if p.is_cancelled() {
        return Err(Error::Cancelled);
    }
    p.report(done, total, message);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct Flag(AtomicBool);
    impl Progress for Flag {
        fn is_cancelled(&self) -> bool {
            self.0.load(Ordering::Relaxed)
        }
    }

    #[test]
    fn step_stops_once_cancelled() {
        let p = Flag(AtomicBool::new(false));
        assert!(step(&p, 0, 1, "x").is_ok());
        p.0.store(true, Ordering::Relaxed);
        assert!(matches!(step(&p, 0, 1, "x"), Err(Error::Cancelled)));
    }
}
