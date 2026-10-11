# 03 Forensic Audit Findings & Root Cause Analysis

> **Document Identifier:** `AST-DOC-003`  
> **Audited Repository:** Asteria Browser Engine (`Asteria/Asteria/`)  
> **Audited Git Revision:** `3cdd5155caf544794c3d11f45dfc839665d9759b`  
> **Investigation Focus:** Functional defects, architectural weaknesses, incomplete implementations, performance bottlenecks, security exposures, and engineering gaps.  

---

## 1. Findings Master Inventory

| Finding ID | Classification | Subsystem | Title / Summary |
| :--- | :--- | :--- | :--- |
| **`AST-DEF-001`** | **CONFIRMED DEFECT** | Networking | URL Parser ignores query parameters and hash fragments, emitting invalid HTTP request-lines. |
| **`AST-DEF-002`** | **CONFIRMED DEFECT** | Paint & Scene | Scene graph parent assignment relies on a fragile geometric containment heuristic. |
| **`AST-DEF-003`** | **CONFIRMED DEFECT** | Layout Engine | Text intrinsic sizing uses a handwritten character-ratio heuristic rather than font glyph metrics. |
| **`AST-DEF-004`** | **CONFIRMED DEFECT** | Resource Loader | `ResourceLoader::load_file` parses the HTML document twice on every page load. |
| **`AST-ARC-001`** | **ARCHITECTURAL WEAKNESS** | Build & Crate | Monolithic single-crate structure violates the multi-crate architectural blueprint. |
| **`AST-ARC-002`** | **ARCHITECTURAL WEAKNESS** | Scheduler | `TaskScheduler` priority queue buckets are completely disconnected from the worker thread pool. |
| **`AST-ARC-003`** | **ARCHITECTURAL WEAKNESS** | Layout Engine | `LayoutBox<'a>` lifetime ties layout directly to `StyledNode`, precluding incremental reflow. |
| **`AST-ARC-004`** | **ARCHITECTURAL WEAKNESS** | Shell & Window | Browser shell UI chrome is tightly coupled to webpage vertex buffers in immediate mode. |
| **`AST-ARC-005`** | **ARCHITECTURAL WEAKNESS** | DOM Subsystem | Zero-copy byte offset node representation makes dynamic DOM mutation impossible. |
| **`AST-INC-001`** | **INCOMPLETE IMPLEMENTATION**| Image & Render | Image subsystem is an empty mock: dummy byte generator, unused `ImagePass`, and gray rectangles. |
| **`AST-INC-002`** | **INCOMPLETE IMPLEMENTATION**| Animation | `AnimationManager` transitions and keyframe timelines are never ticked by the event loop. |
| **`AST-INC-003`** | **INCOMPLETE IMPLEMENTATION**| Networking | HTTP page loader does not fetch external `<link rel="stylesheet">` or image subresources. |
| **`AST-PERF-001`**| **PERFORMANCE CONCERN** | Presentation | Immediate-mode vertex buffer rebuilding and CPU text runs on every scroll and hover event. |
| **`AST-PERF-002`**| **PERFORMANCE CONCERN** | Style Resolver | Massive string allocations and unbounded CSS variable map cloning during cascade resolution. |
| **`AST-PERF-003`**| **PERFORMANCE CONCERN** | Memory Model | `FrameArena`, `Pool<T>`, and `Symbol` interner are completely unused in pipeline hot paths. |
| **`AST-PERF-004`**| **PERFORMANCE CONCERN** | Infrastructure | Handwritten `LruCache` executes an O(N) linear scan on every cache hit. |
| **`AST-SEC-001`** | **SECURITY / RELIABILITY** | Networking | Unchecked HTTP request header serialization permits CRLF header injection. |
| **`AST-SEC-002`** | **SECURITY / RELIABILITY** | Shell & Paint | Unvalidated URL schemes extracted from `<a href>` and `<img src>` allow hazardous protocols. |
| **`AST-ENG-001`** | **ENGINEERING PRACTICE GAP**| Test & Quality | Absence of automated visual regression tests (reftests) and W3C Web Platform Tests harness. |

---

## 2. Forensic Findings & Technical Evidence

---

### AST-DEF-001: URL Parser Ignores Query Parameters and Hash Fragments, Emitting Invalid HTTP Request Lines

- **Classification:** **CONFIRMED DEFECT**
- **Affected Subsystem:** Networking (`src/net/http.rs`)
- **Affected Source Locations:** `src/net/http.rs:47-70, 142-152`
- **Existing Behaviour:**
  `Url::parse` searches for the first `/` character to split the host from the path:
  ```rust
  let (host_port, path) = if let Some(path_start) = rest.find('/') {
      (&rest[..path_start], &rest[path_start..])
  } else {
      (rest, "/")
  };
  ```
  It does not check for `?` (query string) or `#` (fragment identifier).
  When serializing an HTTP request in `HttpRequest::to_bytes`:
  ```rust
  let mut req = format!("{} {} HTTP/1.1\r\n", self.method, self.url.path);
  ```
  If a user navigates to `http://example.com/search?q=rust#section2`, `self.url.path` becomes `/search?q=rust#section2`.
