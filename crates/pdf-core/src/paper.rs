//! Page sizes and orientation, shared by Images to PDF and Resize.

use serde::Deserialize;

use crate::error::{Error, Result};

/// Points per millimetre (a PDF point is 1/72 inch).
pub const MM: f64 = 72.0 / 25.4;

/// Largest side Acrobat and most viewers accept (200 inches).
pub const MAX_SIDE_PT: f64 = 14_400.0;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Paper {
    A3,
    A4,
    A5,
    Letter,
    Legal,
    /// Any size, in millimetres (the UI converts inches).
    Custom {
        width_mm: f64,
        height_mm: f64,
    },
}

impl Paper {
    /// Size in points with the short side as the width ("portrait").
    pub fn portrait_pt(&self) -> Result<(f64, f64)> {
        let (w, h) = match *self {
            Paper::A3 => (297.0 * MM, 420.0 * MM),
            Paper::A4 => (210.0 * MM, 297.0 * MM),
            Paper::A5 => (148.0 * MM, 210.0 * MM),
            Paper::Letter => (612.0, 792.0),
            Paper::Legal => (612.0, 1008.0),
            Paper::Custom {
                width_mm,
                height_mm,
            } => (width_mm * MM, height_mm * MM),
        };
        let ok = |v: f64| v.is_finite() && v > 0.0 && v <= MAX_SIDE_PT;
        if !(ok(w) && ok(h)) {
            return Err(Error::BadPageSize);
        }
        Ok((w.min(h), w.max(h)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageOrientation {
    /// Follow the content: landscape for wide content, portrait otherwise.
    #[default]
    Auto,
    Portrait,
    Landscape,
}

impl PageOrientation {
    /// Orients a portrait size. `wide` says whether the content is wider than tall.
    pub fn orient(self, (w, h): (f64, f64), wide: bool) -> (f64, f64) {
        let landscape = match self {
            PageOrientation::Auto => wide,
            PageOrientation::Portrait => false,
            PageOrientation::Landscape => true,
        };
        if landscape {
            (h, w)
        } else {
            (w, h)
        }
    }
}

/// Checks a margin in millimetres and returns it in points.
pub(crate) fn margin_pt(margin_mm: f64) -> Result<f64> {
    if margin_mm.is_finite() && margin_mm >= 0.0 {
        Ok(margin_mm * MM)
    } else {
        Err(Error::BadMargin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_portrait_in_points() {
        let (w, h) = Paper::A4.portrait_pt().unwrap();
        assert!((w - 595.28).abs() < 0.01 && (h - 841.89).abs() < 0.01);
        assert_eq!(Paper::Letter.portrait_pt().unwrap(), (612.0, 792.0));
    }

    #[test]
    fn custom_sizes_are_validated_and_normalised() {
        let wide = Paper::Custom {
            width_mm: 300.0,
            height_mm: 100.0,
        };
        let (w, h) = wide.portrait_pt().unwrap();
        assert!(w < h);
        for bad in [(0.0, 10.0), (-5.0, 10.0), (f64::NAN, 10.0), (10.0, 1e9)] {
            let p = Paper::Custom {
                width_mm: bad.0,
                height_mm: bad.1,
            };
            assert!(matches!(p.portrait_pt(), Err(Error::BadPageSize)));
        }
    }

    #[test]
    fn orientation_follows_content_or_is_forced() {
        let size = (100.0, 200.0);
        assert_eq!(PageOrientation::Auto.orient(size, true), (200.0, 100.0));
        assert_eq!(PageOrientation::Auto.orient(size, false), size);
        assert_eq!(PageOrientation::Portrait.orient(size, true), size);
        assert_eq!(
            PageOrientation::Landscape.orient(size, false),
            (200.0, 100.0)
        );
    }

    #[test]
    fn margins_must_be_non_negative_numbers() {
        assert!(margin_pt(0.0).is_ok());
        assert!(margin_pt(-1.0).is_err());
        assert!(margin_pt(f64::INFINITY).is_err());
    }
}
