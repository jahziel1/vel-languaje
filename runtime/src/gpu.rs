use std::sync::Arc;

use pollster::block_on;
use vello::kurbo::Affine;
use vello::peniko::{Color, Fill};
use vello::{AaConfig, AaSupport, Renderer, RendererOptions, Scene};
use winit::window::Window;

use crate::draw::{DrawState, draw_tree};
use crate::draw_error::draw_error;
use crate::layout::{LayoutBox, layout_root};
use crate::text::TextPainter;
use crate::tree::UiNode;

pub(super) struct GpuState {
    pub(super) window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    text_painter: Option<TextPainter>,
}

impl GpuState {
    pub(super) fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("failed to create wgpu surface");

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
        }))
        .expect("no suitable GPU adapter found");

        let (device, queue) = block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("vel"),
                ..Default::default()
            },
            None,
        ))
        .expect("failed to create wgpu device");

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let renderer = Renderer::new(
            &device,
            RendererOptions {
                surface_format: Some(format),
                use_cpu: false,
                antialiasing_support: AaSupport::all(),
                num_init_threads: std::num::NonZeroUsize::new(1),
            },
        )
        .expect("failed to create Vello renderer");

        Self {
            window,
            surface,
            device,
            queue,
            surface_config,
            renderer,
            text_painter: TextPainter::load(),
        }
    }

    pub(super) fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width.max(1);
        self.surface_config.height = height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub(super) fn render(
        &mut self,
        nodes: &[UiNode],
        cursor: (f64, f64),
        focused_input: Option<&str>,
        is_mouse_down: bool,
    ) -> Vec<LayoutBox> {
        let mut scene = Scene::new();
        let w = self.surface_config.width as f64;
        let h = self.surface_config.height as f64;

        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(24, 24, 28),
            None,
            &vello::kurbo::Rect::new(0.0, 0.0, w, h),
        );

        let boxes = layout_root(nodes, 16.0, 16.0, w - 32.0);
        let ds = DrawState {
            cursor,
            focused_input,
            is_mouse_down,
        };
        draw_tree(&mut scene, nodes, &boxes, self.text_painter.as_ref(), &ds);

        let surface_texture = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return boxes,
        };

        self.renderer
            .render_to_surface(
                &self.device,
                &self.queue,
                &scene,
                &surface_texture,
                &vello::RenderParams {
                    base_color: Color::from_rgb8(24, 24, 28),
                    width: self.surface_config.width,
                    height: self.surface_config.height,
                    antialiasing_method: AaConfig::Area,
                },
            )
            .expect("vello render failed");

        surface_texture.present();
        self.window.request_redraw();
        boxes
    }

    pub(super) fn render_error(&mut self, error_text: &str) {
        let mut scene = Scene::new();
        let w = self.surface_config.width as f64;
        let h = self.surface_config.height as f64;
        draw_error(&mut scene, error_text, self.text_painter.as_ref(), w, h);
        let surface_texture = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return,
        };
        self.renderer
            .render_to_surface(
                &self.device,
                &self.queue,
                &scene,
                &surface_texture,
                &vello::RenderParams {
                    base_color: Color::from_rgb8(12, 12, 18),
                    width: self.surface_config.width,
                    height: self.surface_config.height,
                    antialiasing_method: AaConfig::Area,
                },
            )
            .expect("vello render failed");
        surface_texture.present();
        self.window.request_redraw();
    }
}
