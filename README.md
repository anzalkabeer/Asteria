# Asteria Rendering Engine 🌠

Asteria is a modular, high-performance web rendering engine written from scratch in Rust.

Most modern software sits on top of massive abstraction towers. Web browsers in particular are among the most complex engineering feats on the planet—so much so that building one from scratch is often considered crazy. We built Asteria to take the cover off the black box: parsing raw bytes of HTML and CSS, resolving cascade rules and selectors, computing 2D geometry layout boxes, generating display lists, and rendering hardware-accelerated GPU graphics—all using a hand-crafted core engine.

---

## 💡 The Philosophy

1. **Custom Web Core**: No third-party web crates or browser engines. Everything from the HTML state-machine tokenizer and DOM arena to CSS selector matching, layout formatting contexts, and display list paint generation is hand-crafted in Rust.
2. **Arena-Allocated DOM**: Instead of a maze of heap pointers (`Box`, `Rc`, `RefCell`), all DOM nodes live in a single `Vec<Node>`. Parent-child relationships use lightweight `NodeId(u32)` handles for cache locality and memory safety without reference-counting overhead.
3. **Zero-Copy Tokenization**: HTML and CSS tokenizers store start/end byte offset pairs into the original input buffer rather than copying text strings during parsing.
4. **Clean Pipeline Separation**: The DOM stays completely immutable after parsing. Style resolution builds a parallel `StyledNode` tree, which feeds into a distinct `LayoutBox` geometry tree, display list generator, and `SceneGraph`.
5. **Hardware Acceleration**: 2D layout boxes and glyphs are batched and rendered using `wgpu` hardware pipelines and native `winit` windowing.

---

## 🏗️ Architecture & Pipeline

Asteria transforms web content through a multi-stage pipeline:

```
HTML Bytes ──► Tokenizer ──► Parser ──► DOM Tree (Arena allocated)
                                             │
CSS Bytes  ──► Tokenizer ──► Parser ──► Stylesheet & @media Rules
                                             │
                         DOM + Stylesheet ──► Style & Cascade Engine (Combinators, @media viewport)
                                                     │
                                       Styled DOM ──► Layout Engine (2D Rects & Box Model)
                                                     │
                                      Layout Tree ──► Display List Paint Generator
                                                     │
                                     Display List ──► SceneGraph & Batch Planner
                                                     │
                                      Scene Graph ──► Hardware GPU Renderer (wgpu & winit loop)
```

### Modular System Split

The project is developed across two distinct engineering tracks:

- **Engine Core & Runtime** (_Anzal Kabeer_): HTML tokenizer & parser, DOM tree, CSS tokenizer/parser, style resolution, block/inline layout algorithms, paint display list generator, and `wgpu` GPU rendering pipeline.
- **Architecture & Infrastructure** (_Keshav Ghai_): Multi-threaded task scheduler, resource loader & cache, string interner, frame arena memory model, browser shell & multi-tab manager, engine profiler, devtools, and windowing integration.

---

## 🚀 Current Progress

Here is where Asteria stands:

- **HTML Parser & DOM Arena**: Zero-copy state-machine tokenizer and parser constructing an arena-allocated DOM with visualization tooling.
- **Advanced CSS Engine & Selectors**: Full tokenizer and stylesheet parser supporting tag, class, ID, universal, compound, descendant, child (`>`), next sibling (`+`), subsequent sibling (`~`), pseudo-classes (`:first-child`, `:last-child`, `:hover`), and `@media` queries (`min-width`, `max-width`). Implements specificity scoring `(ID, Class, Tag)`, property inheritance (`color`, `font-size`), shorthand expansion (`margin`, `padding`), inline `style=""` precedence, typed `ComputedStyle`, and live `@media` viewport evaluation.
- **2D Layout Engine**: Geometry solver featuring Block Formatting Contexts (auto-width expansion, margin centering), Inline Formatting Contexts (character metrics & line wrapping), and anonymous block box generation for mixed children.
- **Paint & Display List Engine**: `paint.rs` flat display list builder (`SolidColor`, `Border`, `Text`, `Image`) ordered by CSS paint hierarchy and z-index.
- **GPU Renderer & Native Windowing**: Hardware-accelerated `wgpu` rendering backend, WGSL shaders, vertex quad batching, pass scheduler (`RectPass`, `TextPass`), and `winit` native OS window event loop (`AsteriaWindow`).
- **Interactive Browser Shell**: `TabManager` (`shell.rs`) managing multi-tab operations, per-tab `NavigationHistory` back/forward/reload stacks, `ShellEvent` dispatcher, interactive keyboard navigation (`Ctrl+T`, `Ctrl+W`, `Alt+Left`, `Alt+Right`, `Ctrl+R`, `F5`), and dynamic content reflow on window resize.
- **Multi-Threaded Scheduler**: Multi-threaded worker thread pool (`scheduler.rs`) using `std::thread` and `std::sync::mpsc` channels with panic isolation.
- **Resource Loader & Cache**: Resource management system (`loader.rs`) with in-memory caching, discovering `<style>` blocks and external `<link rel="stylesheet">` files with path resolution.
- **String Interner**: High-performance string interner (`interner.rs`) using `Rc<str>` for 4-byte `Symbol(u32)` handles, pre-seeded with 67 standard HTML tags, attributes, and CSS properties.
- **Engine Profiler & Devtools**: `EngineProfiler` (`profiler.rs`) with RAII `StageGuard` microsecond timing per pipeline stage, Asteria Observability Framework (AOF) with Chrome Trace JSON exporter, memory inspector, and energy impact diagnostics.

---

## 🛠️ Getting Started

### Prerequisites

- [Rust 2024 edition](https://www.rust-lang.org/)

### Building & Running

Clone the repository and run the CLI inspector:

```bash
# Clone the repository
git clone https://github.com/anzalkabeer/Asteria.git
cd Asteria/Asteria

# Check compilation
cargo check

# Run the full pipeline CLI demo with built-in sample HTML+CSS
cargo run

# Run the pipeline on a custom HTML file from disk
cargo run -- path/to/index.html
```

---

## 🧪 Testing Process & Visual Fixtures

Asteria includes both automated unit/integration tests and visual HTML/CSS test fixtures to verify layout, styling, and rendering fidelity:

### 1. Automated Test Suite

Run the full Rust test suite covering DOM parsing, CSS cascade, layout box calculation, and image decoding:

```bash
# Run all unit and integration tests
cargo test

# Run specific test suites
cargo test --test style_integration
cargo test --test layout_integration
cargo test --test image_integration
```

### 2. Visual HTML/CSS Test Fixtures

Test live hardware rendering (`wgpu`), window reflow, image frames, and flexbox card grid layouts using built-in test fixtures:

```bash
# Test Image Frames, CSS Flexbox Row Layout & Card Wrapping
cargo run -- tests/fixtures/gallery.html

# Test Multi-Column Article Layouts, Headers & Stylesheets
cargo run -- tests/fixtures/blog.html
```

---

## 📄 License

This project is open-source under the MIT License.
