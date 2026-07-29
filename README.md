# Asteria Rendering Engine 🌠

_(Codename: Antigravity)_

Asteria is a modular, zero-dependency web rendering engine written entirely from scratch in Rust.

Most modern software sits on top of massive abstraction towers. Web browsers in particular are among the most complex engineering feats on the planet—so much so that building one from scratch is often considered crazy. We built Asteria to take the cover off the black box: parsing raw bytes of HTML and CSS, resolving cascade rules, computing geometry, and placing 2D layout boxes on a coordinate space, all using only the Rust standard library.

---

## 💡 The Philosophy

1. **Zero External Dependencies**: No crates, no third-party parsers, no shortcuts. Everything from the state-machine HTML tokenizer to the CSS selector cascade is hand-crafted in Rust.
2. **Arena-Allocated DOM**: Instead of a maze of heap pointers (`Box`, `Rc`, `RefCell`), all DOM nodes live in a single `Vec<Node>`. Parent-child relationships use lightweight `NodeId(u32)` handles for cache locality and memory safety without reference-counting overhead.
3. **Zero-Copy Tokenization**: HTML and CSS tokens store start/end byte offset pairs into the original input buffer rather than copying text strings during parsing.
4. **Clean Pipeline Separation**: The DOM stays completely immutable after parsing. Style resolution builds a parallel `StyledNode` tree, which feeds into a distinct `LayoutBox` geometry tree.

---

## 🏗️ Architecture & Pipeline

Asteria transforms web content through a multi-stage pipeline:

```
HTML Bytes ──► Tokenizer ──► Parser ──► DOM Tree (Arena allocated)
                                             │
CSS Bytes  ──► Tokenizer ──► Parser ──► Stylesheet
                                             │
                         DOM + Stylesheet ──► Style & Cascade Engine (Typed ComputedStyle)
                                                     │
                                       Styled DOM ──► Layout Engine (2D Rects & Box Model)
                                                     │
                                       Layout Tree ──► [Paint & Render Engine (wgpu)]
```

### Modular System Split

The project is developed across two distinct engineering tracks:

- **Engine Core & Runtime** (_Anzal Kabeer_): HTML tokenizer & parser, DOM tree, CSS tokenizer/parser, style resolution, block/inline layout algorithms, paint display list, and wgpu rendering.
- **Architecture & Infrastructure** (_Keshav Ghai_): Multi-threaded task scheduler, resource loader & cache, string interner, memory optimization, browser shell, and devtools overlays.

---

## 🚀 Current Progress

Here is where Asteria stands:

- **HTML Parser & DOM**: Fully functional state-machine tokenizer and parser constructing an arena DOM with tree visualization tooling.
- **CSS Engine & Specificity**: Complete tokenization and stylesheet parser supporting tag, class, ID, universal, compound, and descendant selectors. Implements specificity scoring `(ID, Class, Tag)`, property inheritance (`color`, `font-size`), shorthand expansion (`margin`, `padding`), and typed `ComputedStyle`.
- **Layout Engine**: 2D geometry solver featuring Block Formatting Contexts (auto-width expansion, margin centering), Inline Formatting Contexts (character-width metrics & line wrapping), and anonymous block box generation for mixed children.
- **Resource Loader & Cache**: Standalone resource management system (`loader.rs`) with in-memory caching, discovering `<style>` blocks and external `<link rel="stylesheet">` files with relative path resolution.
- **String Interner**: High-performance bidirectional string interner (`interner.rs`) using `Rc<str>` for 4-byte `Symbol(u32)` handles, pre-seeded with 67 standard HTML tags, attributes, and CSS properties.

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

# Check compilation (0 warnings)
cargo check

# Run the full pipeline CLI demo with built-in sample HTML+CSS
cargo run

# Run the pipeline on a custom HTML file from disk
cargo run -- path/to/index.html
```

---

## 📄 License

This project is open-source under the MIT License.
