//! Split and "extract one file per page": cut a document into several files.

use crate::progress::{step, Progress, Silent};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::assemble::{assemble, write_atomic, PageOp};
use crate::document;
use crate::error::{Error, Result};
use crate::ranges::{parse_pages, parse_ranges};

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SplitMode {
    /// A new file every `n` pages.
    Every { n: u32 },
    /// One file per range of the typed text, e.g. `1-3, 4-8, 9-end`.
    Ranges { text: String },
    /// Cut after each listed page number.
    At { after: Vec<u32> },
    /// One file per listed page (used by Extract).
    Pages { pages: Vec<u32> },
}

/// One output file: the source pages it holds and the suffix of its file name.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub label: String,
    pub pages: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Written {
    pub path: PathBuf,
    pub size: u64,
}

fn numbered(parts: Vec<Vec<u32>>) -> Vec<Part> {
    let width = parts.len().to_string().len().max(2);
    parts
        .into_iter()
        .enumerate()
        .map(|(i, pages)| Part {
            label: format!("part{:0width$}", i + 1),
            pages,
        })
        .collect()
}

/// Decides which source pages go into which file, without touching any PDF.
pub fn plan(mode: &SplitMode, page_count: u32) -> Result<Vec<Part>> {
    match mode {
        SplitMode::Every { n } => {
            if *n == 0 {
                return Err(Error::BadSplitSize);
            }
            let all: Vec<u32> = (1..=page_count).collect();
            Ok(numbered(
                all.chunks(*n as usize).map(<[u32]>::to_vec).collect(),
            ))
        }
        SplitMode::Ranges { text } => Ok(parse_ranges(text, page_count)?
            .into_iter()
            .map(|(a, b)| Part {
                label: if a == b {
                    format!("p{a}")
                } else {
                    format!("p{a}-{b}")
                },
                pages: (a..=b).collect(),
            })
            .collect()),
        SplitMode::At { after } => {
            let mut cuts: Vec<u32> = after
                .iter()
                .copied()
                .filter(|p| (1..page_count).contains(p))
                .collect();
            cuts.sort_unstable();
            cuts.dedup();
            if cuts.is_empty() {
                return Err(Error::NoSplitPoints);
            }
            let mut parts = Vec::new();
            let mut start = 1;
            for cut in cuts.into_iter().chain([page_count]) {
                parts.push((start..=cut).collect());
                start = cut + 1;
            }
            Ok(numbered(parts))
        }
        SplitMode::Pages { pages } => {
            let text = pages
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",");
            Ok(parse_pages(&text, page_count)?
                .into_iter()
                .map(|p| Part {
                    label: format!("p{p}"),
                    pages: vec![p],
                })
                .collect())
        }
    }
}

/// `dir/stem.pdf`, or `dir/stem (2).pdf` and so on if that name is taken, so
/// a split never overwrites anything.
pub(crate) fn unique_path(dir: &Path, stem: &str) -> PathBuf {
    let mut path = dir.join(format!("{stem}.pdf"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem} ({n}).pdf"));
        n += 1;
    }
    path
}

/// Splits `source` into files inside `out_dir`, named `<name>_<label>.pdf`.
/// If anything fails, the files already written by this call are removed.
pub fn split_to_folder(source: &Path, out_dir: &Path, mode: &SplitMode) -> Result<Vec<Written>> {
    split_to_folder_with(source, out_dir, mode, &Silent)
}

/// Like [`split_to_folder`], reporting progress and stopping if cancelled
/// (the files already written are removed).
pub fn split_to_folder_with(
    source: &Path,
    out_dir: &Path,
    mode: &SplitMode,
    progress: &dyn Progress,
) -> Result<Vec<Written>> {
    let doc = document::load(source)?;
    let page_count = doc.get_pages().len() as u32;
    let parts = plan(mode, page_count)?;
    let stem = source
        .file_stem()
        .map_or_else(|| "document".into(), |s| s.to_string_lossy().into_owned());

    let mut written: Vec<Written> = Vec::with_capacity(parts.len());
    let result = (|| {
        for (i, part) in parts.iter().enumerate() {
            step(
                progress,
                i as u32,
                parts.len() as u32,
                &format!("Writing file {} of {}", i + 1, parts.len()),
            )?;
            let ops: Vec<PageOp> = part
                .pages
                .iter()
                .map(|&page| PageOp::Page { page, rotate: 0 })
                .collect();
            let mut piece = doc.clone();
            assemble(&mut piece, &ops)?;
            let path = unique_path(out_dir, &format!("{stem}_{}", part.label));
            let size = write_atomic(&mut piece, &path)?;
            written.push(Written { path, size });
        }
        Ok(())
    })();
    if let Err(e) = result {
        for w in &written {
            let _ = std::fs::remove_file(&w.path);
        }
        return Err(e);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pages(parts: &[Part]) -> Vec<Vec<u32>> {
        parts.iter().map(|p| p.pages.clone()).collect()
    }

    #[test]
    fn every_n_pages_keeps_a_short_last_file() {
        let parts = plan(&SplitMode::Every { n: 2 }, 5).unwrap();
        assert_eq!(pages(&parts), [vec![1, 2], vec![3, 4], vec![5]]);
        assert_eq!(parts[0].label, "part01");
        assert!(matches!(
            plan(&SplitMode::Every { n: 0 }, 5),
            Err(Error::BadSplitSize)
        ));
    }

    #[test]
    fn labels_are_wide_enough_for_many_files() {
        let parts = plan(&SplitMode::Every { n: 1 }, 120).unwrap();
        assert_eq!(parts[0].label, "part001");
        assert_eq!(parts[119].label, "part120");
    }

    #[test]
    fn ranges_name_their_pages() {
        let mode = SplitMode::Ranges {
            text: "1-3, 5, 6-end".into(),
        };
        let parts = plan(&mode, 8).unwrap();
        let labels: Vec<_> = parts.iter().map(|p| p.label.as_str()).collect();
        assert_eq!(labels, ["p1-3", "p5", "p6-8"]);
        assert_eq!(parts[2].pages, [6, 7, 8]);
    }

    #[test]
    fn markers_cut_after_the_given_pages() {
        let mode = SplitMode::At {
            after: vec![5, 2, 2, 9],
        };
        let parts = plan(&mode, 6).unwrap();
        assert_eq!(pages(&parts), [vec![1, 2], vec![3, 4, 5], vec![6]]);
        let none = SplitMode::At { after: vec![6] };
        assert!(matches!(plan(&none, 6), Err(Error::NoSplitPoints)));
    }

    #[test]
    fn per_page_mode_sorts_and_validates() {
        let mode = SplitMode::Pages {
            pages: vec![4, 2, 4],
        };
        let parts = plan(&mode, 5).unwrap();
        assert_eq!(pages(&parts), [vec![2], vec![4]]);
        let bad = SplitMode::Pages { pages: vec![7] };
        assert!(plan(&bad, 5).is_err());
        let empty = SplitMode::Pages { pages: vec![] };
        assert!(matches!(plan(&empty, 5), Err(Error::NoRange)));
    }
}
