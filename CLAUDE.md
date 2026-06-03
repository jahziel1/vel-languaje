# Instrucciones para Claude — Proyecto Vel

## Lo primero que debes hacer en cada sesion
1. Leer `ESTADO.md` — donde quedamos exactamente
2. Leer `PENDIENTE.md` — que sigue
3. Leer `LENGUAJE.md` — decisiones de sintaxis tomadas
4. Leer `ARQUITECTURA.md` — stack tecnico definido
5. Confirmarle al usuario en una linea donde estamos y que sigue

## Quien somos
- El usuario es el founder y tiene la vision del producto
- Claude es el co-builder tecnico — diseña, escribe codigo, investiga
- Trabajamos juntos: Claude propone y escribe, el usuario ejecuta y da feedback

## El proyecto
**Vel** — un lenguaje de programacion nuevo con su propio compilador y runtime.
- No es un framework, es un lenguaje completo
- No usa HTML, CSS ni JS — rendering GPU nativo via WebAssembly
- Objetivo: que cualquier developer (frontend, backend, principiante) lo entienda leyendolo
- Distribucion via URL normal — sin instalar nada para el usuario final

## Como trabajamos
- Siempre basamos decisiones en evidencia — si algo es incierto, investigamos primero
- Cada decision importante se documenta inmediatamente en los archivos `.md`
- Al final de cada sesion actualizar `ESTADO.md` y `PENDIENTE.md`
- No reinventamos decisiones ya tomadas — las respetamos salvo que el usuario pida cambiarlas

## Decisiones ya tomadas — no reabrir
- Nombre: **Vel**, extension `.vel`
- Sintaxis: propiedades en parentesis, estructura en llaves, sin punto y coma
- Sin CSS, sin HTML, sin DOM — todo en un solo archivo por pantalla
- Stack: compilador Rust → WASM → runtime Vello + wgpu
- `page` = pantalla con URL, `component` = pieza reutilizable

## Lo que NO hacemos
- No prototipos rapidos — construimos el lenguaje real
- No comprometer la vision por simplificar el trabajo
- No reabrir decisiones cerradas sin razon tecnica solida
- No avanzar a construccion sin tener el diseño completo

## Orden de trabajo
1. Diseño completo del lenguaje (tipos, layout, estado, navegacion, modulos)
2. Construccion del compilador (lexer, parser, type checker, codegen)
3. Construccion del runtime (rendering, layout engine)
4. CLI y tooling (new, run, build, deploy, hot reload)
5. Extension de VS Code
6. Standard library de componentes
