// vel_host.js — Vel browser host (Phase 2)
// Bootstrapper: loads the Vel app WASM and the Rust GPU renderer WASM.
// The developer writes .vel; this file is an implementation detail of the runtime.
// Rendering: Rust/wgpu/WebGPU via vel_renderer_web.wasm (GPU-native, no DOM/CSS).

import initRendererWasm, {
  init_renderer,
  render_frame,
  draw_error as rust_draw_error,
} from './vel_renderer_web.js';

let memory, wasmExports;
const canvas = document.getElementById('vel');
const dpr = window.devicePixelRatio || 1;

// ── Canvas resize ─────────────────────────────────────────────────────────────

function resizeCanvas() {
  canvas.width = window.innerWidth * dpr;
  canvas.height = window.innerHeight * dpr;
  canvas.style.width = window.innerWidth + 'px';
  canvas.style.height = window.innerHeight + 'px';
}
resizeCanvas();
window.addEventListener('resize', () => { resizeCanvas(); scheduleRender(); });

// ── Runtime state ─────────────────────────────────────────────────────────────

const apiStore = new Map();
let nextReqid = 1;
const textState = new Map();
let pendingHeaders = [];
let pendingBody = null;
let bodyFields = [];
let urlBuilder = '';
let pendingUrl = '';
let strBuilder = '';
let navParams = [];
let currentNavParams = [];
let currentPage = null;
const navHistory = [];

let renderScheduled = false;
function scheduleRender() {
  if (!renderScheduled) {
    renderScheduled = true;
    requestAnimationFrame(() => { renderScheduled = false; doRender(); });
  }
}

// ── Memory helpers ────────────────────────────────────────────────────────────

function readStr(ptr, len) {
  if (!memory || ptr < 0 || len <= 0) return '';
  return new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, len));
}

// ── UI tree ───────────────────────────────────────────────────────────────────

const TAG_NAMES = {
  1:'text',2:'column',3:'row',4:'center',5:'stack',6:'grid',7:'scroll',
  8:'fixed',9:'rect',10:'image',11:'icon',12:'spacer',13:'divider',
  14:'spinner',15:'Button',16:'input',17:'toast',
};
const PROP_NAMES = {
  1:'padding',2:'paddingTop',3:'paddingBottom',4:'paddingLeft',5:'paddingRight',
  6:'gap',7:'fontSize',8:'fontWeight',12:'radius',13:'border',14:'width',
  15:'height',16:'bold',17:'italic',18:'color',19:'background',20:'lineHeight',
  21:'align',22:'opacity',23:'shadow',24:'overflow',25:'letterSpacing',
  26:'lines',27:'borderColor',28:'borderBottom',29:'borderTop',
  30:'borderLeft',31:'borderRight',32:'underline',33:'strikethrough',
  34:'animate',35:'visible',36:'cursor',
};

let elemStack = [];
let completed = [];

function begin_element(tag) {
  elemStack.push({ tag, tagName: TAG_NAMES[tag] || 'element', props: {}, text: null,
    children: [], on_click: null, on_change: null,
    hover_props: null, focus_props: null, active_props: null, has_value: false });
}
function end_element() {
  const node = elemStack.pop();
  if (!node) return;
  if (elemStack.length > 0) elemStack[elemStack.length - 1].children.push(node);
  else completed.push(node);
}
function prop_f64(key, val) {
  const pname = PROP_NAMES[key] || `p${key}`;
  if (stateMode !== 0) { stateBuf[pname] = val; return; }
  if (elemStack.length > 0) elemStack[elemStack.length - 1].props[pname] = val;
}
function prop_bool(key, val) {
  const pname = PROP_NAMES[key] || `p${key}`;
  if (stateMode !== 0) { stateBuf[pname] = val !== 0; return; }
  if (elemStack.length > 0) elemStack[elemStack.length - 1].props[pname] = val !== 0;
}
function text_content(ptr, len) {
  if (elemStack.length > 0) elemStack[elemStack.length - 1].text = readStr(ptr, len);
}
function set_on_click(ptr, len) {
  if (elemStack.length > 0) elemStack[elemStack.length - 1].on_click = readStr(ptr, len);
}
function set_on_change(ptr, len) {
  if (elemStack.length > 0) elemStack[elemStack.length - 1].on_change = readStr(ptr, len);
}

