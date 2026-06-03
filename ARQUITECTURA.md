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
