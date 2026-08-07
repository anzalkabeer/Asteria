# How Asteria Works

> **Purpose:** Walk through Asteria's complete rendering pipeline — from raw bytes to GPU pixels.
>
> **Audience:** Anyone who has read [How a Browser Works](01-how-a-browser-works.md) and wants to see how Asteria implements it.
>
> **Estimated reading time:** 12 minutes
>
> **Prerequisites:** [How a Browser Works](01-how-a-browser-works.md) (recommended)

---

## The big picture

Asteria transforms web content through a seven-stage pipeline. Each stage has a clearly defined input and output. Data flows in one direction — forward — and no stage reaches backward into a previous one.

```
  ┌──────────────┐     ┌─────────────┐     ┌──────────────┐
  │  HTML Bytes  │────►│  Tokenizer  │────►│    Parser    │
  └──────────────┘     └─────────────┘     └──────┬───────┘
                                                  │
                                                  ▼
  ┌──────────────┐     ┌─────────────┐     ┌──────────────┐
  │  CSS Bytes   │────►│ CSS Tokenizer────►│  CSS Parser  │
  └──────────────┘     └─────────────┘     └──────┬───────┘
                                                  │
                    ┌─────────────────────────────┤
                    │                             │
                    ▼                             ▼
             ┌──────────┐                  ┌────────────┐
             │   DOM    │─────────────────►│   Style    │
             │  (Arena) │                  │  Resolver  │
             └──────────┘                  └─────┬──────┘
                                                 │
                                                 ▼
                                          ┌──────────────┐
                                          │    Layout    │
                                          │   Engine     │
                                          └──────┬───────┘
                                                 │
                                                 ▼
                                          ┌──────────────┐
                                          │    Paint     │
                                          │   Engine     │
                                          └──────┬───────┘
                                                 │
                                                 ▼
                                          ┌──────────────┐
                                          │ Scene Graph  │
                                          └──────┬───────┘
                                                 │
                                                 ▼
                                          ┌──────────────┐
                                          │     GPU      │
                                          │  Renderer    │
                                          └──────────────┘
```

Let's walk through each stage.

---

## Stage 1: HTML tokenization and parsing

**Input:** Raw HTML bytes (from disk or network)
**Output:** A DOM tree stored in an arena

Asteria's HTML tokenizer is a state machine that reads the input byte by byte. As it encounters `<`, `>`, `=`, quotes, and whitespace, it transitions between states and emits tokens — start tags, end tags, text nodes, attributes, and comments.

The tokenizer is **zero-copy**: it stores byte offset pairs into the original input buffer rather than allocating new strings. If the input is `<div class="main">Hello</div>`, the tokenizer records that the tag name runs from byte 1 to byte 4, the attribute name from byte 5 to byte 10, and so on. This avoids thousands of small string allocations on a typical page.

The parser consumes these tokens and constructs a DOM tree. When it sees a start tag, it creates a new node. When it sees text, it adds a text child. When it sees an end tag, it closes the current node. The parser handles implicit tag closing (like `<p>` followed by another `<p>`) and other HTML quirks.

For network-loaded pages, Asteria uses a **streaming parser** that can process HTML chunks as they arrive, building the DOM incrementally rather than waiting for the full download.

> **Deep dives:** [HTML Engine](04-html-engine.md) · [DOM](05-dom.md)

---

## Stage 2: CSS tokenization and parsing

**Input:** CSS source text (from `<style>` tags or linked `.css` files)
**Output:** A structured stylesheet

CSS parsing runs independently of HTML parsing. The resource loader discovers CSS sources — inline `<style>` blocks and external `<link rel="stylesheet">` references — and feeds them to the CSS tokenizer.

The CSS tokenizer produces tokens for identifiers, strings, numbers, symbols, and whitespace. The CSS parser assembles these into a structured stylesheet containing:

- **Rules** — each with one or more selectors and a block of declarations
- **Selectors** — tag, class, ID, universal, pseudo-class, and compound selectors with combinators (descendant, child, sibling)
- **Declarations** — property-value pairs like `color: blue` or `margin: 16px`
- **Media queries** — conditional rules like `@media (min-width: 768px)`

> **Deep dive:** [CSS Engine](06-css-engine.md)

---

## Stage 3: Style resolution

**Input:** DOM tree + stylesheet
**Output:** A styled tree with computed styles on every element

Style resolution is where the DOM and CSS meet. For each element in the DOM tree, Asteria:

1. **Matches selectors** — finds all CSS rules whose selectors match this element
2. **Calculates specificity** — scores each matching rule as (ID count, class count, tag count)
3. **Sorts by cascade priority** — origin (stylesheet vs. inline style), specificity, source order
4. **Picks winners** — for each CSS property, the highest-priority declaration wins
5. **Expands shorthands** — `margin: 16px` becomes four values (top, right, bottom, left)
6. **Inherits** — properties like `color` and `font-size` are copied from the parent if not explicitly set
7. **Computes values** — relative units like `em` and `%` are resolved to absolute pixel values

The result is a parallel **styled tree** where every element carries a `ComputedStyle` — a fully resolved set of CSS properties ready for layout.

> **Deep dive:** [CSS Engine — Cascade & Specificity](06-css-engine.md)

---

## Stage 4: Layout

**Input:** Styled tree
**Output:** A layout tree with precise positions and dimensions

