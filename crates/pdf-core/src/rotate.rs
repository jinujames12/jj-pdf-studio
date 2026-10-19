//! Planning for the Rotate tool. Rotation itself is lossless (`/Rotate`) and is
//! applied by `assemble`; this module only decides how far each page turns.

use serde::Deserialize;

use crate::document::PageInfo;
use crate::error::{Error, Result};
use crate::ranges::parse_pages;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Target {
    All,
    /// Pages 1, 3, 5, ...
    Odd,
    /// Pages 2, 4, 6, ...
    Even,
    /// Page-range text such as `2, 5-7`.
    Pages {
        text: String,
    },
}

impl Target {
    /// One flag per page (`count` pages): whether the target includes it.
    pub fn matches(&self, count: u32) -> Result<Vec<bool>> {
        Ok(match self {
            Target::All => vec![true; count as usize],
            Target::Odd => (1..=count).map(|n| n % 2 == 1).collect(),
            Target::Even => (1..=count).map(|n| n % 2 == 0).collect(),
            Target::Pages { text } => {
                let wanted = parse_pages(text, count)?;
                (1..=count).map(|n| wanted.contains(&n)).collect()
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum RotateAction {
    /// Turn the targeted pages by `degrees` (90, -90 or 180).
    Turn { target: Target, degrees: i32 },
    /// Turn pages whose shape does not match `orientation` by a quarter turn.
    Orient { orientation: Orientation },
}

/// Applies `action` on top of the rotations already chosen (`current`, one per
/// page, extra turns on top of each page's own `/Rotate`) and returns the new
/// list, each entry normalised to 0, 90, 180 or 270.
pub fn plan_rotation(
    pages: &[PageInfo],
    current: &[i32],
    action: &RotateAction,
) -> Result<Vec<i32>> {
    let mut turns: Vec<i32> = (0..pages.len())
        .map(|i| current.get(i).copied().unwrap_or(0).rem_euclid(360))
        .collect();
    match action {
        RotateAction::Turn { target, degrees } => {
            if degrees % 90 != 0 {
                return Err(Error::BadRotation(*degrees));
            }
            let hit = target.matches(pages.len() as u32)?;
            for (turn, _) in turns.iter_mut().zip(&hit).filter(|(_, h)| **h) {
                *turn = (*turn + degrees).rem_euclid(360);
            }
        }
        RotateAction::Orient { orientation } => {
            for (turn, page) in turns.iter_mut().zip(pages) {
                let sideways = (page.rotate + *turn) % 180 != 0;
                let (w, h) = if sideways {
                    (page.height, page.width)
                } else {
                    (page.width, page.height)
                };
                let wrong = match orientation {
                    Orientation::Portrait => w > h,
                    Orientation::Landscape => h > w,
                };
                if wrong {
                    *turn = (*turn + 90) % 360;
                }
            }
        }
    }
    Ok(turns)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(width: f64, height: f64, rotate: i32) -> PageInfo {
        PageInfo {
            width,
            height,
            rotate,
        }
    }

    fn turn(target: Target, degrees: i32) -> RotateAction {
        RotateAction::Turn { target, degrees }
    }

    #[test]
    fn odd_even_and_page_targets() {
        let pages = vec![page(1.0, 2.0, 0); 4];
        let odd = plan_rotation(&pages, &[], &turn(Target::Odd, 90)).unwrap();
        assert_eq!(odd, [90, 0, 90, 0]);
        let even = plan_rotation(&pages, &odd, &turn(Target::Even, -90)).unwrap();
        assert_eq!(even, [90, 270, 90, 270]);
        let some = Target::Pages { text: "2-3".into() };
        let r = plan_rotation(&pages, &[], &turn(some, 180)).unwrap();
        assert_eq!(r, [0, 180, 180, 0]);
    }

    #[test]
    fn rejects_odd_angles_and_bad_ranges() {
        let pages = vec![page(1.0, 2.0, 0); 2];
        assert!(matches!(
            plan_rotation(&pages, &[], &turn(Target::All, 45)),
            Err(Error::BadRotation(45))
        ));
        let bad = Target::Pages { text: "9".into() };
        assert!(plan_rotation(&pages, &[], &turn(bad, 90)).is_err());
    }

    #[test]
    fn orientation_takes_existing_rotation_into_account() {
        // Portrait, landscape, and portrait media turned sideways by /Rotate.
        let pages = vec![
            page(100.0, 200.0, 0),
            page(200.0, 100.0, 0),
            page(100.0, 200.0, 90),
        ];
        let portrait = RotateAction::Orient {
            orientation: Orientation::Portrait,
        };
        assert_eq!(plan_rotation(&pages, &[], &portrait).unwrap(), [0, 90, 90]);
        let landscape = RotateAction::Orient {
            orientation: Orientation::Landscape,
        };
        assert_eq!(plan_rotation(&pages, &[], &landscape).unwrap(), [90, 0, 0]);
        // Pending turns count: page 1 already turned sideways is landscape.
        assert_eq!(plan_rotation(&pages[..1], &[90], &landscape).unwrap(), [90]);
    }
}
