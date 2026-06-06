use wasm_encoder::ValType;

use crate::parser::ast::{MatchBody, Stmt};

// ── Function compilation context ──────────────────────────────────────────────

#[derive(Clone)]
pub(super) struct Ctx {
    pub(super) page: String,
    pub(super) params: std::collections::HashMap<String, u32>,
    pub(super) locals: std::collections::HashMap<String, u32>,
    /// Variables bound in `Success(data)` arms — maps binding name → local index holding reqid.
    pub(super) api_bindings: std::collections::HashMap<String, u32>,
    /// Variables bound in `list(data) { item -> }` — maps item name → (req_i32_local, idx_i32_local).
    pub(super) list_bindings: std::collections::HashMap<String, (u32, u32)>,
    pub(super) next_local: u32,
    /// Count of i32 locals allocated via alloc_i32_local (placed after the f64 locals).
    pub(super) i32_local_count: u32,
}

impl Ctx {
    pub(super) fn new(page: String, num_params: u32, pre_scanned: Vec<String>) -> Self {
        let mut locals = std::collections::HashMap::new();
        let mut next = num_params;
        for name in pre_scanned {
            locals.insert(name, next);
            next += 1;
        }
        Self {
            page,
            params: std::collections::HashMap::new(),
            locals,
            api_bindings: std::collections::HashMap::new(),
            list_bindings: std::collections::HashMap::new(),
            next_local: next,
            i32_local_count: 0,
        }
    }

    /// Allocate a new i32 local (for loop counters). Returns its local index.
    pub(super) fn alloc_i32_local(&mut self) -> u32 {
        let idx = self.next_local;
        self.next_local += 1;
        self.i32_local_count += 1;
        idx
    }

    pub(super) fn wasm_locals(&self) -> Vec<(u32, ValType)> {
        let mut result = vec![];
        let f64_count = self.locals.len() as u32;
        if f64_count > 0 {
            result.push((f64_count, ValType::F64));
        }
        if self.i32_local_count > 0 {
            result.push((self.i32_local_count, ValType::I32));
        }
        result
    }
}

// ── Pre-scan locals ───────────────────────────────────────────────────────────

pub(super) fn pre_scan_locals(stmts: &[Stmt]) -> Vec<String> {
    let mut names = Vec::new();
    for stmt in stmts {
        match stmt {
            Stmt::Let(l) => names.push(l.name.clone()),
            Stmt::Derived(d) => {
                for (name, _) in &d.entries {
                    names.push(name.clone());
                }
            }
            Stmt::If(i) => {
                names.extend(pre_scan_locals(&i.then_body));
                if let Some(e) = &i.else_body {
                    names.extend(pre_scan_locals(e));
                }
            }
            Stmt::Match(m) => {
                for arm in &m.arms {
                    if let crate::parser::ast::Pattern::Variant(_, bindings) = &arm.pattern {
                        names.extend(bindings.iter().cloned());
                    }
                    if let MatchBody::Block(b) = &arm.body {
                        names.extend(pre_scan_locals(b));
                    }
                }
            }
            Stmt::On(o) => names.extend(pre_scan_locals(&o.body)),
            Stmt::Fn(f) => names.extend(pre_scan_locals(&f.body)),
            Stmt::ForEach(_, body) => names.extend(pre_scan_locals(body)),
            // Recurse into UI element block children (e.g. column { match { Success(data) } })
            Stmt::Expr(crate::parser::ast::Expr::Call(_, _, Some(block))) => {
                names.extend(pre_scan_locals(block));
            }
            _ => {}
        }
    }
    names.sort();
    names.dedup();
    names
}
