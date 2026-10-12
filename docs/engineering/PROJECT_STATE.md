# PROJECT_STATE.md — Verified Current Engineering State

> **Repository:** Asteria Browser Engine (`Asteria/Asteria`)  
> **Audited Git Revision:** `3cdd5155caf544794c3d11f45dfc839665d9759b` (Branch `main`)  
> **Working Tree Status:** `Cargo.lock` modified (unstaged dependency resolution); all source files pristine.  
> **Audited Toolchain:** Rust Stable (x86_64-pc-windows-msvc), Cargo 2024 edition target.  
> **Last Verified Baseline:** October 11, 2026 (`cargo test --no-run` exit code `0`, 302.26s).  

---

## 1. Project Purpose & Engineering Vision

Asteria is an independent, pure-Rust web browser and rendering engine. Built from scratch without WebKit, Blink, or Gecko C++ dependencies, Asteria aims to deliver a modern, secure, and resource-efficient web platform.

### Core Architectural Principles
1. **100% Pure Rust:** Zero C++ build toolchains or unsafe legacy browser runtime components.
2. **Modern GPU Acceleration:** Built natively on **WGPU 0.20** using Portable WGSL shaders across Vulkan, Metal, and DirectX 12.
3. **Safety & Concurrency:** Fearless concurrency utilizing Rust's memory safety guarantees, typed arenas, and isolated background thread pools.

---

## 2. Major Modules & Architectural Inventory

Asteria currently exists as a single monolithic Cargo package (`asteria 0.1.0`) comprising 33 internal source modules organized across 9 conceptual tiers:

| Tier | Subsystem | Key Files | Verified Current Responsibilities |
| :--- | :--- | :--- | :--- |
| **Tier 1** | **Shell & Pipeline** | `src/main.rs`, `src/renderer/window/`, `src/pipeline.rs` | Window creation via `winit`, event loop dispatch, synchronous frame pipeline orchestration. |
| **Tier 2** | **DOM & Tree** | `src/dom.rs`, `src/html.rs`, `src/tree.rs` | HTML5 tokenization, DOM tree representation, arena-backed node hierarchy. |
| **Tier 3** | **CSS & Style** | `src/css.rs`, `src/style.rs`, `src/color.rs` | CSS tokenizer, parser for 33 properties, cascade sorting, inheritance, and computed style computation. |
| **Tier 4** | **Layout & Text** | `src/layout.rs`, `src/geometry.rs`, `src/text.rs` | Block formatting (BFC), inline line-wrapping, experimental single-line Flexbox, text metrics. |
| **Tier 5** | **Scene & Display** | `src/scene.rs`, `src/renderer/command_builder.rs` | Display list generation, 256px tile segments, geometric scene hierarchy construction. |
| **Tier 6** | **GPU Rendering** | `src/renderer/gpu/*`, `src/renderer/font.rs` | WGPU 0.20 render passes, WGSL shaders, vertex batch building, glyph atlas texture caching. |
| **Tier 7** | **Resources** | `src/image.rs`, `src/resource.rs` | Resource cache indexing, stub image decoder (mock gradient generation). |
| **Tier 8** | **Networking** | `src/net/http.rs`, `src/net/fetch.rs`, `src/net/url.rs` | Custom TCP/rustls HTTP client, synchronous URL fetch dispatch, URL parser. |
| **Tier 9** | **Memory & Sched** | `src/scheduler.rs`, `src/arena.rs`, `src/pool.rs`, `src/intern.rs` | Threaded task scheduler, priority queue scaffold, memory arenas, symbol interner (140+ symbols). |

---

## 3. Verified Working Capabilities vs. Known Limitations

