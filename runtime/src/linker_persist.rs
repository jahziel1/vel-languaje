use wasmtime::{Caller, Linker};

use crate::RuntimeError;
use crate::host::VelHost;
use crate::linker::read_wasm_string;

pub(crate) fn register_persist_imports(linker: &mut Linker<VelHost>) -> Result<(), RuntimeError> {
    linker.func_wrap(
        "vel/runtime",
        "persist_get_num",
        |mut c: Caller<VelHost>, key_ptr: i32, key_len: i32, default: f64| -> f64 {
            let key = read_wasm_string(&mut c, key_ptr, key_len);
            c.data().persist_get_num(key, default)
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "persist_get_bool",
        |mut c: Caller<VelHost>, key_ptr: i32, key_len: i32, default: i32| -> i32 {
            let key = read_wasm_string(&mut c, key_ptr, key_len);
            c.data().persist_get_bool(key, default)
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "persist_set_num",
        |mut c: Caller<VelHost>, key_ptr: i32, key_len: i32, val: f64| {
            let key = read_wasm_string(&mut c, key_ptr, key_len);
            c.data_mut().persist_set_num(key, val);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "persist_set_bool",
        |mut c: Caller<VelHost>, key_ptr: i32, key_len: i32, val: i32| {
            let key = read_wasm_string(&mut c, key_ptr, key_len);
            c.data_mut().persist_set_bool(key, val);
        },
    )?;

    Ok(())
}