// ── String builder ────────────────────────────────────────────────────────────

function str_begin() { strBuilder = ''; }
function str_lit(ptr, len) { strBuilder += readStr(ptr, len); }
function str_num(val) {
  strBuilder += (Math.abs(val) < 1e15 && val % 1 === 0) ? String(Math.trunc(val)) : String(val);
}
function str_done() {
  if (elemStack.length > 0) { elemStack[elemStack.length - 1].text = strBuilder; strBuilder = ''; }
}

// ── Navigation ────────────────────────────────────────────────────────────────

let pendingNavigation = null;
function navigate(ptr, len) { pendingNavigation = readStr(ptr, len); }
function nav_param(idx, val) { navParams[idx] = val; }

function pathToPage(path) {
  const seg = (path.replace(/^\//, '') || 'home').toLowerCase();
  for (const key of Object.keys(wasmExports)) {
    if (key.startsWith('page_') && key.slice(5).toLowerCase() === seg) return key.slice(5);
  }
  return null;
}
function doNavigate(path) {
  if (path === '__back__' || path === '/back') {
    if (navHistory.length > 0) { currentPage = navHistory.pop(); currentNavParams = []; }
    scheduleRender(); return;
  }
  const page = pathToPage(path);
  if (page) {
    navHistory.push(currentPage);
    currentPage = page;
    currentNavParams = [...navParams];
    navParams.length = 0;
    window.history.pushState({}, '', path);
  }
  scheduleRender();
}

// ── API ───────────────────────────────────────────────────────────────────────

function header_begin() { pendingHeaders = []; }
function header_field(kp, kl, vp, vl) { pendingHeaders.push([readStr(kp, kl), readStr(vp, vl)]); }
function header_done() {}
function header_field_str(kp, kl) { pendingHeaders.push([readStr(kp, kl), strBuilder]); strBuilder = ''; }
function body_begin() { bodyFields = []; }
function body_field_str(kp, kl, vp, vl) { bodyFields.push([readStr(kp, kl), readStr(vp, vl)]); }
function body_field_num(kp, kl, val) { bodyFields.push([readStr(kp, kl), val]); }
function body_field_bool(kp, kl, val) { bodyFields.push([readStr(kp, kl), val !== 0]); }
function body_done() { pendingBody = JSON.stringify(Object.fromEntries(bodyFields)); bodyFields = []; }
function url_begin() { urlBuilder = ''; }
function url_lit(ptr, len) { urlBuilder += readStr(ptr, len); }
function url_num(val) { urlBuilder += val % 1 === 0 ? String(Math.trunc(val)) : String(val); }
function url_done() { pendingUrl = urlBuilder; urlBuilder = ''; }
function api_url_changed(reqid) {
  const e = apiStore.get(reqid);
  return (!e || e.url !== pendingUrl) ? 1 : 0;
}
function doFetch(reqid, method, url, body, headers) {
  apiStore.set(reqid, { status: 0, body: '', url });
  const opts = { method: method.toUpperCase(), headers: { 'Content-Type': 'application/json' } };
  for (const [k, v] of headers) opts.headers[k] = v;
  if (body && ['POST', 'PUT', 'PATCH'].includes(opts.method)) opts.body = body;
  fetch(url, opts)
    .then(r => r.text())
    .then(t => { const e = apiStore.get(reqid); if (e) { e.status = 1; e.body = t; } scheduleRender(); })
    .catch(err => { const e = apiStore.get(reqid); if (e) { e.status = 2; e.body = err.message; } scheduleRender(); });
  return reqid;
}
function api_fetch(mp, ml, up, ul) {
  const method = readStr(mp, ml), url = readStr(up, ul);
  const rid = nextReqid++, hdrs = pendingHeaders, bod = pendingBody;
  pendingHeaders = []; pendingBody = null;
  return doFetch(rid, method, url, bod, hdrs);
}
function api_fetch_dyn(mp, ml) {
  const method = readStr(mp, ml), url = pendingUrl;
  pendingUrl = '';
  const rid = nextReqid++, hdrs = pendingHeaders, bod = pendingBody;
  pendingHeaders = []; pendingBody = null;
  return doFetch(rid, method, url, bod, hdrs);
}
function api_poll(rid) { return apiStore.get(rid)?.status ?? 0; }
function api_error_str(rid) { strBuilder = apiStore.get(rid)?.body ?? ''; }
function api_field_str(rid, fp, fl) {
  const f = readStr(fp, fl), e = apiStore.get(rid);
  if (!e) return;
  try { const v = JSON.parse(e.body)[f]; if (v != null) strBuilder += String(v); } catch {}
}
function api_field_num(rid, fp, fl) {
  const f = readStr(fp, fl), e = apiStore.get(rid);
  if (!e) return 0;
  try {
    const v = JSON.parse(e.body)[f];
    if (typeof v === 'number') return v;
    if (typeof v === 'boolean') return v ? 1 : 0;
    if (typeof v === 'string') { const n = parseFloat(v); return isNaN(n) ? 0 : n; }
  } catch {}
  return 0;
}
function api_field_bool(rid, fp, fl) {
  const f = readStr(fp, fl), e = apiStore.get(rid);
  if (!e) return 0;
  try {
    const v = JSON.parse(e.body)[f];
    if (typeof v === 'boolean') return v ? 1 : 0;
    if (typeof v === 'number') return v !== 0 ? 1 : 0;
  } catch {}
  return 0;
}

// ── Text state ────────────────────────────────────────────────────────────────

function text_state_get(kp, kl) { strBuilder += textState.get(readStr(kp, kl)) ?? ''; }
function text_state_set(kp, kl, vp, vl) {
  const key = readStr(kp, kl), val = readStr(vp, vl);
  textState.set(key, val);
  if (activeInputVar === key) inputBuffer = val;
}
function text_state_bool(kp, kl) { const v = textState.get(readStr(kp, kl)) ?? ''; return v.length > 0 ? 1 : 0; }
function text_state_set_built(kp, kl) { textState.set(readStr(kp, kl), strBuilder); strBuilder = ''; }

// ── Persist (localStorage) ────────────────────────────────────────────────────

function vel_persist_init() {}
function persist_get_num(kp, kl, def) { const v = localStorage.getItem('vel.' + readStr(kp, kl)); return v !== null ? parseFloat(v) : def; }
function persist_get_bool(kp, kl, def) { const v = localStorage.getItem('vel.' + readStr(kp, kl)); return v !== null ? (v === 'true' ? 1 : 0) : def; }
function persist_set_num(kp, kl, val) { localStorage.setItem('vel.' + readStr(kp, kl), String(val)); }
function persist_set_bool(kp, kl, val) { localStorage.setItem('vel.' + readStr(kp, kl), val !== 0 ? 'true' : 'false'); }

// ── Lists ─────────────────────────────────────────────────────────────────────

function getArr(rid) {
  const e = apiStore.get(rid);
  if (!e || e.status !== 1) return [];
  try { return JSON.parse(e.body); } catch { return []; }
}
function list_count(rid) { const a = getArr(rid); return Array.isArray(a) ? a.length : 0; }
function list_item_str(rid, idx, fp, fl) {
  const f = readStr(fp, fl), a = getArr(rid);
  if (Array.isArray(a) && idx < a.length && a[idx][f] != null) strBuilder += String(a[idx][f]);
}
function list_item_num(rid, idx, fp, fl) {
  const f = readStr(fp, fl), a = getArr(rid);
  return (Array.isArray(a) && idx < a.length) ? (Number(a[idx][f]) || 0) : 0;
}
function list_item_bool(rid, idx, fp, fl) {
  const f = readStr(fp, fl), a = getArr(rid);
  return (Array.isArray(a) && idx < a.length) ? (a[idx][f] ? 1 : 0) : 0;
}
function list_sum(rid, fp, fl) {
  const f = readStr(fp, fl), a = getArr(rid);
  return Array.isArray(a) ? a.reduce((s, item) => s + (typeof item[f] === 'number' ? item[f] : 0), 0) : 0;
}

// ── Print ─────────────────────────────────────────────────────────────────────

function print_str(ptr, len) { console.log('[vel]', readStr(ptr, len)); }
function print_str_built() { console.log('[vel]', strBuilder); strBuilder = ''; }
function window_width() { return window.innerWidth; }

// ── Hover / focus / active state ──────────────────────────────────────────────

let stateMode = 0;
let stateBuf = {};
function state_push(kind) { stateMode = kind; stateBuf = {}; }
function state_pop() {
  const buf = stateBuf, mode = stateMode;
  stateMode = 0; stateBuf = {};
  if (elemStack.length === 0) return;
  const top = elemStack[elemStack.length - 1];
  if (mode === 1) top.hover_props = buf;
  else if (mode === 2) top.focus_props = buf;
  else if (mode === 3) top.active_props = buf;
}

// ── WASM imports ──────────────────────────────────────────────────────────────

const velImports = {
  'vel/runtime': {
    begin_element, end_element, prop_f64, prop_bool, text_content,
    navigate, nav_param, set_on_click, set_on_change,
    str_begin, str_lit, str_num, str_done,
    api_fetch, api_poll, api_error_str, api_field_str, api_field_num, api_field_bool,
    header_begin, header_field, header_done, header_field_str,
    body_begin, body_field_str, body_field_num, body_field_bool, body_done,
    url_begin, url_lit, url_num, url_done, api_url_changed, api_fetch_dyn,
    text_state_get, text_state_set, text_state_bool, text_state_set_built,
    list_count, list_item_str, list_item_num, list_item_bool, list_sum,
    vel_persist_init, persist_get_num, persist_get_bool, persist_set_num, persist_set_bool,
    print_str, print_str_built, window_width,
    state_push, state_pop,
  }
};

// ── Layout (kept for hit-testing) ─────────────────────────────────────────────

function leafH(tag) {
  switch (tag) { case 1: return 22; case 13: return 1; case 14: return 40; case 15: return 38; case 16: return 38; default: return 40; }
}
function nodeLayout(node, x, y, availW) {
  const p = node.props;
  const pad = p.padding || 0;
  const pt = p.paddingTop ?? pad, pb = p.paddingBottom ?? pad;
  const pl = p.paddingLeft ?? pad, pr = p.paddingRight ?? pad;
  const gap = p.gap || 0;
  const w = p.width ?? availW;
  const innerW = Math.max(0, w - pl - pr);
  if (node.children.length === 0) {
    return { x, y, w, h: p.height ?? leafH(node.tag), children: [] };
  }
  if (node.tag === 3) {
    const n = node.children.length;
    const cw = n > 0 ? (innerW - gap * Math.max(0, n - 1)) / n : innerW;
    const children = [];
    let cx = x + pl, maxH = 0;
    for (let i = 0; i < n; i++) {
      const b = nodeLayout(node.children[i], cx, y + pt, cw);
      maxH = Math.max(maxH, b.h);
      cx += b.w + (i + 1 < n ? gap : 0);
      children.push(b);
    }
    return { x, y, w, h: p.height ?? (maxH + pt + pb), children };
  }
  const children = [];
  let cy = y + pt;
  for (let i = 0; i < node.children.length; i++) {
    const b = nodeLayout(node.children[i], x + pl, cy, innerW);
    cy += b.h + (i + 1 < node.children.length ? gap : 0);
    children.push(b);
  }
  return { x, y, w, h: p.height ?? (cy + pb - y), children };
}
function layoutRoot(nodes, availW) {
  const boxes = [];
  let cy = 0;
  for (const node of nodes) {
    const b = nodeLayout(node, 0, cy, availW);
    cy += b.h + 8;
    boxes.push(b);
  }
  return boxes;
}

// ── Hit testing ───────────────────────────────────────────────────────────────

function hitTest(nodes, boxes, mx, my) {
  for (let i = nodes.length - 1; i >= 0; i--) {
    const b = boxes[i];
    if (mx >= b.x && mx < b.x + b.w && my >= b.y && my < b.y + b.h) {
      if (nodes[i].children.length > 0) {
        const r = hitTest(nodes[i].children, b.children, mx, my);
        if (r) return r;
      }
      if (nodes[i].on_click) return { type: 'click', fn: nodes[i].on_click };
      if (nodes[i].tag === 16) return { type: 'input', var: nodes[i].on_change };
    }
  }
  return null;
}
function findEnterHandler(nodes, varName) {
  for (const n of nodes) {
    if (n.tag === 16 && n.on_change === varName && n.on_click) return n.on_click;
    const r = findEnterHandler(n.children, varName);
    if (r) return r;
  }
  return null;
}
function patchInputValues(node) {
  if (node.tag === 16 && node.on_change) {
    const val = textState.get(node.on_change) ?? '';
    node.has_value = val.length > 0;
    if (val.length > 0) node.text = val;
  }
  for (const c of node.children) patchInputValues(c);
}

// ── Mouse tracking ────────────────────────────────────────────────────────────

let mousePos = { x: -1, y: -1 };
let isMouseDown = false;
canvas.addEventListener('mousemove', (e) => {
  const r = canvas.getBoundingClientRect();
  mousePos = { x: e.clientX - r.left, y: e.clientY - r.top };
  scheduleRender();
});
canvas.addEventListener('mousedown', () => { isMouseDown = true; scheduleRender(); });
canvas.addEventListener('mouseup',   () => { isMouseDown = false; scheduleRender(); });

// ── Active input ──────────────────────────────────────────────────────────────

let activeInputVar = null;
let inputBuffer = '';
const hiddenInput = document.createElement('input');
hiddenInput.style.cssText = 'position:fixed;opacity:0;top:-100px;left:0;width:1px;height:1px;';
document.body.appendChild(hiddenInput);
hiddenInput.addEventListener('input', () => {
  if (!activeInputVar) return;
  inputBuffer = hiddenInput.value;
  textState.set(activeInputVar, inputBuffer);
  scheduleRender();
});
hiddenInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter' && activeInputVar) {
    const fn = findEnterHandler(completed, activeInputVar);
    if (fn && wasmExports[fn]) { wasmExports[fn](); scheduleRender(); }
  }
  if (e.key === 'Escape') { activeInputVar = null; hiddenInput.blur(); scheduleRender(); }
});

