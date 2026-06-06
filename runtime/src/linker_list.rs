use wasmtime::{Caller, Linker};

use crate::RuntimeError;
use crate::host::VelHost;
use crate::linker::read_wasm_string;

pub(crate) fn register_list_imports(linker: &mut Linker<VelHost>) -> Result<(), RuntimeError> {
    linker.func_wrap(
        "vel/runtime",
        "list_count",
        |c: Caller<VelHost>, reqid: i32| -> i32 { c.data().list_count(reqid as u32) },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "list_item_str",
        |mut c: Caller<VelHost>, reqid: i32, index: i32, fptr: i32, flen: i32| {
            let field = read_wasm_string(&mut c, fptr, flen).unwrap_or_default();
            c.data_mut().list_item_str(reqid as u32, index, &field);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "list_item_num",
        |mut c: Caller<VelHost>, reqid: i32, index: i32, fptr: i32, flen: i32| -> f64 {
            let field = read_wasm_string(&mut c, fptr, flen).unwrap_or_default();
            c.data().list_item_num(reqid as u32, index, &field)
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "list_item_bool",
        |mut c: Caller<VelHost>, reqid: i32, index: i32, fptr: i32, flen: i32| -> i32 {
            let field = read_wasm_string(&mut c, fptr, flen).unwrap_or_default();
            c.data().list_item_bool(reqid as u32, index, &field)
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "list_sum",
        |mut c: Caller<VelHost>, reqid: i32, fptr: i32, flen: i32| -> f64 {
            let field = read_wasm_string(&mut c, fptr, flen).unwrap_or_default();
            c.data().list_sum(reqid as u32, &field)
        },
    )?;

    Ok(())
}
