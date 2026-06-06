use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::Receiver;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{WindowAttributes, WindowId};

use crate::PageInstance;
use crate::gpu::GpuState;
use crate::layout::{LayoutBox, find_enter_handler, find_input_at, hit_test, patch_input_values};
use crate::tree::UiNode;

// ── Events ────────────────────────────────────────────────────────────────────

pub enum VelEvent {
    /// Hot-reload: replace the running page instance.
    Reload(PageInstance),
    /// An API request completed — re-render with updated state.
    ApiReady,
    /// Hot-reload failed — show error overlay while keeping the current page alive.
    Error(String),
}

// ── Application state ─────────────────────────────────────────────────────────

struct VelApp {
    instance: PageInstance,
    current_nodes: Vec<UiNode>,
    layout_boxes: Vec<LayoutBox>,
    cursor_pos: (f64, f64),
    is_mouse_down: bool,
    title: String,
    state: Option<GpuState>,
    input_values: HashMap<String, String>,
    focused_input: Option<String>,
    /// If set, the error overlay is shown instead of (or on top of) the normal frame.
    error_overlay: Option<String>,
}

impl VelApp {
    fn rerender(&mut self) {
        match self.instance.render() {
            Ok(mut nodes) => {
                patch_input_values(&mut nodes, &self.input_values);
                self.current_nodes = nodes;
                if let Some(gpu) = &self.state {
                    gpu.window.request_redraw();
                }
            }
            Err(e) => eprintln!("render error: {e}"),
        }
    }
}

impl ApplicationHandler<VelEvent> for VelApp {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: VelEvent) {
        match event {
            VelEvent::Reload(new_instance) => {
                self.instance = new_instance;
                self.input_values.clear();
                self.focused_input = None;
                self.error_overlay = None;
                self.rerender();
            }
            VelEvent::ApiReady => {
                self.rerender();
            }
            VelEvent::Error(msg) => {
                self.error_overlay = Some(msg);
                if let Some(gpu) = &self.state {
                    gpu.window.request_redraw();
                }
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = WindowAttributes::default()
            .with_title(&self.title)
            .with_inner_size(winit::dpi::LogicalSize::new(800u32, 600u32));
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("failed to create window"),
        );
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
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = (position.x, position.y);
                if let Some(gpu) = &self.state {
                    gpu.window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.is_mouse_down = false;
                if let Some(gpu) = &self.state {
                    gpu.window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.is_mouse_down = true;
                let (mx, my) = self.cursor_pos;
                self.focused_input = find_input_at(&self.current_nodes, &self.layout_boxes, mx, my);
                if let Some(fn_name) = hit_test(&self.current_nodes, &self.layout_boxes, mx, my) {
                    if let Err(e) = self.instance.call_handler(&fn_name) {
                        eprintln!("handler error: {e}");
                    } else {
                        if let Some(path) = self.instance.take_navigation()
                            && let Err(e) = self.instance.go(&path)
                        {
                            eprintln!("navigation error: {e}");
                        }
                        self.rerender();
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                if let Some(var_name) = self.focused_input.clone() {
                    match &event.logical_key {
                        Key::Named(NamedKey::Backspace) => {
                            self.input_values.entry(var_name.clone()).or_default().pop();
                            let val = self
                                .input_values
                                .get(&var_name)
                                .cloned()
                                .unwrap_or_default();
                            self.instance.set_text_state(&var_name, val);
                            self.rerender();
                        }
                        Key::Named(NamedKey::Escape) => {
                            self.focused_input = None;
                        }
                        Key::Named(NamedKey::Enter) => {
                            if let Some(fn_name) =
                                find_enter_handler(&self.current_nodes, &var_name)
                            {
                                if let Err(e) = self.instance.call_handler(&fn_name) {
                                    eprintln!("enter handler error: {e}");
                                } else {
                                    self.rerender();
                                }
                            }
                        }
                        _ => {
                            if let Some(text) = &event.text {
                                self.input_values
                                    .entry(var_name.clone())
                                    .or_default()
                                    .push_str(text.as_str());
                                let val = self
                                    .input_values
                                    .get(&var_name)
                                    .cloned()
                                    .unwrap_or_default();
                                self.instance.set_text_state(&var_name, val);
                                self.rerender();
                            }
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(gpu) = &mut self.state {
                    if let Some(err) = &self.error_overlay.clone() {
                        gpu.render_error(err);
                    } else {
                        let fi = self.focused_input.as_deref();
                        self.layout_boxes = gpu.render(
                            &self.current_nodes,
                            self.cursor_pos,
                            fi,
                            self.is_mouse_down,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn run_window(mut instance: PageInstance, title: &str) {
    let event_loop = EventLoop::<VelEvent>::with_user_event()
        .build()
        .expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = event_loop.create_proxy();
    instance.set_wakeup(Arc::new(move || {
        let _ = proxy.send_event(VelEvent::ApiReady);
    }));

    let initial_nodes = instance.render().unwrap_or_default();
    let mut app = VelApp {
        instance,
        current_nodes: initial_nodes,
        layout_boxes: vec![],
        cursor_pos: (0.0, 0.0),
        is_mouse_down: false,
        title: title.to_owned(),
        state: None,
        input_values: HashMap::new(),
        focused_input: None,
        error_overlay: None,
    };
    event_loop.run_app(&mut app).expect("event loop error");
}

pub fn run_window_watch(
    mut instance: PageInstance,
    title: &str,
    reload_recv: Receiver<Result<PageInstance, String>>,
) {
    let event_loop = EventLoop::<VelEvent>::with_user_event()
        .build()
        .expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = event_loop.create_proxy();

    // Wire API re-render wakeup
    let api_proxy = proxy.clone();
    instance.set_wakeup(Arc::new(move || {
        let _ = api_proxy.send_event(VelEvent::ApiReady);
    }));

    // Hot-reload thread: forwards Ok(instance) as Reload, Err(msg) as Error
    std::thread::spawn(move || {
        while let Ok(result) = reload_recv.recv() {
            let event = match result {
                Ok(inst) => VelEvent::Reload(inst),
                Err(msg) => VelEvent::Error(msg),
            };
            if proxy.send_event(event).is_err() {
                break;
            }
        }
    });

    let initial_nodes = instance.render().unwrap_or_default();
    let mut app = VelApp {
        instance,
        current_nodes: initial_nodes,
        layout_boxes: vec![],
        cursor_pos: (0.0, 0.0),
        is_mouse_down: false,
        title: title.to_owned(),
        state: None,
        input_values: HashMap::new(),
        focused_input: None,
        error_overlay: None,
    };
    event_loop.run_app(&mut app).expect("event loop error");
}
