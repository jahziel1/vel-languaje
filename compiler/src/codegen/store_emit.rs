use wasm_encoder::{Instruction, ValType};

use crate::parser::ast::{self, FnDef};

use super::emit::{Ctx, pre_scan_locals};
use super::{CodeGen, FuncEntry};

impl CodeGen {
    pub(super) fn emit_store(&mut self, store: &ast::Store) {
        for fn_def in &store.fns {
            self.emit_store_fn(&store.name, fn_def);
        }
    }

    fn emit_store_fn(&mut self, store_name: &str, fn_def: &FnDef) {
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
        let mut fn_ctx = Ctx::new(store_name.to_owned(), num_params, fn_locals);
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
            export_name: Some(format!("store_{}_{}", store_name, fn_def.name)),
        });
    }
}
