//! PDF logic for JJ PDF Studio.
//!
//! This crate is deliberately free of any Tauri dependency so it can be unit
//! tested anywhere and reused later (for example by a command-line tool).

pub mod assemble;
pub mod compress;
pub mod document;
pub mod error;
pub mod export;
pub mod images;
pub mod merge;
pub mod paper;
pub mod progress;
pub mod properties;
pub mod ranges;
pub mod resize;
pub mod rotate;
pub mod security;
pub mod split;
pub mod stamp;

pub use assemble::{assemble, same_file, save_pages, save_pages_with, PageOp};
pub use compress::{compress_file, compress_file_with, CompressOptions, CompressReport, Preset};
pub use document::{
    inspect, load, load_with, unlock_for_session, viewable_bytes, DocInfo, PageInfo,
};
pub use error::{Error, Result};
pub use export::ImageExport;
pub use images::{
    images_to_folder, images_to_folder_with, images_to_pdf, images_to_pdf_with,
    probe as probe_image, thumbnail as image_thumbnail, Fit, ImageInput, ImageMeta, ImageOptions,
    Quality,
};
pub use merge::{merge, merge_files, merge_files_with, MergeInput};
pub use paper::{PageOrientation, Paper};
pub use progress::{Progress, Silent};
pub use properties::{
    edit_properties, properties_file, properties_file_with, read_properties, Properties,
    PropertiesEdit,
};
pub use ranges::{parse_pages, parse_ranges};
pub use resize::{plan_resize, resize_file, resize_file_with, Anchor, ResizeMode, ResizeOptions};
pub use rotate::{plan_rotation, Orientation, RotateAction, Target};
pub use security::{
    inspect_protection, protect, protect_file, protect_file_with, unlock_file, unlock_file_with,
    CopyHints, ProtectOptions, Protection, ProtectionInfo, UnlockReport,
};
pub use split::{
    plan as plan_split, split_to_folder, split_to_folder_with, Part, SplitMode, Written,
};
pub use stamp::{
    add_page_numbers, add_watermark, stamp_file, stamp_file_with, stamp_preview, Corner, Layer,
    PageNumberOptions, Place, Stamping, WatermarkKind, WatermarkOptions,
};

/// Version of the core library, taken from `Cargo.toml` at compile time.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_not_empty_and_looks_like_semver() {
        let v = version();
        assert!(!v.is_empty());
        assert_eq!(v.split('.').count(), 3, "expected x.y.z, got {v}");
    }
}
