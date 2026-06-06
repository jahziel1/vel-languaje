mod api_emit;
mod assemble;
mod assemble_imports;
pub(super) mod call_emit;
mod collect;
mod ctx;
pub(super) mod emit;
pub(super) mod expr;
mod expr_i32;
mod header_emit;
mod lifecycle_emit;
mod list_emit;
mod match_emit;
mod persist_emit;
mod props;
mod store_emit;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_api;
#[cfg(test)]
mod tests_api_body;
#[cfg(test)]
mod tests_api_dyn;
#[cfg(test)]
mod tests_api_headers;
#[cfg(test)]
mod tests_list;
#[cfg(test)]
mod tests_persist;
#[cfg(test)]
mod tests_print;
#[cfg(test)]
mod tests_props;
#[cfg(test)]
mod tests_store_multi;
#[cfg(test)]
mod tests_text_state;
mod text_arg_emit;
mod text_emit;

use std::collections::{HashMap, HashSet};

use wasm_encoder::ValType;

use crate::parser::ast::{Expr, Program};
#[cfg(test)]
mod tests_lifecycle;
#[cfg(test)]
mod tests_store;
#[cfg(test)]
mod tests_theme;

pub(super) use props::{
    color_name_rgb, element_tag, enum_prop_value, is_bool_prop, is_color_prop, is_ui_element,
    parse_hex_rgb, prop_key,
};

// ── Runtime import indices ────────────────────────────────────────────────────

pub(super) const RT_BEGIN: u32 = 0;
pub(super) const RT_END: u32 = 1;
pub(super) const RT_PROP_F64: u32 = 2;
pub(super) const RT_PROP_BOOL: u32 = 3;
pub(super) const RT_TEXT_CONTENT: u32 = 4;
pub(super) const RT_NAVIGATE: u32 = 5;
pub(super) const RT_SET_ON_CLICK: u32 = 6;
pub(super) const RT_SET_ON_CHANGE: u32 = 7;
pub(super) const RT_STR_BEGIN: u32 = 8;
pub(super) const RT_STR_LIT: u32 = 9;
pub(super) const RT_STR_NUM: u32 = 10;
pub(super) const RT_STR_DONE: u32 = 11;
pub(super) const RT_API_FETCH: u32 = 12;
pub(super) const RT_API_POLL: u32 = 13;
pub(super) const RT_API_FIELD_STR: u32 = 15;
pub(super) const RT_API_FIELD_NUM: u32 = 16;
pub(super) const RT_API_FIELD_BOOL: u32 = 17;
pub(super) const RT_BODY_BEGIN: u32 = 18;
pub(super) const RT_BODY_FIELD_STR: u32 = 19;
pub(super) const RT_BODY_FIELD_NUM: u32 = 20;
pub(super) const RT_BODY_FIELD_BOOL: u32 = 21;
pub(super) const RT_BODY_DONE: u32 = 22;
pub(super) const RT_URL_BEGIN: u32 = 23;
pub(super) const RT_URL_LIT: u32 = 24;
pub(super) const RT_URL_NUM: u32 = 25;
pub(super) const RT_URL_DONE: u32 = 26;
pub(super) const RT_API_URL_CHANGED: u32 = 27;
pub(super) const RT_API_FETCH_DYN: u32 = 28;
pub(super) const RT_HEADER_BEGIN: u32 = 29;
pub(super) const RT_HEADER_FIELD: u32 = 30;
pub(super) const RT_HEADER_DONE: u32 = 31;
pub(super) const RT_TEXT_STATE_GET: u32 = 32;
pub(super) const RT_TEXT_STATE_SET: u32 = 33;
pub(super) const RT_TEXT_STATE_BOOL: u32 = 34;
pub(super) const RT_TEXT_STATE_SET_BUILT: u32 = 35;
pub(super) const RT_LIST_COUNT: u32 = 36;
pub(super) const RT_LIST_ITEM_STR: u32 = 37;
pub(super) const RT_LIST_ITEM_NUM: u32 = 38;
pub(super) const RT_LIST_ITEM_BOOL: u32 = 39;
pub(super) const RT_LIST_SUM: u32 = 40;
pub(super) const RT_PERSIST_GET_NUM: u32 = 41;
pub(super) const RT_PERSIST_GET_BOOL: u32 = 42;
pub(super) const RT_PERSIST_SET_NUM: u32 = 43;
pub(super) const RT_PERSIST_SET_BOOL: u32 = 44;
/// header_field_str(key_ptr, key_len) — uses str_builder output as the header value.
pub(super) const RT_HEADER_FIELD_STR: u32 = 45;
/// nav_param(idx: i32, val: f64) — store a navigation parameter before calling navigate().
pub(super) const RT_NAV_PARAM: u32 = 46;
/// print_str(ptr: i32, len: i32) — print a static string to stdout (debug only).
pub(super) const RT_PRINT_STR: u32 = 47;
/// print_str_built() — print the current str_builder content to stdout, then clear it.
pub(super) const RT_PRINT_BUILT: u32 = 48;
/// window_width() → f64 — current window width in logical pixels (used by responsive blocks).
pub(super) const RT_WINDOW_WIDTH: u32 = 49;
/// state_push(kind: i32) — begin a hover/focus/active prop block.
pub(super) const RT_STATE_PUSH: u32 = 50;
/// state_pop() — end a hover/focus/active prop block.
pub(super) const RT_STATE_POP: u32 = 51;
pub(super) const RT_IMPORT_COUNT: u32 = 52;

