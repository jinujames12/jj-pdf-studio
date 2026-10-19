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
    #[error("Enter at least one page range, for example 1-3, 5, 8-end.")]
    NoRange,
    #[error("\"{0}\" is not a valid page range. Use something like 1-3, 5, 8-end.")]
    BadRange(String),
    #[error("Pages per file must be at least 1.")]
    BadSplitSize,
    #[error("Place at least one split marker between two pages.")]
    NoSplitPoints,
    #[error("Choose at least one file to merge.")]
    NoInputs,
    #[error("Add at least one image.")]
    NoImages,
    #[error("\"{0}\" is not a supported image. Use JPG, PNG, BMP, GIF, TIFF or WebP.")]
    UnsupportedImage(String),
    #[error("\"{0}\" could not be read as an image. The file may be damaged.")]
    BadImage(String),
    #[error("The page width and height must be greater than zero.")]
    BadPageSize,
    #[error("The margin must be zero or more, and small enough to leave room on the page.")]
    BadMargin,
    #[error("Could not read or write the file: {0}")]
    Io(#[from] std::io::Error),
    #[error("The PDF could not be processed: {0}")]
    Pdf(#[from] lopdf::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
