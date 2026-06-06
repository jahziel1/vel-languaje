# Vel — Current State

## Where we are
Compiler, runtime, CLI, VS Code extension, hot reload, reactivity y navigation completos.
Codegen sin deuda técnica: condiciones con variables, booleans, strings interpolados — todos correctos y WASM-válidos.
**Tipos custom** ✅: field_ty, error en campo undefined, error en tipo no definido.
**Optional chaining y null coalescing** ✅: `user?.name`, chained `user?.address?.city`, `?? "fallback"` — mismatch detectado.
**Typed API responses** ✅: `Result<User>` y `Result<List<User>>` como anotación. `Success(data)` → `data: User`. `list(data) { item -> text(item.name) }` → `item: User`, `item.name: Text`. Parser: `Result<T>` en type annotations. Checker: `variant_binding_ty`, list elem propagation en ForEach, `ty_compatible` para nested Unknown. 146 tests.
**List avanzados** ✅: `data.length` → Number, `data.isEmpty` → Bool, `data.sum(item -> item.field)` → Number (host-side), `list(data.filter(item -> pred)) { ... }` — inline WASM block/skip. Parser: lambda args `item -> expr`. host: `list_sum()`. RT_LIST_SUM=40, RT_IMPORT_COUNT=41. checker/exhaust.rs split. 150 tests.
**Módulos** ✅: compiler/src/loader.rs — load_program() recursivo, dedup diamante, detección circular. CLI usa loader. Codegen: Button() → component_Button lookup. tempfile en dev-deps. 156 tests.
**Store — estado global compartido** ✅: `store auth { state { ... } derived { ... } fn logout() { ... } }`. WASM globals con key `"StoreName/field"`. Derived inline en uso. Fns exportadas como `store_{Name}_{fn}`. Checker: store names en scope. 135 tests (9 nuevos tests_store). Codegen spliteado: store_emit.rs, text_arg_emit.rs, expr_i32.rs.

**`persist` en stores** ✅: variables marcadas `persist` se guardan en device storage automáticamente. `store settings { state { volume: Number = 80, persist } fn setVolume(v: Number) { volume = v } }` — al llamar setVolume(), el nuevo valor se escribe en `.vel_persist.json`. Al reinstanciar, `vel_persist_init()` restaura los globals desde disco. 4 nuevos imports WASM (persist_get_num/bool, persist_set_num/bool). RT_IMPORT_COUNT=45. 172 tests. Codegen spliteado: ctx.rs, persist_emit.rs, assemble_imports.rs. Runtime: host_persist.rs, linker_persist.rs, linker_list.rs.

**`api.headers {}` en stores** ✅: `api.headers { Authorization: "Bearer {token}" }` en stores. Headers se aplican automáticamente a todas las API calls del programa. Valores estáticos usan `header_field` directo; valores interpolados con text state usan `str_begin/text_state_get/header_field_str` (import idx 45). `RT_IMPORT_COUNT=46`. header_emit.rs split de api_emit.rs. 4 nuevos tests en tests_store.rs. 176 tests totales.

**Navigation params** ✅: `go("/product", id: 42)` + `page Product(id: Number)`. Pages con params tienen firma WASM tipada (f64 por param). Codegen emite `nav_param(idx, val)` antes de `navigate()`. Runtime: `VelHost.nav_params` acumula los params; `PageInstance.go()` los captura; `render()` usa `Func::call` con `Vec<Val>` en lugar de `get_typed_func`. 1 nuevo import WASM `nav_param` (idx 46). RT_IMPORT_COUNT=47. 180 tests.

**Dogfooding checkpoint** ✅: App `examples/shopapp/` (Login → Products → Product) escrita y corrida con `vel run`. Dos bugs encontrados y corregidos:
- Bug #1 (ergonomía): `fontWeight: bold` da "undefined variable" — la sintaxis correcta es `bold: true`. Documentado en 4b-4.
- Bug #2 (crash): match bindings (`Success(data)`, `Error(msg)`) dentro de bloques UI (`column {}`) no se pre-escaneaban → `{msg}` en interpolación emitía `I32Const(0)` donde se esperaba f64 → WASM type error. Fix: `pre_scan_locals` recurse en bloques UI + `emit_expr_f64` garantiza f64 para idents.
**Bloque 4b — Debug experience** documentado en PENDIENTE.md con investigación de industria. 182 tests.