// ── Internal IR ───────────────────────────────────────────────────────────────

pub struct GlobalEntry {
    pub val_type: ValType,
    pub init_f64: f64,
}

pub(super) struct FuncEntry {
    pub(super) type_idx: u32,
    pub(super) extra_locals: Vec<(u32, ValType)>,
    pub(super) body: Vec<wasm_encoder::Instruction<'static>>,
    pub(super) export_name: Option<String>,
}

// ── Code generator ────────────────────────────────────────────────────────────

pub struct CodeGen {
    pub(super) string_bytes: Vec<u8>,
    pub(super) string_map: HashMap<String, (u32, u32)>,
    pub globals: Vec<GlobalEntry>,
    pub(super) global_map: HashMap<String, u32>,
    /// Keys ("Page/var") of state variables with Text type — stored in host, not WASM globals.
    pub(super) text_state: HashSet<String>,
    /// Keys ("Store/var") of state variables marked `persist` — saved to device storage.
    pub(super) persist_state: HashSet<String>,
    pub(super) funcs: Vec<FuncEntry>,
    pub(super) func_map: HashMap<String, u32>,
    pub(super) types: Vec<(Vec<ValType>, Vec<ValType>)>,
    pub(super) type_map: HashMap<(Vec<ValType>, Vec<ValType>), u32>,
    /// Store names — to detect `auth.field` accesses.
    pub(super) store_names: HashSet<String>,
    /// Store derived entries — maps store_name → list of (derived_name, expr).
    pub(super) store_derived: HashMap<String, Vec<(String, Expr)>>,
    /// Store api.headers entries — maps store_name → list of (key, value_expr).
    pub(super) store_api_headers: HashMap<String, Vec<(String, Expr)>>,
    /// "colors/primary" → packed RGB u32
    pub(super) theme_colors: HashMap<String, u32>,
    /// "text/h3" or "radius/lg" → f64
    pub(super) theme_numbers: HashMap<String, f64>,
}

impl Default for CodeGen {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGen {
    pub fn new() -> Self {
        Self {
            string_bytes: Vec::new(),
            string_map: HashMap::new(),
            globals: Vec::new(),
            global_map: HashMap::new(),
            text_state: HashSet::new(),
            persist_state: HashSet::new(),
            funcs: Vec::new(),
            func_map: HashMap::new(),
            types: Vec::new(),
            type_map: HashMap::new(),
            store_names: HashSet::new(),
            store_derived: HashMap::new(),
            store_api_headers: HashMap::new(),
            theme_colors: HashMap::new(),
            theme_numbers: HashMap::new(),
        }
    }

    pub fn generate(&mut self, program: &Program) -> Vec<u8> {
        self.collect(program);
        self.emit(program);
        self.emit_persist_inits();
        self.assemble()
    }

    pub fn func_count(&self) -> usize {
        self.funcs.len()
    }

    #[cfg(test)]
    pub fn func_export_names(&self) -> Vec<Option<&str>> {
        self.funcs
            .iter()
            .map(|f| f.export_name.as_deref())
            .collect()
    }

    // ── Type registry ─────────────────────────────────────────────────────────

    pub(super) fn reg_type(&mut self, params: Vec<ValType>, results: Vec<ValType>) -> u32 {
        let key = (params.clone(), results.clone());
        if let Some(&idx) = self.type_map.get(&key) {
            return idx;
        }
        let idx = self.types.len() as u32;
        self.type_map.insert(key, idx);
        self.types.push((params, results));
        idx
    }

    // ── String table ──────────────────────────────────────────────────────────

    pub(super) fn intern(&mut self, s: &str) -> (u32, u32) {
        if let Some(&entry) = self.string_map.get(s) {
            return entry;
        }
        let offset = self.string_bytes.len() as u32;
        let len = s.len() as u32;
        self.string_bytes.extend_from_slice(s.as_bytes());
        self.string_map.insert(s.to_owned(), (offset, len));
        (offset, len)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns true if the expression is an async API call (`try api.*()`).
pub(super) fn is_async_state(expr: &Expr) -> bool {
    matches!(expr, Expr::Try(_))
}

impl CodeGen {
    pub(super) fn is_text_state(&self, key: &str) -> bool {
        self.text_state.contains(key)
    }

    /// Resolve a color expression to a packed RGB u32.
    /// Handles hex literals, named colors, and theme.colors.X references.
    pub(super) fn resolve_color_rgb(&self, expr: &Expr) -> u32 {
        match expr {
            Expr::Color(hex) => parse_hex_rgb(hex),
            Expr::Ident(name) => color_name_rgb(name).unwrap_or(0),
            Expr::Field(obj, key) => {
                if let Expr::Field(inner, section) = obj.as_ref()
                    && let Expr::Ident(name) = inner.as_ref()
                    && name == "theme"
                {
                    let map_key = format!("{}/{}", section, key);
                    return self.theme_colors.get(&map_key).copied().unwrap_or(0);
                }
                0
            }
            _ => 0,
        }
    }
}
