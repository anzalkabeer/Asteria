# 05 Remediation Specifications & Engineering Blueprints

> **Document Identifier:** `AST-DOC-005`  
> **Audited Repository:** Asteria Browser Engine (`Asteria/Asteria/`)  
> **Audited Git Revision:** `3cdd5155caf544794c3d11f45dfc839665d9759b`  
> **Purpose:** Detailed, implementation-ready engineering specifications for each confirmed defect and architectural weakness identified in `03_FINDINGS_AND_ROOT_CAUSES.md`.  

---

## 1. Remediation Specification Index

| Spec ID | Target Finding | Subsystem | Engineering Scope | Complexity |
| :--- | :--- | :--- | :--- | :--- |
| **`SPEC-01`** | `AST-DEF-001`, `AST-SEC-001` | Networking | RFC 3986 URL parsing, query/hash splitting, and header CRLF injection guard. | Low |
| **`SPEC-02`** | `AST-DEF-002` | Paint & Scene | Structural parent tracking in display list and scene graph; elimination of containment heuristic. | Medium |
| **`SPEC-03`** | `AST-DEF-003` | Layout Engine | Text intrinsic measurement via `glyphon::FontSystem` / font metrics integration. | Medium |
| **`SPEC-04`** | `AST-DEF-004` | Loader | Single-pass HTML parse and DOM transfer in `ResourceLoader::load_file`. | Low |
| **`SPEC-05`** | `AST-ARC-001` | Architecture | 7-crate Cargo workspace modularization (`asteria_dom`, `asteria_style`, etc.). | High |
| **`SPEC-06`** | `AST-ARC-002` | Scheduler | Unified priority-aware task queue connecting `TaskScheduler` to worker pool. | Medium |
| **`SPEC-07`** | `AST-ARC-003` | Layout Engine | Decoupling `LayoutBox` lifetime from `StyledNode` via index handles (`StyledNodeId`). | High |
| **`SPEC-08`** | `AST-PERF-001` | Renderer | Retained-mode GPU viewport scrolling via shader uniforms; elimination of per-frame vertex rebuilds. | Medium |
| **`SPEC-09`** | `AST-PERF-002`, `AST-PERF-003`| Style & Memory | Activation of `Symbol` interner and `Arc<HashMap>` sharing for CSS custom properties. | Medium |
| **`SPEC-10`** | `AST-002` | Layout Engine | Delegation of CSS Flexbox and Grid constraint solving to `taffy 0.14`. | High |
| **`SPEC-11`** | `AST-INC-001` | Image & GPU | Real image decoding (`image` crate), GPU texture cache, and active `ImagePass` drawing. | Medium |
| **`SPEC-12`** | `AST-ARC-004`, `AST-006` | Shell & UI | Decoupling shell chrome into an isolated overlay pass, separating page and UI vertex buffers. | Medium |

---

## 2. Technical Remediation Specifications

---

### SPEC-01: RFC 3986 URL Parsing & Header CRLF Sanitization

- **Target Findings:** `AST-DEF-001`, `AST-SEC-001`
- **Affected Files:**
  - *Confirmed:* `src/net/http.rs`
  - *Candidates:* `src/loader.rs`, `src/shell.rs`

#### 1. Problem Statement & Root Cause
`Url::parse` splits host from path at the first `/` character without inspecting `?` (query parameters) or `#` (fragment identifiers). When serializing an HTTP request (`HttpRequest::to_bytes`), fragments like `#section` are sent over the wire, causing standard HTTP/1.1 servers to return `400 Bad Request`. Furthermore, `HttpRequest::to_bytes` formats user-controlled header keys and values into the stream without checking for `\r` or `\n`, exposing the client to CRLF header injection.

#### 2. Desired Outcome
- `Url` decomposes URLs into `scheme`, `host`, `port`, `path`, `query: Option<String>`, and `fragment: Option<String>`.
- Fragment identifiers are strictly excluded from HTTP request lines.
- HTTP header keys and values containing `\r`, `\n`, or `\0` are rejected with `NetworkError::InvalidHeader`.

