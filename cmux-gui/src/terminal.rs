// Terminal surface painter: draws styled cell runs via shaped text lines.
use crate::model::SurfaceModel;
use crate::theme;
use gpui::{
    canvas, fill, point, px, size, App, Bounds, FontStyle, FontWeight, Pixels,
    StrikethroughStyle, TextRun, UnderlineStyle, Window,
};

// Attr bits from the wire: 0x1 bold, 0x2 italic, 0x4 strike, 0x8 inverse,
// 0x10 dim, 0x20 invisible, 0x40 blink.
const BOLD: u64 = 0x1;
const ITALIC: u64 = 0x2;
const STRIKE: u64 = 0x4;
const INVERSE: u64 = 0x8;
const DIM: u64 = 0x10;
const INVISIBLE: u64 = 0x20;

pub struct Terminal {
    pub model: SurfaceModel,
    pub cell_w: f32,
    pub surface: u64,
    pub report: std::rc::Rc<dyn Fn(u32, u32)>,
}

impl Clone for Terminal {
    fn clone(&self) -> Self {
        Terminal {
            model: self.model.clone(),
            cell_w: self.cell_w,
            surface: self.surface,
            report: self.report.clone(),
        }
    }
}

pub fn element(
    model: SurfaceModel,
    cell_w: f32,
    surface: u64,
    report: std::rc::Rc<dyn Fn(u32, u32)>,
) -> impl gpui::IntoElement {
    let t = Terminal {
        model,
        cell_w,
        surface,
        report,
    };
    canvas(
        move |_bounds, _window, _cx| t.clone(),
        move |bounds, t, window, cx| paint(bounds, &t, window, cx),
    )
}

fn paint(bounds: Bounds<Pixels>, t: &Terminal, window: &mut Window, cx: &mut App) {
    let m = &t.model;
    // Report the grid this view occupies (shared sizing contract).
    let cols = (f32::from(bounds.size.width) / t.cell_w).floor().max(1.) as u32;
    let rows = (f32::from(bounds.size.height) / theme::CELL_H).floor().max(1.) as u32;
    (t.report)(cols, rows);
    let default_fg = theme::hex(&m.default_fg);
    let default_bg = theme::hex(&m.default_bg);
    let cell_w = px(t.cell_w);
    let cell_h = px(theme::CELL_H);
    let font_size = px(theme::FONT_SIZE);

    window.paint_quad(fill(bounds, default_bg));

    for row in &m.cells {
        let y = bounds.origin.y + cell_h * row.row as f32;
        let mut x = bounds.origin.x;
        for run in &row.runs {
            let mut fg = run.fg.as_deref().map(theme::hex).unwrap_or(default_fg);
            let mut bg = run.bg.as_deref().map(theme::hex).unwrap_or(default_bg);
            if run.attrs & INVERSE != 0 {
                std::mem::swap(&mut fg, &mut bg);
            }
            if run.attrs & DIM != 0 {
                fg = theme::dim(fg);
            }
            let mut text = run.text.clone();
            if run.attrs & INVISIBLE != 0 {
                text = " ".repeat(text.chars().count());
            }
            if text.is_empty() {
                continue;
            }
            let font = gpui::Font {
                family: theme::FONT_MONO.into(),
                weight: if run.attrs & BOLD != 0 {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                },
                style: if run.attrs & ITALIC != 0 {
                    FontStyle::Italic
                } else {
                    FontStyle::Normal
                },
                features: Default::default(),
                fallbacks: None,
            };
            let tr = TextRun {
                len: text.len(),
                font,
                color: fg,
                background_color: Some(bg),
                underline: run.underline.as_deref().map(|u| UnderlineStyle {
                    thickness: px(1.),
                    color: Some(fg),
                    wavy: u == "curly",
                }),
                strikethrough: (run.attrs & STRIKE != 0).then_some(StrikethroughStyle {
                    thickness: px(1.),
                    color: Some(fg),
                }),
            };
            let line = window.text_system().shape_line(
                text.into(),
                font_size,
                &[tr],
                None,
            );
            let w = line.width;
            window.paint_quad(fill(
                Bounds {
                    origin: point(x, y),
                    size: size(w, cell_h),
                },
                bg,
            ));
            let _ = line.paint(point(x, y), cell_h, window, cx);
            x += w;
        }
    }

    if m.cursor.visible {
        let cx0 = bounds.origin.x + cell_w * m.cursor.x as f32;
        let cy0 = bounds.origin.y + cell_h * m.cursor.y as f32;
        let color = m
            .cursor
            .color
            .as_deref()
            .map(theme::hex)
            .unwrap_or(default_fg);
        let rect = match m.cursor.style.as_str() {
            "bar" => Bounds {
                origin: point(cx0, cy0),
                size: size(px(2.), cell_h),
            },
            "underline" => Bounds {
                origin: point(cx0, cy0 + cell_h - px(2.)),
                size: size(cell_w, px(2.)),
            },
            _ => Bounds {
                origin: point(cx0, cy0),
                size: size(cell_w, cell_h),
            },
        };
        window.paint_quad(fill(rect, color.opacity(0.75)));
    }
}
