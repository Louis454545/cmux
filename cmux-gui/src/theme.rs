// Design tokens lifted from the macOS SwiftUI sources.
use gpui::{Hsla, Rgba};

pub const ACCENT: Rgba = Rgba { r: 0.0, g: 0.5686, b: 1.0, a: 1.0 };
pub const WINDOW_BG: Rgba = Rgba { r: 0.1137, g: 0.1216, b: 0.1294, a: 1.0 };
pub const TITLEBAR_BG: Rgba = WINDOW_BG;
pub const SIDEBAR_BG: Rgba = Rgba { r: 0.1216, g: 0.1255, b: 0.1373, a: 1.0 };
pub const PANE_HEADER_BG: Rgba = Rgba { r: 0.1451, g: 0.149, b: 0.1686, a: 1.0 };
pub const DIVIDER: Rgba = Rgba { r: 0.0627, g: 0.0667, b: 0.0784, a: 1.0 };
pub const TEXT: Rgba = Rgba { r: 0.902, g: 0.902, b: 0.902, a: 1.0 };
pub const TEXT_DIM: Rgba = Rgba { r: 0.631, g: 0.631, b: 0.631, a: 1.0 };
pub const TEXT_FAINT: Rgba = Rgba { r: 0.431, g: 0.431, b: 0.431, a: 1.0 };
pub const HOVER: Rgba = Rgba { r: 0.1647, g: 0.1725, b: 0.1882, a: 1.0 };
pub const ACTIVE: Rgba = Rgba { r: 0.1922, g: 0.2039, b: 0.2275, a: 1.0 };
pub const TERM_BG: Rgba = Rgba { r: 0.1569, g: 0.1725, b: 0.2039, a: 1.0 };
pub const TERM_FG: Rgba = Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };

pub const CHROME_BAR: f32 = 28.0;
pub const ROW_H: f32 = 28.0;
pub const SIDEBAR_W: f32 = 240.0;
pub const SIDEBAR_MIN: f32 = 120.0;
pub const SIDEBAR_MAX: f32 = 260.0;

pub const FONT_SIZE: f32 = 13.0;
pub const CELL_H: f32 = 17.0; // ceil(13 * 1.3)

#[cfg(target_os = "macos")]
pub const FONT_MONO: &str = "Menlo";
#[cfg(not(target_os = "macos"))]
pub const FONT_MONO: &str = "monospace";

pub fn hex(s: &str) -> Hsla {
    let s = s.trim_start_matches('#');
    let v = u32::from_str_radix(&s[..6.min(s.len())], 16).unwrap_or(0xffffff);
    Rgba {
        r: ((v >> 16) & 0xff) as f32 / 255.0,
        g: ((v >> 8) & 0xff) as f32 / 255.0,
        b: (v & 0xff) as f32 / 255.0,
        a: 1.0,
    }
    .into()
}

pub fn dim(c: Hsla) -> Hsla {
    let mut out = c;
    out.l *= 0.66;
    out
}
