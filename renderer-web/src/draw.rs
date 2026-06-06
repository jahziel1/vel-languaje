use crate::gpu::{floats_as_bytes, GpuState, MAX_RECTS, RECT_FLOATS};
use crate::layout::{layout_root, LayoutBox};
use crate::tree::{packed_to_css, prop_bool, prop_f32, unpack_color, FrameInput, UiNode};
use wasm_bindgen::prelude::*;
use web_sys::OffscreenCanvas;

pub struct Renderer {
    pub gpu: GpuState,
    instances: Vec<f32>,
    text_calls: Vec<TextCall>,
}

struct TextCall {
    text: String,
    font: String,
    color: String,
    x: f32,
    y: f32,
}

impl Renderer {
    pub fn new(gpu: GpuState) -> Self {
        Self {
            gpu,
            instances: Vec::with_capacity(MAX_RECTS * RECT_FLOATS),
            text_calls: Vec::new(),
        }
    }

    pub fn render_frame(&mut self, input: &FrameInput, cw: f32, ch: f32) {
        let pw = (cw * web_sys::window().map(|w| w.device_pixel_ratio()).unwrap_or(1.0) as f32) as u32;
        let ph = (ch * web_sys::window().map(|w| w.device_pixel_ratio()).unwrap_or(1.0) as f32) as u32;
        if pw == 0 || ph == 0 {
            return;
        }

        self.gpu.resize(pw, ph);
        self.instances.clear();
        self.text_calls.clear();

        let boxes = layout_root(&input.nodes, cw);
        for (i, node) in input.nodes.iter().enumerate() {
            self.collect_node(node, &boxes[i], input);
        }

        self.flush(pw, ph, cw, ch);
    }

    pub fn render_error(&mut self, msg: &str, cw: f32, ch: f32) {
        let pw = (cw * dpr()) as u32;
        let ph = (ch * dpr()) as u32;
        if pw == 0 || ph == 0 {
            return;
        }
        self.gpu.resize(pw, ph);
        self.instances.clear();
        self.text_calls.clear();

        // Dark red background rect
        self.push_rect(0.0, 0.0, cw, ch, [0.07, 0.0, 0.0, 1.0], 0.0, 0.0, [0.0; 4]);

        self.text_calls.push(TextCall {
            text: "Vel Error".into(),
            font: "bold 16px monospace".into(),
            color: "#ff4444".into(),
            x: 20.0,
            y: 44.0,
        });
        for (i, line) in msg.lines().enumerate() {
            self.text_calls.push(TextCall {
                text: line.to_string(),
                font: "13px monospace".into(),
                color: "#ffaaaa".into(),
                x: 20.0,
                y: 72.0 + i as f32 * 20.0,
            });
        }

        self.flush(pw, ph, cw, ch);
    }

