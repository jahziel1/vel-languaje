# Vel — Pending

## Language design — 100% complete
All decisions documented in LENGUAJE.md. Do not reopen without a technical reason.

---

## Build status

### Compiler
- [x] Lexer — 11 tests
- [x] Parser — 13 tests, full AST
- [x] Quality pipeline — rustfmt, clippy, pre-commit, CI, coverage 83%
- [x] Type checker — 12 tests
- [x] Code generator (WASM) — 13 tests
- [x] String table — intern(), data section, WASM linear memory
- [x] Match/if branching — WASM if/else chains, enum discriminants
- [x] `onClick` → set_on_click import
- [x] `go()` → navigate import
- [x] `derived` — locals WASM, siempre frescos
- [x] `on change` — shadow globals, disparo condicional
- [x] Codegen correctness — emit_expr_i32, bool globals, string interpolation

### Runtime
- [x] WASM executor — wasmtime 29, PageInstance (persistent Store)
- [x] vel/runtime API — 12 imports (+ str_begin/lit/num/done)
- [x] UI tree builder — VelHost, UiNode with on_click
- [x] GPU renderer — Vello + wgpu + winit, native window
- [x] Text rendering — skrifa, system fonts
- [x] Layout engine — LayoutBox, column/row/center, padding/gap
- [x] Hit testing — click → on_click handler lookup
- [x] Event loop reactivity — click → WASM handler → re-render
- [x] Navigation — go("/path"), go(back), auto-routes, history stack
- [x] Props in block syntax, visual props, input handling
- [x] State reactivity: `derived` + `on change`
- [x] String interpolation — str_begin/lit/num/done en host
- [x] API calls: `api.get/post/put/delete` with Loading/Success/Error

### CLI
- [x] `vel run`, `vel build`, `vel new`, `vel watch`
- [x] Error spans with source line + pointer
- [x] Hot reload — notify crate

### Tooling
- [x] VS Code extension — TextMate grammar, language-configuration

---

## Deuda filosófica — URGENTE (próxima sesión)

Estas deudas violan las promesas centrales del lenguaje. Bloquean features nuevas.

### 1. Stubs silenciosos → errores de compilador ✅

`field_ty` en checker/infer.rs emite error para field access en tipos concretos no-Named. `Optional(T).field` sin `?.` es error. `Number.field`, `Text.field` son error. `Unknown` silencioso para no duplicar errores. 3 nuevos tests. 208 tests totales.

### 2. Web host usa Canvas 2D, no WebGPU

`vel build --web` entrega una app que renderiza con Canvas 2D del browser.
Esto viola la tesis central: "el navegador como contenedor, no como plataforma — sin DOM, rendering GPU nativo".
Canvas 2D es CPU-side con aceleración parcial. No es lo que Vel promete.

**Lo que debe pasar:** reemplazar el renderer en `web/vel_host.js` con wgpu/Vello compilado a WASM via wasm-pack. El mismo código Rust de rendering que corre en nativo corre en el browser apuntando a WebGPU.

**Prerequisito:** investigar el estado de wasm-pack + wgpu + Vello como target WASM/WebGPU. Puede requerir ajustes al feature flag del runtime crate.

---

## Deuda conocida — no urgente

### Re-render coarse-grained
Hoy: state change → page function corre completa → árbol reconstruido → GPU dibuja todo.
Prometido: fine-grained reactivity (solo nodos que leyeron el estado que cambió).
**Atacar antes de:** distribución via URL / producción real.

### Otros stubs
- Collections (`Map`) → 0
- Lambda evaluation → inline, no callable real
- `cursor: pointer` no cambia el cursor del OS en el renderer nativo (gap entre spec y runtime)

---

## Priority order — próximas sesiones

**Sesión siguiente: deuda filosófica primero.**
Ver sección "Deuda filosófica — URGENTE" arriba. No avanzar a grid/theme hasta resolver.

---

### Bloque 1 — API completa (desbloquea apps reales)

1. [x] **API field access** — DONE
   `data.name`, `data.price`, `data.active` en arm `Success(data)`. 99 tests passing.

2. [x] **API request body** — DONE
   `api.post("/order", { qty: count })` — body_begin/field_num/bool/str/done. 105 tests passing.

3. [x] **Re-fetch condicional** — DONE
   `api.get("/products/{id}")` — url_begin/lit/num/done + api_url_changed. 109 tests passing.

