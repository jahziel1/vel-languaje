use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{WindowAttributes, WindowId};

use crate::gpu::GpuState;

struct ErrorApp {
    error_text: String,
    title: String,
    state: Option<GpuState>,
}

impl ApplicationHandler for ErrorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = WindowAttributes::default()
            .with_title(&self.title)
            .with_inner_size(winit::dpi::LogicalSize::new(800u32, 600u32));
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        self.state = Some(GpuState::new(window));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.state {
                    gpu.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(gpu) = &mut self.state {
                    gpu.render_error(&self.error_text.clone());
                }
            }
            _ => {}
        }
    }
}

pub fn run_window_error(error_text: String, title: String) {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = ErrorApp {
        error_text,
        title,
        state: None,
    };
    event_loop.run_app(&mut app).expect("event loop error");
}
