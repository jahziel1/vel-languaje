use wasm_encoder::{Instruction, ValType};

use super::{
    CodeGen, FuncEntry, RT_PERSIST_GET_BOOL, RT_PERSIST_GET_NUM, RT_PERSIST_SET_BOOL,
    RT_PERSIST_SET_NUM,
};

impl CodeGen {
    /// Emit `vel_persist_init()` — called once at startup to restore persisted globals.
    pub(super) fn emit_persist_inits(&mut self) {
        // Collect all persist entries (avoids borrow conflicts during intern())
        let mut entries: Vec<(String, u32, ValType, f64)> = self
            .persist_state
            .iter()
            .filter_map(|key| {
                let gidx = *self.global_map.get(key)?;
                let g = &self.globals[gidx as usize];
                Some((key.clone(), gidx, g.val_type, g.init_f64))
            })
            .collect();

        if entries.is_empty() {
            return;
        }

        entries.sort_by(|a, b| a.0.cmp(&b.0));

        let interned: Vec<(u32, u32)> = entries
            .iter()
            .map(|(key, _, _, _)| self.intern(key))
            .collect();

        let type_idx = self.reg_type(vec![], vec![]);
        let mut body = Vec::new();

        for ((_, gidx, val_type, init_f64), (kptr, klen)) in entries.iter().zip(interned.iter()) {
            body.push(Instruction::I32Const(*kptr as i32));
            body.push(Instruction::I32Const(*klen as i32));
            if *val_type == ValType::F64 {
                body.push(Instruction::F64Const((*init_f64).into()));
                body.push(Instruction::Call(RT_PERSIST_GET_NUM));
            } else {
                body.push(Instruction::I32Const(*init_f64 as i32));
                body.push(Instruction::Call(RT_PERSIST_GET_BOOL));
            }
            body.push(Instruction::GlobalSet(*gidx));
        }

        body.push(Instruction::End);

        self.funcs.push(FuncEntry {
            type_idx,
            extra_locals: vec![],
            body,
            export_name: Some("vel_persist_init".to_owned()),
        });
    }

    /// After a `GlobalSet` on a persist variable, emit `persist_set_*` to save the new value.
    pub(super) fn emit_persist_set_if_needed(
        &mut self,
        key: &str,
        gidx: u32,
        val_type: ValType,
        out: &mut Vec<Instruction<'static>>,
    ) {
        if !self.persist_state.contains(key) {
            return;
        }
        let (kptr, klen) = self.intern(key);
        out.push(Instruction::I32Const(kptr as i32));
        out.push(Instruction::I32Const(klen as i32));
        out.push(Instruction::GlobalGet(gidx));
        if val_type == ValType::F64 {
            out.push(Instruction::Call(RT_PERSIST_SET_NUM));
        } else {
            out.push(Instruction::Call(RT_PERSIST_SET_BOOL));
        }
    }
}
