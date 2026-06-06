# Vel — Filosofía

## La tesis central

HTML, CSS y JavaScript no fueron diseñados para construir aplicaciones.
Fueron diseñados para distribuir documentos de texto con enlaces.
Todo lo que existe hoy — React, Vue, Angular, Tailwind, TypeScript, el DOM virtual —
es infraestructura construida para compensar esa decisión original.

Vel no es un framework. Es la pregunta: **¿cómo sería un lenguaje diseñado desde cero para apps?**

---

## Los problemas originales — no síntomas, causas

### HTML: un modelo de documento, no de aplicación

HTML nació en 1991 para papers académicos con vínculos entre ellos.
Sus elementos son semánticos para texto: `<h1>`, `<p>`, `<a>`, `<table>`.
Hoy los usamos como contenedores genéricos (`<div>`, `<span>`) para construir
dashboards, editores, apps de tiempo real — y el lenguaje pelea contra nosotros.

El DOM (Document Object Model) es el árbol de ese documento en memoria.
Cuando el estado de la app cambia, tienes que sincronizar manualmente el DOM.
Eso es por qué existe el virtual DOM de React: para resolver un problema
que no existiría si el modelo de rendering no fuera un árbol de documento.

### CSS: efectos secundarios globales sin contención

CSS aplica reglas globalmente. Cualquier regla puede afectar cualquier elemento.
El resultado de un elemento depende de qué tan "específica" sea cada regla que compita.

No hay encapsulación. No hay lógica (no puedes escribir `if loading { color: gray }`).
No hay tipos — todo son strings.

BEM, CSS Modules, Styled Components, Tailwind — son todos workarounds para el mismo
problema: CSS no fue diseñado con componentes en mente porque los componentes no
existían en 1996 cuando fue creado.

### JavaScript: diseñado en 10 días para validar formularios

Brendan Eich diseñó JavaScript en 10 días en 1995. Era para validar forms antes
de hacer submit. No para orquestar UIs complejas, manejar estado, hacer llamadas
de red, animar a 60fps.

Las consecuencias:
- `null` Y `undefined` — dos maneras de decir "nada", comportamientos distintos
- Coerción implícita — `"5" + 5 = "55"`, `null == undefined` es `true`
- `this` cambia dependiendo de cómo llamas la función
- Los errores son excepciones — puedes ignorarlas y el programa sigue corriendo
- No hay tipos — TypeScript es un lenguaje separado que compila a JS para taparle los huecos
- Async fue un afterthought: callbacks → Promises → async/await, tres equipos diferentes, tres eras

### Los frameworks son workarounds, no soluciones

React agrega: componentes, virtual DOM, hooks, dependency arrays en useEffect,
closures obsoletos, context, ref vs state...

Vue agrega: sistema de reactividad, templates que mezclan HTML/JS, Options API vs Composition API...

Angular agrega: módulos, decoradores, zones, change detection, inyección de dependencias...

Todos resuelven problemas reales — y todos crean nuevos problemas.
Pero ninguno puede escapar la limitación fundamental: al final, todo compila a HTML + CSS + JS
corriendo en un DOM que fue diseñado para documentos.

El developer que quiere hacer una app hoy necesita aprender:
HTML + CSS + JS + TypeScript + un framework + su ecosistema (routing, estado, forms, queries...).
Cada capa existe para compensar a la anterior.

---

## Lo que Vel apuesta

### 1. Una sola cosa, no tres

En Vel, todo lo de una pantalla vive en un archivo `.vel`.
Layout, lógica, apariencia, estado — juntos. No como una concesión de comodidad,
sino como un principio: **lo que se mueve junto, vive junto**.

No hay archivos CSS separados porque el estilo es parte de la UI.
No hay archivos de tipos separados porque los tipos son parte de la lógica.
No hay archivos de rutas separados porque la estructura de archivos ES el router.

### 2. El navegador como contenedor, no como plataforma

Vel no usa el DOM. No usa CSS. No usa HTML para rendering.
El navegador provee dos cosas: una ventana y acceso a la GPU.
Vel dibuja todo directamente con Vello (compute shaders 2D) vía WebGPU.

Esto no es un detalle técnico — es la apuesta central.
Al salir del DOM, Vel no hereda ninguno de sus límites:
60fps es el piso, no el logro. El layout engine responde a lo que las apps necesitan,
no a lo que los documentos necesitan. El texto se renderiza con control total.

El usuario abre una URL. Sin instalar nada. El navegador es solo el mecanismo de entrega.

### 3. Errores imposibles, no manejados

