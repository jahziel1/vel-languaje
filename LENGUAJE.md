# Vel — Language Design

## Name
**Vel** — file extension `.vel`
- Short, pronounceable in any language
- Connected to the rendering engine (Vello)
- CLI: `vel new`, `vel run`, `vel build`, `vel deploy`

## Philosophy
- Logic and structure together — never in separate files
- The easy path is the correct path (pit of success)
- No ceremony — the compiler infers what it can
- Properties always inside the block, never chained outside
- Reads like pseudocode — any developer understands it
- **Language: English** — keywords, standard library, CLI, errors

---

## Syntax Rules
- Properties go in parentheses next to the element: `text("hello", bold: true)`
- Nested structure uses braces `{}`
- No semicolons
- Types optional — compiler infers them
- No magic symbols (`$`, `@`, `#`)
- `page` = full screen with its own URL
- `component` = reusable piece without URL
- `export` = makes something importable from other files

---

## Hello World
```vel
page Home {
    text("Hello world")
}
```

---

## Imports
```vel
// single component
import Button from "components/button"

// multiple from same file
import { Card, Modal, Avatar } from "components/ui"

// type
import { User, Product } from "types"
```

Only exported things can be imported:
```vel
// button.vel
export component Button(label: Text, onClick: () -> none) { ... }

// private to the file — not importable
component InnerHelper() { ... }
```

---

## Type System

### Primitive types
| Vel | Equivalent in other languages |
|-----|-------------------------------|
| `Text` | string, str, String |
| `Number` | int, float, double |
| `Bool` | boolean, bool |
| `List<T>` | Array, List, []T |
| `Map<K,V>` | Map, Dict, HashMap |

### Custom types
```vel
type User {
    name: Text
    age: Number
    email: Text
    photo: Text?    // ? = may not exist
}
```

### Enums
```vel
enum Status {
    Loading
    Success
    Error(message: Text)    // can carry data
}

enum Direction {
    Left | Right | Up | Down
}
```

### Built-in Result type (for async operations)
```vel
// api.get returns Result<T> automatically
enum Result<T> {
    Loading
    Success(value: T)
    Error(message: Text)
}
```

### Functions as types
```vel
onClick: () -> none           // no args, no return
onSelect: (item: Product) -> none
transform: (n: Number) -> Number
```