    fn collect_node(&mut self, node: &UiNode, b: &LayoutBox, input: &FrameInput) {
        if prop_bool(&node.props, "visible") == false
            && node.props.get("visible").is_some()
        {
            return;
        }

        let p = node.effective_props(
            b.x, b.y, b.w, b.h,
            input.mouse_x, input.mouse_y,
            input.mouse_down,
            &input.active_input,
        );
        let opacity = prop_f32(p, "opacity").unwrap_or(1.0);
        let tag = node.tag;

        match tag {
            1 => {
                // text leaf
                let sz = prop_f32(p, "fontSize").unwrap_or(15.0);
                let bold = prop_bool(p, "bold")
                    || prop_f32(p, "fontWeight").unwrap_or(0.0) >= 600.0;
                let ital = prop_bool(p, "italic");
                let font = format!(
                    "{}{}{}px system-ui,sans-serif",
                    if ital { "italic " } else { "" },
                    if bold { "bold " } else { "" },
                    sz as u32
                );
                let color = p.get("color")
                    .and_then(|v| v.as_f64())
                    .map(|c| packed_to_css(c as f32, opacity))
                    .unwrap_or_else(|| format!("rgba(228,228,228,{opacity})"));
                if let Some(text) = &node.text {
                    self.text_calls.push(TextCall {
                        text: text.clone(),
                        font,
                        color,
                        x: b.x,
                        y: b.y + sz,
                    });
                }
            }
            13 => {
                // divider
                let fill = p.get("color")
                    .and_then(|v| v.as_f64())
                    .map(|c| unpack_color(c as f32, opacity))
                    .unwrap_or([1.0, 1.0, 1.0, 0.16 * opacity]);
                self.push_rect(b.x, b.y, b.w, 1.0, fill, 0.0, 0.0, [0.0; 4]);
            }
            15 => {
                // Button background
                let fill = p.get("background")
                    .and_then(|v| v.as_f64())
                    .map(|c| unpack_color(c as f32, opacity))
                    .unwrap_or([60.0 / 255.0, 110.0 / 255.0, 200.0 / 255.0, opacity]);
                let radius = prop_f32(p, "radius").unwrap_or(6.0);
                self.push_rect(b.x, b.y, b.w, b.h, fill, radius, 0.0, [0.0; 4]);
                // Button label
                if let Some(text) = &node.text {
                    let sz = prop_f32(p, "fontSize").unwrap_or(14.0);
                    let color = p.get("color")
                        .and_then(|v| v.as_f64())
                        .map(|c| packed_to_css(c as f32, opacity))
                        .unwrap_or_else(|| format!("rgba(255,255,255,{opacity})"));
                    self.text_calls.push(TextCall {
                        text: text.clone(),
                        font: format!("{}px system-ui,sans-serif", sz as u32),
                        color,
                        x: b.x + 12.0,
                        y: b.y + b.h * 0.72,
                    });
                }
            }
            16 => {
                // input
                let fill = p.get("background")
                    .and_then(|v| v.as_f64())
                    .map(|c| unpack_color(c as f32, opacity))
                    .unwrap_or([1.0, 1.0, 1.0, 0.08 * opacity]);
                let radius = prop_f32(p, "radius").unwrap_or(6.0);
                let bc = p.get("borderColor")
                    .and_then(|v| v.as_f64())
                    .map(|c| unpack_color(c as f32, 0.47 * opacity))
                    .unwrap_or([200.0 / 255.0, 200.0 / 255.0, 200.0 / 255.0, 0.47 * opacity]);
                self.push_rect(b.x, b.y, b.w, b.h, fill, radius, 1.0, bc);
                // input label / cursor
                if let Some(text) = &node.text {
                    let color = if node.has_value {
                        format!("rgba(228,228,228,{opacity})")
                    } else {
                        format!("rgba(180,180,180,{})", 0.55 * opacity)
                    };
                    self.text_calls.push(TextCall {
                        text: text.clone(),
                        font: "15px system-ui,sans-serif".into(),
                        color,
                        x: b.x + 10.0,
                        y: b.y + b.h * 0.72,
                    });
                }
                // cursor line — drawn as a thin rect
                if node.on_change.as_deref() == Some(input.active_input.as_str()) {
                    // Estimate cursor position (approximation without actual text measure)
                    let char_w = 8.5;
                    let cursor_x = b.x + 10.0 + input.input_buffer.len() as f32 * char_w;
                    self.push_rect(
                        cursor_x, b.y + 8.0, 1.5, b.h - 16.0,
                        [0.9, 0.9, 0.9, 0.9 * opacity], 0.0, 0.0, [0.0; 4],
                    );
                }
            }
            _ if !node.children.is_empty() => {
                // container
                let fill = p.get("background")
                    .and_then(|v| v.as_f64())
                    .map(|c| unpack_color(c as f32, opacity))
                    .unwrap_or(container_bg(tag, opacity));
                let radius = prop_f32(p, "radius").unwrap_or(0.0);
                let border_w = prop_f32(p, "border").unwrap_or(0.0);
                let bc = p.get("borderColor")
                    .and_then(|v| v.as_f64())
                    .map(|c| unpack_color(c as f32, opacity))
                    .unwrap_or([1.0, 1.0, 1.0, 0.16 * opacity]);
                self.push_rect(b.x, b.y, b.w, b.h, fill, radius, border_w, bc);
                for (i, child) in node.children.iter().enumerate() {
                    self.collect_node(child, &b.children[i], input);
                }
            }
            _ => {
                // other leaf
                let fill = p.get("background")
                    .and_then(|v| v.as_f64())
                    .map(|c| unpack_color(c as f32, opacity))
                    .unwrap_or([100.0 / 255.0, 100.0 / 255.0, 100.0 / 255.0, 0.31 * opacity]);
                let radius = prop_f32(p, "radius").unwrap_or(6.0);
                self.push_rect(b.x, b.y, b.w, b.h, fill, radius, 0.0, [0.0; 4]);
            }
        }
    }

