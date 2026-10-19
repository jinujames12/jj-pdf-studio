//! Parsing of page-range text such as `1-3, 5, 8-end`.

use crate::error::{Error, Result};

/// Parses `text` into inclusive 1-based `(first, last)` ranges, in the order
/// typed. `end` means the last page. Pages beyond `page_count` and reversed
/// ranges are rejected so the UI can show the problem live.
pub fn parse_ranges(text: &str, page_count: u32) -> Result<Vec<(u32, u32)>> {
    let mut out = Vec::new();
    for part in text.split([',', ';']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let bad = || Error::BadRange(part.to_string());
        let (a, b) = match part.split_once('-') {
            Some((a, b)) => (
                bound(a, page_count).ok_or_else(bad)?,
                bound(b, page_count).ok_or_else(bad)?,
            ),
            None => {
                let n = bound(part, page_count).ok_or_else(bad)?;
                (n, n)
            }
        };
        if a == 0 || b == 0 || a > b {
            return Err(bad());
        }
        if b > page_count {
            return Err(Error::PageOutOfRange(b));
        }
        out.push((a, b));
    }
    if out.is_empty() {
        return Err(Error::NoRange);
    }
    Ok(out)
}

fn bound(s: &str, page_count: u32) -> Option<u32> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("end") {
        Some(page_count)
    } else {
        s.parse().ok()
    }
}

/// All pages named by `text`, ascending and without duplicates.
pub fn parse_pages(text: &str, page_count: u32) -> Result<Vec<u32>> {
    let mut pages: Vec<u32> = parse_ranges(text, page_count)?
        .into_iter()
        .flat_map(|(a, b)| a..=b)
        .collect();
    pages.sort_unstable();
    pages.dedup();
    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numbers_ranges_and_end() {
        assert_eq!(
            parse_ranges("1-3, 4-8, 9-end", 12).unwrap(),
            [(1, 3), (4, 8), (9, 12)]
        );
        assert_eq!(
            parse_ranges(" 2 ,5-7;END", 9).unwrap(),
            [(2, 2), (5, 7), (9, 9)]
        );
    }

    #[test]
    fn rejects_bad_input_with_the_offending_text() {
        assert!(matches!(parse_ranges("", 5), Err(Error::NoRange)));
        assert!(matches!(parse_ranges(" , ", 5), Err(Error::NoRange)));
        assert!(matches!(parse_ranges("abc", 5), Err(Error::BadRange(s)) if s == "abc"));
        assert!(matches!(parse_ranges("4-2", 5), Err(Error::BadRange(_))));
        assert!(matches!(parse_ranges("0-2", 5), Err(Error::BadRange(_))));
        assert!(matches!(parse_ranges("1--2", 5), Err(Error::BadRange(_))));
        assert!(matches!(
            parse_ranges("3-9", 5),
            Err(Error::PageOutOfRange(9))
        ));
    }

    #[test]
    fn pages_are_sorted_and_deduplicated() {
        assert_eq!(parse_pages("5-6, 2, 6", 8).unwrap(), [2, 5, 6]);
    }
}