### Inference rules
- Local variables: compiler infers — you don't write the type
- Custom type properties: always explicit
- Public functions: explicit return type (it's a contract)

### No null — use ? and none
```vel
type User {
    photo: Text?
}

// compiler forces you to handle it
if user.photo {
    image(user.photo, size: 80)
} else {
    image("/default.png", size: 80)
}
```

### No implicit coercion — ever
```vel
// ERROR: cannot add Text and Number
text("Hello " + age)
// Fix: use interpolation
text("Hello {age}")
```

### Optional chaining and null coalescing
```vel
// ?. — safe access through optional values
user?.address?.city            // Text? — none if any step is none
order?.items?.first()?.price   // Number?

// ?? — default value when none
user?.name ?? "Guest"
config?.timeout ?? 5000
items?.length ?? 0

// combine both
text("{user?.displayName ?? user?.email ?? "Anonymous"}")
```

---

## String Interpolation
```vel
text("Hello {user.name}!")
text("Total: {price * quantity} USD")
text("Status: {user.active ? "Active" : "Inactive"}")
```

---

## Control Flow

### if / else
```vel
if loading {
    spinner()
} else {
    content()
}
```

### match — exhaustive, compiler-checked
```vel
match status {
    Loading -> spinner()
    Success(data) -> Content(data)
    Error(message) -> text(message, color: red)
}

// _ catches remaining cases
match direction {
    Left -> moveLeft()
    Right -> moveRight()
    _ -> none
}
```

If you add a new enum value and forget to handle it in a match:
```
ERROR: match is not exhaustive
  Missing case: Status.Cancelled
```

### Logical operators — words not symbols
```vel
if user.active and user.verified { ... }
if user.isAdmin or user.isModerator { ... }
if not user.active { ... }
if user.active and (user.isAdmin or user.isModerator) { ... }
```

---

## Functions

### Local functions inside page/component
```vel
page Checkout {
    state {
        cart: List<Product> = try api.get("/cart")
    }

    fn calculateTotal(): Number {
        cart.sum(p -> p.price)
    }

    fn handleSubmit() {
        result = try api.post("/order", { items: cart })
        match result {
            Success -> go("/confirmation")
            Error(msg) -> toast(msg, type: error)
        }
    }

    Button("Place Order", onClick: handleSubmit)
}
```

### Functions returning optional
```vel
fn divide(a: Number, b: Number): Number? {
    if b == 0 { none } else { a / b }
}
```

---

## Error Handling

### Async errors — part of the type, cannot be ignored
```vel
page Profile {
    state {
        user = try api.get("/user")
    }

    match user {
        Loading -> spinner()
        Success(data) -> ProfileContent(data)
        Error -> text("Failed to load", color: red)
    }
}
```

### Local errors — return none instead of throwing
```vel
fn divide(a: Number, b: Number): Number? {
    if b == 0 { none } else { a / b }
}

result = divide(10, 0)
if result {
    text("{result}")
} else {
    text("Cannot divide by zero")
}
```

---

## Layout System

### Core blocks
```vel
column { }          // vertical stack
row { }             // horizontal
center { }          // centered both axes
stack { }           // overlapping (z layers)
grid(columns: 3) {} // fixed grid
grid(columns: auto, minWidth: 200) {}  // responsive auto grid
scroll(height: 400) { }               // scrollable container
scroll(direction: horizontal) { }
fixed(top: 0, left: 0, right: 0) { }  // fixed overlay
```

### Spacing
```vel
column(padding: 24, gap: 16) { }             // all sides
column(paddingTop: 16, paddingBottom: 32) { } // per side
column(padding: [16, 24, 32, 24]) { }         // top right bottom left
```

### Sizing
```vel
rect(width: 200, height: 100)   // fixed
rect(width: full)               // 100% of parent
rect(width: half)               // 50% of parent
rect(width: grow)               // fills available space
rect(width: grow, maxWidth: 800, minWidth: 200)
```

### Alignment
```vel
row(align: left) { }
row(align: center) { }
row(align: right) { }
row(align: spread) { }    // space between

column(align: top) { }
column(align: center) { }
column(align: bottom) { }
```

### Visual properties
```vel
rect(
    background: white,
    radius: 12,
    border: 1,
    borderColor: gray,
    borderBottom: 1,          // single side
    shadow: small,            // none | small | medium | large
    opacity: 0.5,
    overflow: clip,
    visible: true
) { }
```

### Aspect ratio
```vel
image(video.thumbnail, ratio: 16/9, width: full)
rect(ratio: 1/1, width: 200) { }
```

### Text overflow
```vel
text(product.name, overflow: ellipsis, lines: 1)
text(product.description, overflow: ellipsis, lines: 2)
text(tag.name, overflow: clip)
```

### Responsive
```vel
on phone { }        // < 480px
on mobile { }       // < 768px
on tablet { }       // < 1024px
on desktop { }      // >= 1024px
on wide { }         // >= 1440px
```

### Animations
```vel
rect(animate: fade, duration: 200) { }
rect(animate: slide, duration: 300) { }
```

### Cursor
```vel
rect(cursor: pointer) { }      // hand — for clickable elements
rect(cursor: default) { }      // arrow
rect(cursor: text) { }         // text cursor
rect(cursor: grab) { }         // drag handle
rect(cursor: not-allowed) { }  // disabled state
```

### Hover, focus, and active states
```vel
// card with hover
rect(
    background: white,
    radius: 12,
    shadow: small,
    hover: { shadow: large, background: gray50 },
    animate: true,
    cursor: pointer
) {
    text("Hover me")
}

// input with focus ring
input(email,
    border: 1,
    borderColor: gray200,
    radius: 8,
    focus: { border: 2, borderColor: #3B82F6 }
)

// button with all states
rect(
    background: #3B82F6,
    radius: 8,
    padding: [12, 24],
    hover:  { background: #2563EB },
    active: { scale: 0.97 },
    animate: true,
    cursor: pointer
) {
    text("Pay now", color: white, weight: 600)
}

// icon with hover animation
icon(heart,
    color: gray300,
    size: 20,
    hover: { color: red, scale: 1.2 },
    animate: true,
    cursor: pointer,
    onClick: toggleLike
)
```

`animate: true` adds smooth transitions between all states automatically. No separate transition syntax.

---

## Complete Real Example

```vel
// types.vel
export type Product {
    id: Number
    name: Text
    price: Number
    image: Text?
    status: ProductStatus
}

export enum ProductStatus {
    Available
    OutOfStock
    Discontinued
}

// components/product-card.vel
import { Product, ProductStatus } from "types"

export component ProductCard(product: Product) {
    column(
        background: white,
        radius: 12,
        shadow: small,
        border: 1,
        borderColor: lightGray,
        overflow: clip
    ) {
        if product.image {
            image(product.image, ratio: 4/3, width: full)
        }

        column(padding: 16, gap: 8) {
            text(product.name, bold: true, overflow: ellipsis, lines: 1)

            match product.status {
                Available -> text("{product.price} USD", color: green, bold: true)
                OutOfStock -> text("Out of stock", color: gray)
                Discontinued -> text("Discontinued", color: red)
            }
        }
    }
}

// pages/store.vel
import ProductCard from "components/product-card"
import { Product } from "types"

page Store {
    state {
        products = try api.get("/products")
        search = ""
    }

    fixed(top: 0, left: 0, right: 0) {
        row(align: spread, padding: 16, background: white, shadow: small) {
            image("/logo.png", size: 32)
            input(search, placeholder: "Search products...")
        }
    }

    center {
        column(width: full, maxWidth: 1200, padding: 24, gap: 24) {
            match products {
                Loading -> spinner()
                Error(msg) -> text("Error: {msg}", color: red)
                Success(data) -> {
                    on phone {
                        grid(columns: 1, gap: 12) {
                            list(data) { p -> ProductCard(product: p) }
                        }
                    }
                    on tablet {
                        grid(columns: 2, gap: 16) {
                            list(data) { p -> ProductCard(product: p) }
                        }
                    }
                    on desktop {
                        grid(columns: 3, gap: 24) {
                            list(data) { p -> ProductCard(product: p) }
                        }
                    }
                }
            }
        }
    }
}
```

---

## State System

### Page-level state
```vel
page Cart {
    state {
        items: List<CartItem> = []
        coupon = ""
        isSubmitting = false
        user = try api.get("/me")    // async — Result<User>
    }

    fn addItem(item: CartItem) {
        items = items + [item]       // assign = automatic reactivity
    }

    fn removeItem(id: Number) {
        items = items.filter(i -> i.id != id)
    }
}
```

Any assignment to a `state` variable updates the UI automatically. No extra calls needed.

### Derived state — computed values
```vel
page Cart {
    state {
        items: List<CartItem> = []
        discountPercent = 0
    }

    derived {
        subtotal  = items.sum(i -> i.price * i.quantity)
        discount  = subtotal * (discountPercent / 100)
        total     = subtotal - discount
        itemCount = items.sum(i -> i.quantity)
        isEmpty   = items.length == 0
    }

    text("Total: {total} USD")
    text("{itemCount} items")
}
```

`derived` recalculates automatically when any `state` it reads changes. It's always fresh — not a function to call.

### Effects — react to state changes
```vel
page Search {
    state {
        query = ""
        results: List<Product> = []
    }

    on change query {
        if query.length > 2 {
            results = try api.get("/search?q={query}")
        } else {
            results = []
        }
    }

    input(query, placeholder: "Search...")
    list(results) { p -> ProductCard(product: p) }
}
```

`on change [variable]` runs after the value changes. Can be async (use `try`). No dependency arrays to declare.

### Global stores — state shared between pages
```vel
// stores/auth.vel
export store auth {
    state {
        user: User? = none
        token: Text? = none
    }

    derived {
        isLoggedIn  = user != none
        displayName = user?.name ?? "Guest"
    }

    fn login(email: Text, password: Text): Result<none> {
        result = try api.post("/auth/login", { email, password })
        match result {
            Success(data) -> {
                user = data.user
                token = data.token
            }
            Error(msg) -> Error(msg)
        }
    }

    fn logout() {
        user = none
        token = none
        go("/login")
    }
}
```

Using a store in any page:
```vel
import auth from "stores/auth"

page Dashboard {
    guard not auth.isLoggedIn -> go("/login")

    text("Welcome, {auth.displayName}")
    Button("Logout", onClick: auth.logout)
}
```

A store is a singleton — all pages see the same state. When it changes in one page, all pages that use it update.

### Reactivity model
Fine-grained reactivity — only what actually changed re-renders, not the whole screen.

```vel
page Dashboard {
    state {
        count = 0
        name = "Ana"
    }

    text("Count: {count}")   // re-renders when count changes
    text("Hello {name}")     // does NOT re-render when count changes

    Button("+1", onClick: { count = count + 1 })
}
```

The compiler tracks which UI nodes read which state variables. Change in `count` → only nodes that read `count` are redrawn on GPU. No virtual DOM, no diffing, no reflows.

### on mount / on unmount
```vel
page Analytics {
    on mount {
        tracker.track("page_view")
        socket.connect()
    }

    on unmount {
        socket.disconnect()
        tracker.flush()
    }
}
```

`on mount` runs once when the page loads. `on unmount` runs when leaving. Both can be async (`try` works inside).

### State persistence
```vel
export store auth {
    state {
        token: Text? = none, persist    // survives closing the browser
        user: User?  = none, persist
        theme = "light", persist
        count = 0                        // not persisted — resets each session
    }
}
```

Variables marked `persist` are saved to device storage automatically. Only for stores — page state is always temporary.

---

## Navigation

### Basic navigation
```vel
go("/store")               // go to a page
go("/login", replace: true) // replace — does not add to history
go(back)                   // go back
```

`go()` works from anywhere: inside `fn`, inside `on change`, inside a Button.

### Parameters between pages
```vel
// passing parameters
go("/product", id: 42)
go("/search", query: "shoes", category: "men")

// receiving parameters — declared in the page signature
page Product(id: Number) {
    state {
        product = try api.get("/products/{id}")
    }
    match product {
        Loading -> spinner()
        Success(data) -> ProductDetail(data)
        Error(msg) -> text(msg, color: red)
    }
}

page Search(query: Text, category: Text?) {
    state {
        results = try api.get("/search?q={query}&cat={category ?? ""}")
    }
}
```

Parameters are named arguments — same syntax as component properties. No manual query strings, no `useParams()`.

### Route structure — files = routes
```
pages/
  home.vel              → /
  store.vel             → /store
  product.vel           → /product   (receives id: Number)
  profile/
    index.vel           → /profile
    settings.vel        → /profile/settings
  admin/
    index.vel           → /admin
    users.vel           → /admin/users
```

No separate router file. The file structure **is** the router.

### Guards — protect pages
```vel
import auth from "stores/auth"

page Dashboard {
    guard not auth.isLoggedIn -> go("/login")

    text("Welcome {auth.displayName}")
}

page Admin {
    guard not auth.isLoggedIn -> go("/login")
    guard not auth.user?.isAdmin -> go("/")

    AdminPanel()
}
```

`guard [condition] -> [action]` — if condition is true, runs the action before rendering anything.

### State lifetime
- Page state is destroyed when leaving the page (reset on return)
- Store state lives for the entire app session

### Persistent layouts — shell with changing content
```vel
// pages/admin/_layout.vel — wraps all pages inside /admin/
layout AdminShell {
    row(height: full) {
        AdminSidebar()
        column(width: grow) {
            AdminTopBar()
            outlet()    // page content renders here
        }
    }
}

// pages/admin/dashboard.vel → /admin/dashboard
// uses AdminShell automatically — sidebar and topbar stay mounted
page Dashboard {
    text("Dashboard content")
}

// pages/admin/users.vel → /admin/users
// also uses AdminShell — no re-render of the shell on navigation
page Users {
    UserTable()
}
```

`_layout.vel` in a folder wraps all pages in that folder. `outlet()` marks where page content goes. Navigating between pages in the same folder keeps the layout mounted — no flicker, no re-render.

A root `pages/_layout.vel` wraps the entire app (for global nav bars, etc.).

### 404 page
```vel
// pages/_404.vel — shown when no route matches
page NotFound {
    center {
        column(align: center, gap: 16) {
            text("404", size: 64, weight: 700, color: gray300)
            text("Page not found", size: 18, color: gray500)
            Button("Go home", onClick: { go("/") })
        }
    }
}
```

---

## API and Data

### Basic operations
```vel
// GET
state { products = try api.get("/products") }

// POST
result = try api.post("/orders", { items: cart.items, address: form.address })

// PUT
result = try api.put("/profile", { name: form.name, bio: form.bio })

// PATCH
result = try api.patch("/users/{id}", { active: true })

// DELETE
result = try api.delete("/products/{id}")
```

All return `Result<T>` — errors can never be ignored.

### Global config — base URL
```vel
// vel.config.vel — project root file
config {
    api {
        baseUrl: "https://api.myapp.com"
        timeout: 10000
    }
}
```

### Auth headers — injected automatically
```vel
// stores/auth.vel
export store auth {
    state { token: Text? = none }

    api.headers {
        Authorization: "Bearer {token}"
    }
}
```

No need to pass headers on every call. If the store has a token, all calls carry it.

### Cache
```vel
state {
    user       = try api.get("/me")                          // no cache
    categories = try api.get("/categories", cache: 5min)    // 5 minutes
    config     = try api.get("/app-config", cache: forever) // until invalidated
}

fn refreshCategories() {
    api.invalidate("/categories")
}
```

### Pagination
```vel
page ProductList {
    state {
        page     = 1
        products = try api.get("/products", page: 1, limit: 20)
    }

    derived { hasMore = products.Success?.hasNextPage ?? false }

    fn loadMore() {
        page = page + 1
        more = try api.get("/products", page: page, limit: 20)
        match more {
            Success(data) -> products.append(data.items)
            Error(msg)    -> showError(msg)
        }
    }

    list(products.items) { p -> ProductCard(product: p) }
    if hasMore { Button("Load more", onClick: loadMore) }
}
```

### Real-time — WebSockets
```vel
page Chat(roomId: Number) {
    state {
        messages: List<Message> = []
        input = ""
    }

    socket = api.socket("/chat/{roomId}")

    on socket.message { msg -> messages = messages + [msg] }
    on socket.error   { err -> showError(err) }

    fn send() {
        if input.length > 0 {
            socket.send({ text: input })
            input = ""
        }
    }

    scroll(height: full) {
        list(messages) { m -> MessageBubble(message: m) }
    }
    row(padding: 16, gap: 8) {
        input(input, placeholder: "Message...", width: grow)
        Button("Send", onClick: send)
    }
}
```

Socket closes automatically when leaving the page.

### Typed API responses
```vel
state {
    // declare the expected type on the variable
    users:   Result<List<User>> = try api.get("/users")
    product: Result<Product>    = try api.get("/products/{id}")
    me:      Result<User>       = try api.get("/me")
}
```

The compiler validates that the API response matches the declared type. Extra or missing fields are a compile-time error — not a runtime crash.

### File upload
```vel
page EditProfile {
    state { avatarUrl: Text? = none }

    fn handleUpload(file: File) {
        result = try api.upload("/upload/avatar", file)
        match result {
            Success(url) -> avatarUrl = url
            Error(msg)   -> toast(msg, type: error)
        }
    }

    filePicker(onSelect: handleUpload, accept: "image/*")

    if avatarUrl {
        image(avatarUrl, size: 80, radius: full)
    }
}
```

### Environment variables in config
```vel
// vel.config.vel
config {
    api {
        baseUrl: env("API_URL", default: "http://localhost:3000")
        timeout: 10000
    }
}
```

`env()` reads environment variables at build time. The `default` is used in development when the variable is not set.

---

## Module System

### Project structure
```
my-app/
  vel.config.vel          ← global config
  types.vel               ← shared types
  pages/
    home.vel
    store.vel
    product.vel
    admin/
      index.vel
      users.vel
  components/
    button.vel
    card.vel
    modal.vel
  stores/
    auth.vel
    cart.vel
  utils/
    format.vel
    validate.vel
```

### Public vs private
```vel
// components/card.vel
export component Card(title: Text) { ... }   // importable
export type CardProps { title: Text }         // importable

component CardHeader(title: Text) { ... }     // private to this file
fn formatTitle(t: Text): Text { ... }         // private to this file
```

No `export` = private to the file.

### Imports
```vel
import Button from "components/button"
import { Card, Modal } from "components/ui"
import auth from "stores/auth"
import { formatPrice } from "utils/format"
import { User, Product } from "types"

// relative (useful within the same folder)
import CardHeader from "./card-header"
```

### Barrel files — public API for a folder
```vel
// components/index.vel
export { Button } from "components/button"
export { Card } from "components/card"
export { Modal } from "components/modal"
```

```vel
// now from any page:
import { Button, Card, Modal } from "components"
```

### Shared types — contract between layers
```vel
// types.vel — source of truth for all project types
export type User {
    id: Number
    name: Text
    email: Text
    role: UserRole
    avatar: Text?
}

export enum UserRole { Admin | Editor | Viewer }
```

All files that need `User` import from here. One place to change them.

### Rules
| Rule | Why |
|------|-----|
| No `export` = private | Encapsulation by default |
| Import only what you use | Compiler eliminates unused code |
| Types in `types.vel` | Single source of truth |
| No circular imports | Compiler detects and gives clear error |

---

## Visual Design System

### Typography
```vel
text("Welcome back",   size: 32, weight: 700, lineHeight: 1.2)
text("Manage account", size: 16, color: gray500, lineHeight: 1.5)
text("LABEL",          size: 12, weight: 600, letterSpacing: 0.08, color: gray400)
text("Note: may vary", size: 14, italic: true)
text("Was $99",        size: 14, strikethrough: true, color: gray400)
text("Terms apply",    size: 14, underline: true)
text("Centered",       size: 24, weight: 700, align: center)
```

| Property | Values |
|----------|--------|
| `size` | number in px |
| `weight` | 400 \| 500 \| 600 \| 700 |
| `lineHeight` | number (1.2 = tight, 1.5 = normal, 1.8 = loose) |
| `letterSpacing` | number in em |
| `italic` | true/false |
| `strikethrough` | true/false |
| `underline` | true/false |
| `align` | left \| center \| right |

### Colors — hex and opacity
```vel
rect(background: #3B82F6)                // hex
rect(background: #3B82F6.opacity(0.15))  // hex with opacity
rect(background: black.opacity(0.5))     // named with opacity
```

### Gradients
```vel
rect(background: gradient(from: #6366F1, to: #8B5CF6))
rect(background: gradient(from: #F59E0B, to: #EF4444, direction: diagonal))
rect(background: gradient(from: #1A1A2E, to: transparent, direction: bottom))
```

### Theme — consistent design tokens
```vel
// theme.vel — define once, use everywhere
export theme {
    colors {
        primary:     #3B82F6
        primaryDark: #2563EB
        danger:      #EF4444
        success:     #10B981
        warning:     #F59E0B

        gray50:  #F9FAFB
        gray100: #F3F4F6
        gray200: #E5E7EB
        gray400: #9CA3AF
        gray500: #6B7280
        gray700: #374151
        gray900: #111827

        text:       #111827
        textMuted:  #6B7280
        background: #FFFFFF
        surface:    #F9FAFB
        border:     #E5E7EB
    }

    text {
        xs: 12
        sm: 14
        md: 16
        lg: 18
        xl: 24
        h3: 28
        h2: 36
        h1: 48
    }

    radius {
        sm: 4
        md: 8
        lg: 12
        xl: 20
        full: 9999
    }
}
```

```vel
import theme from "theme"

rect(
    background: theme.colors.surface,
    border: 1,
    borderColor: theme.colors.border,
    radius: theme.radius.lg
) {
    text("Title",    size: theme.text.h3, color: theme.colors.text)
    text("Subtitle", size: theme.text.md, color: theme.colors.textMuted)
}
```

Literal values always work (`background: white`, `radius: 12`). The theme is optional but enables consistency at scale.

---

## Standard Library

All built-in components are available without imports.

### Inputs
```vel
input(value, placeholder: "Email...")
input(value, type: password, placeholder: "Password")
input(value, type: number, min: 0, max: 100)
input(value, type: search, placeholder: "Search...")
textarea(value, placeholder: "Message...", lines: 4)
checkbox(checked, label: "Accept terms")
toggle(enabled, label: "Notifications")
select(selected, options: ["USD", "EUR", "MXN"])
datepicker(date)
filePicker(onSelect: handleFile, accept: "image/*")
```

### Keyboard events on inputs
```vel
input(query, placeholder: "Search...", onEnter: search)
input(value, onKeyDown: handleKey)
```

### Global keyboard shortcuts
```vel
page Dashboard {
    on key("escape") { closeModal() }
    on key("ctrl+k") { openSearch() }
    on key("ctrl+s") { save() }
}
```

### Feedback
```vel
spinner()
spinner(size: large)

toast("Saved successfully", type: success)
toast("Connection lost",    type: error)
toast("New message",        type: info)
toast("Warning: low disk",  type: warning)

modal(open: showModal) {
    text("Are you sure?", size: 18, weight: 600)
    row(gap: 8, align: right) {
        Button("Cancel", style: secondary, onClick: { showModal = false })
        Button("Delete", style: danger,    onClick: handleDelete)
    }
}

tooltip("Click to save") {
    Button("Save")
}
```

### Navigation components
```vel
tabs(selected: activeTab) {
    tab("Overview") { OverviewContent() }
    tab("Settings") { SettingsContent() }
    tab("Billing")  { BillingContent() }
}

accordion {
    section("Shipping") { ShippingInfo() }
    section("Returns")  { ReturnsPolicy() }
}

divider()
divider(color: gray200, thickness: 1)
```

### Common elements
```vel
Button("Save")
Button("Delete",  style: danger)
Button("Cancel",  style: secondary)
Button("Skip",    style: ghost)
Button("Details", style: link)
Button("Save",    size: large)
Button("Save",    loading: true)
Button("Save",    disabled: true)
Button("Search",  icon: search)
Button(icon: menu)

avatar(user.photo, size: 40)
avatar(user.photo, fallback: user.name, size: 40)

badge("New",          color: blue)
badge("Out of stock", color: red)

image(src, ratio: 16/9, fallback: "/placeholder.png")

spacer(height: 24)
spacer(grow: true)   // pushes following content to the end
```

### Button styles
| Style | Look |
|-------|------|
| (default) | filled primary color |
| `secondary` | outline |
| `danger` | filled red |
| `ghost` | no background |
| `link` | looks like a link |

### List vs virtualList
```vel
// list — renders all items (use for < ~200 items)
list(products) { p -> ProductCard(product: p) }

// virtualList — only renders visible items (use for large datasets)
virtualList(messages, height: full) { m -> MessageRow(message: m) }
virtualList(users, height: 600, itemHeight: 64) { u -> UserRow(user: u) }
```

### Components with children (slots)
```vel
// defining a component that accepts children
export component Card(title: Text, children: Block) {
    column(
        background: white,
        radius: 12,
        border: 1,
        borderColor: gray200,
        overflow: clip
    ) {
        row(padding: [16, 16, 0, 16]) {
            text(title, size: 16, weight: 600)
        }
        column(padding: 16) {
            children    // renders whatever was passed
        }
    }
}

// using it
Card(title: "Order Summary") {
    text("3 items")
    text("Total: $47.00", weight: 600)
    Button("Checkout", width: full)
}
```

`children: Block` is the type for arbitrary content passed to a component. Inside the component, `children` renders that content.

### Icons — built-in set
```vel
icon(search,       size: 20)
icon(menu,         size: 24, color: gray500)
icon(heart,        size: 20, color: red)
icon(check,        size: 16, color: green)
icon(creditCard,   size: 20)
icon(notification, size: 20)
```

Available icons: `search` `menu` `close` `back` `forward` `heart` `star` `share` `edit` `delete` `add` `check` `warning` `info` `user` `settings` `home` `cart` `camera` `image` `file` `download` `upload` `link` `lock` `unlock` `eye` `eyeOff` `mail` `phone` `location` `calendar` `clock` `filter` `sort` `creditCard` `notification` `refresh` `copy` `drag`

### Utilities
```vel
// text
"hello world".toUpperCase()       // "HELLO WORLD"
"  hello  ".trim()                // "hello"
"hello world".contains("world")   // true
"email@x.com".isEmail()           // true
"123".isNumeric()                 // true

// numbers
(3.14159).toFixed(2)              // "3.14"
(1234567).format()                // "1,234,567"
(0.25).toPercent()                // "25%"
Math.min(a, b)
Math.max(a, b)
Math.round(n)
Math.abs(n)

// lists
items.sum(i -> i.price)
items.filter(i -> i.active)
items.map(i -> i.name)
items.find(i -> i.id == 5)
items.sort(i -> i.name)
items.sort(i -> i.price, descending: true)
items.groupBy(i -> i.category)
items.first()
items.last()
items.length
items.isEmpty
items.contains(item)

// dates
Date.today()
Date.now()
date.format("MMM DD, YYYY")       // "Jun 03, 2026"
date.fromNow()                    // "2 hours ago"
date.addDays(7)
date.isBefore(otherDate)
```

---

## Patterns we do NOT use and why
| Pattern | Example | Why not |
|---------|---------|---------|
| Chained properties | `text("hi").bold().size(24)` | Confuses where config ends |
| Significant indentation | Python style | Invisible errors, tabs vs spaces |
| XML/XAML | `<Button Text="hi" />` | Verbose, not real code |
| Magic symbols | `$user`, `@State`, `#slot` | Each symbol is something new to memorize |
| Separate CSS | styles.css | Separates what should be together |
| Implicit coercion | `"5" + 5 = "55"` | Silent bugs in production |
| Null | `null`, `nil`, `undefined` | Billion dollar mistake — use `?` instead |
| throw/catch exceptions | `throw new Error()` | Errors must be part of the type |
| `&&` `\|\|` `!` | `if a && b` | Words are more readable for all developers |
