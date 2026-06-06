#[cfg(feature = "renderer")]
mod draw;
#[cfg(feature = "renderer")]
mod draw_error;
#[cfg(feature = "renderer")]
mod gpu;
mod host;
mod host_api;
mod host_list;
mod host_nav;
mod host_persist;
mod host_state;
mod host_text;
pub mod layout;
mod linker;
mod linker_api;
mod linker_list;
mod linker_persist;
mod linker_text;
pub mod render;
#[cfg(feature = "renderer")]
mod renderer;
#[cfg(feature = "renderer")]
mod renderer_error;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_api;
#[cfg(test)]
mod tests_grid;
#[cfg(test)]
mod tests_lifecycle;
#[cfg(test)]
mod tests_list;
#[cfg(test)]
mod tests_nav;
#[cfg(test)]
mod tests_state;
#[cfg(feature = "renderer")]
mod text;
pub mod tree;

use std::collections::HashMap;
use std::sync::Arc;

use wasmtime::{Engine, Instance, Module, Store};

use host::VelHost;
pub use render::render_debug;
pub use tree::UiNode;

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct RuntimeError(pub String);

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RuntimeError: {}", self.0)
    }
}

impl std::error::Error for RuntimeError {}

impl From<wasmtime::Error> for RuntimeError {
    fn from(e: wasmtime::Error) -> Self {
        Self(format!("{e:#}"))
    }
}

// ── PageInstance — persistent WASM state for reactive rendering ───────────────

/// A live WASM page that keeps its state (globals) across calls.
/// Call `render()` to get the current UI tree; call `call_handler()` to
/// trigger an event handler (which mutates state), then `render()` again.
pub struct PageInstance {
    store: Store<VelHost>,
    instance: Instance,
    pub page_fn: String,
    /// path → "page_Name" — built from WASM exports at instantiation time.
    routes: HashMap<String, String>,
    /// Navigation history for `go(back)`.
    history: Vec<String>,
    /// Parameters to pass to the next page render (set by go() with named args).
    pub nav_params: Vec<f64>,
    /// Whether the current page's `on mount` handler has already been called.
    mounted: bool,
}

impl PageInstance {
    /// Re-run the page function and return the resulting UI tree.
    /// On the first call after a navigation, also invokes `page_X__mount` if it exists.
    pub fn render(&mut self) -> Result<Vec<UiNode>, RuntimeError> {
        self.store.data_mut().reset();
        let func = self
            .instance
            .get_func(&mut self.store, &self.page_fn)
            .ok_or_else(|| RuntimeError(format!("page fn `{}` not found", self.page_fn)))?;
        let params: Vec<wasmtime::Val> = self
            .nav_params
            .iter()
            .map(|&v| wasmtime::Val::F64(v.to_bits()))
            .collect();
        let mut results = vec![];
        func.call(&mut self.store, &params, &mut results)?;

        if !self.mounted {
            self.mounted = true;
            let mount_fn = format!("{}__mount", self.page_fn);
            if let Ok(f) = self
                .instance
                .get_typed_func::<(), ()>(&mut self.store, &mount_fn)
            {
                f.call(&mut self.store, ())?;
            }
        }

        Ok(self.store.data().completed.clone())
    }

    /// Call a named event handler (e.g. `"Counter_increment"`).
    pub fn call_handler(&mut self, fn_name: &str) -> Result<(), RuntimeError> {
        let func = self
            .instance
            .get_typed_func::<(), ()>(&mut self.store, fn_name)
            .map_err(|_| RuntimeError(format!("handler `{fn_name}` not found")))?;
        func.call(&mut self.store, ())?;
        Ok(())
    }

    /// Consume a pending navigation request set by `go()` during a handler call.
    pub fn take_navigation(&mut self) -> Option<String> {
        self.store.data_mut().take_navigation()
    }

    /// Update a Text state variable from the host side (e.g. when the user types in an input).
    pub fn set_text_state(&mut self, key: &str, value: String) {
        self.store
            .data_mut()
            .text_state
            .insert(key.to_owned(), value);
    }

    /// Update the window width used by responsive breakpoint blocks.
    /// Call before `render()` whenever the window is resized.
    pub fn set_window_width(&mut self, width: f64) {
        self.store.data_mut().window_width = width;
    }