4. [x] **Headers / auth** — DONE
   `api.get("/me", headers: { Authorization: "Bearer token" })`.
   3 imports WASM: header_begin/field/done. Valores estáticos. 114 tests.

5. [x] **Cancelación inflight** — DONE
   `ApiEntry.cancelled` + `cancel_pending_requests()` en `go()`. Threads no llaman wakeup si la página cambió.

5. **Cancelación inflight**
   Al navegar, cancelar requests pendientes para evitar re-renders en páginas desmontadas.

---

### Bloque 2 — Tipo system completo (desbloquea seguridad real)

6. [x] **Text state** — DONE. `name = ""` → host-side HashMap. get/set/bool/set_built imports. Input binding funcional.

7. [x] **Tipos custom** — DONE. `type User { name: Text, age: Number }` — field_ty resolver, error en campo undefined, error en tipo no definido. 135 tests.

7b. [x] **Optional chaining y null coalescing** — DONE. `user?.name`, `user?.address?.city` (chained, flatten doubles), `user?.name ?? "Guest"` — type mismatch en fallback. 141 tests.

8. [x] **Typed API responses** — DONE. `Result<User>` y `Result<List<User>>` como annotations. `Success(data)` → `data: User`, `item.name: Text` en `list(data)`. Parser: `Result<T>`. `ty_compatible` nested. 146 tests.

9. [x] **List básica** — DONE. `list(data) { item -> text(item.name) }`. Codegen: WASM loop, 4 nuevos imports (list_count/item_str/num/bool). 129 tests passing.

9b. [x] **List avanzados** — DONE. `length`, `isEmpty`, `sum(item -> item.field)`, `filter(item -> pred)`. host: list_sum. RT_IMPORT_COUNT=41. 150 tests.

---

### Bloque 3 — Módulos y estado global (desbloquea apps multi-pantalla)

10. [x] **Sistema de módulos** — DONE. loader.rs: resolución recursiva, dedup, circular detection. CLI integrada. component call fix. 156 tests.

11. [x] **`store` — estado global compartido** — DONE. WASM globals keyed `"StoreName/field"`. Derived inline. Fns exportadas `store_{Name}_{fn}`. Checker: store names en scope. 135 tests.

12. [x] **`persist` en stores** — DONE. `count: Number = 0, persist` guarda en `.vel_persist.json`. `vel_persist_init()` restaura globals al arrancar. 4 nuevos imports WASM. 172 tests.

13. [x] **`api.headers {}` en stores** — DONE. `api.headers { Authorization: "Bearer {token}" }` en stores. Headers automáticos en todas las API calls. 1 nuevo import WASM `header_field_str` (idx 45). RT_IMPORT_COUNT=46. 176 tests.

---

### Bloque 4 — Navegación completa

14. [x] **Navigation params** — DONE.
    `go("/product", id: 42)` + `page Product(id: Number)`. Pages con params tipados en WASM. 180 tests.

15. **`guard` — protección de rutas**
    ```vel
    page Dashboard {
        guard not auth.isLoggedIn -> go("/login")
    }
    ```
    Corre antes de renderizar. Puede ser async.

16. **`_layout.vel` — layouts persistentes**
    `outlet()` marca dónde va el contenido de la página.
    Navegar entre páginas del mismo folder no re-renderiza el layout.

17. **`_404.vel` — página de not found**
    Se muestra cuando ninguna ruta coincide.

---

### Bloque 4b — Debug experience

> Sin debug el lenguaje no es usable fuera de los autores. Estos ítems desbloquean que cualquier developer pueda trabajar con Vel sin necesitar el terminal abierto al lado, y alinean Vel con lo que la industria considera baseline hoy.

---

**4b-1. Error overlay en la ventana**

Cuando el compilador o el runtime fallan, dibujar el error directamente en el canvas con Vello. El developer ve el error en la misma ventana de la app, sin abrir el terminal.
- Compile error → overlay con mensaje + span antes de abrir la ventana
- Runtime error (render/handler/navigation) → overlay sobre el frame actual
- Hot reload fallido (`vel watch`) → overlay con el error mientras mantiene la versión anterior

