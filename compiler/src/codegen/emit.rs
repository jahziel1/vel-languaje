use wasm_encoder::{BlockType, Instruction, ValType};

use crate::parser::ast::{self, FnDef, OnEvent, Stmt, Type};

use super::{CodeGen, FuncEntry};

pub(super) use super::ctx::{Ctx, pre_scan_locals};

fn is_lifecycle_on(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::On(on) if matches!(on.event, OnEvent::Mount | OnEvent::Unmount))
}

impl CodeGen {
    pub(super) fn emit(&mut self, program: &crate::parser::ast::Program) {
        for item in &program.items {
            match item {
                ast::Item::Page(page) => self.emit_page(page),
                ast::Item::Component(comp) => self.emit_component(comp),
                ast::Item::Store(store) => self.emit_store(store),
                _ => {}
            }
        }
    }

    pub(super) fn emit_page(&mut self, page: &ast::Page) {
        let name = page.name.clone();
        let param_types: Vec<ValType> = page
            .params
            .iter()
            .map(|p| match &p.ty {
                Type::Bool => ValType::I32,
                _ => ValType::F64,
            })
            .collect();
        let num_params = param_types.len() as u32;
        let type_idx = self.reg_type(param_types, vec![]);
        let locals = pre_scan_locals(&page.body);
        let mut ctx = Ctx::new(name.clone(), num_params, locals);
        for (i, param) in page.params.iter().enumerate() {
            ctx.params.insert(param.name.clone(), i as u32);
        }
        let mut body = Vec::new();

        self.emit_api_inits(&name, &page.body, &mut ctx, &mut body);

        for stmt in &page.body {
            match stmt {
                Stmt::Fn(_) => {}
                s if is_lifecycle_on(s) => {}
                _ => self.emit_stmt(stmt, &mut ctx, &mut body),
            }
        }
        body.push(Instruction::End);

        self.funcs.push(FuncEntry {
            type_idx,
            extra_locals: ctx.wasm_locals(),
            body,
            export_name: Some(format!("page_{}", name)),
        });

        for stmt in &page.body {
            if let Stmt::Fn(fn_def) = stmt {
                self.emit_fn(&name, fn_def);
            }
        }

        self.emit_lifecycle_fns(&name, &page.body);
    }

    pub(super) fn emit_component(&mut self, comp: &ast::Component) {
        let name = comp.name.clone();
        let type_idx = self.reg_type(vec![], vec![]);
        let locals = pre_scan_locals(&comp.body);
        let mut ctx = Ctx::new(name.clone(), 0, locals);
        let mut body = Vec::new();

        self.emit_api_inits(&name, &comp.body, &mut ctx, &mut body);

        for stmt in &comp.body {
            if !matches!(stmt, Stmt::Fn(_)) {
                self.emit_stmt(stmt, &mut ctx, &mut body);
            }
        }
        body.push(Instruction::End);
        self.funcs.push(FuncEntry {
            type_idx,
            extra_locals: ctx.wasm_locals(),
            body,
            export_name: Some(format!("component_{}", name)),
        });
    }

    fn emit_fn(&mut self, page_name: &str, fn_def: &FnDef) {
        let num_params = fn_def.params.len() as u32;
        let fn_type_idx = self.reg_type(
            fn_def.params.iter().map(|_| ValType::F64).collect(),
            if fn_def.return_ty.is_some() {
                vec![ValType::F64]
            } else {
                vec![]
            },
        );
        let fn_locals = pre_scan_locals(&fn_def.body);
        let mut fn_ctx = Ctx::new(page_name.to_owned(), num_params, fn_locals);
        for (i, param) in fn_def.params.iter().enumerate() {
            fn_ctx.params.insert(param.name.clone(), i as u32);
        }
        let mut fn_body = Vec::new();
        for s in &fn_def.body {
            self.emit_stmt(s, &mut fn_ctx, &mut fn_body);
        }
        fn_body.push(Instruction::End);
        self.funcs.push(FuncEntry {
            type_idx: fn_type_idx,
            extra_locals: fn_ctx.wasm_locals(),
            body: fn_body,
            export_name: Some(format!("{}_{}", page_name, fn_def.name)),
        });
    }

