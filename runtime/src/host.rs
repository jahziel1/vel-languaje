use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::tree::{PropValue, UiNode, UiProp, prop_name, tag_name};
use serde_json::Value as JsonValue;

// ── API request storage ───────────────────────────────────────────────────────

/// Status codes mirroring the Vel `Result` discriminants (Loading=0, Success=1, Error=2).
pub struct ApiEntry {
    pub status: u8,
    pub body: String,    // JSON body on success, error message on failure
    pub url: String,     // URL used for this request (for re-fetch detection)
    pub cancelled: bool, // set on navigation; suppresses wakeup when true
}

pub type ApiStore = Arc<Mutex<HashMap<u32, ApiEntry>>>;

// ── Host ──────────────────────────────────────────────────────────────────────

/// State held in the wasmtime `Store` during page execution.
pub struct VelHost {
    pub(crate) stack: Vec<UiNode>,
    pub completed: Vec<UiNode>,
    pub navigate_to: Option<String>,
    pub(crate) str_builder: String,
    /// Shared store of in-flight and completed API requests.
    pub api_store: ApiStore,
    pub next_reqid: u32,
    /// Called from background threads when an API request completes, to wake the event loop.
    pub api_wakeup: Option<Arc<dyn Fn() + Send + Sync>>,
    /// Accumulates fields for the next api_fetch body (body_begin/field_*/done).
    pub(crate) body_fields: Vec<(String, serde_json::Value)>,
    /// JSON string ready to send with the next api_fetch call.
    pub pending_body: Option<String>,
    /// Accumulates characters for a dynamic URL (url_begin/lit/num/done).
    pub(crate) url_builder: String,
    /// Finalized dynamic URL ready for api_fetch_dyn or api_url_changed.
    pub pending_url: String,
    /// Headers accumulated by header_begin/field/done, consumed by api_fetch.
    pub(crate) pending_headers: Vec<(String, String)>,
    /// Text state variables — keyed by "Page/var", stored in host (not WASM globals).
    pub text_state: HashMap<String, String>,
    /// Persist state — keyed by "Store/var", survives across instantiations.
    pub persist_store: HashMap<String, JsonValue>,
    /// Parameters passed via go("/path", key: val) — consumed by PageInstance.go().
    pub nav_params: Vec<f64>,
    /// Current window width in logical pixels — read by responsive breakpoint blocks.
    pub window_width: f64,
    /// Active state kind: 0=none 1=hover 2=focus 3=active.
    pub(crate) state_mode: u8,
    /// Props accumulated while a state_push/state_pop pair is open.
    pub(crate) state_buf: Vec<UiProp>,
}

impl VelHost {
    pub fn new() -> Self {
        let persist_store: HashMap<String, JsonValue> = {
            #[cfg(feature = "renderer")]
            {
                std::fs::read_to_string(".vel_persist.json")
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default()
            }
            #[cfg(not(feature = "renderer"))]
            HashMap::new()
        };
        Self {
            stack: Vec::new(),
            completed: Vec::new(),
            navigate_to: None,
            str_builder: String::new(),
            api_store: Arc::new(Mutex::new(HashMap::new())),
            next_reqid: 1,
            api_wakeup: None,
            body_fields: Vec::new(),
            pending_body: None,
            url_builder: String::new(),
            pending_url: String::new(),
            pending_headers: Vec::new(),
            text_state: HashMap::new(),
            persist_store,
            nav_params: Vec::new(),
            window_width: 1024.0,
            state_mode: 0,
            state_buf: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.stack.clear();
        self.completed.clear();
        self.navigate_to = None;
        self.str_builder.clear();
        self.state_mode = 0;
        self.state_buf.clear();
        // api_store and next_reqid intentionally NOT reset — requests persist across renders
    }

    pub fn take_pending_body(&mut self) -> Option<String> {
        self.pending_body.take()
    }

    pub fn take_pending_headers(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.pending_headers)
    }

    // ── API ───────────────────────────────────────────────────────────────────

    /// Mark all in-flight requests as cancelled so their threads skip the wakeup.
    pub fn cancel_pending_requests(&mut self) {
        if let Ok(mut store) = self.api_store.lock() {
            for entry in store.values_mut() {
                if entry.status == 0 {
                    entry.cancelled = true;
                }
            }
        }
    }

    /// Allocate a new request ID (starts at 1; 0 means "not yet fetched").
    pub fn alloc_reqid(&mut self) -> u32 {
        let id = self.next_reqid;
        self.next_reqid += 1;
        id
    }

