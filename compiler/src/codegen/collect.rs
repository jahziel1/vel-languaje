use wasm_encoder::ValType;

use crate::parser::ast::{Expr, Item, OnEvent, Stmt};

use super::{CodeGen, GlobalEntry, RT_IMPORT_COUNT, is_async_state, parse_hex_rgb};

impl CodeGen {
    pub(super) fn collect(&mut self, program: &crate::parser::ast::Program) {
        self.reg_type(vec![ValType::I32], vec![]);
        self.reg_type(vec![], vec![]);
        self.reg_type(vec![ValType::I32, ValType::F64], vec![]);
        self.reg_type(vec![ValType::I32, ValType::I32], vec![]);
        self.reg_type(vec![ValType::F64], vec![]);
        // api_fetch: (i32, i32, i32, i32) -> i32
        self.reg_type(
            vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::I32],
        );
        // api_poll / api_error_str: i32 -> i32  and  i32 -> void  (share lookup)
        self.reg_type(vec![ValType::I32], vec![ValType::I32]);
        // api_field_str: (i32, i32, i32) -> void
        self.reg_type(vec![ValType::I32, ValType::I32, ValType::I32], vec![]);
        // api_field_num: (i32, i32, i32) -> f64
        self.reg_type(
            vec![ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::F64],
        );
        // api_field_bool: (i32, i32, i32) -> i32
        self.reg_type(
            vec![ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::I32],
        );
        // body_field_str: (i32, i32, i32, i32) -> void
        self.reg_type(
            vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            vec![],
        );
        // body_field_num: (i32, i32, f64) -> void
        self.reg_type(vec![ValType::I32, ValType::I32, ValType::F64], vec![]);
        // api_fetch_dyn: (i32, i32) -> i32
        self.reg_type(vec![ValType::I32, ValType::I32], vec![ValType::I32]);
        // text_state_bool: (i32, i32) -> i32
        self.reg_type(vec![ValType::I32, ValType::I32], vec![ValType::I32]);
        // list_item_num: (i32, i32, i32, i32) -> f64
        self.reg_type(
            vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::F64],
        );
        // persist_get_num: (i32, i32, f64) -> f64
        self.reg_type(
            vec![ValType::I32, ValType::I32, ValType::F64],
            vec![ValType::F64],
        );

        for item in &program.items {
            match item {
                Item::Page(page) => self.collect_state_from_stmts(&page.name, &page.body),
                Item::Component(comp) => self.collect_state_from_stmts(&comp.name, &comp.body),
                Item::Store(store) => {
                    self.store_names.insert(store.name.clone());
                    if let Some(state) = &store.state {
                        for entry in &state.entries {
                            self.add_global(&store.name, &entry.name, &entry.value);
                            if entry.persist {
                                let key = format!("{}/{}", store.name, entry.name);
                                self.persist_state.insert(key);
                            }
                        }
                    }
                    if let Some(derived) = &store.derived {
                        self.store_derived
                            .insert(store.name.clone(), derived.entries.clone());
                    }
                    if !store.api_headers.is_empty() {
                        self.store_api_headers
                            .insert(store.name.clone(), store.api_headers.clone());
                    }
                }
                Item::ThemeDef(td) => {
                    for section in &td.sections {
                        for (key, val) in &section.entries {
                            let map_key = format!("{}/{}", section.name, key);
                            match val {
                                crate::parser::ast::ThemeTokenValue::Color(hex) => {
                                    let rgb = parse_hex_rgb(hex);
                                    self.theme_colors.insert(map_key, rgb);
                                }
                                crate::parser::ast::ThemeTokenValue::Number(n) => {
                                    self.theme_numbers.insert(map_key, *n);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Second pass: shadow globals for `on change` watchers.
        // Must run after state globals are collected so we can copy their type/init.
        for item in &program.items {
            if let Item::Page(page) = item {
                self.collect_on_change_shadows(&page.name, &page.body);
            }
        }

        let mut next = RT_IMPORT_COUNT;
        for item in &program.items {
            match item {
                Item::Page(page) => {
                    self.func_map.insert(format!("page_{}", page.name), next);
                    next += 1;
                    for stmt in &page.body {
                        if let Stmt::Fn(f) = stmt {
                            self.func_map
                                .insert(format!("{}_{}", page.name, f.name), next);
                            next += 1;
                        }
                    }
                }
                Item::Component(comp) => {
                    self.func_map
                        .insert(format!("component_{}", comp.name), next);
                    next += 1;
                }
                Item::Store(store) => {
                    for fn_def in &store.fns {
                        self.func_map
                            .insert(format!("store_{}_{}", store.name, fn_def.name), next);
                        next += 1;
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_on_change_shadows(&mut self, owner: &str, stmts: &[Stmt]) {
        for stmt in stmts {
            if let Stmt::On(on_stmt) = stmt
                && let OnEvent::Change(var) = &on_stmt.event
            {
                let var_key = format!("{}/{}", owner, var);
                let shadow_key = format!("{}/{}__prev", owner, var);
                if let Some(&gidx) = self.global_map.get(&var_key) {
                    let val_type = self.globals[gidx as usize].val_type;
                    let init_f64 = self.globals[gidx as usize].init_f64;
                    let sidx = self.globals.len() as u32;
                    self.global_map.insert(shadow_key, sidx);
                    self.globals.push(GlobalEntry { val_type, init_f64 });
                }
            }
        }
    }

    fn collect_state_from_stmts(&mut self, owner: &str, stmts: &[Stmt]) {
        for stmt in stmts {
            if let Stmt::State(state) = stmt {
                for entry in &state.entries {
                    if is_async_state(&entry.value) {
                        // Async state: two i32 globals — status (Loading=0) and reqid
                        let sidx = self.globals.len() as u32;
                        self.global_map
                            .insert(format!("{}/{}", owner, entry.name), sidx);
                        self.globals.push(GlobalEntry {
                            val_type: ValType::I32,
                            init_f64: 0.0,
                        });
                        let ridx = self.globals.len() as u32;
                        self.global_map
                            .insert(format!("{}/{}__reqid", owner, entry.name), ridx);
                        self.globals.push(GlobalEntry {
                            val_type: ValType::I32,
                            init_f64: 0.0,
                        });
                    } else {
                        self.add_global(owner, &entry.name, &entry.value);
                    }
                }
            }
        }
    }

    pub(super) fn add_global(&mut self, owner: &str, var: &str, init_expr: &Expr) {
        let key = format!("{}/{}", owner, var);
        if matches!(init_expr, Expr::Str(_)) {
            // Text state — stored in host, not as a WASM global
            self.text_state.insert(key);
            return;
        }
        let idx = self.globals.len() as u32;
        self.global_map.insert(key, idx);
        let (val_type, init_f64) = match init_expr {
            Expr::Number(n) => (ValType::F64, *n),
            Expr::Bool(b) => (ValType::I32, if *b { 1.0 } else { 0.0 }),
            _ => (ValType::F64, 0.0),
        };
        self.globals.push(GlobalEntry { val_type, init_f64 });
    }
}
