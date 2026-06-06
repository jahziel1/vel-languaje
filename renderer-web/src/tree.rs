use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct UiNode {
    pub tag: u32,
    pub text: Option<String>,
    #[serde(default)]
    pub props: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub children: Vec<UiNode>,
    pub on_click: Option<String>,
    pub on_change: Option<String>,
    pub hover_props: Option<HashMap<String, serde_json::Value>>,
    pub focus_props: Option<HashMap<String, serde_json::Value>>,
    pub active_props: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub has_value: bool,
}

#[derive(Deserialize, Debug)]
pub struct FrameInput {
    pub nodes: Vec<UiNode>,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_down: bool,
    #[serde(default)]
    pub active_input: String,
    #[serde(default)]
    pub input_buffer: String,
}

impl UiNode {
    /// Resolve effective props applying hover/focus/active overrides.
    pub fn effective_props<'a>(
        &'a self,
        bx: f32,
        by: f32,
        bw: f32,
        bh: f32,
        mx: f32,
        my: f32,
        mouse_down: bool,
        active_input: &str,
    ) -> &'a HashMap<String, serde_json::Value> {
        let inside = mx >= bx && mx < bx + bw && my >= by && my < by + bh;
        if inside && mouse_down {
            if let Some(ap) = &self.active_props {
                return ap;
            }
        }
        if inside {
            if let Some(hp) = &self.hover_props {
                return hp;
            }
        }
        if let Some(on_ch) = &self.on_change {
            if on_ch == active_input {
                if let Some(fp) = &self.focus_props {
                    return fp;
                }
            }
        }
        &self.props
    }
}

pub fn prop_f32(props: &HashMap<String, serde_json::Value>, key: &str) -> Option<f32> {
    props.get(key)?.as_f64().map(|v| v as f32)
}

pub fn prop_bool(props: &HashMap<String, serde_json::Value>, key: &str) -> bool {
    props.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Packed color r*65536 + g*256 + b → [r,g,b,a] in 0..1
pub fn unpack_color(packed: f32, alpha: f32) -> [f32; 4] {
    let p = packed as u32;
    [
        ((p >> 16) & 0xFF) as f32 / 255.0,
        ((p >> 8) & 0xFF) as f32 / 255.0,
        (p & 0xFF) as f32 / 255.0,
        alpha,
    ]
}

/// Packed color → CSS string "rgba(r,g,b,a)"
pub fn packed_to_css(packed: f32, alpha: f32) -> String {
    let p = packed as u32;
    let r = (p >> 16) & 0xFF;
    let g = (p >> 8) & 0xFF;
    let b = p & 0xFF;
    format!("rgba({r},{g},{b},{alpha})")
}
