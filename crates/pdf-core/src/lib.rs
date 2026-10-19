//! PDF logic for JJ PDF Studio.
//!
//! This crate is deliberately free of any Tauri dependency so it can be unit
//! tested anywhere and reused later (for example by a command-line tool).

pub mod assemble;
pub mod document;
pub mod error;
pub mod images;
pub mod merge;
pub mod paper;
pub mod ranges;
pub mod resize;
pub mod rotate;
pub mod split;

pub use assemble::{assemble, same_file, save_pages, PageOp};
pub use document::{inspect, load, DocInfo, PageInfo};
pub use error::{Error, Result};
pub use images::{
    images_to_folder, images_to_pdf, probe as probe_image, thumbnail as image_thumbnail, Fit,
    ImageInput, ImageMeta, ImageOptions, Quality,
};
pub use merge::{merge, merge_files, MergeInput};
pub use paper::{PageOrientation, Paper};
pub use ranges::{parse_pages, parse_ranges};
pub use resize::{plan_resize, resize_file, Anchor, ResizeMode, ResizeOptions};
pub use rotate::{plan_rotation, Orientation, RotateAction, Target};
pub use split::{plan as plan_split, split_to_folder, Part, SplitMode, Written};

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
