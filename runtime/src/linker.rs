use std::sync::Arc;

use wasmtime::{Caller, Engine, Linker};

use crate::RuntimeError;
use crate::host::{ApiEntry, VelHost};

// ── Linker setup ──────────────────────────────────────────────────────────────

pub(crate) fn make_linker(engine: &Engine) -> Result<Linker<VelHost>, RuntimeError> {
    let mut linker: Linker<VelHost> = Linker::new(engine);

    linker.func_wrap(
        "vel/runtime",
        "begin_element",
        |mut c: Caller<VelHost>, tag: i32| {
            c.data_mut().begin_element(tag);
        },
    )?;

    linker.func_wrap("vel/runtime", "end_element", |mut c: Caller<VelHost>| {
        c.data_mut().end_element();
    })?;

    linker.func_wrap(
        "vel/runtime",
        "prop_f64",
        |mut c: Caller<VelHost>, key: i32, val: f64| {
            c.data_mut().prop_f64(key, val);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "prop_bool",
        |mut c: Caller<VelHost>, key: i32, val: i32| {
            c.data_mut().prop_bool(key, val);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "text_content",
        |mut c: Caller<VelHost>, ptr: i32, len: i32| {
            let text = read_wasm_string(&mut c, ptr, len);
            c.data_mut().text_content(text);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "navigate",
        |mut c: Caller<VelHost>, ptr: i32, len: i32| {
            let path = read_wasm_string(&mut c, ptr, len);
            c.data_mut().set_navigation(path);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "nav_param",
        |mut c: Caller<VelHost>, idx: i32, val: f64| {
            c.data_mut().set_nav_param(idx, val);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "set_on_click",
        |mut c: Caller<VelHost>, ptr: i32, len: i32| {
            let fn_name = read_wasm_string(&mut c, ptr, len);
            c.data_mut().set_on_click(fn_name);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "set_on_change",
        |mut c: Caller<VelHost>, ptr: i32, len: i32| {
            let binding = read_wasm_string(&mut c, ptr, len);
            c.data_mut().set_on_change(binding);
        },
    )?;

    linker.func_wrap("vel/runtime", "str_begin", |mut c: Caller<VelHost>| {
        c.data_mut().str_begin();
    })?;

    linker.func_wrap(
        "vel/runtime",
        "str_lit",
        |mut c: Caller<VelHost>, ptr: i32, len: i32| {
            let s = read_wasm_string(&mut c, ptr, len);
            c.data_mut().str_lit(s);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "str_num",
        |mut c: Caller<VelHost>, val: f64| {
            c.data_mut().str_num(val);
        },
    )?;

    linker.func_wrap("vel/runtime", "str_done", |mut c: Caller<VelHost>| {
        c.data_mut().str_done();
    })?;

    // ── API imports ───────────────────────────────────────────────────────────

    linker.func_wrap(
        "vel/runtime",
        "api_fetch",
        |mut c: Caller<VelHost>,
         method_ptr: i32,
         method_len: i32,
         url_ptr: i32,
         url_len: i32|
         -> i32 {
            let method = read_wasm_string(&mut c, method_ptr, method_len)
                .unwrap_or_default()
                .to_lowercase();
            let url = read_wasm_string(&mut c, url_ptr, url_len).unwrap_or_default();

            let host = c.data_mut();
            let reqid = host.alloc_reqid();
            let request_body = host.take_pending_body().unwrap_or_default();
            let request_headers = host.take_pending_headers();
            host.api_store.lock().unwrap().insert(
                reqid,
                ApiEntry {
                    status: 0,
                    body: String::new(),
                    url: url.clone(),
                    cancelled: false,
                },
            );
            let store_ref = Arc::clone(&host.api_store);
            let wakeup = host.api_wakeup.clone();

            #[cfg(feature = "renderer")]
            {
                let _ = std::thread::spawn(move || {
                    let result = crate::linker_api::do_http_request(
                        &method,
                        &url,
                        &request_body,
                        &request_headers,
                    );
                    {
                        let mut store = store_ref.lock().unwrap();
                        if let Some(entry) = store.get_mut(&reqid) {
                            match result {
                                Ok(body) => {
                                    entry.status = 1;
                                    entry.body = body;
                                }
                                Err(e) => {
                                    entry.status = 2;
                                    entry.body = e;
                                }
                            }
                        }
                    }
                    let cancelled = store_ref
                        .lock()
                        .ok()
                        .and_then(|s| s.get(&reqid).map(|e| e.cancelled))
                        .unwrap_or(true);
                    if !cancelled && let Some(w) = wakeup {
                        w();
                    }
                });
            }

            // In test mode (no renderer): immediately succeed with an empty body
            #[cfg(not(feature = "renderer"))]
            {
                let _ = (method, url, request_body, request_headers, wakeup);
                let mut store = store_ref.lock().unwrap();
                if let Some(entry) = store.get_mut(&reqid) {
                    entry.status = 1;
                }
            }

            reqid as i32
        },
    )?;

    linker.func_wrap("vel/runtime", "header_begin", |mut c: Caller<VelHost>| {
        c.data_mut().header_begin();
    })?;

    linker.func_wrap(
        "vel/runtime",
        "header_field",
        |mut c: Caller<VelHost>, key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32| {
            let key = read_wasm_string(&mut c, key_ptr, key_len);
            let val = read_wasm_string(&mut c, val_ptr, val_len);
            c.data_mut().header_field(key, val);
        },
    )?;

    linker.func_wrap("vel/runtime", "header_done", |mut c: Caller<VelHost>| {
        c.data_mut().header_done();
    })?;

    linker.func_wrap(
        "vel/runtime",
        "header_field_str",
        |mut c: Caller<VelHost>, key_ptr: i32, key_len: i32| {
            let key = read_wasm_string(&mut c, key_ptr, key_len);
            c.data_mut().header_field_str(key);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "print_str",
        |mut c: Caller<VelHost>, ptr: i32, len: i32| {
            let s = read_wasm_string(&mut c, ptr, len);
            c.data().print_str(s);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "print_str_built",
        |mut c: Caller<VelHost>| {
            c.data_mut().print_str_built();
        },
    )?;

    linker.func_wrap("vel/runtime", "window_width", |c: Caller<VelHost>| -> f64 {
        c.data().window_width
    })?;

    linker.func_wrap(
        "vel/runtime",
        "state_push",
        |mut c: Caller<VelHost>, kind: i32| {
            c.data_mut().state_push(kind);
        },
    )?;

    linker.func_wrap("vel/runtime", "state_pop", |mut c: Caller<VelHost>| {
        c.data_mut().state_pop();
    })?;

    crate::linker_text::register_text_imports(&mut linker)?;
    crate::linker_api::register_core_api_imports(&mut linker)?;
    crate::linker_api::register_api_imports(&mut linker)?;
    crate::linker_list::register_list_imports(&mut linker)?;
    crate::linker_persist::register_persist_imports(&mut linker)?;

    Ok(linker)
}

// ── WASM memory helper ────────────────────────────────────────────────────────

pub(crate) fn read_wasm_string(caller: &mut Caller<VelHost>, ptr: i32, len: i32) -> Option<String> {
    if ptr < 0 || len <= 0 {
        return None;
    }
    let memory = caller.get_export("memory")?.into_memory()?;
    let data = memory.data(caller);
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    if end > data.len() {
        return None;
    }
    String::from_utf8(data[start..end].to_vec()).ok()
}