#### 3. Recommended Solution
```rust
// Proposed Url struct in src/net/http.rs:13-21
#[derive(Debug, Clone, PartialEq)]
pub struct Url {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub query: Option<String>,
    pub fragment: Option<String>,
    pub raw: String,
}

impl Url {
    pub fn request_target(&self) -> String {
        match &self.query {
            Some(q) => format!("{}?{}", self.path, q),
            None => self.path.clone(),
        }
    }
}
```
In `HttpRequest::to_bytes`:
```rust
let mut req = format!("{} {} HTTP/1.1\r\n", self.method, self.url.request_target());
for (k, v) in &self.headers {
    if k.contains(|c: char| c == '\r' || c == '\n' || c == '\0') 
        || v.contains(|c: char| c == '\r' || c == '\n' || c == '\0') {
        return Err(NetworkError::InvalidHeader(format!("CRLF injection in header '{}'", k)));
    }
    req.push_str(&format!("{}: {}\r\n", k, v));
}
```

#### 4. Implementation Steps
1. Update `Url` struct with `query` and `fragment` fields.
2. In `Url::parse`, split the path component on `#` first (extracting `fragment`), then on `?` (extracting `query`).
3. Add `Url::request_target(&self)` method.
4. Update `HttpRequest::to_bytes` to use `request_target()` and validate headers against CRLF.
5. Add unit tests for `#hash` stripping, `?query` preservation, and CRLF rejection.

#### 5. Verification & Completion Criteria
- Unit tests in `tests/network_integration.rs` verify that:
  - `Url::parse("http://example.com/page?q=1#sec")` sets `path: "/page"`, `query: Some("q=1")`, `fragment: Some("sec")`.
  - `HttpRequest::to_bytes()` outputs `GET /page?q=1 HTTP/1.1` (no `#sec`).
  - Attempting to add a header with value `"val\r\nInjected: true"` returns `Err(NetworkError::InvalidHeader)`.

---

### SPEC-02: Structural Parent Tracking in Display List & Scene Graph

- **Target Finding:** `AST-DEF-002`
- **Affected Files:**
  - *Confirmed:* `src/paint.rs`, `src/scene.rs`
  - *Candidates:* `src/layout.rs`

#### 1. Problem Statement & Root Cause
In `src/scene.rs:360-372`, parent-child relationships are reconstructed on a flat 1D stream of display commands using a geometric bounding box containment heuristic (`rect_contains`). Transparent containers emit no background rect and are omitted from `parent_stack`, while overflowing children cause premature popping of the parent stack, breaking `SceneGraph::invalidate()` dirty propagation.

#### 2. Desired Outcome
Every scene node receives an explicit, deterministic `parent: Option<SceneNodeId>` reflecting the true structural layout hierarchy, regardless of background visibility or overflow boundaries.

#### 3. Recommended Solution
1. Add an explicit identifier `box_id: u32` to each `LayoutBox`.
2. Wrap nested layout box painting in `src/paint.rs` with scope commands: `DisplayCommand::BeginBox { box_id: u32 }` and `DisplayCommand::EndBox`.
3. In `src/scene.rs::build_scene_graph`, maintain an explicit `parent_node_stack: Vec<SceneNodeId>` driven by `BeginBox`/`EndBox`:
```rust
match cmd {
    DisplayCommand::BeginBox { box_id } => {
        // Push container scene node or record current parent
        let id = scene.push(SceneNode {
            rect: Rect::default(),
            kind: SceneNodeKind::Container,
            parent: parent_node_stack.last().copied(),
            ...
        }, [0.0; 4], None);
        parent_node_stack.push(id);
    }
    DisplayCommand::EndBox => {
        parent_node_stack.pop();
    }
    ...
}
```

#### 4. Implementation Steps
1. Add `DisplayCommand::BeginBox` and `DisplayCommand::EndBox` to `src/paint.rs`.
2. Update `render_stacking_context` and `render_layout_box` in `src/paint.rs` to emit `BeginBox` before painting children and `EndBox` afterwards.
3. Replace the geometric heuristic in `src/scene.rs:360-372` with the deterministic `parent_node_stack`.
4. Update `tests/paint_integration.rs` to verify that transparent containers correctly parent their child nodes.