// ── Render ────────────────────────────────────────────────────────────────────

let layoutBoxes = [];

let rendererReady = false;

function doRender() {
  if (!wasmExports || !currentPage || !rendererReady) return;
  elemStack = [];
  completed = [];

  try {
    const fn = wasmExports[`page_${currentPage}`];
    if (fn) fn(...currentNavParams);
  } catch (err) {
    drawError(String(err));
    return;
  }

  if (pendingNavigation) {
    const path = pendingNavigation;
    pendingNavigation = null;
    doNavigate(path);
    return;
  }

  for (const node of completed) patchInputValues(node);

  const cw = canvas.width / dpr;
  const ch = canvas.height / dpr;

  // Layout in JS for hit testing
  layoutBoxes = layoutRoot(completed, cw);

  // GPU rendering via Rust WASM renderer
  render_frame(JSON.stringify({
    nodes: completed,
    mouse_x: mousePos.x,
    mouse_y: mousePos.y,
    mouse_down: isMouseDown,
    active_input: activeInputVar ?? '',
    input_buffer: inputBuffer,
  }), cw, ch);
}

function drawError(msg) {
  console.error('[vel]', msg);
  if (!rendererReady) return;
  const cw = canvas.width / dpr;
  const ch = canvas.height / dpr;
  rust_draw_error(msg, cw, ch);
}

