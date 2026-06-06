use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, ExportKind, ExportSection, Function, FunctionSection,
    GlobalSection, GlobalType, MemorySection, MemoryType, Module, TypeSection, ValType,
};

use super::{CodeGen, RT_IMPORT_COUNT};

impl CodeGen {
    pub(super) fn assemble(&mut self) -> Vec<u8> {
        // Ensure import-only types are in the type map before building the import section.
        self.reg_type(vec![], vec![ValType::F64]); // window_width() → f64

        let mut module = Module::new();

        // Type section
        let mut types = TypeSection::new();
        for (params, results) in &self.types {
            types
                .ty()
                .function(params.iter().copied(), results.iter().copied());
        }
        module.section(&types);

        // Import section
        let imports = self.build_import_section();
        module.section(&imports);

        // Function section
        let mut funcs = FunctionSection::new();
        for entry in &self.funcs {
            funcs.function(entry.type_idx);
        }
        module.section(&funcs);

        // Memory section (must precede globals per WASM spec)
        let mut mems = MemorySection::new();
        mems.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        module.section(&mems);

        // Global section
        let mut globals = GlobalSection::new();
        for g in &self.globals {
            let ty = GlobalType {
                val_type: g.val_type,
                mutable: true,
                shared: false,
            };
            let init = match g.val_type {
                ValType::F64 => ConstExpr::f64_const(g.init_f64.into()),
                _ => ConstExpr::i32_const(g.init_f64 as i32),
            };
            globals.global(ty, &init);
        }
        module.section(&globals);

        // Export section
        let mut exports = ExportSection::new();
        for (i, entry) in self.funcs.iter().enumerate() {
            if let Some(name) = &entry.export_name {
                exports.export(name, ExportKind::Func, RT_IMPORT_COUNT + i as u32);
            }
        }
        exports.export("memory", ExportKind::Memory, 0);
        module.section(&exports);

        // Code section
        let mut code = CodeSection::new();
        for entry in &self.funcs {
            let mut func = Function::new(entry.extra_locals.clone());
            for inst in &entry.body {
                func.instruction(inst);
            }
            code.function(&func);
        }
        module.section(&code);

        // Data section (interned strings)
        if !self.string_bytes.is_empty() {
            let mut data = DataSection::new();
            data.active(0, &ConstExpr::i32_const(0), self.string_bytes.clone());
            module.section(&data);
        }

        module.finish()
    }
}