    /// Set the callback invoked from background threads when an API request completes.
    pub fn set_wakeup(&mut self, wakeup: Arc<dyn Fn() + Send + Sync>) {
        self.store.data_mut().api_wakeup = Some(wakeup);
    }

    /// Return the most recently allocated request ID (test helper).
    pub fn last_reqid(&self) -> u32 {
        self.store.data().next_reqid - 1
    }

    /// Inject a JSON body for a specific reqid (test helper).
    /// Call after the first render (which allocates the reqid) to supply mock data.
    pub fn inject_api_body(&mut self, reqid: u32, body: &str) {
        if let Ok(mut store) = self.store.data_mut().api_store.lock()
            && let Some(entry) = store.get_mut(&reqid)
        {
            entry.body = body.to_owned();
            entry.status = 1;
        }
    }

    /// Navigate to a path (e.g. `"/counter"` or `"back"`), capturing any nav params set by go().
    /// Calls `page_X__unmount` on the current page before switching, if it exists.
    pub fn go(&mut self, path: &str) -> Result<(), RuntimeError> {
        self.store.data_mut().cancel_pending_requests();
        let params = self.store.data_mut().take_nav_params();

        // Call unmount on the page we're leaving.
        let unmount_fn = format!("{}__unmount", self.page_fn);
        if let Ok(f) = self
            .instance
            .get_typed_func::<(), ()>(&mut self.store, &unmount_fn)
        {
            let _ = f.call(&mut self.store, ());
        }

        if path == "back" {
            if let Some(prev) = self.history.pop() {
                self.page_fn = prev;
            }
            self.nav_params.clear();
            self.mounted = false;
            return Ok(());
        }
        match self.routes.get(path).cloned() {
            Some(fn_name) => {
                self.history.push(self.page_fn.clone());
                self.page_fn = fn_name;
                self.nav_params = params;
                self.mounted = false;
                Ok(())
            }
            None => Err(RuntimeError(format!("no page at '{path}'"))),
        }
    }
}

// ── Runtime ───────────────────────────────────────────────────────────────────

/// The Vel runtime — executes compiled `.wasm` modules and produces UI trees.
pub struct Runtime {
    engine: Engine,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            engine: Engine::default(),
        }
    }

    /// Create a live `PageInstance` from WASM bytes.
    /// The instance keeps its state (WASM globals) alive across calls.
    pub fn instantiate(
        &self,
        wasm_bytes: &[u8],
        page_fn: &str,
    ) -> Result<PageInstance, RuntimeError> {
        let module = Module::new(&self.engine, wasm_bytes)?;
        let routes = build_routes(&module);
        let mut store = Store::new(&self.engine, VelHost::new());
        let linker = linker::make_linker(&self.engine)?;
        let instance = linker.instantiate(&mut store, &module)?;
        // Restore persisted globals from device storage (no-op if no persist variables).
        if let Ok(init_fn) = instance.get_typed_func::<(), ()>(&mut store, "vel_persist_init") {
            init_fn.call(&mut store, ())?;
        }
        Ok(PageInstance {
            store,
            instance,
            page_fn: page_fn.to_owned(),
            routes,
            history: Vec::new(),
            nav_params: Vec::new(),
            mounted: false,
        })
    }

    /// Execute a page function once and return the UI tree (no persistent state).
    pub fn run_page(&self, wasm_bytes: &[u8], page_fn: &str) -> Result<Vec<UiNode>, RuntimeError> {
        let mut inst = self.instantiate(wasm_bytes, page_fn)?;
        inst.render()
    }
}

#[cfg(feature = "renderer")]
pub use renderer::{run_window, run_window_watch};
#[cfg(feature = "renderer")]
pub use renderer_error::run_window_error;

// ── Route utilities ───────────────────────────────────────────────────────────

fn build_routes(module: &Module) -> HashMap<String, String> {
    module
        .exports()
        .filter_map(|e| {
            let name = e.name();
            if !name.starts_with("page_") {
                return None;
            }
            let page_part = &name["page_".len()..];
            // Skip sub-functions like "page_Counter_increment"
            if page_part.contains('_') {
                return None;
            }
            Some((page_name_to_path(page_part), name.to_owned()))
        })
        .collect()
}

fn page_name_to_path(name: &str) -> String {
    if name == "Home" {
        return "/".to_owned();
    }
    let mut path = String::from("/");
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            path.push('-');
        }
        path.extend(c.to_lowercase());
    }
    path
}