The layout engine is where geometry happens. It takes every styled element and computes its exact position (x, y) and size (width, height), respecting the CSS box model (content, padding, border, margin).

Asteria supports three formatting contexts:

- **Block layout** — elements stack vertically, each taking the full width of their container
- **Inline layout** — elements flow left to right, wrapping at line boundaries
- **Flex layout** — elements are arranged along an axis with flexible sizing (`display: flex`)

The layout algorithm walks the styled tree top-down. For each container, it determines its children's formatting context, calculates available space, positions each child, and reports the container's total height back up to its parent.

When the browser window is resized, the layout engine re-runs with new viewport dimensions, reflowing all content to fit.

The output is a **layout tree** — a tree of boxes where every node has computed `Dimensions` (content rect, padding, border, margin) in document coordinates.

> **Deep dive:** [Layout Engine](07-layout-engine.md)

---

## Stage 5: Painting

**Input:** Layout tree
**Output:** A flat display list of drawing commands

The paint engine walks the layout tree and generates a flat, ordered list of visual instructions. Each instruction is a **display command** — one of:

| Command | What it draws |
|---|---|
| `SolidColor` | A filled rectangle (backgrounds) |
| `Border` | Box borders with per-edge widths |
| `Text` | A text string at a position with font size and colour |
| `Image` | A decoded image mapped to a rectangle |

Commands are emitted in **CSS paint order**: backgrounds first, then borders, then text, then child content. This ensures correct visual stacking.

The display list is flat — no tree structure, no hierarchy. It's a simple sequence of "draw this thing at this position." This makes it easy to hand off to the GPU.

> **Deep dive:** [Painting](08-painting.md)

---

## Stage 6: Scene graph

**Input:** Display list
**Output:** A GPU-optimized scene graph

Before reaching the GPU, the display list is converted into a **scene graph** — a flat, data-oriented representation optimised for rendering. Scene nodes are stored in a contiguous `Vec<SceneNode>` for maximum cache locality.

Each scene node carries:

- A bounding rectangle in document coordinates
- A visual type (solid rect, border, text, image)
- A paint stacking order (z-order)
- A segment ID for region-based GPU tile caching
- A dirty flag for incremental re-rendering
- Interactive state tracking (normal, hovered, active)

The scene graph is the bridge between the paint engine's logical commands and the GPU renderer's physical vertex buffers.

---

## Stage 7: GPU rendering

**Input:** Scene graph
**Output:** Pixels on screen

Asteria renders everything on the GPU using [wgpu](https://wgpu.rs/) — a cross-platform graphics API based on the WebGPU standard. The windowing is handled by [winit](https://docs.rs/winit/), which provides native OS windows and event handling.

The GPU renderer works in passes:

1. **Rect pass** — draws all solid-colour backgrounds and borders using batched quad vertices
2. **Image pass** — draws decoded images mapped to texture quads
3. **Text pass** — draws text glyphs rendered through the glyphon text engine

Each pass prepares vertex buffers, uploads them to the GPU, and executes a render pipeline with WGSL shaders. Passes are managed by a **render graph** that coordinates pipeline state and draw ordering.

GPU pipelines are created once at startup. Only the vertex data changes per frame, minimising GPU state transitions.

> **Deep dive:** [GPU Renderer](09-gpu-renderer.md)

---

## The event loop

After the initial render, Asteria enters an event loop powered by winit. It responds to:

| Event | Response |
|---|---|
| **Window resize** | Re-run layout with new dimensions, rebuild scene, re-render |
| **Mouse move** | Hit-test against scene nodes, update hover state |
| **Mouse click** | Hit-test, update active state, navigate if link clicked |
| **Scroll** | Adjust scroll offset, rebuild batches, re-render |
| **Keyboard shortcuts** | Tab management (Ctrl+T, Ctrl+W), navigation (Alt+←, Alt+→), reload (Ctrl+R, F5) |

The event loop never blocks. Async tasks run on a separate thread pool managed by Asteria's scheduler.

---

## Supporting systems

Behind the main pipeline, several infrastructure systems keep everything running:

| System | Purpose |
|---|---|
| **Resource Loader** | Discovers and fetches HTML, CSS, and linked resources from disk or network |
| **Networking Stack** | Custom HTTP/1.1 + TLS client with DNS caching and connection pooling |
| **Streaming Parser** | Progressive HTML parsing — rendering starts before the full download completes |
| **String Interner** | Deduplicates common strings (tag names, CSS properties) into 4-byte integer handles |
| **Frame Arena** | Bump allocator for zero-overhead per-frame memory allocation |
| **LRU Cache** | In-memory resource caching to avoid redundant disk or network loads |
| **Task Scheduler** | Multi-threaded worker pool with priority queues and panic isolation |
| **Engine Profiler** | Microsecond-precision timing for every pipeline stage |
| **Devtools** | Chrome Trace export, memory inspection, and energy diagnostics |

> **Deep dives:** [Resource Loading](10-resource-loading.md) · [Browser Shell](11-browser-shell.md) · [Performance](12-performance.md)

---

## What's next

- [Project Architecture](03-project-architecture.md) — the repository structure and module layout
- [Roadmap](13-roadmap.md) — where Asteria is heading
- [Design Decisions](18-design-decisions.md) — why the architecture looks this way