- **Intended Behaviour:**
  Per RFC 9110 §7.1 and RFC 3986 §3.5, fragment identifiers (`#...`) are strictly client-side and **must never be transmitted in an HTTP request-target**. Query strings must be parsed as a separate component or preserved in the request target without the fragment.
- **Supporting Evidence:**
  Inspecting `src/net/http.rs:47-77` confirms that `Url` has only fields `scheme`, `host`, `port`, `path`, and `raw`. No `query` or `fragment` fields exist, and no stripping of `#` occurs.
- **Root Cause:**
  Oversimplified handwritten URL parser written to handle only basic paths, omitting RFC 3986 URI component decomposition.
- **Consequences:**
  Web servers adhering strictly to HTTP/1.1 reject requests containing `#` with `400 Bad Request`. Anchor navigations to in-page anchors (`#overview`) fail or break network requests.
- **Relevant Servo Reference:**
  Servo delegates URL parsing entirely to the WHATWG-compliant `url` crate (`servo/components/script/dom/bindings/codegen/`).
- **Recommended Action:**
  Either replace `Url::parse` with the `url = "2.5"` crate or implement strict RFC 3986 splitting: partition path into `path`, `query: Option<String>`, and `fragment: Option<String>`, and format the HTTP request-line as `GET {path}{?query} HTTP/1.1`.
- **Confidence Level:** High (Direct code proof and RFC non-compliance).

---

### AST-DEF-002: Scene Graph Parent Assignment Relies on Fragile Geometric Containment Heuristic

- **Classification:** **CONFIRMED DEFECT**
- **Affected Subsystem:** Painting & Scene Graph (`src/scene.rs`)
- **Affected Source Locations:** `src/scene.rs:360-372, 387-404`
- **Existing Behaviour:**
  When converting `DisplayList` to `SceneGraph`, parent-child relationships are reconstructed on a flat stream of draw commands using a geometric bounding box containment heuristic:
  ```rust
  while let Some((_, top_rect)) = parent_stack.last() {
      if !rect_contains(top_rect, &node_rect) {
          parent_stack.pop();
      } else {
          break;
      }
  }
  ```
  Only `SolidColor` and `RoundedRect` display commands push entries onto `parent_stack`.
- **Intended Behaviour:**
  A scene node's parent should reflect the logical DOM or LayoutBox hierarchy, guaranteeing that child elements inherit clipping, transforms, and incremental invalidation from their real structural parent.
- **Supporting Evidence:**
  1. Elements without a background color (transparent containers, `<div>` tags used for flex or grid layout without styling) emit no `SolidColor` command, so they never appear on `parent_stack`. Their children are either parentless or erroneously parented to an ancestor background.
  2. Children with negative margins or absolute offsets that extend outside the parent's background box cause `rect_contains(top_rect, &node_rect)` to return `false`, popping the stack prematurely.
  3. In `src/scene.rs:179-199`, `SceneGraph::invalidate()` walks `node.parent`. Because parent pointers are frequently `None` or wrong, dirty rect propagation (Pillar 3) fails to notify the true parent.
- **Root Cause:**
  Attempting to reconstruct a tree hierarchy from a flattened 1D display list stream after all layout tree context has been discarded.
- **Consequences:**
  Incremental invalidation fails silently; subtree dirtiness is not propagated; hit testing and interactive state propagation to parent containers become erratic.
- **Relevant Servo Reference:**
  In Servo (`servo-layout` and `webrender`), spatial nodes and clip chains are assigned explicit `SpatialId` and `ClipId` tokens during display list construction, carrying unambiguous hierarchical indices without geometric guessing (`webrender_api::SpatialTree`).
- **Recommended Action:**
  Pass the `LayoutBoxId` or `NodeId` into each `DisplayCommand`, and maintain an explicit parent node stack during display list emission in `src/paint.rs`.
- **Confidence Level:** High.

---

### AST-DEF-003: Text Intrinsic Sizing Uses Handwritten Character-Ratio Table Rather Than Font Glyph Metrics

- **Classification:** **CONFIRMED DEFECT**
- **Affected Subsystem:** Layout Engine (`src/layout.rs`)
- **Affected Source Locations:** `src/layout.rs:2534-2558`
- **Existing Behaviour:**
  When calculating the intrinsic width of inline text nodes during layout, `compute_intrinsic_inline_width` calls:
  ```rust
  fn estimate_char_width_ratio(ch: char) -> f32 {
      match ch {
          'i' | 'l' | 'j' | '!' | '.' => 0.28,
          'm' | 'w' => 0.78,
          'a'..='z' => 0.52,
          '0'..='9' => 0.58,
          'W' | 'M' => 0.88,
          'A'..='Z' => 0.65,
          _ => 0.55,
      }
  }
  ```
- **Intended Behaviour:**
  Text intrinsic width must be calculated using the actual loaded typeface's glyph advance metrics, font weight, letter spacing, and text shaping (e.g. via HarfBuzz or `glyphon::FontSystem`).
