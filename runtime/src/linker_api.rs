use std::sync::Arc;

use wasmtime::{Caller, Linker};

use crate::RuntimeError;
use crate::host::{ApiEntry, VelHost};
use crate::linker::read_wasm_string;

pub(crate) fn register_core_api_imports(linker: &mut Linker<VelHost>) -> Result<(), RuntimeError> {
    linker.func_wrap(
        "vel/runtime",
        "api_poll",
        |c: Caller<VelHost>, reqid: i32| -> i32 {
            c.data()
                .api_store
                .lock()
                .unwrap()
                .get(&(reqid as u32))
                .map(|e| e.status as i32)
                .unwrap_or(0)
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "api_error_str",
        |mut c: Caller<VelHost>, reqid: i32| {
            c.data_mut().api_error_str(reqid as u32);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "api_field_str",
        |mut c: Caller<VelHost>, reqid: i32, field_ptr: i32, field_len: i32| {
            let field = read_wasm_string(&mut c, field_ptr, field_len).unwrap_or_default();
            c.data_mut().api_field_str(reqid as u32, &field);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "api_field_num",
        |mut c: Caller<VelHost>, reqid: i32, field_ptr: i32, field_len: i32| -> f64 {
            let field = read_wasm_string(&mut c, field_ptr, field_len).unwrap_or_default();
            c.data().api_field_num(reqid as u32, &field)
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "api_field_bool",
        |mut c: Caller<VelHost>, reqid: i32, field_ptr: i32, field_len: i32| -> i32 {
            let field = read_wasm_string(&mut c, field_ptr, field_len).unwrap_or_default();
            c.data().api_field_bool(reqid as u32, &field)
        },
    )?;

    Ok(())
}

pub(crate) fn register_api_imports(linker: &mut Linker<VelHost>) -> Result<(), RuntimeError> {
    linker.func_wrap("vel/runtime", "body_begin", |mut c: Caller<VelHost>| {
        c.data_mut().body_begin();
    })?;

    linker.func_wrap(
        "vel/runtime",
        "body_field_str",
        |mut c: Caller<VelHost>, key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32| {
            let key = read_wasm_string(&mut c, key_ptr, key_len);
            let val = read_wasm_string(&mut c, val_ptr, val_len);
            c.data_mut().body_field_str(key, val);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "body_field_num",
        |mut c: Caller<VelHost>, key_ptr: i32, key_len: i32, val: f64| {
            let key = read_wasm_string(&mut c, key_ptr, key_len);
            c.data_mut().body_field_num(key, val);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "body_field_bool",
        |mut c: Caller<VelHost>, key_ptr: i32, key_len: i32, val: i32| {
            let key = read_wasm_string(&mut c, key_ptr, key_len);
            c.data_mut().body_field_bool(key, val);
        },
    )?;

    linker.func_wrap("vel/runtime", "body_done", |mut c: Caller<VelHost>| {
        c.data_mut().body_done();
    })?;

    linker.func_wrap("vel/runtime", "url_begin", |mut c: Caller<VelHost>| {
        c.data_mut().url_begin();
    })?;

    linker.func_wrap(
        "vel/runtime",
        "url_lit",
        |mut c: Caller<VelHost>, ptr: i32, len: i32| {
            let s = read_wasm_string(&mut c, ptr, len);
            c.data_mut().url_lit(s);
        },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "url_num",
        |mut c: Caller<VelHost>, val: f64| {
            c.data_mut().url_num(val);
        },
    )?;

    linker.func_wrap("vel/runtime", "url_done", |mut c: Caller<VelHost>| {
        c.data_mut().url_done();
    })?;

    linker.func_wrap(
        "vel/runtime",
        "api_url_changed",
        |mut c: Caller<VelHost>, reqid: i32| -> i32 { c.data_mut().api_url_changed(reqid as u32) },
    )?;

    linker.func_wrap(
        "vel/runtime",
        "api_fetch_dyn",
        |mut c: Caller<VelHost>, method_ptr: i32, method_len: i32| -> i32 {
            let method = read_wasm_string(&mut c, method_ptr, method_len)
                .unwrap_or_default()
                .to_lowercase();
            let host = c.data_mut();
            let url = std::mem::take(&mut host.pending_url);
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
                    let result = do_http_request(&method, &url, &request_body, &request_headers);
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

    Ok(())
}

#[cfg(feature = "renderer")]
pub(crate) fn do_http_request(
    method: &str,
    url: &str,
    body: &str,
    headers: &[(String, String)],
) -> Result<String, String> {
    let mut req = match method {
        "post" => ureq::post(url).set("Content-Type", "application/json"),
        "put" => ureq::put(url).set("Content-Type", "application/json"),
        "delete" => ureq::delete(url),
        _ => ureq::get(url),
    };
    for (k, v) in headers {
        req = req.set(k, v);
    }
    let resp = match method {
        "post" | "put" => req.send_string(body),
        _ => req.call(),
    };
    resp.map_err(|e| e.to_string())
        .and_then(|r| r.into_string().map_err(|e| e.to_string()))
}
