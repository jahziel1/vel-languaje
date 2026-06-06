mod draw;
mod gpu;
mod layout;
mod tree;

use draw::Renderer;
use gpu::GpuState;
use std::cell::RefCell;
use tree::FrameInput;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

// Global renderer — initialized once, used per frame.
thread_local! {
    static RENDERER: RefCell<Option<Renderer>> = const { RefCell::new(None) };
}

/// Initialize the WebGPU renderer bound to the given canvas.
/// Must be awaited before calling render_frame or draw_error.
#[wasm_bindgen]
pub async fn init_renderer(canvas: HtmlCanvasElement) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let gpu = GpuState::new(canvas)
        .await
        .map_err(|e| JsValue::from_str(&e))?;

    RENDERER.with(|r| {
        *r.borrow_mut() = Some(Renderer::new(gpu));
    });

    Ok(())
}

/// Render a frame. `json` is a serialized FrameInput. `cw`/`ch` are CSS pixels.
#[wasm_bindgen]
pub fn render_frame(json: &str, cw: f32, ch: f32) {
    let Ok(input) = serde_json::from_str::<FrameInput>(json) else {
        return;
    };
    RENDERER.with(|r| {
        if let Some(renderer) = r.borrow_mut().as_mut() {
            renderer.render_frame(&input, cw, ch);
        }
    });
}

/// Render an error message overlay.
#[wasm_bindgen]
pub fn draw_error(msg: &str, cw: f32, ch: f32) {
    RENDERER.with(|r| {
        if let Some(renderer) = r.borrow_mut().as_mut() {
            renderer.render_error(msg, cw, ch);
        }
    });
}