#### 5. Verification & Completion Criteria
- In a document with `<div id="parent" style="background: transparent"><div id="child" style="background: red"></div></div>`, `scene.nodes[child_idx].parent` is `Some(parent_idx)`.
- `SceneGraph::invalidate(child_idx)` marks `parent_idx.dirty = true` as verified in `tests/paint_integration.rs`.

---

### SPEC-03: Font Glyph Metrics Integration into Layout Intrinsic Text Sizing

- **Target Finding:** `AST-DEF-003`
- **Affected Files:**
  - *Confirmed:* `src/layout.rs`
  - *Candidates:* `src/renderer/passes/text_pass.rs`, `src/frame.rs`

#### 1. Problem Statement & Root Cause
`src/layout.rs:2534-2558` measures text during layout using a handwritten character lookup table (`estimate_char_width_ratio`), while `TextPass` renders text on the GPU using `glyphon` / `cosmic-text`. Because estimated ratios diverge from true font metrics, text line breaks and box dimensions mismatch actual rendered glyphs.

#### 2. Desired Outcome
`compute_intrinsic_inline_width` queries real glyph advances from the active font system, ensuring that layout box dimensions match GPU-rendered text exactly.

#### 3. Recommended Solution
1. Introduce a lightweight `FontContext` trait or struct:
```rust
pub struct FontContext {
    pub font_system: glyphon::FontSystem,
}

impl FontContext {
    pub fn measure_text(&mut self, text: &str, font_size: f32, font_family: &[String]) -> f32 {
        // Shapes text with cosmic-text / glyphon and returns exact advance width in pixels
        let mut buffer = glyphon::Buffer::new(&mut self.font_system, glyphon::Metrics::new(font_size, font_size * 1.2));
        buffer.set_text(&mut self.font_system, text, glyphon::Attrs::new(), glyphon::Shaping::Advanced);
        buffer.layout_runs().map(|run| run.line_w).fold(0.0, f32::max)
    }
}
```
2. Pass `Option<&mut FontContext>` into `layout_document`. If present, use `measure_text`; if absent (e.g. lightweight unit tests), fall back to `estimate_char_width_ratio`.

#### 4. Implementation Steps
1. Add `FontContext` helper wrapping `glyphon::FontSystem` in `src/layout.rs` (or a dedicated `asteria_text` module).
2. Update `compute_intrinsic_inline_width` to use `font_ctx.measure_text()`.
3. Wire `FontContext` from `AsteriaWindow` into `build_active_scene`.
4. Add integration test verifying that monospace text (`font-family: monospace`) measures uniform character advances.

#### 5. Verification & Completion Criteria
- For a string `"iiii"` vs `"WWWW"` in a monospace font, computed intrinsic widths are identical ($4 \times \text{advance}$).
- Visual tests confirm that paragraph text on `tests/fixtures/blog.html` wraps without words clipping into card borders.

---

### SPEC-04: Single-Pass HTML Parsing in ResourceLoader

- **Target Finding:** `AST-DEF-004`
- **Affected Files:**
  - *Confirmed:* `src/loader.rs`, `src/main.rs`, `src/renderer/window/window.rs`

#### 1. Problem Statement & Root Cause
`ResourceLoader::load_file` parses the HTML document into a `Dom` solely to discover `<link>` and `<style>` tags, and then discards the `Dom`. The caller immediately re-parses the identical HTML bytes through `StreamingHtmlProcessor`.

#### 2. Desired Outcome
The HTML document is parsed exactly once. `PageResources` carries the parsed `Dom` directly to the downstream pipeline.