### Next session starts here

**Prioridad reordenada — decisión tomada:**
El web es el target principal de distribución. El orden queda:
1. **Bloque 4b** (debug tools) — error overlay, print(), vel check, errores descriptivos
2. **Bloque 9** (Phase 2 web) — Host JS + canvas WebGPU, vel serve, vel build --web
3. Resto de bloques (UI, stdlib, guard, etc.)

Ver ARQUITECTURA.md § "Phase 2: target browser" para el plan técnico detallado.
Ver PENDIENTE.md § "Bloque 9" para los items concretos.

**4b-3 `vel check`** ✅ — `vel check <file.vel>` implementado. Exit 0 sin errores, exit 1 con errores + spans. tests_nav.rs split de tests.rs (regla 300 líneas). 182 tests.

**4b-2 `print()`** ✅ — `print("msg")` y `print("val: {count}")` implementados. RT_PRINT_STR=47 (ptr/len), RT_PRINT_BUILT=48 (lee str_builder). Output a stderr con prefijo `[vel]`. RT_IMPORT_COUNT=49. 185 tests.

**4b-1 Error overlay** ✅ — Errores de compile/runtime se dibujan en la ventana con Vello. Background negro, header rojo, mensaje + span con `^`. `run_window_error(msg, title)` abre ventana standalone. `VelEvent::Error(msg)` muestra overlay sobre el frame actual en `vel watch` sin matar la instancia. draw_error.rs + renderer_error.rs. 185 tests.

**4b-4 Mensajes de error descriptivos** ✅ — `TypeError` tiene campo `help: Option<String>`. `suggest_for_undefined()` en checker/suggest.rs detecta patrones comunes y emite sugerencias concretas: bool props (`bold` → `bold: true`, etc.), color names, align/cursor/overflow/shadow keywords. CLI: `format_type_error()` muestra `help:` tras el pointer. Error overlay: líneas `help:` en verde suave. 3 nuevos tests (188 totales).

**Bloque 9 — Phase 2 browser** ✅ — `web/vel_host.js` implementa los 49 imports vel/runtime en JavaScript + Canvas 2D renderer (layout column/row/center/rect/text/input/Button + hit testing + input handling via hidden HTML input + SSE hot reload). `vel serve <file.vel>` levanta HTTP devserver en :4000 con hot reload via SSE. `vel build --web <file.vel>` produce `dist/` (app.wasm + vel_host.js + index.html) listo para CDN. Archivos web/ embebidos con `include_str!()`. 188 tests, 4 checks verdes, 300-line rule limpia (splits: tests_print.rs, tests_store_multi.rs, host_nav.rs).

**Responsive** ✅ — `on phone/mobile/tablet/desktop/wide { }` condicionan el render según el ancho de ventana. Compile-time: `emit_responsive_block` emite `call window_width / f64.const / f64.lt|ge / if`. RT_WINDOW_WIDTH=49, RT_IMPORT_COUNT=50. `VelHost.window_width: f64` (default 1024). `PageInstance.set_window_width(w)`. 201 tests. lifecycle_emit.rs + tests_lifecycle.rs splitean 300-line.

**`on mount` / `on unmount`** ✅ — Lifecycle hooks para páginas. `on mount { ... }` corre una vez al cargar la página (o al navegar a ella); `on unmount { ... }` corre al salir. Codegen emite `page_X__mount` / `page_X__unmount` como funciones WASM exportadas. Runtime: `PageInstance.mounted: bool`; `render()` llama mount después del primer render; `go()` llama unmount antes de cambiar. 195 tests (157 compiler, 38 runtime). 4 checks verdes.

