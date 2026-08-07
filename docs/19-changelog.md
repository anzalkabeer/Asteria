# Changelog

> **Purpose:** A human-readable record of major milestones and releases.
>
> **Audience:** Everyone.
>
> **Estimated reading time:** 3 minutes
>
> **Prerequisites:** None

---

## v0.1.0 — Foundation (August 2026)

The first public milestone. Asteria can load and render styled HTML pages with GPU acceleration.

### Engine

- **HTML tokenizer** — zero-copy state-machine tokenizer producing tokens from raw HTML bytes
- **HTML parser** — tree-building parser constructing an arena-allocated DOM
- **Streaming parser** — progressive chunk parsing for network-loaded pages
- **CSS tokenizer** — produces CSS tokens from source text
- **CSS parser** — parses rules, selectors (tag, class, ID, universal, compound, combinators, pseudo-classes), declarations, and `@media` queries
- **Style resolver** — specificity scoring, cascade, inheritance, shorthand expansion, inline style priority, computed styles, live `@media` viewport evaluation
- **Layout engine** — block, inline, and flex formatting contexts with CSS box model geometry (margins, padding, borders, content sizing, auto centering, text wrapping)
- **Paint engine** — flat display list generation (SolidColor, Border, Text, Image) with CSS paint ordering
- **Scene graph** — data-oriented scene representation with z-ordering, segments, dirty flags, and interactive state tracking
- **GPU renderer** — wgpu hardware rendering with WGSL shaders, rect/text/image passes, batched vertex buffers, and render graph coordination

### Browser

- **Tab manager** — multi-tab support with per-tab DOM, stylesheet, and navigation history
- **Navigation** — back/forward/reload per tab
- **Keyboard shortcuts** — Ctrl+T (new tab), Ctrl+W (close tab), Alt+←/→ (history), Ctrl+R/F5 (reload)
- **Scrolling** — mouse wheel scroll with viewport offset
- **Hover and click** — hit testing against scene nodes, hover state, link navigation
- **Window resize** — live content reflow on window resize

### Networking

- **DNS resolver** — with TTL in-memory caching
- **TCP connections** — connection pool with stream abstraction
- **TLS** — HTTPS support via rustls and webpki-roots
- **HTTP client** — HTTP/1.1 GET with URL parsing, redirect following, and response parsing
- **Streaming resource bus** — MPSC channel for progressive resource delivery

### Infrastructure

- **String interner** — `Symbol(u32)` handles with 67 pre-seeded common strings
- **Frame arena** — bump allocator for per-frame memory
- **LRU cache** — bounded in-memory resource caching
- **Object pool** — reusable buffer pool
- **Engine profiler** — microsecond-precision stage timing with RAII guards
- **Devtools** — Asteria Observability Framework (AOF) with Chrome Trace export, memory inspector, energy diagnostics, and engine snapshots
- **Resource loader** — HTML/CSS loading from disk and network with stylesheet discovery
- **Task scheduler** — multi-threaded worker pool with priority queues and panic isolation
- **Image decoder** — format detection (PNG, JPEG, BMP, GIF, WebP, TIFF, SVG) and decode pipeline

### Documentation

- Complete documentation suite (21 documents) covering introduction, browser fundamentals, engine pipeline, architecture, HTML, DOM, CSS, layout, painting, GPU rendering, resource loading, browser shell, performance, roadmap, contributing, testing, FAQ, glossary, design decisions, known limitations, and this changelog

---

*Future releases will be documented here as milestones are reached.*
