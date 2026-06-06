use vello::Scene;
use vello::kurbo::{Affine, RoundedRect, Stroke};
use vello::peniko::{Color, Fill};

use crate::layout::LayoutBox;
use crate::text::TextPainter;
use crate::tree::{PropValue, UiNode, UiProp};

const DEFAULT_TEXT_SIZE: f32 = 15.0;
const DEFAULT_BUTTON_SIZE: f32 = 14.0;
const DEFAULT_TEXT_COLOR: Color = Color::from_rgba8(228, 228, 228, 255);
const DEFAULT_BUTTON_BG: Color = Color::from_rgba8(60, 110, 200, 255);
const DEFAULT_BUTTON_COLOR: Color = Color::from_rgba8(255, 255, 255, 255);
const DEFAULT_INPUT_BG: Color = Color::from_rgba8(255, 255, 255, 20);
const DEFAULT_INPUT_BORDER: Color = Color::from_rgba8(200, 200, 200, 120);

// ── Draw state ────────────────────────────────────────────────────────────────

pub struct DrawState<'a> {
    pub cursor: (f64, f64),
    pub focused_input: Option<&'a str>,
    pub is_mouse_down: bool,
}

// ── Prop helpers ──────────────────────────────────────────────────────────────

/// Look up a numeric prop, letting `overlay` override `base`.
fn get_f64_state(base: &[UiProp], overlay: &[UiProp], key: i32) -> Option<f64> {
    overlay
        .iter()
        .find(|p| p.key == key)
        .or_else(|| base.iter().find(|p| p.key == key))
        .and_then(|p| {
            if let PropValue::Number(v) = p.value {
                Some(v)
            } else {
                None
            }
        })
}

/// Decode a packed-RGB f64 (r*65536 + g*256 + b) into a Vello Color.
fn decode_color(v: f64) -> Color {
    let packed = v as u32;
    Color::from_rgb8(
        ((packed >> 16) & 0xFF) as u8,
        ((packed >> 8) & 0xFF) as u8,
        (packed & 0xFF) as u8,
    )
}

fn get_color_state(base: &[UiProp], overlay: &[UiProp], key: i32) -> Option<Color> {
    get_f64_state(base, overlay, key).map(decode_color)
}

fn get_radius_state(base: &[UiProp], overlay: &[UiProp]) -> f64 {
    get_f64_state(base, overlay, 12).unwrap_or(6.0)
}

// ── Per-tag default colors ────────────────────────────────────────────────────

fn container_bg(tag: i32) -> Color {
    match tag {
        2 => Color::from_rgba8(60, 100, 180, 18),
        3 => Color::from_rgba8(60, 160, 80, 18),
        4 => Color::from_rgba8(140, 60, 180, 18),
        5 => Color::from_rgba8(180, 140, 40, 18),
        _ => Color::from_rgba8(100, 100, 100, 12),
    }
}

// ── State prop selection ──────────────────────────────────────────────────────

fn effective_overlay<'a>(node: &'a UiNode, b: &LayoutBox, ds: &DrawState) -> &'a [UiProp] {
    let (cx, cy) = ds.cursor;
    let inside = cx >= b.x && cx < b.x + b.w && cy >= b.y && cy < b.y + b.h;
    if inside && ds.is_mouse_down && !node.active_props.is_empty() {
        return &node.active_props;
    }
    if inside && !node.hover_props.is_empty() {
        return &node.hover_props;
    }
    if let Some(fi) = ds.focused_input
        && node.on_change.as_deref() == Some(fi)
        && !node.focus_props.is_empty()
    {
        return &node.focus_props;
    }
    &[]
}

// ── Draw pass ─────────────────────────────────────────────────────────────────

pub fn draw_tree(
    scene: &mut Scene,
    nodes: &[UiNode],
    boxes: &[LayoutBox],
    tp: Option<&TextPainter>,
    ds: &DrawState,
) {
    for (node, b) in nodes.iter().zip(boxes.iter()) {
        draw_node(scene, node, b, tp, ds);
    }
}