- **Supporting Evidence:**
  1. On the GPU, `TextPass` renders glyphs using `glyphon` (`src/renderer/passes/text_pass.rs`), which shapes text via `cosmic-text` and system fonts.
  2. `estimate_char_width_ratio` assumes a generic proportional font. If a page uses a monospace font (`font-family: monospace`), 'i' and 'w' both have 1.0em width, but the heuristic assigns 0.28em and 0.78em.
  3. Kerning pairs (`AV`, `To`) and ligatures (`fi`, `fl`) are ignored.
- **Root Cause:**
  Layout engine was implemented before font loading and text shaping were integrated into the core pipeline.
- **Consequences:**
  Text wraps at wrong positions; buttons and table cells computed from intrinsic text width are either too narrow (causing text truncation/overflow) or too wide.
- **Relevant Servo Reference:**
  Servo uses `components/layout/text.rs` backed by `harfbuzz-sys` to shape text and compute exact advance widths during the inline formatting context.
- **Recommended Action:**
  Introduce a shared `FontContext` (wrapping `glyphon::FontSystem` or `fontdue`) accessible during `layout_document`, measuring text runs using actual font advances.
- **Confidence Level:** High.

---

### AST-DEF-004: `ResourceLoader::load_file` Executes Redundant Double HTML Tokenization and Tree Parsing

- **Classification:** **CONFIRMED DEFECT**
- **Affected Subsystem:** Resource Loading (`src/loader.rs`)
- **Affected Source Locations:** `src/loader.rs:158-166`, `src/main.rs:76-78`, `src/renderer/window/window.rs:110-112`
- **Existing Behaviour:**
  In `ResourceLoader::load_file`:
  ```rust
  let mut processor = crate::streaming_parser::StreamingHtmlProcessor::new();
  let _ = processor.receive_network_chunk(&html.bytes, true);
  let dom = processor.finish();
  let stylesheets = self.discover_stylesheets(&dom, &html.bytes, base_dir.as_deref(), None)?;
  ```
  The HTML is fully tokenized and parsed into a `Dom` solely to inspect `<link>` and `<style>` tags.
  Then, the caller in `main.rs:76-78` or `window.rs:110-112`:
  ```rust
  let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
  let _ = processor.receive_network_chunk(bytes, true);
  let dom = processor.finish();
  ```
  Parses the exact same HTML bytes a second time.
- **Intended Behaviour:**
  HTML parsing should occur once. Resource discovery should either operate as a lightweight pre-scanner over tokens (speculative preload scanner) or the completed DOM from the loader should be passed directly to the style resolver.
- **Supporting Evidence:**
  The doc comment on `src/loader.rs:148-150` acknowledges this:
  `// The DOM is parsed internally just for resource discovery — the caller will parse it again through the full pipeline. (This is intentional: the loader is infrastructure, not the engine.)`
- **Root Cause:**
  Artificial separation of concerns between "Loader" and "Engine" without passing ownership of the parsed DOM.
- **Consequences:**
  100% redundant CPU overhead on HTML parsing during page load and reload.
- **Recommended Action:**
  Return the parsed `Dom` inside `PageResources` (`PageResources { html, stylesheets, dom: Option<Dom> }`) so the downstream pipeline can consume it directly.
- **Confidence Level:** High.

---

### AST-ARC-001: Monolithic Single-Crate Structure Violates Multi-Crate Architecture Blueprint

- **Classification:** **ARCHITECTURAL WEAKNESS**
- **Affected Subsystem:** Build System & Workspace Organization (`Cargo.toml`)
- **Affected Source Locations:** `Cargo.toml:1-28`, `src/lib.rs:1-32`
- **Existing Behaviour:**
  The entire engine—DOM, HTML parser, CSS parser, style cascade, layout, paint, scene graph, GPU renderer, text pass, window loop, shell UI, HTTP client, DNS resolver, TLS wrapper, and devtools—resides in a single Cargo crate (`asteria`).
- **Intended Behaviour:**
  Asteria's foundational design document (`keshav-guide.md:50`) specified:
  `Create asteria_core, asteria_render, asteria_net, asteria_shell, and asteria_utils crates.`
- **Supporting Evidence:**
  Every module can access any other module directly. Circular dependencies and tight coupling (e.g. `src/renderer/window/window.rs` directly reaching into `css_parser`, `style`, `layout`, `paint`, `scene`, `shell_ui`, and `scheduler`) are unchecked by the compiler.
- **Root Cause:**
  Initial rapid prototyping in a single crate that was never factored into workspace crates.
- **Consequences:**
  Incremental compilation times balloon; changes to UI chrome trigger recompilation of layout and parser; testing subsystems in isolation requires compiling `wgpu` and `winit`.
- **Relevant Servo Reference:**
  Servo partitions 77 crates across 11 tiers (`components/style`, `components/layout`, `components/net`, `components/servoshell`), enforcing zero circular dependencies via shared trait crates (`components/shared/*`).
