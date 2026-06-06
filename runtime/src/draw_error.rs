use vello::Scene;
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Color, Fill};

use crate::text::TextPainter;

const OVERLAY_BG: Color = Color::from_rgba8(12, 12, 18, 255);
const HEADER_BG: Color = Color::from_rgba8(180, 35, 35, 255);
const TEXT_ERROR: Color = Color::from_rgba8(255, 110, 110, 255);
const TEXT_NORMAL: Color = Color::from_rgba8(210, 210, 220, 255);
const TEXT_DIM: Color = Color::from_rgba8(130, 130, 150, 255);
const TEXT_HELP: Color = Color::from_rgba8(130, 200, 120, 255);
const LINE_H: f64 = 22.0;
const PAD: f64 = 40.0;
const HEADER_H: f64 = 52.0;

pub fn draw_error(
    scene: &mut Scene,
    error_text: &str,
    tp: Option<&TextPainter>,
    width: f64,
    height: f64,
) {
    scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        OVERLAY_BG,
        None,
        &Rect::new(0.0, 0.0, width, height),
    );
    scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        HEADER_BG,
        None,
        &Rect::new(0.0, 0.0, width, HEADER_H),
    );

    let Some(tp) = tp else { return };

    tp.draw(
        scene,
        "Vel — Error",
        16.0,
        PAD,
        HEADER_H * 0.72,
        Color::from_rgb8(255, 255, 255),
    );

    let mut y = HEADER_H + PAD;
    for line in error_text.lines() {
        if y > height - PAD {
            break;
        }
        let color = if line.starts_with("error:") {
            TEXT_ERROR
        } else if line.starts_with("help:") {
            TEXT_HELP
        } else if line.trim_start().starts_with("-->") || line.trim_start().starts_with('|') {
            TEXT_DIM
        } else {
            TEXT_NORMAL
        };
        tp.draw(scene, line, 13.5, PAD, y + LINE_H * 0.8, color);
        y += LINE_H;
    }
}
