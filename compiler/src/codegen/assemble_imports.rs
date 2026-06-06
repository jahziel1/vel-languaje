use std::collections::HashMap;

use wasm_encoder::{EntityType, ImportSection, ValType};

use super::CodeGen;

impl CodeGen {
    pub(super) fn build_import_section(&self) -> ImportSection {
        let get = |map: &HashMap<(Vec<ValType>, Vec<ValType>), u32>,
                   p: Vec<ValType>,
                   r: Vec<ValType>| { *map.get(&(p, r)).unwrap_or(&0) };
        let m = &self.type_map;
        let ty_i32_void = get(m, vec![ValType::I32], vec![]);
        let ty_void_void = get(m, vec![], vec![]);
        let ty_i32f64 = get(m, vec![ValType::I32, ValType::F64], vec![]);
        let ty_i32i32 = get(m, vec![ValType::I32, ValType::I32], vec![]);
        let ty_f64_void = get(m, vec![ValType::F64], vec![]);
        let ty_4i32_to_i32 = get(
            m,
            vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::I32],
        );
        let ty_i32_to_i32 = get(m, vec![ValType::I32], vec![ValType::I32]);
        let ty_3i32_void = get(m, vec![ValType::I32, ValType::I32, ValType::I32], vec![]);
        let ty_3i32_f64 = get(
            m,
            vec![ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::F64],
        );
        let ty_3i32_i32 = get(
            m,
            vec![ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::I32],
        );
        let ty_4i32_void = get(
            m,
            vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            vec![],
        );
        let ty_i32i32f64_void = get(m, vec![ValType::I32, ValType::I32, ValType::F64], vec![]);
        let ty_i32i32_to_i32 = get(m, vec![ValType::I32, ValType::I32], vec![ValType::I32]);
        let ty_4i32_to_f64 = get(
            m,
            vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::F64],
        );
        let ty_i32i32f64_to_f64 = get(
            m,
            vec![ValType::I32, ValType::I32, ValType::F64],
            vec![ValType::F64],
        );
        let ty_void_to_f64 = get(m, vec![], vec![ValType::F64]);

        let mut imports = ImportSection::new();
        imports.import(
            "vel/runtime",
            "begin_element",
            EntityType::Function(ty_i32_void),
        );
        imports.import(
            "vel/runtime",
            "end_element",
            EntityType::Function(ty_void_void),
        );
        imports.import("vel/runtime", "prop_f64", EntityType::Function(ty_i32f64));
        imports.import("vel/runtime", "prop_bool", EntityType::Function(ty_i32i32));
        imports.import(
            "vel/runtime",
            "text_content",
            EntityType::Function(ty_i32i32),
        );
        imports.import("vel/runtime", "navigate", EntityType::Function(ty_i32i32));
        imports.import(
            "vel/runtime",
            "set_on_click",
            EntityType::Function(ty_i32i32),
        );
        imports.import(
            "vel/runtime",
            "set_on_change",
            EntityType::Function(ty_i32i32),
        );
        imports.import(
            "vel/runtime",
            "str_begin",
            EntityType::Function(ty_void_void),
        );
        imports.import("vel/runtime", "str_lit", EntityType::Function(ty_i32i32));
        imports.import("vel/runtime", "str_num", EntityType::Function(ty_f64_void));
        imports.import(
            "vel/runtime",
            "str_done",
            EntityType::Function(ty_void_void),
        );
        imports.import(
            "vel/runtime",
            "api_fetch",
            EntityType::Function(ty_4i32_to_i32),
        );
        imports.import(
            "vel/runtime",
            "api_poll",
            EntityType::Function(ty_i32_to_i32),
        );
        imports.import(
            "vel/runtime",
            "api_error_str",
            EntityType::Function(ty_i32_void),
        );
        imports.import(
            "vel/runtime",
            "api_field_str",
            EntityType::Function(ty_3i32_void),
        );
        imports.import(
            "vel/runtime",
            "api_field_num",
            EntityType::Function(ty_3i32_f64),
        );
        imports.import(
            "vel/runtime",
            "api_field_bool",
            EntityType::Function(ty_3i32_i32),
        );
        imports.import(
            "vel/runtime",
            "body_begin",
            EntityType::Function(ty_void_void),
        );
        imports.import(
            "vel/runtime",
            "body_field_str",
            EntityType::Function(ty_4i32_void),
        );
        imports.import(
            "vel/runtime",
            "body_field_num",
            EntityType::Function(ty_i32i32f64_void),
        );
        imports.import(
            "vel/runtime",
            "body_field_bool",
            EntityType::Function(ty_3i32_void),
        );
        imports.import(
            "vel/runtime",
            "body_done",
            EntityType::Function(ty_void_void),
        );
        imports.import(
            "vel/runtime",
            "url_begin",
            EntityType::Function(ty_void_void),
        );
        imports.import("vel/runtime", "url_lit", EntityType::Function(ty_i32i32));
        imports.import("vel/runtime", "url_num", EntityType::Function(ty_f64_void));
        imports.import(
            "vel/runtime",
            "url_done",
            EntityType::Function(ty_void_void),
        );
        imports.import(
            "vel/runtime",
            "api_url_changed",
            EntityType::Function(ty_i32_to_i32),
        );
        imports.import(
            "vel/runtime",
            "api_fetch_dyn",
            EntityType::Function(ty_i32i32_to_i32),
        );
        imports.import(
            "vel/runtime",
            "header_begin",
            EntityType::Function(ty_void_void),
        );
        imports.import(
            "vel/runtime",
            "header_field",
            EntityType::Function(ty_4i32_void),
        );
        imports.import(
            "vel/runtime",
            "header_done",
            EntityType::Function(ty_void_void),
        );
        imports.import(
            "vel/runtime",
            "text_state_get",
            EntityType::Function(ty_i32i32),
        );
        imports.import(
            "vel/runtime",
            "text_state_set",
            EntityType::Function(ty_4i32_void),
        );
        imports.import(
            "vel/runtime",
            "text_state_bool",
            EntityType::Function(ty_i32i32_to_i32),
        );
        imports.import(
            "vel/runtime",
            "text_state_set_built",
            EntityType::Function(ty_i32i32),
        );
        for (name, ty) in [
            ("list_count", ty_i32_to_i32),
            ("list_item_str", ty_4i32_void),
            ("list_item_num", ty_4i32_to_f64),
            ("list_item_bool", ty_4i32_to_i32),
            ("list_sum", ty_3i32_f64),
        ] {
            imports.import("vel/runtime", name, EntityType::Function(ty));
        }
        for (name, ty) in [
            ("persist_get_num", ty_i32i32f64_to_f64),
            ("persist_get_bool", ty_3i32_i32),
            ("persist_set_num", ty_i32i32f64_void),
            ("persist_set_bool", ty_3i32_void),
        ] {
            imports.import("vel/runtime", name, EntityType::Function(ty));
        }
        // header_field_str(key_ptr: i32, key_len: i32) — value taken from str_builder.
        imports.import(
            "vel/runtime",
            "header_field_str",
            EntityType::Function(ty_i32i32),
        );
        // nav_param(idx: i32, val: f64) — stores a navigation parameter for the next page.
        imports.import("vel/runtime", "nav_param", EntityType::Function(ty_i32f64));
        // print_str(ptr: i32, len: i32) — print static string to stdout (debug).
        imports.import("vel/runtime", "print_str", EntityType::Function(ty_i32i32));
        // print_str_built() — print str_builder content to stdout, then clear it (debug).
        imports.import(
            "vel/runtime",
            "print_str_built",
            EntityType::Function(ty_void_void),
        );
        // window_width() → f64 — current window width for responsive breakpoints.
        imports.import(
            "vel/runtime",
            "window_width",
            EntityType::Function(ty_void_to_f64),
        );
        // state_push(kind: i32) — begin hover/focus/active prop block.
        imports.import(
            "vel/runtime",
            "state_push",
            EntityType::Function(ty_i32_void),
        );
        // state_pop() — end hover/focus/active prop block.
        imports.import(
            "vel/runtime",
            "state_pop",
            EntityType::Function(ty_void_void),
        );
        imports
    }
}