**Hover / focus / active states** ✅ — `rect(hover: { background: gray50 })`, `input(focus: { border: 2, borderColor: blue })`, `Button("OK", active: { background: #1a3a8a })`. Codegen emite `state_push(kind)` + props + `state_pop()`. RT_STATE_PUSH=50, RT_STATE_POP=51, RT_IMPORT_COUNT=52. Host: `state_mode`/`state_buf` routing en `prop_f64`/`prop_bool`. Draw pass: `DrawState { cursor, focused_input, is_mouse_down }` → `effective_overlay()` selecciona hover > active > focus. Web host: `getEffectiveProps()` con `mousemove`/`mousedown`/`mouseup` tracking. También: `window_width` agregado al web host (faltaba). Splits: `host_state.rs`, `parser/ast_ops.rs` (BinOp/UnOp), `parse_stmts_until_rbrace` movida a `parser/mod.rs`. 205 tests. 4 checks verdes.

**Stubs silenciosos → errores de compilador** ✅ — `field_ty` en checker/infer.rs ahora emite error para field access en tipos concretos no-Named (Number, Text, Bool, Optional, Result, etc.). `Unknown` sigue silencioso para no duplicar errores upstream. Test `test_custom_type_field_number` corregido para usar `p?.price` (correcto). 3 nuevos tests: `test_field_on_optional_without_chaining_is_error`, `test_field_on_number_is_error`, `test_field_on_text_is_error`. 208 tests. 4 checks verdes.

**9-WebGPU Rust WASM** ✅ — Renderer del browser migrado a Rust WASM. Crate `renderer-web/` (wgpu 23 + wasm-bindgen, sin winit/wasmtime). WGSL shaders para rects con SDF rounded corners. Text via OffscreenCanvas web-sys → GPU texture. `vel_host.js` reducido de ~700 líneas a ~350 líneas de bootstrapper puro — cero lógica de rendering en JS. `vel serve` y `vel build --web` sirven `vel_renderer_web.wasm` (231KB) + `vel_renderer_web.js`. Toolchain: `wasm32-unknown-unknown` + `wasm-pack 0.15`. 208 tests. 4 checks verdes.

**Deuda filosófica completa.** Ambos puntos resueltos:
1. ✅ Stubs silenciosos → errores de compilador
2. ✅ Renderer del browser → Rust WASM (wgpu/WebGPU), no JS

**Web browser render fix** ✅ — `vel serve` ahora pinta en el primer frame sin necesitar resize. Root cause: `GpuState::new` llama `configure(1,1)` que resetea `canvas.width/height` a 1. Fix: `resizeCanvas()` después de `init_renderer()` + prime render vacío para warm-up del surface. También: guard `rendererReady` previene renders prematuros antes de que init() complete; `drawError` cae a `console.error` si el renderer no está listo; página inicial usa `Login` como fallback en vez del primer export arbitrario. Test flakiness fix: `last_reqid()` en `PageInstance` reemplaza reqids hardcodeados en tests_list.rs. 208 tests, 4 checks verdes.

**`grid` layout** ✅ — `grid(columns: 3, gap: 24)` y `grid(columns: auto, minWidth: 200)`. Props: `columns` (key 37), `minWidth` (key 38). Layout engine: grid real con rows/cols, gap entre celdas. `layout` movido a `pub mod` (sin feature gate). tests_grid.rs con 5 tests unitarios de posición. 216 tests. 4 checks verdes.

**Theme system** ✅ — `export theme { colors { primary: #3B82F6 } text { xs: 12 } radius { sm: 4 } }`. `theme.colors.X` → packed RGB inline (compile-time constant, no new WASM imports). `theme.text.X` / `theme.radius.X` → `F64Const` inline. Checker: `theme.section.key` → `Ty::Text` (color) / `Ty::Number`; token desconocido → error. Token::Theme en lexer, ThemeDef en AST, `resolve_color_rgb` helper en CodeGen. 7 nuevos tests (tests_theme.rs). 223 tests. 4 checks verdes. RT_IMPORT_COUNT=52.