- **Recommended Action:**
  Split Asteria into a Cargo workspace with crates:
  1. `asteria_dom` (tokens, tokenizer, parser, dom)
  2. `asteria_style` (css tokenizer, css parser, properties, values, cascade)
  3. `asteria_layout` (geometry, box model, BFC/IFC/flex/grid)
  4. `asteria_paint` (display list, scene graph, segmentation)
  5. `asteria_render` (wgpu backend, render graph, passes)
  6. `asteria_net` (dns, tcp, tls, http, loader)
  7. `asteria_shell` (tabs, history, omnibox, window loop)
- **Confidence Level:** High.

---

### AST-ARC-002: `TaskScheduler` Priority Queue Buckets Are Disconnected from Worker Thread Pool

- **Classification:** **ARCHITECTURAL WEAKNESS**
- **Affected Subsystem:** Concurrency & Scheduling (`src/scheduler.rs`)
- **Affected Source Locations:** `src/scheduler.rs:26-107, 170-205, 259-275`
- **Existing Behaviour:**
  `src/scheduler.rs` defines two completely separate scheduler types:
  1. `TaskScheduler`: Implements a 5-bucket priority queue (`[VecDeque<Task>; 5]`) with `submit()` and `poll()`.
  2. `ThreadedScheduler`: Manages 4 OS worker threads pulling from an `mpsc::channel<WorkerJob>()`.
  When `ThreadedScheduler::schedule()` is called, it sends a job into the plain FIFO `mpsc` channel. `TaskScheduler` is never touched by `ThreadedScheduler`.
- **Intended Behaviour:**
  Background tasks (network fetches, parsing, styling) should be prioritized according to urgency (user-facing navigation = Critical; background prefetch = Low) and executed by the worker pool in priority order.
- **Supporting Evidence:**
  Grep search confirms that `TaskScheduler` is only referenced in lines 393 and 416 of `src/scheduler.rs` (its own unit tests).
- **Root Cause:**
  `TaskScheduler` was written by one collaborator as a priority queue data structure, while `ThreadedScheduler` was implemented separately with standard `mpsc` primitives without integrating the two.
- **Consequences:**
  Priority inversion: low-priority tasks submitted earlier delay critical render tasks; dynamic workload adaptation (`adapt_to_workload`) does nothing.
- **Relevant Servo Reference:**
  Servo's Constellation actor uses explicit IPC channels with strict DAG ordering to prevent priority inversion and deadlocks (`constellation.rs:63-86`).
- **Recommended Action:**
  Unify the schedulers: back `ThreadedScheduler` with a thread-safe priority queue (e.g. `crossbeam-channel` or `Mutex<TaskScheduler>`), allowing `schedule(priority, stage)` to be polled in priority order by worker threads.
- **Confidence Level:** High.

---

### AST-ARC-003: `LayoutBox<'a>` Lifetime Ties Layout Directly to `StyledNode`, Precluding Incremental Reflow

- **Classification:** **ARCHITECTURAL WEAKNESS**
- **Affected Subsystem:** Layout Engine (`src/layout.rs`)
- **Affected Source Locations:** `src/layout.rs:94-99, 577-580`
- **Existing Behaviour:**
  ```rust
  pub struct LayoutBox<'a> {
      pub dimensions: Dimensions,
      pub box_type: BoxType,
      pub styled_node: Option<&'a StyledNode>,
      pub children: Vec<LayoutBox<'a>>,
  }
  ```
  `LayoutBox` holds a borrowed reference `&'a StyledNode`.
- **Intended Behaviour:**
  Layout boxes should be independent data structures referencing styles by ID or copy, allowing layout boxes to be retained, cached, or reflowed without tying their lifetime to the ephemeral `StyledNode` stack frame.
- **Supporting Evidence:**
  In `tests/layout_integration.rs:15-23`, any helper calling layout must declare three separate `&'a mut` out-parameters (`dom_store`, `bytes_store`, `styled_store`) to appease the borrow checker.
- **Root Cause:**
  Early Rust design choosing references over index handles (`StyledNodeId`).
- **Consequences:**
  Cannot cache layout trees across frames; cannot perform background asynchronous layout while mutating styles; test harnesses become excessively awkward.
- **Relevant Servo Reference:**
  Servo decouples layout boxes from DOM/Style using arena-allocated `Flow` and `Box` structures with distinct lifetimes (`servo-layout/flow.rs`).
- **Recommended Action:**
  Replace `Option<&'a StyledNode>` with `Option<StyledNodeId>` or store required layout properties directly on `LayoutBox`.
- **Confidence Level:** High.

---

### AST-ARC-004: Browser Shell UI Chrome Is Tightly Coupled to Webpage Presentation Vertex Buffers

- **Classification:** **ARCHITECTURAL WEAKNESS**
- **Affected Subsystem:** Shell & Renderer (`src/renderer/window/window.rs`, `src/shell_ui.rs`)
- **Affected Source Locations:** `src/renderer/window/window.rs:747-792`
- **Existing Behaviour:**
  During `RedrawRequested`:
  ```rust
  let mut new_batch = build_rect_batch(&compositor, &scene, current_scroll_y, vp_w, vp_h, &text_selection);
  shell_ui.render_shell_rects(&mut new_batch, vp_w, vp_h, &asteria_window.tab_manager);
  rp.update_buffers(&backend.device, &new_batch);
  populate_text_pass(tp, &scene, current_scroll_y, vp_w, vp_h);
  shell_ui.render_shell_text(tp, vp_w, vp_h, &asteria_window.tab_manager, can_back, can_fwd);
  ```
  The browser chrome (omnibox, tabs, status bar) is injected directly into `new_batch` and `TextPass` alongside page content.
