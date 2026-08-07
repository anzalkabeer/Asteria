<div align="center">

# Asteria

**A browser engine built from scratch in Rust.**

<!-- If you have a logo or banner, place it here -->
<!-- ![Asteria Banner](assets/banner.png) -->

[![Rust CI](https://github.com/anzalkabeer/Asteria/actions/workflows/ci.yml/badge.svg)](https://github.com/anzalkabeer/Asteria/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/Rust-2024-b7410e?logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-blue)
![Status](https://img.shields.io/badge/status-active_development-brightgreen)

</div>

---

<!-- 🎬 DEMO GIF — This is the single most important visual on this page. -->
<!-- It should show ~10–20 seconds of: launching Asteria, a page loading,    -->
<!-- scrolling, images rendering, flexbox layouts, window resizing, and       -->
<!-- smooth GPU-rendered frames. Record once the browser is visually mature.  -->

<div align="center">

> 🎬 **[Demo GIF coming soon]**
>
> _A short recording showing Asteria loading a real web page, rendering styled content on the GPU, and resizing the window — all in real time._

</div>

---

## What is Asteria?

Asteria is a web browser engine being built entirely from scratch.

It reads web pages, understands HTML and CSS, figures out where every element belongs on the screen, and paints the result using modern GPU graphics — all without borrowing code from Chrome, Firefox, or any existing browser engine.

Every part of the pipeline — the HTML parser, the CSS engine, the layout solver, the renderer — is hand-written in Rust with no third-party web engine crates. The networking layer, memory allocators, task scheduler, and browser shell are all custom-built too.

Asteria isn't a wrapper around someone else's work. It's the work itself.

---

## Why does this exist?

Web browsers are among the most complex pieces of software ever built. So complex that only a handful of engines power every browser on the planet.

We wanted to understand how it all works — not by reading about it, but by building it. From the first byte of HTML to the last pixel on the screen.

Asteria started as a deep-dive into browser internals. It's growing into something that can actually render real web content with GPU acceleration, tabbed browsing, and live network fetching.

---

## What can Asteria do today?

### 🌐 Web Engine

| Feature        | Status | What it does                                                           |
| -------------- | ------ | ---------------------------------------------------------------------- |
| HTML Parsing   | ✅     | Reads raw HTML and builds a structured document tree                   |
| CSS Parsing    | ✅     | Understands selectors, properties, media queries, and specificity      |
| Style Cascade  | ✅     | Applies CSS rules with proper inheritance, inline styles, and `@media` |
| DOM            | ✅     | Represents the page structure in a fast, memory-efficient arena        |
| Block Layout   | ✅     | Positions block-level elements (headings, paragraphs, divs)            |
| Inline Layout  | ✅     | Flows text and inline elements with line wrapping                      |
| Flexbox        | ✅     | CSS `display: flex` horizontal row layouts with explicit item widths   |
| GPU Rendering  | ✅     | Hardware-accelerated painting via wgpu with shader-based batching      |
| Image Decoding | ✅     | Format detection and decoding pipeline (renders fitted placeholder frames; SVG detected but not rendered) |
| Text Rendering | ✅     | GPU-rendered glyphs with proper font metrics                           |

### 🖥️ Browser

| Feature             | Status | What it does                                               |
| ------------------- | ------ | ---------------------------------------------------------- |
| Tabbed Browsing     | ✅     | Multiple tabs with keyboard shortcuts (`Ctrl+T`, `Ctrl+W`) |
| Navigation History  | ✅     | Back, forward, and reload per tab                          |
| Window Resizing     | ✅     | Live content reflow when you resize the window             |
| Keyboard Navigation | ✅     | Full shortcut support (`Alt+←`, `Alt+→`, `Ctrl+R`, `F5`)   |
| Scrolling           | ✅     | Scroll through content that exceeds the viewport           |

### ⚡ Networking

| Feature          | Status | What it does                                                       |
| ---------------- | ------ | ------------------------------------------------------------------ |
| HTTP/HTTPS       | ✅     | Custom HTTP/1.1 client with TLS, redirects, and connection pooling |
| DNS Resolution   | ✅     | Built-in resolver with TTL caching                                 |
| Streaming HTML   | ✅     | Pages start rendering before the full download completes           |
| Resource Loading | ✅     | Discovers and fetches stylesheets and linked resources             |

### 🔧 Internals

| Feature                  | Status | What it does                                                   |
| ------------------------ | ------ | -------------------------------------------------------------- |
| Multi-threaded Scheduler | ✅     | Worker pool for parallel tasks with panic isolation            |
| String Interner          | ✅     | Efficient string deduplication for fast comparisons            |
| Frame Arena Allocator    | ✅     | Zero-overhead per-frame memory management                      |
| LRU Cache                | ✅     | In-memory resource caching to avoid redundant loads            |
| Engine Profiler          | ✅     | Microsecond-precision timing for every pipeline stage          |
| Devtools & Tracing       | ✅     | Chrome Trace JSON export, memory inspector, energy diagnostics |

### 🚧 Coming Next

| Feature            | Status | What it will do                                    |
| ------------------ | ------ | -------------------------------------------------- |
| `!important` rules | 🔜     | Override specificity for priority CSS declarations |
| CSS Animations     | 🔜     | `@keyframes` and animated transitions              |
| Advanced Flexbox   | 🔜     | Column layouts, wrapping, and alignment options    |
| `@import` support  | 🔜     | External stylesheet inclusion                      |
| JavaScript Engine  | 📋     | Script execution and DOM manipulation              |

---

## Showcase

<!-- Replace these placeholders with real screenshots as Asteria matures.     -->
<!-- Name files descriptively and place them in an assets/ or docs/ folder.   -->

> 📸 **[Screenshots coming soon]**

Screenshots will be added here showing:

| Screenshot             | Description                                                            |
| ---------------------- | ---------------------------------------------------------------------- |
| _Blog article layout_  | A styled page with headers, paragraphs, callouts, and a navigation bar |
| _Flexbox card gallery_ | Cards arranged in a horizontal flex row with images and descriptions   |
| _Window resize reflow_ | Content reflowing live as the window changes size                      |
| _GPU rendering output_ | Hardware-accelerated frames with text, colors, and borders             |
| _Tabbed browsing_      | Multiple tabs open with navigation controls                            |

<!-- Example format once screenshots are available:
<div align="center">
<img src="docs/screenshots/blog_layout.png" width="720" alt="Blog article rendered by Asteria" />
<p><em>A blog article with styled headings, callout boxes, and a navigation bar — rendered entirely by Asteria's GPU pipeline.</em></p>
</div>
-->

---

## How Asteria works

At a high level, Asteria transforms a web page into pixels through a series of steps:

```
     📄 HTML source
          │
          ▼
    ┌─────────────┐
    │   Parsing    │  Read the HTML and build a tree of elements
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐
    │  Structure   │  Organize elements into a document (the DOM)
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐
    │   Styling    │  Apply CSS rules — colors, sizes, spacing
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐
    │   Layout     │  Calculate where every element goes on the screen
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐
    │  Painting    │  Generate a list of drawing instructions
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐
    │     GPU      │  Send everything to the graphics card for rendering
    └─────────────┘
```

Each stage is a separate module. The output of one stage feeds into the next. HTML chunks can be parsed and processed progressively as they arrive from the network, flowing through the engine stages to rendered pixels.

**Want to go deeper?** The documentation covers each stage in detail:

- [Architecture Overview](docs/03-project-architecture.md)
- [Pipeline Deep-Dive](docs/02-how-asteria-works.md)
- [HTML Engine](docs/04-html-engine.md) & [DOM Arena](docs/05-dom.md)
- [CSS Engine & Cascade](docs/06-css-engine.md)
- [Layout Engine](docs/07-layout-engine.md)
- [Painting](docs/08-painting.md) & [GPU Renderer](docs/09-gpu-renderer.md)
- [Resource Loading](docs/10-resource-loading.md) & [Browser Shell](docs/11-browser-shell.md)
- [Detailed Roadmap](docs/13-roadmap.md)

---

## Roadmap

Asteria is under active development. Here's where it's heading:

| Area               | Goal                                                            |
| ------------------ | --------------------------------------------------------------- |
| **CSS**            | Broader property support, animations, transitions, `@import`    |
| **Layout**         | Advanced flexbox, CSS Grid, positioned elements                 |
| **JavaScript**     | Script execution engine and DOM API bindings                    |
| **Networking**     | HTTP/2, WebSockets, better caching strategies                   |
| **Browser UI**     | Address bar, bookmarks, settings, and a polished browser chrome |
| **Standards**      | Progressive compliance with W3C and WHATWG specifications       |
| **Performance**    | Incremental layout, layer compositing, and render optimizations |
| **Cross-platform** | Native builds for Windows, macOS, and Linux                     |

> 📋 **See the [Detailed Roadmap](docs/13-roadmap.md)**

---

## Using Asteria

Asteria is not yet a daily-driver browser. It's an engine in active development.

When it's ready, this section will cover:

- 📦 Downloading releases
- 🚀 Launching the browser
- 🌍 Opening websites
- ⚙️ Configuration and settings

**For now**, you can build and run the engine from source to see it in action:

```bash
# Clone the repository
git clone https://github.com/anzalkabeer/Asteria.git
cd Asteria/Asteria

# Build and run with a sample page
cargo run

# Or point it at your own HTML file
cargo run -- path/to/page.html
```

**Requirements:** [Rust](https://www.rust-lang.org/) (2024 edition)

---

## For developers

Interested in contributing or exploring the codebase?

| Resource           | Link                                                             |
| ------------------ | ---------------------------------------------------------------- |
| Contributing guide | [Contributing Guide](docs/14-contributing.md)                    |
| Architecture docs  | [Documentation Index](docs/00-introduction.md)                  |
| Issue tracker      | [GitHub Issues](https://github.com/anzalkabeer/Asteria/issues)   |
| CI pipeline        | [GitHub Actions](https://github.com/anzalkabeer/Asteria/actions) |

The project is written in Rust (2024 edition) and uses `wgpu` for GPU rendering and `winit` for windowing. All web engine components — parsing, styling, layout, painting — are built from scratch with no third-party browser engine dependencies.

---

## Built by

Asteria is built by Anzal Kabeer ([GitHub Link](https://github.com/anzalkabeer)) and Keshav Ghai ([GitHub Link](https://github.com/Keshav76315)).

---

<div align="center">

**Asteria** — Rendering the web, one pixel at a time.

MIT License

</div>
