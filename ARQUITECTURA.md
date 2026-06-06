# Vel — Arquitectura Tecnica

## Stack completo
```
Developer escribe .vel
    ↓
Compilador (Rust) — lexer, parser, type checker, codegen
    ↓
WebAssembly (.wasm)
    ↓
Runtime (Rust + Vello + wgpu)
    ├── Rendering GPU (Vello — 2D compute shader)
    ├── Layout engine (propio, no CSS)
    ├── Sistema de estado
    ├── Red (HTTP, WebSockets)
    └── Tipografia (Parley)
    ↓
WebGPU / WebGL en el navegador
    ↓
Usuario ve UI nativa de alta calidad via URL
```

## Principios de arquitectura
- El navegador es solo un contenedor — no usamos HTML ni CSS ni DOM
- Rendering GPU-first desde el dia 1 — 60fps garantizados
- El developer nunca ve WASM, GPU ni Rust — solo su lenguaje
- Distribucion via URL — sin instalacion para el usuario final

## Componentes a construir (en orden)
1. Compilador: lexer → parser → type checker → codegen WASM
2. Runtime: rendering, layout, estado, red
3. CLI: new, run, build, deploy
4. Devserver con hot reload
5. Extension de VS Code (syntax, autocomplete, errores)
6. Standard library de componentes

## Tecnologias del runtime
- **Vello** — motor de rendering 2D GPU compute
- **wgpu** — acceso GPU (Metal, Vulkan, DX12, WebGPU)
- **Parley** — tipografia y texto
- **Rust** — lenguaje del compilador y runtime

## Modelo de distribucion
- Developer compila con `vel build` → produce .wasm + assets
- Sube a cualquier servidor (o `vel deploy`)
- Usuario abre URL en Chrome/Firefox/Safari — sin instalar nada
- El navegador carga el WASM y el runtime dibuja con WebGPU

---

## Phase 2: target browser — decisiones y plan

> El web es el target principal de distribución. Phase 1 (ventana nativa) es el entorno
> de desarrollo. Phase 2 es donde viven los usuarios finales. Atacar antes de stdlib completa.

### Qué NO cambia en Phase 2

| Componente | Estado |
|---|---|
| Compilador (lexer, parser, checker, codegen) | Idéntico — genera el mismo .wasm |
| El binario .wasm | Idéntico — el navegador lo corre nativamente |
| El lenguaje Vel | Idéntico — el developer no nota la diferencia |
| Vello (rendering 2D) | Idéntico — ya soporta WebGPU como backend |
| wgpu | Idéntico — fue diseñado desde el día 1 para WebGPU |

### Qué SÍ cambia en Phase 2

| Componente Phase 1 | Reemplazo Phase 2 | Notas |
|---|---|---|
| wasmtime (ejecuta el WASM) | Browser nativo (el browser corre WASM) | El browser reemplaza wasmtime — no escribimos nada |
| winit (event loop nativo) | Web event loop via wasm-bindgen | Click, teclado, resize desde el browser |
| VelHost en Rust | VelHost en JavaScript (~300 líneas) | Misma interfaz, diferente implementación |
| Ventana nativa (wgpu surface) | `<canvas>` con WebGPU context | Un canvas, sin DOM más allá de ese elemento |
| `vel_persist.json` en disco | localStorage del browser | Misma API para el developer Vel |
| Servidor HTTP (ureq) | fetch() del browser | Transparente — el developer escribe igual |

### El rol de JavaScript en Phase 2

JavaScript es un bootstrapper, no una dependencia. El developer Vel nunca lo ve ni lo escribe.
Son ~300 líneas totales que hacen exactamente esto:

```javascript
// Todo el "JavaScript de Vel" en Phase 2
const wasm = await WebAssembly.instantiate(wasmBytes, {
  "vel/runtime": {
    begin_element: (tag, len) => { /* inicia nodo */ },
    end_element: () => { /* cierra nodo */ },
    prop_f64: (key, klen, val) => { /* propiedad numérica */ },
    text_content: (ptr, len) => { /* texto */ },
    navigate: (ptr, len) => { /* navegación */ },
    // ... los mismos imports que hoy tiene VelHost en Rust
  }
})

// Inicializar WebGPU en el canvas
const canvas = document.querySelector('canvas')
const gpu = await initWgpu(canvas)
const vello = initVello(gpu)

// Event loop
canvas.addEventListener('click', (e) => wasm.exports.handle_click(e.x, e.y))
wasm.exports.page_Home()
renderFrame(vello)
```

El DOM queda reducido a un solo `<canvas>`. Nada más.

### Gaps reales que heredamos del browser (plataforma)

Estos son constraints de la plataforma browser, no del ecosistema web legado (DOM/CSS/npm).
El developer de Vel no los ve — los maneja el runtime.

| Constraint | Impacto | Manejo |
|---|---|---|
| WebGPU requiere browser moderno | Chrome 113+, Edge 113+, Safari 18+, Firefox experimental | Vel requiere WebGPU — no hay fallback a DOM |
| CORS | Las API calls necesitan que el servidor tenga los headers correctos | Documentado en el error del tipo checker |
| Sin filesystem directo | `persist` usa localStorage, no archivo en disco | Transparente para el developer |
| Sandbox de seguridad | Sin acceso a puertos, procesos, red local | Esperado — igual que cualquier app web |
| Tamaño del .wasm inicial | Primera carga descarga el runtime | Mitigable con compression + CDN |

### Gaps que NO heredamos (confirmado por investigación)

- ❌ DOM lento — no lo usamos
- ❌ CSS/reflow/repaint — no existe en Vel
- ❌ Inconsistencias de layout entre browsers — nuestro layout engine es determinista
- ❌ npm / webpack / bundlers — el developer no toca nada de esto
- ❌ JavaScript como lenguaje de runtime — el developer escribe Vel

### Referentes que usan el mismo patrón

- **Figma** — WebAssembly + WebGL/WebGPU, renderiza en canvas, no usa DOM para su UI
- **Photoshop Web** — WebAssembly + WebGL, sin DOM para el editor
- **Google Earth Web** — WebAssembly + WebGL
- Ninguno "arrastra los gaps de la web legada" aunque corren en browser

### Orden de ataque para Phase 2

1. **Host JavaScript** — implementar los mismos imports de VelHost en JS (~300 líneas)
2. **wgpu WebGPU target** — cambiar el surface de nativo a canvas WebGPU (wgpu lo soporta, posiblemente sea solo cambiar el feature flag)
3. **Event loop web** — click/teclado/resize desde el canvas via wasm-bindgen
4. **`vel serve`** — devserver local que sirve el .wasm + el host JS + el canvas HTML
5. **`vel build --web`** — output listo para subir a CDN
6. **`vel deploy`** — subir a hosting propio o CDN

### Cuándo atacar

El lenguaje es el suficientemente completo hoy para construir el target web. La decisión
es de prioridad, no de prerequisitos técnicos. Opciones:

- **Ahora (antes de stdlib)**: el web es el target principal de distribución — validar la
  experiencia completa usuario-final antes de construir más features. Riesgo: descubrir
  gaps del browser target que obligan a retrabajo del runtime.
- **Después de Bloque 4b**: tener debug tools primero hace más fácil debuggear el web
  target cuando lo construyamos. El overlay de errores en canvas es útil en ambas plataformas.
- **Después de Bloque 5 (UI completa)**: tener más features antes de lanzar al web.
  Riesgo: construir mucho sobre un target que no hemos validado.

**Recomendación:** después de Bloque 4b. Los debug tools se construyen una vez y sirven
en ambas plataformas. El web target es un sprint bien definido de ~1 semana.