### Next session starts here

**Siguiente: Bloque 6 (stdlib) — spinner, toast, modal, tooltip, checkbox, toggle, select, textarea, tabs, accordion, avatar, badge, virtualList.**

**223 tests. 4 checks verdes. RT_IMPORT_COUNT=52.**

---

## Completed

### Language design — 100% (documented in LENGUAJE.md)
- Type system, syntax rules, control flow, error handling, string interpolation
- Layout system, visual design, state system, navigation, API, modules, stdlib design

### Compiler — complete (phase 1)
- [x] Lexer — tokenizes all Vel syntax — 11 tests
- [x] Parser — full AST, Pratt expression parser — 13 tests
- [x] Quality pipeline — rustfmt, clippy, pre-commit hook, CI, coverage 83%
- [x] Type checker — undefined vars, no coercion, exhaustive match — 12 tests
- [x] Code generator (WASM) — valid WASM binary, globals, exports, UI calls — 13 tests
- [x] String table — intern(), data section, strings read from WASM linear memory
- [x] Match/if branching — WASM if/else chains, true/false patterns, enum discriminants
- [x] `go()` navigation — emits `navigate(ptr, len)` WASM call
- [x] `onClick` prop — emits `set_on_click(ptr, len)` with exported handler name
- [x] **Input handling** — `input(var, placeholder: "text", onEnter: fn)` completo
- [x] **Props in block syntax** — `text("x") { bold: true }` y `column { padding: 24 }` compilados
- [x] **Visual props** — color, background, fontSize/fontWeight, radius, shadow, align, overflow, opacity, borderColor, lines, letterSpacing, cursor, underline, strikethrough, animate, visible
- [x] **`derived`** — locals WASM pre-scanned, computados antes del UI tree. Siempre frescos.
- [x] **`on change`** — shadow global `var__prev`. Body corre solo cuando el valor cambió.
- [x] **API request body** — `api.post("/order", { qty: count, active: true })` — body_begin/field_*/done
- [x] **Re-fetch condicional** — `api.get("/products/{id}")` — url_begin/lit/num/done + api_url_changed + api_fetch_dyn. 109 tests passing
- [x] **Headers / auth** — `api.get("/me", headers: { Authorization: "Bearer token" })` — header_begin/field/done, 3 nuevos imports WASM (idx 29-31), RT_IMPORT_COUNT=32. 114 tests passing
- [x] **Cancelación inflight** — `ApiEntry.cancelled`, `cancel_pending_requests()` en `go()`. Threads no disparan wakeup si la página ya cambió.
- [x] **Text state** — `name = ""` → host-side `text_state: HashMap`. 4 imports WASM (idx 32-35): get/set/bool/set_built. `text("{name}")`, `if name {}`, `name = "val"`, input binding → set_text_state. RT_IMPORT_COUNT=36. 122 tests.
- [x] **List básica** — `list(data) { item -> text(item.name) }` — renderiza arrays de API. Parser: `ForEach` stmt en bloque de element. Codegen: WASM loop (block/loop/br_if), 4 imports (idx 36-39): list_count, list_item_str/num/bool. RT_IMPORT_COUNT=40. 129 tests. host_list.rs separado.
- [x] **API calls** — `api.get/post/put/delete` con Loading/Success/Error:
  - Compilador: estado async → dos globals i32 (status + reqid), `emit_api_inits` en cada página
  - WASM: 3 nuevos imports (`api_fetch`, `api_poll`, `api_error_str`), ahora RT_IMPORT_COUNT=18
  - Match: `Success(data)` y `Error(msg)` bindean reqid en local f64