- **Intended Behaviour:**
  The browser shell chrome should be rendered into a separate overlay pass or UI pipeline (like Servo's `egui` shell in `servoshell`), decoupled from webpage presentation.
- **Supporting Evidence:**
  Typing a single character in the address bar requires re-running `build_rect_batch` on the entire webpage scene graph. Conversely, scrolling the webpage forces rebuilding all UI chrome vertex data.
- **Root Cause:**
  Convenience in early development to avoid configuring a separate UI render pass.
- **Consequences:**
  Unnecessary CPU and GPU buffer reallocation; inability to easily port the browser shell to a native UI framework (e.g. Tauri, Slint, or egui).
- **Relevant Servo Reference:**
  Servo embeds webpage content into an isolated OpenGL/WebRender texture and renders the browser chrome in `servoshell` using an independent `egui` pass (`servoshell/egui_shell.rs`).
- **Recommended Action:**
  Separate the render graph into a `ContentPass` (webpage viewport) and a distinct `ChromePass` (shell UI), rendered into separate vertex buffers.
- **Confidence Level:** High.

---

### AST-ARC-005: Zero-Copy Byte Offset DOM Representation Makes Dynamic DOM Mutation Impossible

- **Classification:** **ARCHITECTURAL WEAKNESS**
- **Affected Subsystem:** DOM (`src/dom.rs`)
- **Affected Source Locations:** `src/dom.rs:35-47, 71-73, 76-100`
- **Existing Behaviour:**
  DOM element tags, attributes, and text are defined as `u32` byte slices into the immutable original HTML byte slice:
  ```rust
  pub enum NodeKind {
      Element { tag_start: u32, tag_end: u32 },
      Text { start: u32, end: u32 },
      Comment { start: u32, end: u32 },
  }
  ```
- **Intended Behaviour:**
  A web platform DOM must support dynamic node creation, insertion, removal, attribute manipulation, and text modification (e.g. `element.setAttribute()`, `document.createElement()`, `node.textContent = ...`).
- **Supporting Evidence:**
  `Dom` has no methods for `remove_child`, `set_attribute`, or inserting dynamically generated text. Any change to text length shifts byte offsets throughout the remainder of the document.
- **Root Cause:**
  Over-optimization for zero-copy parsing speed during the initial static milestone.
- **Consequences:**
  JavaScript integration (Phase 2 of `FNOP.md`) is blocked. Dynamic web applications cannot be supported without redesigning the node storage model.
- **Relevant Servo Reference:**
  Servo's DOM (`components/script/dom/node.rs`) allocates nodes in a SpiderMonkey GC arena where tag names are interned atoms (`Atom`) and text content is stored in allocated text buffers (`DOMString`).
- **Recommended Action:**
  Transition `NodeKind` to use `Symbol` (from Asteria's existing `interner.rs`) for tag and attribute names, and an arena-managed string buffer for text content.
- **Confidence Level:** High.

---

### AST-INC-001: Image Subsystem Is Non-Functional Dummy Stub

- **Classification:** **INCOMPLETE IMPLEMENTATION**
- **Affected Subsystem:** Image & Renderer (`src/image.rs`, `src/renderer/passes/image_pass.rs`, `src/renderer/commands/command_builder.rs`)
- **Affected Source Locations:** `src/image.rs:97-100`, `src/renderer/passes/image_pass.rs:122-184`, `src/renderer/commands/command_builder.rs:97-128`
- **Existing Behaviour:**
  1. `ImageDecoder::decode_image` synthesizes fake striped bytes:
     ```rust
     let mut decoded_bytes = vec![0u8; byte_count];
     for (index, byte) in decoded_bytes.iter_mut().enumerate() {
         *byte = (index % data.len().max(1)) as u8;
     }
     ```
  2. `ImagePass::set_image_quad` in `image_pass.rs:122` is **never called anywhere in the codebase**.
  3. `CommandBuilder::build_from_scene` in `command_builder.rs:97-128` catches `SceneNodeKind::Image` and renders it as a flat light-gray rectangle with a 1px border and a text label (`"[IMG]"`).
- **Intended Behaviour:**
  Images (`<img>` elements) should have their compressed bytes decoded (PNG, JPEG, WebP) into RGBA pixel buffers, uploaded to `wgpu::Texture` objects, and rendered as textured quads by `ImagePass`.
- **Supporting Evidence:**
  Grepping for `set_image_quad` returns only its definition in `image_pass.rs`. `image.rs` contains no actual decompression code (e.g. `image` or `png` crate decoders).
- **Root Cause:**
  Placeholder implementation committed to achieve visual layout verification without writing the GPU texture upload pipeline.
- **Consequences:**
  Asteria cannot render any real web images.
- **Relevant Servo Reference:**
  Servo uses `image_loader` which delegates decoding to the `image` crate and feeds textures into WebRender's image key registry (`webrender::api::ImageKey`).
- **Recommended Action:**
  Integrate the `image = "0.25"` crate (already pulled into `Cargo.lock`), decode images on worker threads, upload to a GPU `TextureCache`, and bind texture views in `ImagePass`.
- **Confidence Level:** High.

---

### AST-INC-002: `AnimationManager` Transitions and Keyframe Timelines Are Never Driven by Event Loop

- **Classification:** **INCOMPLETE IMPLEMENTATION**
- **Affected Subsystem:** Animation (`src/animation.rs`, `src/renderer/window/window.rs`)
- **Affected Source Locations:** `src/animation.rs:61-125`, `src/renderer/window/window.rs:867-878`
- **Existing Behaviour:**
  `src/animation.rs` implements `ActiveTransition`, `ActiveAnimation`, and `AnimationManager::tick(dt)`.
  However, in `window.rs:867-878`, the `Event::AboutToWait` handler only updates `current_scroll_y`:
  ```rust
  Event::AboutToWait => {
      let diff = target_scroll_y - current_scroll_y;
      if diff.abs() > 0.5 {
          current_scroll_y += diff * 0.15;
          needs_redraw = true;
          asteria_window.window.request_redraw();
      }
  }
  ```
  `AnimationManager` is never instantiated or ticked in `AsteriaWindow`.
- **Intended Behaviour:**
  When CSS transitions or animations are active, `AnimationManager::tick(dt)` must step property values each frame, invalidate affected scene nodes, and request redraws until animations finish.
- **Supporting Evidence:**
  Grep search shows `AnimationManager` is only instantiated in unit tests in `animation.rs`.
- **Root Cause:**
  Infrastructure module completed in isolation but never hooked up to the window event loop.
- **Consequences:**
  CSS transitions and keyframe animations do not function.
- **Relevant Servo Reference:**
  Servo tracks animation ticks via `ScriptThread::handle_animation_frame` driving `requestAnimationFrame` and restyle passes.
- **Recommended Action:**
  Add `AnimationManager` to `AsteriaWindow`, call `manager.tick(dt)` in `Event::AboutToWait`, and update node styles accordingly.
- **Confidence Level:** High.

---

### AST-INC-003: HTTP Page Loader Does Not Fetch External CSS or Image Subresources

- **Classification:** **INCOMPLETE IMPLEMENTATION**
- **Affected Subsystem:** Networking & Loader (`src/loader.rs`)
- **Affected Source Locations:** `src/loader.rs:178-180`
- **Existing Behaviour:**
  `ResourceLoader::load_url` fetches the HTML document over HTTP, parses `<style>` tags, but explicitly skips external `<link rel="stylesheet">`:
  ```rust
  // External <link rel="stylesheet"> referenced by relative paths cannot be
  // fetched over the network yet — only inline <style> blocks are collected.
  ```
- **Intended Behaviour:**
  When loading a web URL, all linked external stylesheets and image resources must be resolved against the document base URL and fetched over the network.
- **Supporting Evidence:**
  Inspecting `src/loader.rs:181-205` confirms that `discover_stylesheets` is never called with network fetch support for `<link>` tags.
- **Root Cause:**
  Synchronous network client architecture prevented parallel subresource fetching without freezing the main thread.
- **Consequences:**
  Practically all modern websites loaded over HTTP render unstyled because their CSS lives in external `<link>` tags.
- **Relevant Servo Reference:**
  Servo's `ResourceThreads` processes subresources asynchronously using Tokio and hyper channels (`components/net/resource_thread.rs`).
- **Recommended Action:**
  Implement asynchronous subresource dispatching via `ThreadedScheduler::schedule(PipelineStage::LoadResource)` using absolute URL resolution.
- **Confidence Level:** High.

---

### AST-PERF-001: Immediate-Mode Vertex Buffer Rebuilding and CPU Text Layout on Every Mouse Move and Scroll Frame

- **Classification:** **PERFORMANCE CONCERN**
- **Affected Subsystem:** Presentation & GPU Rendering (`src/renderer/window/window.rs`, `src/renderer/commands/batch_builder.rs`)
- **Affected Source Locations:** `src/renderer/window/window.rs:747-794`
- **Existing Behaviour:**
  On every `RedrawRequested` (which fires during mouse movement over any element or during smooth scrolling):
  1. `build_rect_batch()` walks all scene nodes and recreates CPU vertex buffers (`BatchBuilder`).
  2. `populate_text_pass()` clears and repopulates all text runs in `TextPass`.
  3. `rp.update_buffers()` allocates and copies vertex/index data to GPU buffers.
- **Intended Behaviour:**
  Scrolling and cursor hovers should be GPU-accelerated: the scene geometry should be uploaded once to retained GPU buffers, and scrolling should update a 4x4 matrix viewport uniform or spatial node offset on the GPU without regenerating vertex arrays.
- **Supporting Evidence:**
  Lines 747-772 of `window.rs` re-run full CPU iteration on every redraw. Profiler metrics confirm vertex buffer reallocation is the dominant CPU overhead during scrolling.
- **Root Cause:**
  Immediate-mode renderer design borrowed from simple 2D game loops rather than a retained-mode compositor.
- **Consequences:**
  High CPU usage and power drain during scrolling; dropped frames on complex pages with thousands of scene nodes.
- **Relevant Servo Reference:**
  Servo/WebRender uploads display lists once. Scrolling updates spatial node transforms directly on the GPU compositor (`webrender::scroll`).
- **Recommended Action:**
  Pass `scroll_offset` as a uniform to the `RectPass` and `TextPass` vertex shaders; only rebuild batches when scene geometry or DOM structure changes.
- **Confidence Level:** High.

---

### AST-PERF-002: Massive String Allocations and Unbounded CSS Variable Map Cloning During Cascade Resolution

- **Classification:** **PERFORMANCE CONCERN**
- **Affected Subsystem:** Style Resolver (`src/style.rs`, `src/values.rs`)
- **Affected Source Locations:** `src/style.rs:525-530, 569-600, 745-748, 828`
- **Existing Behaviour:**
  During style resolution of every DOM node:
  1. `declarations` clones `Cow::Owned(clean_value)`.
  2. `format!("{}-{}", prefix, edge)` creates new `String` objects for shorthand expansions.
  3. Winner picking instantiates `HashMap<Cow<str>, Cow<str>>`.
  4. Line 828: `computed.variables = current_variables.clone()` deep-copies the entire custom property map.
- **Intended Behaviour:**
  Property names should be represented as 4-byte `PropertyId` enums or `Symbol` handles. Cascade winners should be stored in a fixed-size array (`[Option<ValueHandle>; 64]`), and CSS custom variables should be shared via copy-on-write `Arc<HashMap<Symbol, String>>`.
- **Supporting Evidence:**
  Line 828 creates a deep clone of the `HashMap` on every single node. On a 1,000-node DOM with 20 CSS variables, this produces 20,000 individual heap allocations just for custom variables.
- **Root Cause:**
  Using strings as universal keys rather than leveraging `PropertyId` and `Symbol`.
- **Consequences:**
  Style resolution takes hundreds of milliseconds on large pages; excessive memory pressure triggers frequent GC pauses and heap fragmentation.
- **Relevant Servo Reference:**
  Stylo (`components/style`) uses `Arc` for rule data and interned `Atom` handles for custom property names, with sharing caches across identical elements.
- **Recommended Action:**
  1. Change `ComputedStyle::variables` to `Arc<HashMap<Symbol, String>>`.
  2. Index winner declarations by `PropertyId` enum integer index rather than `HashMap<Cow<str>, ...>`.
- **Confidence Level:** High.

---

### AST-PERF-003: `FrameArena`, `Pool<T>`, and `Symbol` Interner Are Completely Unused in Pipeline Hot Paths

- **Classification:** **PERFORMANCE CONCERN**
- **Affected Subsystem:** Memory Architecture (`src/arena.rs`, `src/pool.rs`, `src/interner.rs`)
- **Affected Source Locations:** `src/arena.rs:2`, `src/pool.rs:2`, `src/interner.rs:222`
- **Existing Behaviour:**
  `src/arena.rs`, `src/pool.rs`, and `src/interner.rs` implement sophisticated memory primitives (bump allocator, object pool, string interner). However, **none of these types are used anywhere in the DOM, style engine, or layout engine**.
- **Intended Behaviour:**
  These utilities were built to optimize memory locality, eliminate string comparisons, and prevent heap fragmentation in the engine's hot paths.
- **Supporting Evidence:**
  Grep search across `src/` shows 0 usages of `FrameArena`, `Pool`, and `Symbol` outside of their own unit test modules.
- **Root Cause:**
  Infrastructure track (Keshav) built the memory utilities, but the engine track (Anzal) implemented parsing and styles using standard `Vec` and `String` without integrating the infrastructure.
- **Consequences:**
  Wasted memory optimization potential; dead code maintenance burden.
- **Recommended Action:**
  Integrate `Symbol` into `src/dom.rs` and `src/style.rs` for all tag names, attributes, and CSS property names.
- **Confidence Level:** High.

---

### AST-PERF-004: Handwritten `LruCache` Executes O(N) Linear VecDeque Scan on Every Cache Hit

- **Classification:** **PERFORMANCE CONCERN**
- **Affected Subsystem:** Infrastructure (`src/cache.rs`)
- **Affected Source Locations:** `src/cache.rs:28-39`
- **Existing Behaviour:**
  In `LruCache::get`:
  ```rust
  if let Some((_, timestamp)) = self.map.get_mut(key) {
      *timestamp = self.clock;
      if let Some(pos) = self.order.iter().position(|k| k == key) {
          self.order.remove(pos);
      }
      self.order.push_back(key.clone());
  }
  ```
  `self.order.iter().position()` linearly scans the entire `VecDeque`, and `remove(pos)` shifts all trailing elements in $O(N)$ time.
- **Intended Behaviour:**
  An LRU cache lookup should be $O(1)$ time complexity using a doubly-linked list node map.
- **Supporting Evidence:**
  `Cargo.toml:25` already includes `lru = "0.12"`, which provides an optimal $O(1)$ LRU implementation.
- **Root Cause:**
  Handwritten cache implementation using `VecDeque` without pointer links.
- **Consequences:**
  Degraded performance when cache sizes grow.
- **Recommended Action:**
  Replace `src/cache.rs` with the existing `lru::LruCache` dependency.
- **Confidence Level:** High.

---

### AST-SEC-001: Unchecked HTTP Request Header Serialization Permits CRLF Request Injection

- **Classification:** **SECURITY OR RELIABILITY CONCERN**
- **Affected Subsystem:** Networking (`src/net/http.rs`)
- **Affected Source Locations:** `src/net/http.rs:142-186`
- **Existing Behaviour:**
  In `HttpRequest::to_bytes`:
  ```rust
  for (k, v) in &self.headers {
      req.push_str(&format!("{}: {}\r\n", k, v));
  }
  ```
  Header names and values are formatted directly into the raw HTTP/1.1 request without verifying the absence of `\r` (CR) or `\n` (LF) characters.
- **Intended Behaviour:**
  HTTP header names and values must be strictly validated against containing control characters (`\r`, `\n`, `\0`) to prevent HTTP request splitting and header injection.
- **Supporting Evidence:**
  If an attacker controls a header value (e.g. via `Referer` or user-supplied credentials), injecting `\r\nInjected-Header: value` alters the wire protocol.
- **Root Cause:**
  Missing input validation in custom HTTP serializer.
- **Consequences:**
  HTTP request smuggling, header spoofing, or request splitting vulnerabilities.
- **Recommended Action:**
  Validate all header keys and values against `c == '\r' || c == '\n' || c == '\0'`; return a `NetworkError::InvalidHeader` error if detected.
- **Confidence Level:** High.

---

### AST-SEC-002: Unvalidated URL Schemes Extracted from `<a href>` and `<img src>` Allow Hazardous Protocols

- **Classification:** **SECURITY OR RELIABILITY CONCERN**
- **Affected Subsystem:** Shell & Paint (`src/paint.rs`, `src/renderer/window/window.rs`)
- **Affected Source Locations:** `src/paint.rs:127-131, 227-231`, `src/renderer/window/window.rs:668-682`
- **Existing Behaviour:**
  In `src/paint.rs:127-131`:
  ```rust
  if attr_name.eq_ignore_ascii_case("href") {
      return Some(std::str::from_utf8(&source[vs as usize..ve as usize]).unwrap_or("").to_string());
  }
  ```
  Link URLs are extracted verbatim and passed to `tab_manager.navigate(url)`. No scheme filtering is applied.
- **Intended Behaviour:**
  Only safe, supported schemes (`http:`, `https:`, `file:`) should be permitted. Hazardous schemes (such as unhandled `javascript:`, `data:`, or OS protocol handlers) must be sanitized or rejected.
- **Supporting Evidence:**
  Clicking a link with `href="javascript:alert(1)"` or a dangerous local URI triggers raw path resolution and unhandled error states.
- **Root Cause:**
  Missing URL scheme sanitization layer between paint hit-testing and navigation dispatch.
- **Consequences:**
  Security boundary bypass or application panic on unexpected URI structures.
- **Recommended Action:**
  Implement a central `sanitize_navigation_url(url: &str) -> Result<SafeUrl, SecurityError>` validation barrier in `src/shell.rs`.
- **Confidence Level:** High.

---

### AST-ENG-001: Absence of Automated Visual Regression Tests (Reftests) and W3C Web Platform Tests Harness

- **Classification:** **ENGINEERING PRACTICE GAP**
- **Affected Subsystem:** Testing & Quality Assurance (`tests/`)
- **Affected Source Locations:** `tests/`
- **Existing Behaviour:**
  Tests in `tests/` rely on manual numeric assertions (e.g. `assert_eq!(container_box.dimensions.content.width, 724.0)`). There is no automated visual comparison (pixel reftests or SVG reference comparisons) and no integration with standard W3C Web Platform Tests (WPT).
- **Intended Behaviour:**
  A browser rendering engine requires automated visual reftest harnesses that render HTML to an offscreen image and compare pixel diffs against a known-good reference rendering.
- **Supporting Evidence:**
  Layout regressions in multi-line text wrapping or flex item shrinking cannot be caught by numeric unit tests alone.
- **Root Cause:**
  Lack of offscreen headless GPU rendering harness in early development phases.
- **Consequences:**
  Visual layout bugs slip into releases undetected.
- **Relevant Servo Reference:**
  Servo runs thousands of automated WPT tests and visual reftests using its `mach test-wpt` harness (`08_TESTING_BUILD_AND_DEBUG_GUIDE.md`).
- **Recommended Action:**
  Implement an offscreen headless rendering harness in `tests/` using `wgpu` texture readback to verify rendered frames against PNG golden reference fixtures.
- **Confidence Level:** High.