#### 3. Recommended Solution
```rust
// In src/loader.rs:53-61
pub struct PageResources {
    pub html: Resource,
    pub stylesheets: Vec<Resource>,
    pub dom: Option<Dom>, // Carries the completed DOM tree
}
```
In `ResourceLoader::load_file`:
```rust
let mut processor = crate::streaming_parser::StreamingHtmlProcessor::new();
let _ = processor.receive_network_chunk(&html.bytes, true);
let dom = processor.finish();
let stylesheets = self.discover_stylesheets(&dom, &html.bytes, base_dir.as_deref(), None)?;

Ok(PageResources {
    html,
    stylesheets,
    dom: Some(dom),
})
```
In `src/main.rs` and `src/renderer/window/window.rs`:
```rust
let dom = if let Some(resources) = &mut active_tab.page_resources
    && let Some(d) = resources.dom.take() {
    d
} else {
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    processor.finish()
};
```

#### 4. Implementation Steps
1. Add `pub dom: Option<Dom>` to `PageResources`.
2. In `ResourceLoader::load_file`, assign `dom: Some(dom)`.
3. In `main.rs` and `window.rs`, reuse `resources.dom` if available.
4. Verify that total page load time in `EngineProfiler` drops by the duration of the redundant parse stage.

#### 5. Verification & Completion Criteria
- Profiler output confirms `EngineStage::ParseHtml` executes once per page load.
- All existing tests in `tests/layout_integration.rs` pass unchanged.

---

### SPEC-05: Cargo Workspace Modularization (7-Crate Architecture)

- **Target Finding:** `AST-ARC-001`
- **Detailed Blueprint:** See [`specifications/SPEC_05_WORKSPACE_MODULARIZATION.md`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/ASTERIA_IMPROVEMENT_AUDIT/specifications/SPEC_05_WORKSPACE_MODULARIZATION.md).
- **Executive Summary:**
  Partition Asteria into a clean Cargo workspace:
  1. `crates/asteria_dom`: Tokens, HTML tokenizer, HTML parser, DOM arena (`Vec<Node>`).
  2. `crates/asteria_style`: CSS tokenizer, CSS parser, properties (62 IDs), values (`ComputedStyle`), cascade resolver.
  3. `crates/asteria_layout`: Geometry (`Rect`, `Dimensions`), box model, formatting contexts (BFC, IFC, Taffy adapter).
  4. `crates/asteria_paint`: Display commands, display list, scene graph (SoA), spatial segments.
  5. `crates/asteria_render`: `wgpu` backend, render graph, passes (`RectPass`, `TextPass`, `ImagePass`), compositor.
  6. `crates/asteria_net`: DNS resolver, TCP connection pool, TLS wrapper, HTTP/1.1 client, resource loader.
  7. `crates/asteria_shell`: Tab manager, navigation history, shell UI state, `winit` window loop, binary executable.
- **Benefits:**
  - Fast parallel incremental compilation.
  - Headless layout and rendering unit tests without compiling `winit`.
  - Zero circular dependencies enforced by Cargo compilation graph.

---

### SPEC-06: Unified Priority-Aware Task Scheduling

- **Target Finding:** `AST-ARC-002`
- **Affected Files:**
  - *Confirmed:* `src/scheduler.rs`
  - *Candidates:* `src/main.rs`, `src/renderer/window/window.rs`

#### 1. Problem Statement & Root Cause
`TaskScheduler` implements a 5-bucket priority queue (`TaskPriority::Idle` to `Critical`), but is only invoked in unit tests. `ThreadedScheduler` uses a plain FIFO `mpsc` channel. Tasks cannot be prioritized, and background jobs delay critical render tasks.

#### 2. Desired Outcome
Worker threads pull jobs in priority order: user-initiated navigation and frame layout jobs take precedence over background resource prefetching.

#### 3. Recommended Solution
Replace the FIFO `mpsc::channel<WorkerJob>` in `ThreadedScheduler` with a thread-safe priority queue shared across workers:
```rust
struct SharedPriorityQueue {
    scheduler: Mutex<TaskScheduler>,
    condvar: Condvar,
    shutdown: AtomicBool,
}
```
When `ThreadedScheduler::schedule(priority, stage)` is called, it pushes the task into the designated bucket in `TaskScheduler` and notifies one worker thread via `condvar`. Worker threads sleep on `condvar` and wake to pop the highest-priority task available ($O(1)$ across 5 buckets).

