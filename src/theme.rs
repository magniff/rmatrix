//! Color themes and the gradient that turns a trail position into a color.
//!
//! This is the heart of what makes the rain look richer than cmatrix: instead
//! of a couple of flat green attributes, every cell gets a true-color (24-bit)
//! value interpolated along a glow curve — a bright, near-white head bleeding
//! down into the theme color and finally into darkness at the tail.

use crossterm::style::Color;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Green,
    Cyan,
    Amber,
    Purple,
    Rainbow,
}

impl FromStr for Theme {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "green" | "matrix" => Ok(Theme::Green),
            "cyan" | "blue" => Ok(Theme::Cyan),
            "amber" | "orange" | "gold" => Ok(Theme::Amber),
            "purple" | "magenta" | "violet" => Ok(Theme::Purple),
            "rainbow" | "rgb" => Ok(Theme::Rainbow),
            other => Err(format!("unknown theme '{other}'")),
        }
    }
}

impl Theme {
    /// The base (fully-lit, non-head) RGB color for this theme.
    fn base(self, col: u16, total_cols: u16) -> (u8, u8, u8) {
        match self {
            Theme::Green => (40, 255, 90),
            Theme::Cyan => (40, 220, 255),
            Theme::Amber => (255, 176, 40),
            Theme::Purple => (200, 70, 255),
            Theme::Rainbow => {
                // Hue swept across the screen width so columns differ smoothly.
                let h = if total_cols > 0 {
                    (col as f32 / total_cols as f32) * 360.0
                } else {
                    0.0
                };
                hsv_to_rgb(h, 0.85, 1.0)
            }
        }
    }

    /// Color for a cell.
    ///
    /// * `brightness` in `0.0..=1.0` is how lit the cell is (1.0 at the head).
    /// * `is_head` gets the hot, near-white leading glyph.
    pub fn color(self, brightness: f32, is_head: bool, col: u16, total_cols: u16) -> Color {
        let (br, bg, bb) = self.base(col, total_cols);

        if is_head {
            // Hot head: blow the base color out toward white for a glow/bloom.
            let mix = 0.75; // how far toward white
            let r = lerp(br as f32, 255.0, mix);
            let g = lerp(bg as f32, 255.0, mix);
            let b = lerp(bb as f32, 255.0, mix);
            return Color::Rgb {
                r: r as u8,
                g: g as u8,
                b: b as u8,
            };
        }

        // Gamma-shaped falloff makes the trail linger bright then drop off fast,
        // which reads as a glow rather than a linear ramp.
        let t = brightness.clamp(0.0, 1.0).powf(1.6);
        Color::Rgb {
            r: (br as f32 * t) as u8,
            g: (bg as f32 * t) as u8,
            b: (bb as f32 * t) as u8,
        }
    }
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Standard HSV → RGB. `h` in degrees `[0,360)`, `s`/`v` in `[0,1]`.
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let h2 = (h / 60.0).rem_euclid(6.0);
    let x = c * (1.0 - (h2 % 2.0 - 1.0).abs());
    let (r, g, b) = match h2 as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
