//! Errors for pdf-core. The `Display` text is shown to users as-is, so every
//! message is written in plain language.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("This PDF is password protected. Enter its password to open it.")]
    Encrypted,
    #[error("That password is not correct.")]
    WrongPassword,
    #[error("That is this file's owner password, which only works for changing its restrictions. Enter the password that opens the file instead.")]
    OwnerPasswordOnly,
    #[error("This PDF has no password or restrictions to remove.")]
    NotProtected,
    #[error("Enter a password to open the protected file.")]
    NoPassword,
    #[error("To restrict printing, copying or editing, enter an owner password that is different from the open password.")]
    BadOwnerPassword,
    #[error("This file looks damaged and could not be opened.")]
    Damaged,
    #[error("Cancelled.")]
    Cancelled,
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
    #[error("The text contains \"{0}\", which the standard PDF font cannot draw. Use letters from the Latin alphabet, digits and common punctuation.")]
    UnsupportedText(char),
    #[error("{0}")]
    BadOption(&'static str),
    #[error("Could not read or write the file: {0}")]
    Io(#[from] std::io::Error),
    #[error("The PDF could not be processed: {0}")]
    Pdf(#[from] lopdf::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
