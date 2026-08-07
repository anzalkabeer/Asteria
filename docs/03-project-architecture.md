# Project Architecture

> **Purpose:** Explain the repository layout so someone can understand the codebase before opening any code.
>
> **Audience:** Contributors, Rust developers, anyone exploring the source.
>
> **Estimated reading time:** 10 minutes
>
> **Prerequisites:** [How Asteria Works](02-how-asteria-works.md) (recommended)

---

## Repository overview

Asteria is a single Rust crate that produces both a library (`libasteria`) and a binary (`asteria`). The entire engine lives in one workspace with no sub-crates.

```
Asteria/
├── Cargo.toml              ← Crate manifest (dependencies, build config)
├── Cargo.lock              ← Pinned dependency versions
├── README.md               ← Project front page
├── .github/
│   └── workflows/
│       ├── ci.yml          ← Rust CI: check, fmt, clippy, test
│       └── sonarqube.yml   ← Code quality analysis
├── docs/                   ← This documentation (you are here)
├── src/                    ← All engine source code
│   ├── lib.rs              ← Library root — module declarations
│   ├── main.rs             ← Binary entry point — CLI demo & pipeline runner
│   ├── ...                 ← Engine modules (detailed below)
│   ├── net/                ← Networking stack
│   ├── renderer/           ← GPU rendering subsystem
│   └── devtools/           ← Observability & diagnostics
└── tests/                  ← Integration tests & visual fixtures
    ├── fixtures/           ← HTML/CSS test pages
    ├── style_integration.rs
    ├── layout_integration.rs
    ├── paint_integration.rs
    ├── image_integration.rs
    ├── network_integration.rs
    ├── renderer_integration.rs
    └── observability_trace.rs
```

---

## Source modules

Every `.rs` file in `src/` maps to one subsystem of the engine. Here's what each one does and why it exists.

### Core pipeline modules

These are the modules that implement the main rendering pipeline, listed in the order data flows through them:

| Module | File | Role |
|---|---|---|
| **HTML Tokens** | `tokens.rs` | Token types (`StartTag`, `EndTag`, `Text`, `Comment`, `Eof`) and `Attribute` struct |
| **HTML Tokenizer** | `tokenizer.rs` | State machine that converts raw HTML bytes into tokens. Zero-copy — stores byte offset pairs |
| **HTML Parser** | `parser.rs` | Consumes tokens and builds the DOM tree. Handles implicit closing, nesting, and error recovery |
| **Streaming Parser** | `streaming_parser.rs` | Wraps the tokenizer + parser for progressive network chunk processing |
| **DOM** | `dom.rs` | Arena-allocated document tree: `NodeId`, `Node`, `NodeKind`, `Dom` |
| **CSS Tokens** | `css_tokens.rs` | CSS token types (`Ident`, `Hash`, `String`, `Number`, etc.) |
| **CSS Tokenizer** | `css_tokenizer.rs` | Converts CSS source bytes into CSS tokens |
| **CSS Parser** | `css_parser.rs` | Parses tokens into `Stylesheet` — rules, selectors, declarations, `@media` blocks |
| **CSS Properties** | `properties.rs` | Property registry: `PropertyId` enum, inheritance flags, shorthand expansion rules |
| **CSS Values** | `values.rs` | Typed value system: `Color`, `Display`, `Length`, `Edges`, `ComputedStyle` |
| **Style Resolver** | `style.rs` | Selector matching, specificity, cascade, inheritance, shorthand expansion, computed styles |
| **Layout Engine** | `layout.rs` | Box model geometry: `Rect`, `Dimensions`, `LayoutBox`. Block, inline, and flex formatting contexts |
| **Paint Engine** | `paint.rs` | Converts layout boxes into a flat `DisplayList` of `DisplayCommand` draw instructions |
| **Scene Graph** | `scene.rs` | Data-oriented scene representation: `SceneNode`, `SceneNodeId`, z-ordering, dirty flags, segments |
| **Segment Builder** | `segment.rs` | Divides the viewport into GPU tile segments for region-based invalidation |

### Infrastructure modules

These modules support the pipeline but don't directly participate in it:

| Module | File | Role |
|---|---|---|
| **Resource Loader** | `loader.rs` | Discovers and loads HTML, CSS from disk and network. `PageResources` bundling, `ResourceCache` |
| **String Interner** | `interner.rs` | Maps strings to `Symbol(u32)` handles. Pre-seeded with 67 common HTML tags, attributes, CSS properties |
| **Task Scheduler** | `scheduler.rs` | Priority task queue + multi-threaded worker pool with `std::thread` and `mpsc` channels |
| **Browser Shell** | `shell.rs` | `TabManager`, `Tab`, `NavigationHistory`, `ShellEvent` dispatcher |
| **Engine Profiler** | `profiler.rs` | RAII `StageGuard` microsecond timing, `ProfileReport` with FPS estimation |
| **Frame Arena** | `arena.rs` | Bump allocator — `FrameArena` with typed allocation and instant reset |
| **Image Decoder** | `image.rs` | Format detection (PNG, JPEG, BMP, GIF, WebP, TIFF, SVG) and decode pipeline |
| **LRU Cache** | `cache.rs` | Least-recently-used eviction cache for loaded resources |
| **Object Pool** | `pool.rs` | Reusable buffer pool to reduce allocation pressure |
| **Frame** | `frame.rs` | Frame lifecycle management |

### Networking stack (`src/net/`)

A custom, dependency-light networking layer:

| Module | File | Role |
|---|---|---|
| **TCP** | `net/tcp.rs` | `TcpConnection`, `ConnectionPool`, `Stream` abstraction, `NetworkError` |
| **DNS** | `net/dns.rs` | `DnsResolver` with TTL in-memory caching |
| **TLS** | `net/tls.rs` | `TlsConnector` and `TlsConnection` wrapping rustls + webpki |
| **HTTP** | `net/http.rs` | `HttpClient` — HTTP/1.1 GET client with `Url` parser, redirects, and response parsing |
| **Streaming Bus** | `net/bus.rs` | `StreamingResourceBus` — MPSC channel for async resource delivery |

### GPU renderer (`src/renderer/`)

The hardware rendering subsystem:

```
renderer/
├── mod.rs              ← Module declarations
├── backend/
│   └── wgpu_backend.rs ← GPU device, surface, queue, adapter setup
├── commands/
│   ├── batch_builder.rs ← Batches scene nodes into vertex buffers
│   └── command_builder.rs ← Translates scene data into GPU draw commands
├── graph/
│   └── render_graph.rs ← Coordinates render passes and pipeline state
├── passes/
│   ├── rect_pass.rs    ← Renders solid rectangles and borders
│   ├── text_pass.rs    ← Renders text glyphs via glyphon
│   ├── image_pass.rs   ← Renders decoded images as textured quads
│   ├── shader.wgsl     ← WGSL shader for rect/border rendering
│   └── image_shader.wgsl ← WGSL shader for image rendering
├── resources/          ← GPU resource management (buffers, textures)
├── scheduler/
│   └── batching.rs     ← Batch planning and draw call optimisation
├── text/               ← Text rendering integration with glyphon
└── window/
    └── window.rs       ← winit event loop, input handling, interactive rendering
```

### Devtools (`src/devtools/`)

Observability and diagnostics framework:

| Module | File | Role |
|---|---|---|
| **Config** | `devtools/config.rs` | `AofConfig` — feature flags for inspection depth |
| **Inspector** | `devtools/inspector.rs` | `AofInspector` — main inspection entry point |
| **Metrics** | `devtools/metrics.rs` | Atomic counters for memory allocation and GPU VRAM usage |
| **Trace** | `devtools/trace.rs` | Chrome Trace JSON event recording |
| **Export** | `devtools/export.rs` | Trace file export to `trace.json` |
| **Snapshot** | `devtools/snapshot.rs` | `EngineSnapshot` — point-in-time capture of engine state |
| **Formatter** | `devtools/formatter.rs` | Human-readable output formatting for diagnostics |

---

## Module boundaries

Asteria enforces clear boundaries between subsystems:

```
                    ┌──────────┐
                    │  loader  │ ─── discovers resources, feeds bytes to parsers
                    └────┬─────┘
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
    ┌──────────┐  ┌──────────┐   ┌──────────┐
    │ tokenizer│  │css_token.│   │   net/    │ ─── networking
    │ + parser │  │ + parser │   │          │
    └────┬─────┘  └────┬─────┘   └──────────┘
         │             │
         ▼             ▼
    ┌──────────┐  ┌──────────┐
    │   dom    │  │stylesheet│
    └────┬─────┘  └────┬─────┘
         │             │
         └──────┬──────┘
                ▼
          ┌──────────┐
          │  style   │ ─── cascade, specificity, inheritance
          └────┬─────┘
               ▼
          ┌──────────┐
          │  layout  │ ─── geometry computation
          └────┬─────┘
               ▼
          ┌──────────┐
          │  paint   │ ─── display list generation
          └────┬─────┘
               ▼
          ┌──────────┐
          │  scene   │ ─── GPU-optimised scene graph
          └────┬─────┘
               ▼
          ┌──────────┐
          │ renderer │ ─── wgpu + winit GPU rendering
          └──────────┘
```

Key boundary rules:

- **The DOM is immutable after construction.** No later stage modifies it. Style, layout, and paint each produce their own data structures.
- **Each stage outputs a distinct data type.** Tokenizer → `Vec<Token>`. Parser → `Dom`. Style → `StyledTree`. Layout → `LayoutTree`. Paint → `DisplayList`. Scene → `SceneGraph`.
- **Infrastructure modules are shared.** The interner, cache, profiler, and scheduler are used across multiple pipeline stages.

---

## External dependencies

Asteria's core web engine uses no third-party browser crates. External dependencies are used only for non-web-engine concerns:

| Dependency | Version | Purpose |
|---|---|---|
| `wgpu` | 22.1 | GPU API (WebGPU standard) |
| `winit` | 0.29 | Cross-platform windowing and event handling |
| `pollster` | 0.3 | Minimal async executor for wgpu initialization |
| `bytemuck` | 1.16 | Safe casting of vertex data to byte slices for GPU upload |
| `glyphon` | 0.6 | Text shaping and glyph rendering |
| `rustls` | 0.23 | TLS implementation (for HTTPS) |
| `webpki-roots` | 0.26 | Mozilla's root certificate store |
| `rustls-pki-types` | 1.7 | PKI type definitions for rustls |
| `lru` | 0.12 | LRU cache implementation |
| `log` | 0.4 | Logging facade |
| `env_logger` | 0.11 | Environment-based log configuration |

---

## Build and run

```bash
# Check compilation
cargo check

# Run with built-in sample page
cargo run

# Run with a custom HTML file
cargo run -- path/to/page.html

# Run without opening a window (CLI-only pipeline inspection)
cargo run -- path/to/page.html --cli

# Run all tests
cargo test
```

**Requirements:** Rust (2024 edition). GPU-capable system with Vulkan, Metal, or DX12 support.

---

## Related documents

- [How Asteria Works](02-how-asteria-works.md) — pipeline overview
- [HTML Engine](04-html-engine.md) — tokenizer and parser details
- [DOM](05-dom.md) — arena allocation and tree structure
- [Contributing](14-contributing.md) — how to work on the codebase