**Investigación — qué hace la industria:**
- **React/Next.js** muestra un overlay negro con mensaje, stack trace, y la línea fuente resaltada. Next.js 15.2 sumó el fragmento de código con el error subrayado. Es el estándar que cualquier developer de web ya espera.
- **Flutter** muestra pantalla roja con el error en debug mode; en release usa `ErrorWidget.builder` configurable.
- **Svelte/SvelteKit** transmite el error HMR al cliente y lo muestra en pantalla con la ubicación del componente.

**Alineación con Vel:** La filosofía dice "sin magia, lo que ves es lo que pasa". Un error invisible (solo en stderr) contradice esto. El overlay es la implementación natural: Vel ya sabe dibujar texto en GPU, solo necesita un modo de error que lo use. No requiere infraestructura nueva.

**Mínimo viable:** Overlay semitransparente con: mensaje en rojo, `archivo:línea:col`, 3-5 líneas de fuente centradas en el error con `^` pointer.

---

**4b-2. `print()` builtin en Vel**

```vel
fn submit() {
    print("submitting: {email}")
    api.post("/login", { email: email })
}
```
Output al terminal donde corre `vel run`. Un import WASM `print_str(ptr, len)`. Mismo patrón que `navigate()`. No persiste, no bloquea — solo para desarrollo.

**Investigación — qué hace la industria:**
- **Elm**: `Debug.log "tag" value` — imprime al browser console y devuelve el valor (se puede insertar en cualquier expresión sin cambiar el tipo). Se elimina automáticamente en production builds.
- **Dart/Flutter**: `print()` a stdout; `debugPrint()` evita truncación. `dart:developer`'s `log()` para structured logging con niveles.
- **Swift**: `print()` a stdout, visible en la consola de Xcode.

**Convención de la industria:** Output a stdout (terminal/IDE console). Los lenguajes compilados no tienen `console.log` mágico — el developer sabe dónde mirar. Elm es el referente más cercano filosóficamente: `Debug.log` es explícito, no existe en production, y el compilador avisa si lo dejas puesto.

**Decisión de diseño para Vel:** Dos opciones posibles:
- `print("msg")` — simple, al terminal, siempre disponible
- `debug("tag", value)` — como Elm, devuelve el valor para que puedas inspeccionar sin cambiar la lógica

La segunda es más robusta pero más compleja. La primera es más simple y suficiente para empezar. Decidir antes de implementar.

---

**4b-3. `vel check <file>` — type-check sin correr**

Corre el compilador hasta el type checker y reporta todos los errores con spans. Sin abrir ventana, sin generar WASM. Para integración con editores y CI.
- Ya existe el checker, solo falta exponer el comando CLI
- Habilita scripts de CI: `vel check src/ && vel build src/main.vel`
- Base para la extensión de VS Code LSP (item 37)

**Investigación — qué hace la industria:**
- **Rust**: `cargo check` — el comando más usado del ecosistema. Hace type-check sin compilar a binario, es 3-5x más rápido que `cargo build`. Rust-analyzer lo usa internamente para feedback en tiempo real.
- **TypeScript**: `tsc --noEmit` — type-check sin emitir archivos. Estándar en cualquier CI de TypeScript.
- **Dart**: `dart analyze` — análisis estático integrado, corre en todos los IDEs en cada save.
- **Elm**: `elm make src/Main.elm --output=/dev/null` — workaround porque Elm no tiene un `check` dedicado. La comunidad lo pide constantemente.

**Expectativa baseline:** Exit code 0 = sin errores, nonzero = errores. Salida legible con `archivo:línea:col: mensaje`. Los editores modernos (VS Code, JetBrains) esperan poder correr este comando en cada save para mostrar squiggles inline.

**Alineación con Vel:** `vel check` es literalmente ejecutar el compilador sin el último paso. La infraestructura ya existe. Es el comando que un developer escribe antes de `vel run` y que el LSP llamará 10 veces por minuto.

---

**4b-4. Mensajes de error del type checker — calidad y sugerencias** ✅

`TypeError.help: Option<String>` — sugerencias en CLI (verde) y error overlay (verde suave).
`suggest_for_undefined()` en checker/suggest.rs cubre: bool props, color names, align/cursor/overflow/shadow keywords. 3 nuevos tests. 188 tests totales.

~~Hoy: `undefined variable \`bold\`` — sin contexto de dónde, qué se esperaba, ni cómo arreglarlo.
Meta: mensajes que explican el problema y sugieren la solución.~~