    fn push_rect(
        &mut self,
        x: f32, y: f32, w: f32, h: f32,
        fill: [f32; 4],
        radius: f32, border_w: f32, bc: [f32; 4],
    ) {
        if self.instances.len() / RECT_FLOATS >= MAX_RECTS {
            return;
        }
        self.instances.extend_from_slice(&[
            x, y, w, h,
            fill[0], fill[1], fill[2], fill[3],
            radius, border_w, 0.0, 0.0,
            bc[0], bc[1], bc[2], bc[3],
        ]);
    }

    fn flush(&mut self, pw: u32, ph: u32, cw: f32, ch: f32) {
        // ── Text overlay via OffscreenCanvas ──────────────────────────────────
        let has_text = !self.text_calls.is_empty();
        if has_text {
            self.gpu.ensure_overlay_tex(pw, ph);
            if let Some(pixels) = rasterize_text(
                &self.text_calls, pw, ph, cw, ch, dpr(),
            ) {
                self.gpu.upload_overlay_pixels(&pixels, pw, ph);
            }
        }

        // ── GPU render ────────────────────────────────────────────────────────
        let frame = match self.gpu.surface.get_current_texture() {
            Ok(f) => f,
            Err(_) => return,
        };
        let view = frame.texture.create_view(&Default::default());
        let rect_count = (self.instances.len() / RECT_FLOATS) as u32;

        if rect_count > 0 {
            self.gpu.queue.write_buffer(
                &self.gpu.instance_buf,
                0,
                floats_as_bytes(&self.instances),
            );
        }

        let mut enc = self.gpu.device.create_command_encoder(&Default::default());

        // Pass 1 — background + rects
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.102, g: 0.102, b: 0.180, a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.gpu.rect_pipeline);
            pass.set_bind_group(0, &self.gpu.camera_bg, &[]);
            pass.set_vertex_buffer(0, self.gpu.instance_buf.slice(..));
            if rect_count > 0 {
                pass.draw(0..4, 0..rect_count);
            }
        }

        // Pass 2 — text overlay (alpha-blended)
        if has_text {
            if let Some(tex) = &self.gpu.overlay_tex {
                let tex_view = tex.create_view(&Default::default());
                let bg = self.gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.gpu.overlay_pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Sampler(&self.gpu.overlay_sampler) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex_view) },
                    ],
                });
                let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_pipeline(&self.gpu.overlay_pipeline);
                pass.set_bind_group(0, &bg, &[]);
                pass.draw(0..4, 0..1);
            }
        }

        self.gpu.queue.submit([enc.finish()]);
        frame.present();
    }
}

fn container_bg(tag: u32, opacity: f32) -> [f32; 4] {
    match tag {
        2 => [60.0/255.0, 100.0/255.0, 180.0/255.0, 0.07 * opacity],
        3 => [60.0/255.0, 160.0/255.0, 80.0/255.0,  0.07 * opacity],
        4 => [140.0/255.0, 60.0/255.0, 180.0/255.0, 0.07 * opacity],
        _ => [100.0/255.0, 100.0/255.0, 100.0/255.0, 0.05 * opacity],
    }
}

fn dpr() -> f32 {
    web_sys::window()
        .map(|w| w.device_pixel_ratio() as f32)
        .unwrap_or(1.0)
}

/// Rasterize text calls to an RGBA pixel buffer via OffscreenCanvas.
fn rasterize_text(
    calls: &[TextCall],
    pw: u32,
    ph: u32,
    _cw: f32,
    _ch: f32,
    scale: f32,
) -> Option<Vec<u8>> {
    let oc = OffscreenCanvas::new(pw, ph).ok()?;
    let ctx = oc
        .get_context("2d")
        .ok()??
        .dyn_into::<web_sys::OffscreenCanvasRenderingContext2d>()
        .ok()?;

    ctx.set_global_alpha(1.0);
    ctx.clear_rect(0.0, 0.0, pw as f64, ph as f64);
    ctx.scale(scale as f64, scale as f64).ok()?;

    for call in calls {
        ctx.set_font(&call.font);
        ctx.set_fill_style_str(&call.color);
        ctx.fill_text(&call.text, call.x as f64, call.y as f64).ok()?;
    }

    let image_data = ctx.get_image_data(0.0, 0.0, pw as f64, ph as f64).ok()?;
    Some(image_data.data().0)
}