Los errores que existen en producción hoy mayormente no son errores de lógica —
son errores que el lenguaje permitió cometer:
- `null` que llega donde no debía
- Un string sumado a un número sin querer
- Un estado async accedido antes de que resuelva
- Un match que no cubre un caso nuevo que se agregó

Vel los hace imposibles a nivel de compilador:
- No hay `null` — usas `?` y el compilador te fuerza a manejar el caso
- No hay coerción implícita — `"x" + 5` es error en compile time
- `Result<T>` es parte del tipo — no puedes usar datos que pueden ser un error sin hacer match
- Match es exhaustivo — agregar un valor al enum rompe el build donde no se cubre

El compilador no te da errores para hacerte la vida difícil.
Te los da para que los bugs aparezcan antes de que el usuario los vea.

### 4. Legible por cualquier developer

El objetivo de diseño de la sintaxis no es "menos caracteres" ni "más expresivo".
Es: **un developer que nunca vio Vel puede leer un archivo y entender qué hace**.

Eso significa:
- Palabras en inglés, no símbolos (`and`, `or`, `not` — no `&&`, `||`, `!`)
- Sin magia — lo que ves es lo que pasa
- El compilador infiere lo que puede, explicas solo lo que aporta información
- El camino fácil es el correcto — no hay un camino corto que introduzca bugs

Un frontend developer, un backend developer, alguien que acaba de aprender a programar
— todos deberían poder leer un `.vel` y entender la pantalla que describe.

### 5. Reactivo por diseño, no por convención

En HTML/JS, el estado y la UI son dos cosas separadas que tienes que mantener sincronizadas.
Esa sincronización es la fuente de la mayoría de los bugs en apps web.

En Vel, el estado mutado actualiza la UI. Sin llamadas extra. Sin dependency arrays.
`derived` siempre está fresco — el compilador sabe qué estado lee.
`on change x { }` observa sin que declares dependencias manualmente.

La reactividad no es una feature de framework — es como funciona el lenguaje.

---

## Lo que no somos

| Decisión | Por qué |
|----------|---------|
| No somos un framework sobre JS | Los frameworks heredan los límites de JS/DOM — nosotros no queremos esos límites |
| No somos un lenguaje académico | Vel existe para construir apps reales, hoy |
| No somos React con mejor sintaxis | Vel reemplaza el paradigma, no el framework |
| No somos Dart/Flutter | Flutter es mobile-first y tiene un modelo de distribución diferente — Vel distribuye via URL |
| No somos un transpilador a HTML | Vel no genera HTML — dibuja en GPU |

---

## El usuario que imaginamos

No imaginamos un tipo de developer — imaginamos a cualquier developer.

El backend developer que quiere construir su propia pantalla sin aprender 5 herramientas.
El frontend developer cansado de que cada año el ecosistema se reinvente.
El founder técnico que quiere entender qué está construyendo su equipo leyendo el código.
El developer nuevo que aprende a programar y aprende Vel como primer lenguaje de apps.

Si cualquiera de ellos puede leer un `.vel` y entender lo que hace — lo logramos.
Si cualquiera de ellos puede construir una app funcional en un día — lo logramos.

---

## La promesa

Un developer escribe código en un lenguaje.
El usuario abre una URL.
Entre los dos, Vel hace todo el trabajo.

Sin HTML que aprender. Sin CSS que debuggear. Sin JavaScript que parchear.
Sin frameworks que versionar. Sin bundlers que configurar. Sin DOM que sincronizar.

Solo el lenguaje y la app.

---

## Dónde estamos hoy vs. la meta

La promesa de distribución via URL es el destino, no el estado actual.
Esto es intencional — no una deuda.

**Fase 1 — runtime nativo (hoy):**
`vel run` compila a WASM y ejecuta en una ventana de escritorio usando Vello + wgpu nativos (Metal/Vulkan/DX12).
Es el laboratorio correcto: velocidad de iteración máxima, sin las restricciones del sandbox del navegador,
para validar que el lenguaje, el compilador y el modelo de rendering son correctos.

**Fase 2 — runtime web (la meta):**
El mismo WASM corre en el navegador. El rendering cambia de wgpu nativo a WebGPU.
El usuario abre una URL — sin instalar nada. La app corre a 60fps en GPU, sin DOM, sin HTML.

El compilador no cambia entre fases. El lenguaje no cambia.
Solo cambia el target del runtime: GPU nativo → WebGPU en el navegador.

El stack fue elegido desde el día 1 para que esta transición sea posible:
WASM es el mismo binario, Vello soporta WebGPU, wgpu abstrae ambos targets.
No estamos tomando un desvío — estamos construyendo en el orden correcto.