```
error: unknown identifier `bold`
  --> Login.vel:8:17
   |
 8 |     text("Sign in") { fontWeight: bold }
   |                                   ^^^^
   |
help: `fontWeight` expects a number — did you mean `bold: true`?
```

**Investigación — qué hace la industria:**
- **Rust**: Los errores incluyen `help:`, `note:`, y `consider:` lines. El compilador detecta patrones comunes y sugiere fixes concretos (`consider adding \`mut\``, `did you mean \`foo\`?`). Tiene códigos de error (`E0382`) que llevan a documentación. Considerado el mejor estándar de la industria hoy.
- **Elm**: Tono en primera persona ("I found a problem"), explica el razonamiento del type checker en lenguaje natural, y da contexto de por qué el tipo no coincide. Diseñado específicamente para developers que aprenden el lenguaje por primera vez — exactamente el usuario objetivo de Vel.
- **Swift**: Fix-its que el IDE puede aplicar automáticamente (un clic arregla el error). Integrado con SourceKit.

**Expectativa baseline:** Ubicación exacta (archivo:línea:col), línea de código con el problema subrayado, mensaje en inglés simple, y al menos una sugerencia concreta cuando el fix es obvio.

**Lo que Vel necesita específicamente:**
- Detectar `fontWeight: bold` → sugerir `bold: true` (el caso que encontramos en dogfooding)
- Detectar variables desconocidas que son colores conocidos → sugerir `color: #hex` o nombre
- Detectar tipos incorrectos en props → decir qué tipo se esperaba
- El tono debe ser neutral y directo — no condescendiente, no técnico

---

### Bloque 5 — UI completa

18. [x] **`on mount` / `on unmount`** — DONE. `page_X__mount` / `page_X__unmount` exportados. `PageInstance.mounted` + lógica en render/go. 195 tests.

19. [x] **Responsive** — DONE. `on phone/mobile/tablet/desktop/wide {}`. `window_width()` WASM import. 201 tests.

20. [x] **Hover / focus / active states** — DONE. `rect(hover: { background: gray50 })`, `input(focus: { border: 2, borderColor: blue })`. state_push/pop WASM imports. DrawState threads cursor+focus through draw pass. Web: getEffectiveProps() merges state props. 205 tests.

21. [x] **`grid` — layout de grilla** — DONE. `grid(columns: 3)` y `grid(columns: auto, minWidth: 200)`. Props keys 37/38. Layout real en runtime. 216 tests.

22. [x] **Theme system** — DONE. `export theme { colors { primary: #3B82F6 } text { xs: 12 } radius { sm: 4 } }`. Compile-time constants — no nuevos WASM imports. Checker: tipo inferido, error en token desconocido. 223 tests.

---

### Bloque 6 — Standard library

23. **Componentes de feedback**
    `spinner()`, `toast(msg, type: success)`, `modal(open: show) {}`, `tooltip("text") {}`

24. **Componentes de navegación**
    `tabs(selected: active) { tab("Name") {} }`, `accordion { section("Title") {} }`, `divider()`

25. **Inputs completos**
    `textarea`, `checkbox`, `toggle`, `select`, `datepicker`, `filePicker`

26. **Componentes comunes**
    `avatar`, `badge`, `spacer`, iconos built-in

27. **`virtualList` — listas grandes**
    Solo renderiza los ítems visibles. Para datasets de miles de registros.

28. **`on key()` — keyboard shortcuts**
    `on key("ctrl+k") { openSearch() }` a nivel de página.

---

### Bloque 7 — API avanzada

29. **Cache de API**
    `api.get("/categories", cache: 5min)` y `api.invalidate("/categories")`.

30. **Paginación**
    `api.get("/products", page: 1, limit: 20)` + `products.append(data.items)`.

31. **WebSockets**
    `socket = api.socket("/chat/{roomId}")`, `on socket.message {}`, `socket.send({})`.
    Socket cierra automáticamente al salir de la página.

32. **File upload**
    `api.upload("/upload/avatar", file)` — retorna `Result<Text>` (URL).

33. **`vel.config.vel` — config global**
    `baseUrl`, `timeout`, `env()` para variables de entorno en build time.

---

### Bloque 8 — Fine-grained reactivity

