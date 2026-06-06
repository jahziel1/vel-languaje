use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::{fs, process};

use notify::{EventKind, RecursiveMode, Watcher, recommended_watcher};

use crate::{compile_source, try_compile};

const VEL_HOST_JS: &str = include_str!("../../web/vel_host.js");
const INDEX_HTML: &str = include_str!("../../web/index.html");
const RENDERER_JS: &str = include_str!("../../renderer-web/pkg/vel_renderer_web.js");
const RENDERER_WASM: &[u8] = include_bytes!("../../renderer-web/pkg/vel_renderer_web_bg.wasm");
const DEFAULT_PORT: u16 = 4000;

// ── vel serve ─────────────────────────────────────────────────────────────────

pub fn cmd_serve(path_arg: Option<&String>) {
    let path = path_arg.unwrap_or_else(|| {
        eprintln!("error: missing file\nUsage: vel serve <file.vel>");
        process::exit(1);
    });

    let compiled = compile_source(path);
    let wasm_ref: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(compiled.wasm));
    let clients: Arc<Mutex<Vec<mpsc::SyncSender<()>>>> = Arc::new(Mutex::new(vec![]));

    let wasm_srv = Arc::clone(&wasm_ref);
    let clients_srv = Arc::clone(&clients);
    std::thread::spawn(move || serve_http(DEFAULT_PORT, wasm_srv, clients_srv));

    eprintln!("[vel] serving at http://localhost:{DEFAULT_PORT}");
    eprintln!("[vel] watching {path}...");

    let abs = fs::canonicalize(path).unwrap_or_else(|_| Path::new(path).to_path_buf());
    let (tx, rx) = mpsc::channel();
    let mut watcher = recommended_watcher(move |res| {
        let _ = tx.send(res);
    })
    .unwrap_or_else(|e| {
        eprintln!("error: cannot start watcher: {e}");
        process::exit(1);
    });
    watcher
        .watch(&abs, RecursiveMode::NonRecursive)
        .unwrap_or_else(|e| {
            eprintln!("error: cannot watch '{path}': {e}");
            process::exit(1);
        });

    let path_owned = path.to_owned();
    loop {
        match rx.recv() {
            Ok(Ok(ev)) if matches!(ev.kind, EventKind::Modify(_) | EventKind::Create(_)) => {
                match try_compile(&path_owned) {
                    Ok(c) => {
                        eprintln!("[vel] recompiled ({} bytes)", c.wasm.len());
                        *wasm_ref.lock().unwrap() = c.wasm;
                        let mut locked = clients.lock().unwrap();
                        locked.retain(|tx| tx.try_send(()).is_ok());
                    }
                    Err(e) => eprintln!("{e}"),
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
}

// ── vel build --web ───────────────────────────────────────────────────────────

pub fn cmd_build_web(path_arg: Option<&String>) {
    let path = path_arg.unwrap_or_else(|| {
        eprintln!("error: missing file\nUsage: vel build --web <file.vel>");
        process::exit(1);
    });

    let compiled = compile_source(path);

    let dist = Path::new("dist");
    fs::create_dir_all(dist).unwrap_or_else(|e| {
        eprintln!("error: cannot create dist/: {e}");
        process::exit(1);
    });

    fs::write(dist.join("app.wasm"), &compiled.wasm).unwrap_or_else(|e| {
        eprintln!("error: cannot write dist/app.wasm: {e}");
        process::exit(1);
    });
    fs::write(dist.join("vel_host.js"), VEL_HOST_JS).unwrap_or_else(|e| {
        eprintln!("error: cannot write dist/vel_host.js: {e}");
        process::exit(1);
    });
    fs::write(dist.join("vel_renderer_web.js"), RENDERER_JS).unwrap_or_else(|e| {
        eprintln!("error: cannot write dist/vel_renderer_web.js: {e}");
        process::exit(1);
    });
    fs::write(dist.join("vel_renderer_web_bg.wasm"), RENDERER_WASM).unwrap_or_else(|e| {
        eprintln!("error: cannot write dist/vel_renderer_web_bg.wasm: {e}");
        process::exit(1);
    });
    fs::write(dist.join("index.html"), INDEX_HTML).unwrap_or_else(|e| {
        eprintln!("error: cannot write dist/index.html: {e}");
        process::exit(1);
    });

    println!("built → dist/");
    println!("  app.wasm ({} bytes)", compiled.wasm.len());
    println!("  vel_renderer_web_bg.wasm ({} bytes)", RENDERER_WASM.len());
    println!("  vel_renderer_web.js");
    println!("  vel_host.js");
    println!("  index.html");
    println!();
    println!("deploy dist/ to any static CDN or server");
}

// ── HTTP server ───────────────────────────────────────────────────────────────

fn serve_http(
    port: u16,
    wasm: Arc<Mutex<Vec<u8>>>,
    clients: Arc<Mutex<Vec<mpsc::SyncSender<()>>>>,
) {
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).unwrap_or_else(|e| {
        eprintln!("error: cannot bind port {port}: {e}");
        process::exit(1);
    });

    for stream in listener.incoming().flatten() {
        let wasm = Arc::clone(&wasm);
        let clients = Arc::clone(&clients);
        std::thread::spawn(move || handle_http(stream, wasm, clients));
    }
}

fn handle_http(
    mut stream: std::net::TcpStream,
    wasm: Arc<Mutex<Vec<u8>>>,
    clients: Arc<Mutex<Vec<mpsc::SyncSender<()>>>>,
) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).unwrap_or(0);
    let req = std::str::from_utf8(&buf[..n]).unwrap_or("");
    let path = req
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/");

    match path {
        "/" | "/index.html" => respond(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            INDEX_HTML.as_bytes(),
        ),
        "/vel_host.js" => respond(
            &mut stream,
            "200 OK",
            "application/javascript; charset=utf-8",
            VEL_HOST_JS.as_bytes(),
        ),
        "/vel_renderer_web.js" => respond(
            &mut stream,
            "200 OK",
            "application/javascript; charset=utf-8",
            RENDERER_JS.as_bytes(),
        ),
        "/vel_renderer_web_bg.wasm" => {
            respond(&mut stream, "200 OK", "application/wasm", RENDERER_WASM)
        }
        "/app.wasm" => {
            let bytes = wasm.lock().unwrap().clone();
            respond(&mut stream, "200 OK", "application/wasm", &bytes);
        }
        "/events" => serve_sse(&mut stream, clients),
        _ => respond(&mut stream, "404 Not Found", "text/plain", b"Not found"),
    }
}

fn serve_sse(stream: &mut std::net::TcpStream, clients: Arc<Mutex<Vec<mpsc::SyncSender<()>>>>) {
    let _ = stream.set_read_timeout(None);
    let _ = stream.write_all(
        b"HTTP/1.1 200 OK\r\n\
          Content-Type: text/event-stream\r\n\
          Cache-Control: no-cache\r\n\
          Connection: keep-alive\r\n\
          Access-Control-Allow-Origin: *\r\n\r\n\
          : keep-alive\n\n",
    );
    let (tx, rx) = mpsc::sync_channel(1);
    clients.lock().unwrap().push(tx);
    if rx.recv().is_ok() {
        let _ = stream.write_all(b"event: reload\ndata: {}\n\n");
    }
}

fn respond(stream: &mut std::net::TcpStream, status: &str, ct: &str, body: &[u8]) {
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {ct}\r\nContent-Length: {}\r\n\
         Cache-Control: no-cache\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(body);
}
