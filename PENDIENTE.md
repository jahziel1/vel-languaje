# Vel — Pending

## COMPLETED
- [x] Language name: Vel, extension .vel
- [x] Philosophy and syntax rules
- [x] Type system (primitives, custom types, enums, inference, no null)
- [x] String interpolation
- [x] Control flow (if/else, match, logical operators)
- [x] Error handling (Result type, none returns)
- [x] Local functions inside page/component
- [x] Imports and exports
- [x] Layout system (column, row, center, stack, grid, scroll, fixed)
- [x] Spacing (padding, gap, asymmetric)
- [x] Sizing (fixed, full, half, grow, min, max)
- [x] Alignment
- [x] Visual properties (background, radius, border, shadow, opacity)
- [x] Responsive (on phone/mobile/tablet/desktop/wide)
- [x] Text overflow (ellipsis, lines, clip)
- [x] Aspect ratio
- [x] Animations (basic)

## NEXT — Design (in order)

### ~~1. State system~~ ✓ DONE

### ~~2. Navigation~~ ✓ DONE

### ~~3. API and data~~ ✓ DONE

### ~~4. Module system~~ ✓ DONE

### ~~5. Standard library~~ ✓ DONE
### ~~6. Gaps + visual design system~~ ✓ DONE
- Optional chaining ?. and ?? documented
- Typography (size, weight, lineHeight, etc.)
- Color system (hex, opacity, gradients)
- Hover/focus/active states
- Theme system
- on mount / on unmount
- State persistence
- Persistent layouts (_layout.vel + outlet)
- 404 page
- Typed API responses
- File upload
- env() in config
- virtualList
- children: Block (slots)
- Keyboard events

## BUILD — Status

### Compiler
- [x] Lexer (`compiler/src/lexer/`) — 11 tests
- [x] Parser (`compiler/src/parser/`) — 13 tests, full AST
- [x] Quality pipeline — rustfmt, clippy, pre-commit, CI, coverage 83%
- [ ] **Type checker** ← NEXT
- [ ] Code generator (WASM output)

### Runtime
- [ ] Rendering engine (Vello + wgpu)
- [ ] Layout engine
- [ ] State management
- [ ] Network layer

### Tooling
- [ ] CLI: new, run, build, deploy
- [ ] Dev server with hot reload
- [ ] VS Code extension (syntax, autocomplete, errors)
- [ ] Standard library of components