- [x] **API field access** — `data.name`, `data.price`, `data.active` en arm `Success(data)`:
  - 3 nuevos imports: `api_field_str`, `api_field_num`, `api_field_bool` (JSON parsing via serde_json)
  - Codegen: `ctx.api_bindings` trackea bindings de Success; `Expr::Field` emite el import correcto por contexto
  - `text(data.name)` → str_begin + api_field_str + str_done; `data.price * 2` → api_field_num; `data.active` → api_field_bool
  - Interpolación `"{data.name}"` soportada vía api_field_str dentro del loop str_*
  - Runtime: `VelHost` con `ApiStore` (Arc<Mutex>), wakeup callback para re-render
  - HTTP: ureq (renderer only), background thread, `EventLoopProxy<VelEvent>` dispara re-render
  - Código partido en módulos: `api_emit.rs`, `linker.rs`, `tests_api.rs`, `tests_props.rs`, `tests_state.rs`
- [x] **Codegen correctness fixes** — 86 tests passing, 0 deuda técnica:
  - `emit_expr_i32` correcto: comparaciones (`count > 0`), `and`/`or`, variables f64 → i32 via F64Ne
  - Bool globals: asignación usa emit_expr_i32 para globals i32 (evita type mismatch WASM)
  - String interpolation: `text("Hola {name}")` emite str_begin/lit/num/done; rinde valor dinámico

### Runtime — complete (phase 1)
- [x] WASM executor (wasmtime 29) — persistent `PageInstance` (Store + globals survive across calls)
- [x] vel/runtime host API — 12 imports: begin_element, end_element, prop_f64, prop_bool, text_content, navigate, set_on_click, set_on_change, str_begin, str_lit, str_num, str_done
- [x] UI tree builder — VelHost builds UiNode tree during execution; reset() clears for re-render
- [x] String builder — str_begin/lit/num/done assembla strings interpolados en el host
- [x] Debug renderer — render_debug() for tests
- [x] GPU renderer — Vello 0.4 + wgpu 23 + winit 0.30, native window
- [x] Text rendering — skrifa 0.26 + vello draw_glyphs, system fonts (Segoe UI/Arial)
- [x] Layout engine — LayoutBox, layout_root(), padding/gap/width/height, column/row/center
- [x] Hit testing — hit_test(nodes, boxes, x, y) returns on_click handler of deepest clicked element
- [x] Event loop reactivity — Click → call_handler() → WASM mutates globals → render() → redraw
- [x] Navigation — go("/path") / go(back): routes auto-built from WASM exports, history stack
- [x] 86 tests passing

### CLI — complete
- [x] `vel run <file>` — compile + instantiate + open native window
- [x] `vel watch <file>` — hot reload via notify crate (OS native events)
- [x] `vel build <file>` — compile to .wasm file on disk
- [x] `vel new <name>` — scaffold new project with interactive counter starter
- [x] Error messages with span — shows source line + `^` pointer at error column

### VS Code extension — `editors/vscode/`
- [x] TextMate grammar — keywords, types, strings + interpolation, operators, function calls, properties, comments
- [x] language-configuration.json — brackets, auto-close, indentation rules, line comments

---

## Deuda conocida — no urgente

### Performance: re-render coarse-grained
El runtime re-ejecuta la page function completa en cada render.
`LENGUAJE.md` promete fine-grained reactivity (solo los nodos que leyeron el estado que cambió).
Hoy: state change → full page re-run → árbol completo reconstruido → GPU dibuja todo.
Para apps simples: irrelevante (GPU + WASM son rápidos). Para apps complejas: bottleneck.

**Cuándo atacar:** antes de distribución via URL / producción. Requiere que el runtime trackee
dependencias estado→nodo y redibuje solo los afectados.

### Stubs sin implementar
- Field access (`user.name`) → devuelve 0.0 (fuera de contexto list/api)
- Collections (`Map`) → devuelve 0
- Lambda evaluation → inline, no callable real

### Features diseñadas en LENGUAJE.md — no iniciadas
El lenguaje está 100% diseñado. Lo siguiente existe en el spec pero no en el compilador/runtime:

**Tipo system:**
- Tipos custom (`type User { name: Text }`)
- Optional chaining `?.` y null coalescing `??`
- Typed API responses (`users: Result<List<User>>`)
- List y Map con operaciones reales (filter, map, sum, sort...)