#### 4. Implementation Steps
1. Refactor `WorkerJob` to include `priority: TaskPriority`.
2. Wrap `TaskScheduler` in `Arc<(Mutex<TaskScheduler>, Condvar)>`.
3. Update worker threads in `ThreadedScheduler::new` to block on `condvar.wait()` and poll `scheduler.lock().unwrap().poll()`.
4. Update `schedule` signature to accept `priority: TaskPriority`.
5. Add unit test verifying that a `Critical` task dispatched after a `Low` task executes first.

#### 5. Verification & Completion Criteria
- Unit tests verify that high-priority tasks preempt low-priority queue backlog.
- No thread hangs or deadlocks occur during graceful `shutdown()`.

---

### SPEC-07: Decoupling `LayoutBox` Lifetimes from `StyledNode`

- **Target Finding:** `AST-ARC-003`
- **Affected Files:**
  - *Confirmed:* `src/layout.rs`
  - *Candidates:* `src/style.rs`, `src/paint.rs`, `tests/layout_integration.rs`

#### 1. Problem Statement & Root Cause
`LayoutBox<'a>` borrows `&'a StyledNode`. The lifetime parameter `'a` infects the entire layout tree and all downstream paint functions, preventing layout caching and requiring cumbersome multi-variable lifetime anchors in tests.

#### 2. Desired Outcome
`LayoutBox` is an owned, self-contained struct without lifetime parameters (`LayoutBox`), referencing styled nodes via an integer index `StyledNodeId` or `NodeId`.

#### 3. Recommended Solution
```rust
// Proposed LayoutBox in src/layout.rs:94-100
#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub dimensions: Dimensions,
    pub box_type: BoxType,
    pub node_id: Option<NodeId>,
    pub style: ComputedStyle, // Or Arc<ComputedStyle>
    pub children: Vec<LayoutBox>,
}
```
Alternatively, store styles in an arena `Vec<ComputedStyle>` and reference them via `style_id: u32`.

#### 4. Implementation Steps
1. Remove lifetime `'a` from `LayoutBox<'a>`.
2. Store `style: Arc<ComputedStyle>` (from `SPEC-09`) directly on `LayoutBox` or store `node_id: Option<NodeId>`.
3. Update `build_layout_tree` to populate owned `LayoutBox`.
4. Update `src/paint.rs` to read styles from `layout_box.style`.
5. Simplify test helpers in `tests/layout_integration.rs`, removing external `mut Option<StyledNode>` store anchors.

#### 5. Verification & Completion Criteria
- `tests/layout_integration.rs` compiles without requiring lifetime arguments on `parse_and_layout`.
- Layout trees can be cached across frames in `Tab`.

---

### SPEC-08: Retained-Mode GPU Viewport Scrolling & Uniform-Based Transform Pipeline

- **Target Finding:** `AST-PERF-001`
- **Detailed Blueprint:** See [`specifications/SPEC_08_RETAINED_GPU_SCROLLING.md`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/ASTERIA_IMPROVEMENT_AUDIT/specifications/SPEC_08_RETAINED_GPU_SCROLLING.md).
- **Executive Summary:**
  - Introduce a dedicated GPU uniform buffer `ViewportUniforms { scroll_offset: [f32; 2], viewport_size: [f32; 2] }` bound to `RectPass` and `TextPass`.
  - In WGSL shaders (`shader.wgsl`), offset vertex screen positions dynamically:
    ```wgsl
    out.position = vec4<f32>(
        (in.position.x - uniforms.scroll_offset.x) / (uniforms.viewport_size.x * 0.5) - 1.0,
        1.0 - (in.position.y - uniforms.scroll_offset.y) / (uniforms.viewport_size.y * 0.5),
        0.0, 1.0
    );
    ```
  - When the user scrolls with the mouse wheel, update **only the 16-byte uniform buffer on the GPU queue**. Do not regenerate vertex arrays, do not repopulate text runs, and do not re-walk the scene graph.