fn draw_node(
    scene: &mut Scene,
    node: &UiNode,
    b: &LayoutBox,
    tp: Option<&TextPainter>,
    ds: &DrawState,
) {
    let ov = effective_overlay(node, b, ds);
    let p = &node.props;

    match node.tag {
        // text — glyphs only
        1 => {
            if let (Some(tp), Some(text)) = (tp, &node.text) {
                let size = get_f64_state(p, ov, 7)
                    .map(|v| v as f32)
                    .unwrap_or(DEFAULT_TEXT_SIZE);
                let color = get_color_state(p, ov, 18).unwrap_or(DEFAULT_TEXT_COLOR);
                tp.draw(scene, text, size, b.x, b.y + b.h * 0.82, color);
            }
        }

        // Button
        15 => {
            let bg = get_color_state(p, ov, 19).unwrap_or(DEFAULT_BUTTON_BG);
            let radius = get_radius_state(p, ov);
            let rect = RoundedRect::new(b.x, b.y, b.x + b.w, b.y + b.h, radius);
            scene.fill(Fill::NonZero, Affine::IDENTITY, bg, None, &rect);
            if let (Some(tp), Some(text)) = (tp, &node.text) {
                let size = get_f64_state(p, ov, 7)
                    .map(|v| v as f32)
                    .unwrap_or(DEFAULT_BUTTON_SIZE);
                let color = get_color_state(p, ov, 18).unwrap_or(DEFAULT_BUTTON_COLOR);
                tp.draw(scene, text, size, b.x + 12.0, b.y + b.h * 0.72, color);
            }
        }

        // divider
        13 => {
            let color = get_color_state(p, ov, 18).unwrap_or(Color::from_rgba8(255, 255, 255, 40));
            let rect = RoundedRect::new(b.x, b.y, b.x + b.w, b.y + 1.0, 0.0);
            scene.fill(Fill::NonZero, Affine::IDENTITY, color, None, &rect);
        }

        // input
        16 => {
            let bg = get_color_state(p, ov, 19).unwrap_or(DEFAULT_INPUT_BG);
            let radius = get_radius_state(p, ov);
            let rect = RoundedRect::new(b.x, b.y, b.x + b.w, b.y + b.h, radius);
            scene.fill(Fill::NonZero, Affine::IDENTITY, bg, None, &rect);
            let border_w = get_f64_state(p, ov, 13).unwrap_or(1.0);
            let border_color = get_color_state(p, ov, 27).unwrap_or(DEFAULT_INPUT_BORDER);
            scene.stroke(
                &Stroke::new(border_w),
                Affine::IDENTITY,
                border_color,
                None,
                &rect,
            );
            if let (Some(tp), Some(text)) = (tp, &node.text) {
                let is_value = node
                    .on_change
                    .as_ref()
                    .map(|_| !text.is_empty())
                    .unwrap_or(false);
                let color = if is_value {
                    get_color_state(p, ov, 18).unwrap_or(Color::from_rgb8(228, 228, 228))
                } else {
                    Color::from_rgba8(180, 180, 180, 140)
                };
                tp.draw(
                    scene,
                    text,
                    DEFAULT_TEXT_SIZE,
                    b.x + 10.0,
                    b.y + b.h * 0.72,
                    color,
                );
            }
        }

        // containers
        _ if !node.children.is_empty() => {
            let bg = get_color_state(p, ov, 19).unwrap_or_else(|| container_bg(node.tag));
            let radius = get_radius_state(p, ov);
            let rect = RoundedRect::new(b.x, b.y, b.x + b.w, b.y + b.h, radius);
            scene.fill(Fill::NonZero, Affine::IDENTITY, bg, None, &rect);
            if let Some(thickness) = get_f64_state(p, ov, 13) {
                let border_color =
                    get_color_state(p, ov, 27).unwrap_or(Color::from_rgba8(255, 255, 255, 40));
                scene.stroke(
                    &Stroke::new(thickness),
                    Affine::IDENTITY,
                    border_color,
                    None,
                    &rect,
                );
            }
            draw_tree(scene, &node.children, &b.children, tp, ds);
        }

        // other leaves
        _ => {
            let bg = get_color_state(p, ov, 19).unwrap_or(Color::from_rgba8(100, 100, 100, 80));
            let radius = get_radius_state(p, ov);
            let rect = RoundedRect::new(b.x, b.y, b.x + b.w, b.y + b.h, radius);
            scene.fill(Fill::NonZero, Affine::IDENTITY, bg, None, &rect);
        }
    }
}