**Estado global:**
- `store` — singleton compartido entre páginas
- `persist` — variables guardadas en device storage
- `api.headers {}` en stores — auth automático en todas las calls

**Navegación:**
- Navigation params — `go("/product", id: 42)` + `page Product(id: Number)`
- `guard` — protección de rutas antes de render
- `_layout.vel` — layouts persistentes con `outlet()`
- `_404.vel` — página de not found

**UI:**
- `on mount` / `on unmount` — lifecycle
- Responsive — `on phone {}` / `on tablet {}` / `on desktop {}`
- Hover / focus / active states
- `grid` layout
- Theme system (`theme.vel`)

**Standard library:**
- spinner, toast, modal, tooltip, tabs, accordion, divider
- textarea, checkbox, toggle, select, datepicker, filePicker
- avatar, badge, spacer, iconos built-in
- `virtualList`, `on key()`

**API avanzada:**
- Cache, paginación, WebSockets, file upload
- `vel.config.vel` con baseUrl y env()

**Módulos:**
- Sistema de imports entre archivos
- Barrel files, circular import detection

**Phase 2:**
- WebGPU target en el navegador
- `vel deploy`

---

## Compiler structure
```
compiler/src/
  lexer/     mod.rs, token.rs, keywords.rs, string.rs, tests.rs
  parser/    mod.rs, ast.rs, toplevel.rs, stmt.rs, expr.rs, tests.rs
  checker/   mod.rs, ty.rs, scope.rs, infer.rs, builtins.rs, tests.rs
  codegen/   mod.rs, props.rs, emit.rs, expr.rs, call_emit.rs, match_emit.rs, assemble.rs, tests.rs
```

## Runtime structure
```
runtime/src/
  lib.rs        — Runtime, PageInstance (render/call_handler/go/take_navigation)
  host.rs       — VelHost (element stack, string builder, navigate_to)
  tree.rs       — UiNode (tag, props, text, children, on_click)
  layout.rs     — LayoutBox, layout_root(), hit_test()
  draw.rs       — draw_tree() → Vello scene, prop-driven color/size/radius
  text.rs       — TextPainter (skrifa glyph rendering)
  gpu.rs        — GpuState (wgpu device, surface, Vello renderer)
  renderer.rs   — VelApp (ApplicationHandler), run_window, run_window_watch
  render.rs     — render_debug() for tests
```

## Code generator — what it emits
- State variables → WASM mutable globals (Number=f64, Bool=i32)
- `derived` entries → WASM locals (f64), computed at top of page function
- `on change var` → shadow global `var__prev`; comparison + conditional body each render
- `page X` → exported fn `page_X()`
- `fn f` inside page → exported fn `Page_f()`
- `component X` → exported fn `component_X()`
- UI elements → vel/runtime import calls (begin_element, props, text_content/str_*, end_element)
- `onClick: fn` → set_on_click("Page_fn") before end_element
- `go("/path")` → navigate(ptr, len)
- Arithmetic → f64 WASM ops; booleans → i32; comparisons → i32 (no F64Convert in bool context)
- Static strings → data section (ptr+len); interpolated strings → str_begin/lit/num/done

## Type checker — what it checks
- Undefined variables (built-ins pre-loaded: layout, components, go, api…)
- No implicit coercion (Text + Number → error)
- Arithmetic and comparison only on Number
- Match exhaustiveness on named enums and Result<T>
- Wildcard `_` covers remaining cases
- Enum variants auto-populated in scope
- Function and lambda params bound in their scope
- State entries inferred and added to scope

## Key files
- `LENGUAJE.md` — all language decisions (source of truth)
- `FILOSOFIA.md` — why Vel exists, what we're replacing, phase 1 vs phase 2
- `ARQUITECTURA.md` — technical stack
- `PENDIENTE.md` — build status
- `CLAUDE.md` — instructions for Claude at session start
- `examples/counter.vel` — basic counter with state
- `examples/nav.vel` — multi-page navigation demo