- **Benefits:**
  - 60+ FPS smooth scrolling on all pages.
  - Near-zero CPU utilization during mouse wheel and drag scrolling.

---

### SPEC-09: String Interner Activation and Custom Property Sharing

- **Target Finding:** `AST-PERF-002`, `AST-PERF-003`
- **Affected Files:**
  - *Confirmed:* `src/interner.rs`, `src/dom.rs`, `src/style.rs`, `src/values.rs`

#### 1. Problem Statement & Root Cause
Asteria contains a complete string interner (`src/interner.rs`) with 140+ pre-seeded constants (`SYM_HTML`, `SYM_DIV`, etc.), but `src/style.rs` uses raw `String` and `Cow<str>` throughout. Furthermore, line 828 of `style.rs` deep-copies the entire custom variable `HashMap<String, String>` onto every DOM node.

#### 2. Desired Outcome
- Tag names, attribute names, and CSS property names use 4-byte `Symbol` handles.
- Custom CSS properties are shared across elements using `Arc<HashMap<Symbol, String>>` with copy-on-write semantics.

#### 3. Recommended Solution
1. In `src/values.rs:770`:
   ```rust
   pub variables: std::sync::Arc<std::collections::HashMap<crate::interner::Symbol, String>>,
   ```
2. When resolving styles:
   - If an element defines no new CSS variables (`--*`), inherit the parent's `Arc::clone(&parent_style.variables)` ($O(1)$ pointer increment).
   - If an element overrides or defines variables, clone the map using `Arc::make_mut` and insert the new entries.
3. In `src/style.rs:745`, index winner declarations by `PropertyId` enum integer index (`[Option<Cow<str>>; 64]`) rather than `HashMap<Cow<str>, Cow<str>>`.

#### 4. Implementation Steps
1. Change `ComputedStyle::variables` to `Arc<HashMap<Symbol, String>>`.
2. Update `style.rs` custom property resolution to use `Arc::clone` and copy-on-write.
3. Replace `specified: HashMap<Cow<str>, Cow<str>>` with fixed array `[Option<Cow<str>>; 64]`.
4. Run `cargo test` and benchmark allocation counts with `AofInspector`.

#### 5. Verification & Completion Criteria
- Allocation counters in `trace.json` confirm that variable map allocations decrease by >95% on documents with custom properties.
- All cascade and inheritance tests pass.

---

### SPEC-10: Delegation of CSS Flexbox and Grid Sizing to Taffy 0.14

- **Target Finding:** `AST-002`
- **Detailed Blueprint:** See [`specifications/SPEC_10_TAFFY_DELEGATION.md`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/ASTERIA_IMPROVEMENT_AUDIT/specifications/SPEC_10_TAFFY_DELEGATION.md).
- **Executive Summary:**
  - Add `taffy = "0.14"` to `Cargo.toml`.
  - Implement a mapping function `fn to_taffy_style(style: &ComputedStyle) -> taffy::style::Style`.
  - In `src/layout.rs`, when encountering `BoxType::FlexNode` or `BoxType::GridNode`, instantiate a Taffy tree, populate child nodes with their Taffy styles, and call `taffy.compute_layout(root, available_space)`.
  - Extract computed child `layout.location` and `layout.size` directly into Asteria's `LayoutBox::dimensions`.
  - Delete ~1,200 lines of brittle handwritten flex and grid code in `layout.rs`.
- **Benefits:**
  - Full W3C CSS Flexbox and CSS Grid Level 1 compliance.
  - Zero ongoing maintenance burden for flex/grid constraint algorithms.

---

### SPEC-11: Functional Image Decoding & GPU Texture Quad Pipeline

- **Target Finding:** `AST-INC-001`
- **Affected Files:**
  - *Confirmed:* `src/image.rs`, `src/renderer/passes/image_pass.rs`, `src/renderer/commands/command_builder.rs`
  - *Candidates:* `src/renderer/resources/texture_cache.rs`

#### 1. Problem Statement & Root Cause
`ImageDecoder` synthesizes fake striped byte patterns; `ImagePass::set_image_quad` is never called; `CommandBuilder` renders `<img>` nodes as grey rectangles with `"[IMG]"` labels.

