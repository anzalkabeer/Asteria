# ARCHITECTURE.md — Verified System Architecture & Technical Reference

> **Audited Repository:** Asteria Browser Engine (`Asteria/Asteria`)  
> **Source Ground Truth:** Implemented code in `src/` as of Git commit `3cdd5155caf544794c3d11f45dfc839665d9759b`.  
> **Target Scope:** Document actual implemented architecture, contrast with target architectural evolution, and outline exact dataflow paths.

---

## 1. System Execution Planes & Threading Model

Asteria's runtime architecture operates across three distinct execution planes:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        HOST / UI PLANE (Main Thread)                   │
│   src/main.rs  ──>  src/renderer/window/window.rs  (winit event loop)   │
│   - OS Windowing, Keystrokes, Mouse Wheel, Navigation History          │
│   - Frame Pacing & WGPU Surface Presentation                           │
└────────────────────────────────────┬───────────────────────────────────┘
                                     │ dispatches tasks
                                     ▼
┌────────────────────────────────────────────────────────────────────────┐
│                    BACKGROUND EXECUTION PLANE (Worker Pool)            │
│   src/scheduler.rs  ──>  std::sync::mpsc / threadpool                  │
│   - Network Resource Fetch (src/net/fetch.rs)                          │
│   - HTML Parsing & Tokenization (src/html.rs, src/dom.rs)              │
│   - Style Cascade & Computed Style (src/css.rs, src/style.rs)          │
│   - Layout Box Tree & Text Line Wrap (src/layout.rs, src/text.rs)      │
│   - Display List & Command Generation (src/renderer/command_builder.rs)│
└────────────────────────────────────┬───────────────────────────────────┘
                                     │ submits GPU commands
                                     ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        GPU ACCELERATION PLANE                          │
│   src/renderer/gpu/renderer.rs  ──>  wgpu 0.20 Device & Queue          │
│   - RectPass (WGSL solid colors & borders)                             │
│   - TextPass (WGSL glyph atlas texture sampling)                       │
│   - ImagePass (Declared WGPU pipeline, awaiting activation)            │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Core Subsystems & Interfaces

### 2.1 DOM & Tree Subsystem (`src/dom.rs`, `src/html.rs`, `src/tree.rs`)
- **Structure:** Arena-backed `Node` tree. Nodes store string slices referencing input HTML buffers for zero-copy parsing.
- **Interfaces:** `Document::parse(html: &str) -> Document`, `Node::children() -> &[NodeId]`.
- **Constraint:** The DOM is completely static and read-only post-parse. DOM mutation APIs are currently missing ([`AST-INC-003`](issues/AST-INC-003.md)).

### 2.2 Style Engine (`src/css.rs`, `src/style.rs`, `src/color.rs`)
- **Structure:** Parses CSS rules into `Rule` ASTs with 3-tuple specificity `(id, class, type)`.
- **Computation:** Computes a monolithic `ComputedStyle` struct containing 33 longhand CSS properties.
- **Constraint:** Lacks a selector Bloom filter or rule sharing tree. Evaluates all rules linearly across all nodes ([`AST-PERF-003`](issues/AST-PERF-003.md)).

### 2.3 Layout Engine (`src/layout.rs`, `src/geometry.rs`, `src/text.rs`)
- **Structure:** Transforms styled nodes into a recursive `LayoutBox` tree (`BlockNode`, `InlineNode`, `AnonymousBlock`, `FlexNode`).
- **Formatting:** Computes Block Formatting Context (BFC) margin collapsing and inline line-wrapping.
- **Constraint:** Text line breaking uses `estimate_char_width_ratio` hardcoded ratios rather than font metrics ([`AST-ARC-002`](issues/AST-ARC-002.md)). Flexbox lacks wrapping and Grid is missing ([`AST-ARC-003`](issues/AST-ARC-003.md)).

### 2.4 Scene Graph & Display List (`src/scene.rs`, `src/renderer/command_builder.rs`)
- **Structure:** Flattens layout boxes into an array of `RenderCommand` primitives (`DrawRect`, `DrawText`, `DrawBorder`).
- **Tile Segments:** Partitions display commands into 256px spatial tiles.
- **Constraint:** Infers parent-child relationships using geometric bounding box containment rather than DOM hierarchy ([`AST-ARC-004`](issues/AST-ARC-004.md)).

### 2.5 GPU Renderer (`src/renderer/gpu/*`)
- **Backend:** WGPU 0.20 on Vulkan, Metal, DX12.
- **Batches:** `BatchBuilder` aggregates primitives into vertex/index arrays and uploads them to WGPU buffers.
- **Constraint:** Allocates fresh vertex and index buffers every frame via `create_buffer_init` ([`AST-PERF-001`](issues/AST-PERF-001.md)), and rebuilds them on every scroll event ([`AST-DEF-001`](issues/AST-DEF-001.md)).

### 2.6 Networking & Loader (`src/net/*`, `src/resource.rs`)
- **Client:** Custom TCP and TLS socket client utilizing `rustls`.
- **Constraint:** Synchronous socket reads accumulate unbounded byte vectors in memory ([`AST-DEF-004`](issues/AST-DEF-004.md)), and request lines include URL fragment identifiers ([`AST-DEF-003`](issues/AST-DEF-003.md)).

---

## 3. End-to-End Pipeline Execution Flow

```
HTTP Fetch (net/http.rs)
       │ Raw HTML Bytes
       ▼
HTML Parser (html.rs)  ──>  DOM Tree (dom.rs)
       │                          │
       │ Inline/External CSS      │
       ▼                          ▼
CSS Parser (css.rs)   ──>  Style Engine (style.rs)
                                  │
                                  ▼ ComputedStyle
                           Layout Engine (layout.rs)
                                  │
                                  ▼ LayoutBox Tree (BFC, IFC)
                           Command Builder (command_builder.rs)
                                  │
                                  ▼ RenderCommands (Display List)
                           Batch Builder (batch_builder.rs)
                                  │
                                  ▼ Vertex & Index Buffers
                           WGPU GPU Renderer (renderer.rs)
                                  │
                                  ▼
                           Surface Present (Window)
```

---

## 4. Architectural Evolution: Target Workspace Modularization

Under [`SPEC-05`](specifications/SPEC-05.md) and [`ADR-004`](decisions/ADR-004-workspace-modularization.md), Asteria will transition from its current monolithic crate into a clean 7-crate workspace:

```
               crates/asteria_shell (Window, Event Loop, CLI)
                     │                 │              │
                     ▼                 ▼              ▼
       crates/asteria_render   crates/asteria_paint  crates/asteria_net
                     │                 │              │
                     └────────►────────┼──────◄───────┘
                                       ▼
                             crates/asteria_layout
                                       │
                                       ▼
                             crates/asteria_style
                                       │
                                       ▼
                              crates/asteria_dom
```
