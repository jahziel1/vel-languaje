use wasmtime::{Caller, Linker};

use crate::RuntimeError;
use crate::host::VelHost;
use crate::linker::read_wasm_string;

pub(crate) fn register_text_imports(linker: &mut Linker<VelHost>) -> Result<(), RuntimeError> {
    linker.func_wrap(
        "vel/runtime",
        "text_state_get",
        |mut c: Caller<VelHost>, key_ptr: i32, key_len: i32| {
            let key = read_wasm_string(&mut c, key_ptr, key_len);
            c.data_mut().text_state_get(key);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "text_state_set",
        |mut c: Caller<VelHost>, key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32| {
            let key = read_wasm_string(&mut c, key_ptr, key_len);
            let val = read_wasm_string(&mut c, val_ptr, val_len);
            c.data_mut().text_state_set(key, val);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "text_state_bool",
        |mut c: Caller<VelHost>, key_ptr: i32, key_len: i32| -> i32 {
            let key = read_wasm_string(&mut c, key_ptr, key_len);
            c.data().text_state_bool(key)
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "text_state_set_built",
        |mut c: Caller<VelHost>, key_ptr: i32, key_len: i32| {
            let key = read_wasm_string(&mut c, key_ptr, key_len);
            c.data_mut().text_state_set_built(key);
        },
    )?;

    Ok(())
}