#### 2. Desired Outcome
Real image files (PNG, JPEG, WebP) are decoded into RGBA pixels, uploaded to GPU textures, and rendered by `ImagePass` with correct aspect ratios and texture coordinates.

#### 3. Recommended Solution
1. Add `image = { version = "0.25", default-features = false, features = ["png", "jpeg", "webp"] }` to `Cargo.toml`.
2. In `src/image.rs`:
   ```rust
   pub fn decode_image(&self, id: &str, data: &[u8]) -> Result<DecodeImage, String> {
       let img = image::load_from_memory(data).map_err(|e| e.to_string())?.to_rgba8();
       let (width, height) = img.dimensions();
       Ok(DecodeImage {
           id: id.to_string(),
           format: detect_image_format(data).unwrap_or(ImageFormat::Png),
           width,
           height,
           data: img.into_raw(),
       })
   }
   ```
3. In `src/renderer/resources/texture_cache.rs`:
   Implement `GpuTextureCache` managing `wgpu::Texture` and `wgpu::TextureView` handles keyed by image ID.
4. In `src/renderer/passes/image_pass.rs`:
   Support batching multiple image quads using an atlas or dynamic bind groups, and call `set_image_quad` during frame preparation.
5. In `command_builder.rs:97-128`:
   Emit real `RenderCommand::Image { rect, image_id }` instead of grey rectangle fallbacks.

#### 4. Implementation Steps
1. Integrate `image` crate decoding in `src/image.rs`.
2. Implement GPU texture creation in `texture_cache.rs`.
3. Update `ImagePass` to bind active texture views and draw quads.
4. Test with `tests/fixtures/gallery.html`.

#### 5. Verification & Completion Criteria
- Running `cargo run -- tests/fixtures/gallery.html` visually renders actual photos rather than grey placeholder boxes.
- `tests/image_integration.rs` verifies that PNG and JPEG byte fixtures decode to valid RGBA buffers.

---

### SPEC-12: Chrome UI Decoupling from Webpage Content Render Graph

- **Target Finding:** `AST-ARC-004`, `AST-006`
- **Affected Files:**
  - *Confirmed:* `src/renderer/window/window.rs`, `src/shell_ui.rs`
  - *Candidates:* `src/renderer/graph/render_graph.rs`

#### 1. Problem Statement & Root Cause
Browser shell UI (tabs, address bar, navigation buttons) is batched directly into `RectPass` and `TextPass` alongside webpage content. UI hover states and typing in the omnibox invalidate webpage batches, and webpage scrolling invalidates UI vertex buffers.

#### 2. Desired Outcome
Shell UI chrome is isolated into an independent render pass (`ChromePass`) with its own vertex and index buffers. Webpage content is confined to a clipping viewport (`HEADER_HEIGHT..height - STATUS_BAR_HEIGHT`).

#### 3. Recommended Solution
1. In `src/renderer/graph/render_graph.rs`:
   Register two distinct pipeline stages:
   - `ContentPasses`: `WebpageRectPass`, `WebpageTextPass`, `WebpageImagePass` (clipped to web viewport).
   - `ChromePasses`: `ShellRectPass`, `ShellTextPass` (rendered directly in window coordinates).
2. Typing in the omnibox or hovering over tabs updates only `ShellRectPass` and `ShellTextPass` buffers ($<50$ vertices), leaving webpage batches completely untouched.

#### 4. Implementation Steps
1. Add `ShellRectPass` and `ShellTextPass` to `RenderGraph`.
2. Move `shell_ui.render_shell_rects` and `shell_ui.render_shell_text` into the chrome passes.
3. Remove UI drawing calls from `build_rect_batch`.
4. Verify that typing in the address bar does not trigger `Compositor::render_to_batch` on the scene graph.

#### 5. Verification & Completion Criteria
- Profiler traces confirm zero webpage batch rebuilding during omnibox key events.
- Shell UI chrome renders crisply above web content with zero visual regressions.