34. **Re-render fine-grained**
    Runtime trackea dependencias estado→nodo. Solo redibuja los nodos afectados.
    **Atacar antes de producción / URL distribution.**

---

### Bloque 9 — Phase 2: browser (TARGET PRINCIPAL)

35. [x] **Host JavaScript + Canvas 2D** — `web/vel_host.js`: 49 imports vel/runtime en JS, layout column/row/rect/text/input/Button, hit testing, input vía hidden HTML input. Rendering Canvas 2D.

35b. [x] **Event loop web** — Click/teclado via canvas + hidden input, navigate via history.pushState(), SSE hot reload.

36. [x] **`vel serve`** — HTTP devserver :4000, sirve app.wasm + vel_host.js + index.html, hot reload via SSE.

36b. [x] **`vel build --web`** — produce `dist/` (app.wasm + vel_host.js + index.html) lista para CDN.

36c. **`vel deploy`** — Compila a .wasm + assets, sube a CDN/servidor. Usuario abre URL — sin instalar nada.

36d. [x] **Rendering WebGPU Rust WASM** ✅

**Estado:** `vel_host.js` ya usa WebGPU (`navigator.gpu`) en lugar de Canvas 2D — mejora real. Pero el renderer sigue siendo JavaScript. La promesa es "rendering GPU nativo *via WebAssembly*" — el renderer debe ser Rust WASM.

**Plan detallado:**

```
renderer-web/
  Cargo.toml      — wgpu 23, vello 0.4, wasm-bindgen, serde_json
                    NO: winit, wasmtime, ureq
  src/
    lib.rs        — #[wasm_bindgen] pub fn init_renderer(canvas) + render_tree(json) + draw_error(msg)
    gpu.rs        — GpuState para wasm32: surface desde HtmlCanvasElement vía web-sys
    draw.rs       — draw_tree() — mismo que runtime/src/draw.rs, portable
    layout.rs     — layout_root() — mismo que runtime/src/layout.rs, portable
    text.rs       — TextPainter con font bundleado (include_bytes! Inter/NotoSans)
    tree.rs       — UiNode — deserializado desde JSON (serde)
```

**vel build --web actualizado:**
1. Compilar app.vel → app.wasm (igual que hoy)
2. `wasm-pack build renderer-web --target web` → renderer.wasm + renderer.js
3. Producir dist/: app.wasm + renderer.wasm + renderer.js + vel_host.js (reducido) + index.html

**vel_host.js reducido (~80 líneas):**
```javascript
import initRenderer, { init_renderer, render_tree, draw_error } from './renderer.js';
await initRenderer();
const canvas = document.getElementById('vel');
await init_renderer(canvas);
// 49 imports vel/runtime: construyen el árbol UI en memoria JS
// Al final de cada render: render_tree(JSON.stringify(tree))
```

**Prerequisitos a instalar:**
- `rustup target add wasm32-unknown-unknown`
- `cargo install wasm-pack`
- Fuente bundleada: descargar Inter o NotoSans .ttf, incluir en renderer-web/assets/

---

### Bloque 10 — Tooling

37. **VS Code LSP**
    Error squiggles, autocomplete, go-to-definition.
    **Después de que el lenguaje esté frozen** — no antes.

---

### Bloque 11 — Protocolos de API avanzados

> Vel soporta hoy REST + JSON. Lo siguiente amplía el abanico de APIs consumibles.

38. **GraphQL**
    `api.query("/graphql", { query: "{ users { id name } }" })`.
    GraphQL es POST con un body específico — workaroundeable hoy con `api.post`.
    Soporte nativo significaría: sintaxis de query literal, variables tipadas, cache por operación.
    **Prioridad:** media — muchas APIs empresariales usan GraphQL.

39. **WebSockets** ← ya en Bloque 7 (item 31)

40. **XML / SOAP**
    APIs legacy (bancos, gobierno, ERP) devuelven XML.
    Requiere un parser XML en el runtime y un modelo de acceso por path (`data.at("user.name")`).
    **Prioridad:** baja — ecosistema moderno usa JSON. Solo si hay demanda concreta.

41. **gRPC**
    Protocolo binario sobre HTTP/2. Requiere definiciones Protobuf y un cliente gRPC en el runtime.
    Complejidad alta, uso principalmente backend-to-backend.
    **Prioridad:** muy baja — fuera del scope de apps de usuario final.
