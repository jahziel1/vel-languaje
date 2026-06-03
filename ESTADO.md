# Vel — Current State

## Where we are
Language design phase — core design complete.

### Completed

#### Language design — 100%
- Language name and philosophy
- Full syntax (types, enums, inference, no null, no coercion)
- Optional chaining ?. and null coalescing ??
- Layout system (column, row, grid, stack, scroll, fixed, responsive)
- Visual design (typography, colors, hex, gradients, hover/focus/active, cursor, theme)
- State system (page state, derived, effects, global stores, reactivity, persistence)
- Navigation (go, params, file-based routes, guards, _layout.vel, _404.vel)
- API and data (get/post/put/patch/delete, typed responses, auth headers, cache, pagination, WebSockets, file upload, env config)
- Module system (public/private, imports, barrel files, shared types)
- Standard library (components, inputs, feedback, icons, utilities, virtualList, slots)
- Control flow, error handling, functions, string interpolation

#### Compiler build — in progress
- [x] Lexer — tokenizes all Vel syntax, 11 tests
- [x] Parser — builds full AST, Pratt expression parser, 13 tests
- [x] Quality pipeline — rustfmt, clippy, pre-commit hook, CI, coverage 83%
- [ ] Type checker
- [ ] Code generator (WASM)
- [ ] Runtime

### Next session starts here
**Type checker** — verifies types, exhaustive match, optional handling, no coercion.

## Compiler structure (so far)
```
compiler/src/
  main.rs
  lexer/
    mod.rs        — Lexer struct, main dispatch, cursor helpers
    token.rs      — Token enum with all Vel tokens
    keywords.rs   — keyword-to-token mapping
    string.rs     — string interpolation state machine
    tests.rs      — 11 lexer tests
  parser/
    mod.rs        — Parser struct, helpers (expect/eat/advance)
    ast.rs        — all AST node types (Program, Item, Stmt, Expr...)
    toplevel.rs   — page, component, layout, store, type, enum, import
    stmt.rs       — state, derived, guard, on, fn, if, match, let
    expr.rs       — Pratt expression parser (correct precedence)
    tests.rs      — 13 parser tests
```

## Key files
- `LENGUAJE.md` — all language decisions (source of truth)
- `ARQUITECTURA.md` — technical stack
- `PENDIENTE.md` — what's done and what's next
- `CLAUDE.md` — instructions for Claude at session start