// ── Click events ──────────────────────────────────────────────────────────────

canvas.addEventListener('click', (e) => {
  const r = canvas.getBoundingClientRect();
  const mx = e.clientX - r.left, my = e.clientY - r.top;
  const hit = hitTest(completed, layoutBoxes, mx, my);
  if (!hit) { activeInputVar = null; return; }
  if (hit.type === 'click' && wasmExports[hit.fn]) {
    wasmExports[hit.fn]();
    scheduleRender();
  } else if (hit.type === 'input') {
    activeInputVar = hit.var;
    inputBuffer = textState.get(hit.var) ?? '';
    hiddenInput.value = inputBuffer;
    hiddenInput.focus();
    scheduleRender();
  }
});

// ── Hot reload (SSE) ──────────────────────────────────────────────────────────

async function reloadWasm() {
  try {
    const bytes = await fetch('app.wasm', { cache: 'no-store' }).then(r => r.arrayBuffer());
    const { instance } = await WebAssembly.instantiate(bytes, velImports);
    wasmExports = instance.exports;
    memory = wasmExports.memory;
    if (wasmExports.vel_persist_init) wasmExports.vel_persist_init();
    scheduleRender();
  } catch (err) {
    console.error('[vel] reload failed:', err);
  }
}
function setupHotReload() {
  const es = new EventSource('/events');
  es.addEventListener('reload', () => { console.log('[vel] hot reload'); reloadWasm(); });
  es.onerror = () => setTimeout(setupHotReload, 3000);
}

