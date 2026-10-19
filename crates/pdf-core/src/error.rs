//! Errors for pdf-core. The `Display` text is shown to users as-is, so every
//! message is written in plain language.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("This PDF is password protected. Password support is coming in a later version.")]
    Encrypted,
    #[error("This file looks damaged and could not be opened.")]
    Damaged,
    #[error("Choose a different name, the original is kept safe.")]
    OutputIsSource(PathBuf),
    #[error("At least one page must remain.")]
    NoPages,
    #[error("Page {0} does not exist in this document.")]
    PageOutOfRange(u32),
    #[error("Rotation must be a multiple of 90 degrees (got {0}).")]
    BadRotation(i32),
    #[error("A blank page needs a width and height greater than zero.")]
    BadBlankSize,
    #[error("Could not read or write the file: {0}")]
    Io(#[from] std::io::Error),
    #[error("The PDF could not be processed: {0}")]
    Pdf(#[from] lopdf::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