    pub(super) fn emit_stmt(
        &mut self,
        stmt: &Stmt,
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        match stmt {
            Stmt::State(_) | Stmt::Fn(_) | Stmt::ForEach(_, _) => {}

            Stmt::Derived(d) => {
                for (name, expr) in &d.entries {
                    if let Some(&lidx) = ctx.locals.get(name.as_str()) {
                        self.emit_expr_f64(expr, ctx, out);
                        out.push(Instruction::LocalSet(lidx));
                    }
                }
            }

            Stmt::Let(l) => {
                let key = format!("{}/{}", ctx.page, l.name);
                if self.is_text_state(&key) {
                    self.emit_text_state_write(&key, &l.value, ctx, out);
                } else if let Some(&gidx) = self.global_map.get(&key) {
                    let val_type = self.globals[gidx as usize].val_type;
                    if val_type == ValType::I32 {
                        self.emit_expr_i32(&l.value, ctx, out);
                    } else {
                        self.emit_expr_f64(&l.value, ctx, out);
                    }
                    out.push(Instruction::GlobalSet(gidx));
                    self.emit_persist_set_if_needed(&key, gidx, val_type, out);
                } else if let Some(&lidx) = ctx.locals.get(&l.name) {
                    self.emit_expr_f64(&l.value, ctx, out);
                    out.push(Instruction::LocalSet(lidx));
                } else {
                    self.emit_expr_push(&l.value, ctx, out);
                    out.push(Instruction::Drop);
                }
            }

            Stmt::Guard(g) => {
                self.emit_expr_i32(&g.condition, ctx, out);
                out.push(Instruction::If(BlockType::Empty));
                self.emit_expr_push(&g.action, ctx, out);
                out.push(Instruction::Drop);
                out.push(Instruction::End);
            }

            Stmt::If(if_stmt) => {
                self.emit_expr_i32(&if_stmt.condition, ctx, out);
                out.push(Instruction::If(BlockType::Empty));
                for s in &if_stmt.then_body {
                    self.emit_stmt(s, ctx, out);
                }
                if let Some(else_body) = &if_stmt.else_body {
                    out.push(Instruction::Else);
                    for s in else_body {
                        self.emit_stmt(s, ctx, out);
                    }
                }
                out.push(Instruction::End);
            }

            Stmt::Match(m) => {
                self.emit_match_stmt(m, ctx, out);
            }

            Stmt::On(on_stmt) => {
                match &on_stmt.event {
                    OnEvent::Change(var) => {
                        let var_key = format!("{}/{}", ctx.page, var);
                        let shadow_key = format!("{}/{}__prev", ctx.page, var);
                        match (
                            self.global_map.get(&var_key).copied(),
                            self.global_map.get(&shadow_key).copied(),
                        ) {
                            (Some(gidx), Some(sidx)) => {
                                let val_type = self.globals[gidx as usize].val_type;
                                out.push(Instruction::GlobalGet(gidx));
                                out.push(Instruction::GlobalGet(sidx));
                                if val_type == ValType::F64 {
                                    out.push(Instruction::F64Ne);
                                } else {
                                    out.push(Instruction::I32Ne);
                                }
                                out.push(Instruction::If(BlockType::Empty));
                                for s in &on_stmt.body {
                                    self.emit_stmt(s, ctx, out);
                                }
                                // Update shadow so body doesn't re-fire next render
                                out.push(Instruction::GlobalGet(gidx));
                                out.push(Instruction::GlobalSet(sidx));
                                out.push(Instruction::End);
                            }
                            _ => {
                                for s in &on_stmt.body {
                                    self.emit_stmt(s, ctx, out);
                                }
                            }
                        }
                    }
                    // Lifecycle hooks are emitted as separate exported functions by emit_page.
                    OnEvent::Mount | OnEvent::Unmount => {}
                    OnEvent::Responsive(bp) => {
                        self.emit_responsive_block(bp, &on_stmt.body, ctx, out);
                    }
                    _ => {
                        for s in &on_stmt.body {
                            self.emit_stmt(s, ctx, out);
                        }
                    }
                }
            }

            Stmt::Expr(e) => {
                self.emit_expr_push(e, ctx, out);
                out.push(Instruction::Drop);
            }
        }
    }
}