// ── Init ──────────────────────────────────────────────────────────────────────

async function init() {
  // Initialize the Rust WASM renderer
  await initRendererWasm();
  await init_renderer(canvas);
  // configure(1,1) inside init_renderer resets canvas.width/height to 1 — restore.
  resizeCanvas();

  // Prime the WebGPU surface with correct dimensions so get_current_texture()
  // succeeds on the first real render frame.
  render_frame(JSON.stringify({
    nodes: [], mouse_x: 0, mouse_y: 0, mouse_down: false,
    active_input: '', input_buffer: '',
  }), canvas.width / dpr, canvas.height / dpr);
  await new Promise(r => requestAnimationFrame(r));

  // Load the Vel app
  const bytes = await fetch('app.wasm').then(r => r.arrayBuffer());
  const { instance } = await WebAssembly.instantiate(bytes, velImports);
  wasmExports = instance.exports;
  memory = wasmExports.memory;

  // Determine initial page from URL path, falling back to 'Login' then first export.
  const urlSeg = location.pathname.replace(/^\//, '').toLowerCase();
  const allPages = Object.keys(wasmExports).filter(k => k.startsWith('page_'));
  if (urlSeg) {
    currentPage = allPages.find(k => k.slice(5).toLowerCase() === urlSeg)?.slice(5) ?? null;
  }
  if (!currentPage) {
    currentPage = allPages.find(k => k.slice(5).toLowerCase() === 'login')?.slice(5)
      ?? allPages[0]?.slice(5)
      ?? null;
  }
  if (!currentPage) { drawError('No page found in app.wasm'); return; }

  if (wasmExports.vel_persist_init) wasmExports.vel_persist_init();

  window.addEventListener('popstate', () => {
    const seg = (location.pathname.replace(/^\//, '') || 'home').toLowerCase();
    const page = Object.keys(wasmExports).find(k => k.startsWith('page_') && k.slice(5).toLowerCase() === seg);
    if (page) { currentPage = page.slice(5); currentNavParams = []; scheduleRender(); }
  });

  setupHotReload();
  rendererReady = true;
  scheduleRender();
}

init().catch(err => drawError(String(err)));