    /// Append the string value of `field` from the JSON body of `reqid` into str_builder.
    pub fn api_field_str(&mut self, reqid: u32, field: &str) {
        let body = self
            .api_store
            .lock()
            .ok()
            .and_then(|s| s.get(&reqid).map(|e| e.body.clone()));
        if let Some(s) = body
            .and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok())
            .and_then(|json| json.get(field).cloned())
            .map(|val| match val {
                serde_json::Value::String(s) => s,
                other => other.to_string(),
            })
        {
            self.str_builder.push_str(&s);
        }
    }

    /// Return the numeric value of `field` from the JSON body of `reqid`.
    pub fn api_field_num(&self, reqid: u32, field: &str) -> f64 {
        let body = self
            .api_store
            .lock()
            .ok()
            .and_then(|s| s.get(&reqid).map(|e| e.body.clone()));
        body.and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok())
            .and_then(|json| json.get(field).cloned())
            .and_then(|val| match val {
                serde_json::Value::Number(n) => n.as_f64(),
                serde_json::Value::Bool(b) => Some(if b { 1.0 } else { 0.0 }),
                serde_json::Value::String(s) => s.parse().ok(),
                _ => None,
            })
            .unwrap_or(0.0)
    }

    /// Return 1 if `field` in the JSON body of `reqid` is truthy, 0 otherwise.
    pub fn api_field_bool(&self, reqid: u32, field: &str) -> i32 {
        let body = self
            .api_store
            .lock()
            .ok()
            .and_then(|s| s.get(&reqid).map(|e| e.body.clone()));
        body.and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok())
            .and_then(|json| json.get(field).cloned())
            .and_then(|val| match val {
                serde_json::Value::Bool(b) => Some(if b { 1 } else { 0 }),
                serde_json::Value::Number(n) => Some(if n.as_f64().unwrap_or(0.0) != 0.0 {
                    1
                } else {
                    0
                }),
                _ => None,
            })
            .unwrap_or(0)
    }

    /// Set str_builder to the error body for `reqid` (used by api_error_str import).
    pub fn api_error_str(&mut self, reqid: u32) {
        let msg = self
            .api_store
            .lock()
            .unwrap()
            .get(&reqid)
            .map(|e| e.body.clone())
            .unwrap_or_default();
        self.str_builder = msg;
    }

    // ── String builder ────────────────────────────────────────────────────────

    pub fn str_begin(&mut self) {
        self.str_builder.clear();
    }

    pub fn str_lit(&mut self, s: Option<String>) {
        if let Some(s) = s {
            self.str_builder.push_str(&s);
        }
    }

    pub fn str_num(&mut self, val: f64) {
        if val.fract() == 0.0 && val.abs() < 1e15 {
            self.str_builder.push_str(&(val as i64).to_string());
        } else {
            self.str_builder.push_str(&val.to_string());
        }
    }

    pub fn str_done(&mut self) {
        let s = self.str_builder.clone();
        if let Some(top) = self.stack.last_mut() {
            top.text = Some(s);
        }
        self.str_builder.clear();
    }

    // ── UI tree ───────────────────────────────────────────────────────────────

    pub fn begin_element(&mut self, tag: i32) {
        self.stack.push(UiNode {
            tag,
            tag_name: tag_name(tag),
            props: Vec::new(),
            text: None,
            children: Vec::new(),
            on_click: None,
            on_change: None,
            hover_props: Vec::new(),
            focus_props: Vec::new(),
            active_props: Vec::new(),
        });
    }

    pub fn end_element(&mut self) {
        if let Some(node) = self.stack.pop() {
            if let Some(parent) = self.stack.last_mut() {
                parent.children.push(node);
            } else {
                self.completed.push(node);
            }
        }
    }

    pub fn set_on_click(&mut self, fn_name: Option<String>) {
        if let Some(top) = self.stack.last_mut() {
            top.on_click = fn_name;
        }
    }

    pub fn set_on_change(&mut self, binding: Option<String>) {
        if let Some(top) = self.stack.last_mut() {
            top.on_change = binding;
        }
    }

    pub fn prop_f64(&mut self, key: i32, val: f64) {
        let prop = UiProp {
            key,
            key_name: prop_name(key),
            value: PropValue::Number(val),
        };
        if self.state_mode != 0 {
            self.state_buf.push(prop);
        } else if let Some(top) = self.stack.last_mut() {
            top.props.push(prop);
        }
    }

    pub fn prop_bool(&mut self, key: i32, val: i32) {
        let prop = UiProp {
            key,
            key_name: prop_name(key),
            value: PropValue::Bool(val != 0),
        };
        if self.state_mode != 0 {
            self.state_buf.push(prop);
        } else if let Some(top) = self.stack.last_mut() {
            top.props.push(prop);
        }
    }

    pub fn text_content(&mut self, text: Option<String>) {
        if let Some(top) = self.stack.last_mut() {
            top.text = Some(text.unwrap_or_default());
        }
    }
}
