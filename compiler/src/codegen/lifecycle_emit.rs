use wasm_encoder::{BlockType, Instruction};

use crate::parser::ast::{Breakpoint, OnEvent, Stmt};

use super::emit::{Ctx, pre_scan_locals};
use super::{CodeGen, FuncEntry, RT_WINDOW_WIDTH};

impl CodeGen {
    /// Emit a conditional block rendered only when the window width matches the breakpoint.
    pub(super) fn emit_responsive_block(
        &mut self,
        bp: &Breakpoint,
        body: &[Stmt],
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        let (threshold, is_ge) = match bp {
            Breakpoint::Phone => (480.0_f64, false),
            Breakpoint::Mobile => (768.0, false),
            Breakpoint::Tablet => (1024.0, false),
            Breakpoint::Desktop => (1024.0, true),
            Breakpoint::Wide => (1440.0, true),
        };
        out.push(Instruction::Call(RT_WINDOW_WIDTH));
        out.push(Instruction::F64Const(threshold.into()));
        if is_ge {
            out.push(Instruction::F64Ge);
        } else {
            out.push(Instruction::F64Lt);
        }
        out.push(Instruction::If(BlockType::Empty));
        for s in body {
            self.emit_stmt(s, ctx, out);
        }
        out.push(Instruction::End);
    }

    /// Emit `page_X__mount` / `page_X__unmount` exported functions from lifecycle blocks.
    pub(super) fn emit_lifecycle_fns(&mut self, page_name: &str, body: &[Stmt]) {
        let mount_stmts: Vec<Stmt> = body
            .iter()
            .filter_map(|s| {
                if let Stmt::On(on) = s {
                    if matches!(on.event, OnEvent::Mount) {
                        Some(on.body.iter().cloned())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .flatten()
            .collect();

        let unmount_stmts: Vec<Stmt> = body
            .iter()
            .filter_map(|s| {
                if let Stmt::On(on) = s {
                    if matches!(on.event, OnEvent::Unmount) {
                        Some(on.body.iter().cloned())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .flatten()
            .collect();

        if !mount_stmts.is_empty() {
            self.emit_lifecycle_fn(page_name, "mount", &mount_stmts);
        }
        if !unmount_stmts.is_empty() {
            self.emit_lifecycle_fn(page_name, "unmount", &unmount_stmts);
        }
    }

    fn emit_lifecycle_fn(&mut self, page_name: &str, suffix: &str, stmts: &[Stmt]) {
        let type_idx = self.reg_type(vec![], vec![]);
        let locals = pre_scan_locals(stmts);
        let mut ctx = Ctx::new(page_name.to_owned(), 0, locals);
        let mut fn_body = Vec::new();
        for s in stmts {
            self.emit_stmt(s, &mut ctx, &mut fn_body);
        }
        fn_body.push(Instruction::End);
        self.funcs.push(FuncEntry {
            type_idx,
            extra_locals: ctx.wasm_locals(),
            body: fn_body,
            export_name: Some(format!("page_{}__{}", page_name, suffix)),
        });
    }
}