### Verified Working Capabilities (Confirmed by Code & Tests)
- **HTML/CSS Parsing:** Full parsing of standard HTML documents and CSS stylesheets into typed ASTs.
- **Computed Style Cascade:** Specificity sorting, `@media` query evaluation, CSS variables (`var()`), and inheritance rules execute properly.
- **Normal Flow Layout:** Block-level box generation, margin collapsing, padding/border offsets, and basic inline line breaking work reliably.
- **GPU Accelerated Rendering:** WGPU renders colored rectangles, borders, and text runs with texture-atlas glyph caching on desktop GPUs.
- **Browser Shell:** Tab management, navigation history (back/forward), address bar input, and basic window resize events work smoothly.
- **TLS / HTTPS Networking:** Native encrypted HTTPS connections function via `rustls` without OpenSSL dependencies.

### Verified Known Limitations (Substantiated Deficiencies)
- **Resolved in Stage 1:**
  - **GPU Buffer Churn ([`AST-PERF-001`](issues/AST-PERF-001.md)):** **RESOLVED** via `GeometryBufferPool` retained GPU vertex/index buffer pool in `src/renderer/resources/buffer_pool.rs`.
  - **Non-Functional Image Rendering ([`AST-DEF-002`](issues/AST-DEF-002.md)):** **RESOLVED** via `image = "0.25"` PNG/JPEG decoding and `RenderCommand::Image` activation.
  - **URL Parser RFC 9110 Bug ([`AST-DEF-003`](issues/AST-DEF-003.md)):** **RESOLVED** via `Url::request_target()` stripping fragment identifiers over the wire.
  - **Unbounded Network Buffer ([`AST-DEF-004`](issues/AST-DEF-004.md)):** **RESOLVED** via 100 MB body size ceiling (`MAX_RESPONSE_BODY_SIZE`) in `src/net/http.rs`.
- **Active Deficiencies (Scheduled for Stage 2 & Stage 3):**
  - **Immediate-Mode Redraw Lag ([`AST-DEF-001`](issues/AST-DEF-001.md)):** Scrolling and cursor motion rebuild all vertex buffers and iterate the entire display list on CPU, introducing frame stutter (Scheduled for Stage 3 via `SPEC-08`).
  - **Font Metric Disconnect ([`AST-ARC-002`](issues/AST-ARC-002.md)):** Layout line breaks use hardcoded character ratios (`estimate_char_width_ratio`) rather than font metrics, desyncing line wraps from GPU glyphs (Scheduled for Stage 2 via `SPEC-06`).
  - **Incomplete Flexbox & Missing Grid ([`AST-ARC-003`](issues/AST-ARC-003.md)):** Handcrafted flexbox lacks multi-line wrapping and flex-grow/shrink factors; CSS Grid is unimplemented (Scheduled for Stage 3 via `SPEC-10`).
  - **Dead Memory Infrastructure ([`AST-INC-001`](issues/AST-INC-001.md)):** `FrameArena`, `Pool<T>`, and 140+ pre-seeded `Symbol` constants are completely unused in engine hot paths (Scheduled for Stage 3 via `SPEC-11`).
  - **No JavaScript Engine ([`AST-INC-003`](issues/AST-INC-003.md)):** DOM is static and read-only after initial load; `<script>` tags are skipped (Scheduled for Stage 5 via `ENH-01`).

---

## 4. Current Build & Test Invariants

Empirical verification executed on October 12, 2026:
- **Command:** `cargo test`
- **Result:** Exit code `0` (Success) across all 11 test suites.
- **Artifacts:** Successfully compiled and executed all 11 test runner suites in `target/debug/deps/` with zero failures and zero regressions.

---

## 5. Active Prerequisites & Immediate Engineering Focus

Implementation work is organized into 5 sequential stages ([`ROADMAP.md`](ROADMAP.md)):
1. **Prerequisite:** Stage 1 (`SPEC-01` to `SPEC-04`) must stabilize critical rendering and networking defects before Stage 2 workspace modularization begins.
2. **Current Active Status:** Audit completed and signed off; implementation authorization pending for **Stage 1: Core Engine Stabilization**.
