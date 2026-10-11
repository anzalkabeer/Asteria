# ASTERIA ARCHITECTURAL DIAGRAMS: REMEDIATED WORKSPACE ARCHITECTURE

## 1. Modular Multi-Crate Architecture (`SPEC-05`)

The diagram below illustrates the target 7-crate workspace structure, eliminating the monolith (`AST-ARC-001`) and enforcing strict compile-time dependency boundaries.

```mermaid
graph TD
    classDef shell fill:#e1f5fe,stroke:#0288d1,stroke-width:2px;
    classDef render fill:#ede7f6,stroke:#512da8,stroke-width:2px;
    classDef dom fill:#e8f5e9,stroke:#388e3c,stroke-width:2px;
    classDef net fill:#fff3e0,stroke:#f57c00,stroke-width:2px;

    Shell["crates/asteria_shell<br/>(Winit, CLI, Window, EventLoop)"]:::shell
    Render["crates/asteria_render<br/>(WGPU 0.20, WGSL, Textures, Shaders)"]:::render
    Paint["crates/asteria_paint<br/>(DisplayList, GlyphAtlas, CommandBuilder)"]:::render
    Layout["crates/asteria_layout<br/>(BoxTree, TaffyAdapter, InlineFormatting)"]:::dom
    Style["crates/asteria_style<br/>(CSS Parser, Cascade, SelectorMatching)"]:::dom
    Dom["crates/asteria_dom<br/>(HTML5 Parser, DOM NodeTree, Arena)"]:::dom
    Net["crates/asteria_net<br/>(HTTP Client, URL, Fetch, ResourceCache)"]:::net

    Shell --> Render
    Shell --> Paint
    Shell --> Layout
    Shell --> Net

    Render --> Paint
    Paint --> Layout
    Layout --> Style
    Layout --> Dom
    Style --> Dom
    Net --> Dom
```

---

## 2. Dependency Flow Rules
1. **Zero Circular Dependencies:** Downstream crates (e.g. `asteria_dom`, `asteria_net`) cannot import upstream presentation crates (`asteria_render`, `asteria_shell`).
2. **Headless Testability:** `asteria_dom`, `asteria_style`, `asteria_layout`, and `asteria_paint` can be compiled and unit tested without linking WGPU, graphics drivers, or windowing dependencies.
3. **Parallel Crate Compilation:** Rust's `cargo` builds independent crates in parallel, cutting workspace build times by ~60%.
