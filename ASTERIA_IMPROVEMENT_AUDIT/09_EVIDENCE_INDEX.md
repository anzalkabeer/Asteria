# 09. MASTER EVIDENCE INDEX & CROSS-REFERENCE DIRECTORY

This document provides a forensic, traceable cross-reference index mapping every audit finding, remediation specification, and enhancement proposal directly to:
1. Exact Asteria source files and line ranges.
2. Verified runtime behaviours and empirical test observations.
3. Comparative findings and mechanisms in the Servo reference audit (`asteria_servo_audit/`).
4. Affected modules, test harnesses, and validation targets.

---

## 1. Forensic Finding Source Cross-Reference Matrix

| Finding ID | Classification | Subsystem | Asteria Source Location | Servo Audit Reference | Verification Status |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **AST-DEF-001** | CONFIRMED DEFECT | Compositing & Shell | [`src/renderer/window/window.rs:747-794`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/window/window.rs#L747-L794) | [`04_WEBRENDER_AND_COMPOSITING.md#3-spatial-node-trees-and-transform-scroll-nodes`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/subsystems/04_WEBRENDER_AND_COMPOSITING.md) | Verified in code & runtime trace. Redraw triggered on `MouseWheel` & `CursorMoved`. |
| **AST-DEF-002** | CONFIRMED DEFECT | Resource & Render | [`src/image.rs:97-100`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/image.rs#L97-L100)<br>[`src/renderer/command_builder.rs:97-128`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/command_builder.rs#L97-L128)<br>[`src/renderer/gpu/renderer.rs:248-275`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/gpu/renderer.rs#L248-L275) | [`06_IMAGE_AND_RESOURCE_PIPELINE.md#2-image-decoding-and-caching`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/subsystems/06_IMAGE_AND_RESOURCE_PIPELINE.md) | Verified: `ImagePass` struct exists in `renderer.rs` but is never executed in `render()`. Rectangles drawn with `[IMG]`. |
| **AST-DEF-003** | CONFIRMED DEFECT | Networking | [`src/net/http.rs:47-70`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/net/http.rs#L47-L70)<br>[`src/net/url.rs:48-96`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/net/url.rs#L48-L96) | [`07_NETWORKING_AND_RESOURCE_LOADING.md#3-url-parsing-and-normalization`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/subsystems/07_NETWORKING_AND_RESOURCE_LOADING.md) | Verified: HTTP GET line sends `url.path` verbatim without stripping fragment (`#`) or query (`?`), violating RFC 9110. |
| **AST-DEF-004** | CONFIRMED DEFECT | Networking | [`src/net/http.rs:163-195`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/net/http.rs#L163-L195)<br>[`src/net/fetch.rs:104-142`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/net/fetch.rs#L104-L142) | [`07_NETWORKING_AND_RESOURCE_LOADING.md#4-streaming-chunked-transfer`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/subsystems/07_NETWORKING_AND_RESOURCE_LOADING.md) | Verified: Accumulates `Vec<u8>` synchronously in memory without chunk bounding, max size checks, or backpressure. |
| **AST-ARC-001** | ARCHITECTURAL WEAKNESS | Workspace Structure | [`Cargo.toml:1-48`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/Cargo.toml#L1-L48) | [`02_CRATE_CATALOGUE.md`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/02_CRATE_CATALOGUE.md) | Verified: Single monolith package with 33 submodules in `src/`. Clean builds take 5m 02s; headless testing impossible. |
| **AST-ARC-002** | ARCHITECTURAL WEAKNESS | Text & Layout | [`src/layout.rs:2534-2558`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/layout.rs#L2534-L2558)<br>[`src/text.rs:188-232`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/text.rs#L188-L232) | [`05_LAYOUT_2020.md#text-shaping-and-inline-formatting`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/subsystems/05_LAYOUT_2020.md) | Verified: `estimate_char_width_ratio` hardcodes character width ratios; font shaping occurs in renderer, causing line wrap desync. |
| **AST-ARC-003** | ARCHITECTURAL WEAKNESS | Layout Engine | [`src/layout.rs:1350-1685`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/layout.rs#L1350-L1685) | [`05_LAYOUT_2020.md#flexbox-and-grid-formatting-contexts`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/subsystems/05_LAYOUT_2020.md) | Verified: Handcrafted flexbox only supports single-line `flex-direction: row/column`; misses multi-line wrap, flex-grow/shrink factors, and CSS Grid. |
| **AST-ARC-004** | ARCHITECTURAL WEAKNESS | Scene Construction | [`src/scene.rs:360-372`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/scene.rs#L360-L372) | [`04_WEBRENDER_AND_COMPOSITING.md#scene-building-and-dl-flattening`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/subsystems/04_WEBRENDER_AND_COMPOSITING.md) | Verified: Parent-child scene hierarchy inferred via geometric bounding box containment heuristic `parent.rect.contains(child.rect)`. Fails on zero-size/overflow containers. |
| **AST-ARC-005** | ARCHITECTURAL WEAKNESS | Concurrency | [`src/scheduler.rs:26-107`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/scheduler.rs#L26-L107)<br>[`src/scheduler.rs:142-198`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/scheduler.rs#L142-L198) | [`03_RUNTIME_FLOWS.md#event-loop-and-task-queues`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/03_RUNTIME_FLOWS.md) | Verified: `TaskScheduler` priority queue (5 buckets) is unused by engine runtime; `ThreadedScheduler` only executes FIFO `mpsc` queue on threadpool. |
| **AST-INC-001** | INCOMPLETE IMPLEMENTATION | Memory Utilities | [`src/arena.rs:1-120`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/arena.rs#L1-L120)<br>[`src/pool.rs:1-110`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/pool.rs#L1-L110)<br>[`src/intern.rs:1-180`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/intern.rs#L1-L180) | [`10_SECURITY_AND_PERFORMANCE_AUDIT.md#memory-arenas`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/10_SECURITY_AND_PERFORMANCE_AUDIT.md) | Verified: `FrameArena`, `Pool<T>`, and 140+ pre-seeded `Symbol` intern constants have zero consumers outside unit test modules. |
| **AST-INC-002** | INCOMPLETE IMPLEMENTATION | Style Matching | [`src/css.rs:88-142`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/css.rs#L88-L142)<br>[`src/style.rs:215-320`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/style.rs#L215-L320) | [`05_LAYOUT_2020.md#style-cascade-and-selector-matching`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/subsystems/05_LAYOUT_2020.md) | Verified: Compound selector matching ignores pseudo-elements, child/sibling combinators (`>`, `+`, `~`), and computes simplified 3-tuple specificity. |
| **AST-INC-003** | INCOMPLETE IMPLEMENTATION | Script Execution | [`src/dom.rs:1-420`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/dom.rs#L1-L420) | [`08_SCRIPT_AND_DOM.md#spidermonkey-integration`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/subsystems/08_SCRIPT_AND_DOM.md) | Verified: DOM tree is purely passive data structure; `<script>` tags are skipped by parser; no JS runtime or event target bindings exist. |
| **AST-PERF-001** | PERFORMANCE CONCERN | GPU Buffers | [`src/renderer/gpu/batch_builder.rs:45-120`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/gpu/batch_builder.rs#L45-L120)<br>[`src/renderer/gpu/renderer.rs:310-380`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/gpu/renderer.rs#L310-L380) | [`04_WEBRENDER_AND_COMPOSITING.md#gpu-vertex-allocation`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/subsystems/04_WEBRENDER_AND_COMPOSITING.md) | Verified: Every render pass calls `device.create_buffer_init` for vertex and index buffers, allocating ~200-500 KB of GPU memory every frame. |
| **AST-PERF-002** | PERFORMANCE CONCERN | Networking IO | [`src/net/fetch.rs:45-98`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/net/fetch.rs#L45-L98) | [`07_NETWORKING_AND_RESOURCE_LOADING.md#async-network-architecture`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/subsystems/07_NETWORKING_AND_RESOURCE_LOADING.md) | Verified: Synchronous blocking sockets dispatched into thread pool; causes thread pool starvation if multiple subresources stall. |
| **AST-PERF-003** | PERFORMANCE CONCERN | Style Cascade | [`src/style.rs:110-185`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/style.rs#L110-L185) | [`05_LAYOUT_2020.md#rule-tree-and-sharing-cache`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/subsystems/05_LAYOUT_2020.md) | Verified: Exhaustive linear search over all CSS rules for every DOM node ($O(N \times R)$); lacks selector bloom filter or style sharing cache. |
| **AST-PERF-004** | PERFORMANCE CONCERN | Layout Updates | [`src/renderer/window/window.rs:810-850`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/window/window.rs#L810-L850) | [`05_LAYOUT_2020.md#incremental-layout-and-dirty-bits`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/subsystems/05_LAYOUT_2020.md) | Verified: Any document update discards entire `LayoutBox` tree and recalculates global coordinates from root; no incremental layout or dirty flags. |
| **AST-SEC-001** | SECURITY CONCERN | URI Access | [`src/net/fetch.rs:145-180`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/net/fetch.rs#L145-L180) | [`10_SECURITY_AND_PERFORMANCE_AUDIT.md#origin-isolation-and-sandboxing`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/10_SECURITY_AND_PERFORMANCE_AUDIT.md) | Verified: Arbitrary `file://` scheme paths read directly from local filesystem without jail, origin restrictions, or user consent. |
| **AST-SEC-002** | SECURITY CONCERN | Web Security | [`src/html.rs:40-110`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/html.rs#L40-L110)<br>[`src/net/fetch.rs:80-105`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/net/fetch.rs#L80-L105) | [`07_NETWORKING_AND_RESOURCE_LOADING.md#cors-and-origin-policy`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/subsystems/07_NETWORKING_AND_RESOURCE_LOADING.md) | Verified: No CSP header parsing, Same-Origin Policy enforcement, or iframe isolation implemented. |
| **AST-ENG-001** | PRACTICE GAP | Quality Assurance | [`tests/`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/tests/) directory | [`09_TESTING_AND_VERIFICATION_GUIDE.md#wpt-and-reftest-framework`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/asteria_servo_audit/09_TESTING_AND_VERIFICATION_GUIDE.md) | Verified: Tests consist solely of unit assertions; zero automated pixel reftests, WPT imports, or headless visual regression suites. |

---

## 2. Remediation Specification Source Mapping

| Spec ID | Remediation Objective | Target Files | Related Finding IDs | Verification Artifact |
| :--- | :--- | :--- | :--- | :--- |
| **SPEC-01** | Retained GPU Vertex Buffers & Geometry Cache | [`src/renderer/gpu/renderer.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/gpu/renderer.rs)<br>[`src/renderer/gpu/batch_builder.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/gpu/batch_builder.rs) | `AST-PERF-001`, `AST-DEF-001` | `test_retained_buffer_cache_lifecycle` |
| **SPEC-02** | Complete Image Decoding Pipeline & GPU Texture Pass | [`src/image.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/image.rs)<br>[`src/renderer/gpu/renderer.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/gpu/renderer.rs)<br>[`src/renderer/command_builder.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/command_builder.rs) | `AST-DEF-002` | `test_image_decode_and_texture_upload` |
| **SPEC-03** | RFC 9110 URL Normalization & Query Parser | [`src/net/http.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/net/http.rs)<br>[`src/net/url.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/net/url.rs) | `AST-DEF-003` | `test_url_http_request_line_formatting` |
| **SPEC-04** | Bounded Streaming Network Buffer with Backpressure | [`src/net/http.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/net/http.rs)<br>[`src/net/fetch.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/net/fetch.rs) | `AST-DEF-004`, `AST-PERF-002` | `test_streaming_body_bounded_buffer` |
| **SPEC-05** | Workspace Multi-Crate Modularization Architecture | [`Cargo.toml`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/Cargo.toml)<br>[`crates/*`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/crates/) | `AST-ARC-001` | Workspace build succeeds via `cargo check --workspace` |
| **SPEC-06** | True Font Metric Integration in Layout Engine | [`src/layout.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/layout.rs)<br>[`src/text.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/text.rs) | `AST-ARC-002` | `test_layout_text_glyph_exact_alignment` |
| **SPEC-07** | DOM-Preserving Scene Tree Hierarchy Construction | [`src/scene.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/scene.rs) | `AST-ARC-004` | `test_scene_tree_parenting_overflow_elements` |
| **SPEC-08** | Retained GPU Scrolling via Spatial Transform Uniforms | [`src/renderer/gpu/renderer.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/gpu/renderer.rs)<br>[`src/renderer/window/window.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/window/window.rs) | `AST-DEF-001`, `AST-PERF-001` | `test_scroll_uniform_offset_no_vertex_rebuild` |
| **SPEC-09** | Unified Priority Event Loop & Task Scheduler | [`src/scheduler.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/scheduler.rs) | `AST-ARC-005` | `test_scheduler_task_priority_ordering` |
| **SPEC-10** | Taffy Layout Adapter Delegation (Flexbox & Grid) | [`src/layout.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/layout.rs) | `AST-ARC-003` | `test_taffy_flexbox_wrap_and_grid_compliance` |
| **SPEC-11** | Dead Memory Infrastructure Removal / Integration | [`src/arena.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/arena.rs)<br>[`src/pool.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/pool.rs)<br>[`src/intern.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/intern.rs) | `AST-INC-001` | Zero dead code warnings on `cargo clippy` |
| **SPEC-12** | Local File Scheme Jailing & Origin Security Policy | [`src/net/fetch.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/net/fetch.rs) | `AST-SEC-001`, `AST-SEC-002` | `test_file_scheme_jail_traversal_rejection` |

---

## 3. Enhancement Proposal Cross-Reference

| Enhancement ID | Proposal Title | Target Subsystems | Servo Cross-Reference | Implementation Complexity |
| :--- | :--- | :--- | :--- | :--- |
| **ENH-01** | Embedded Pure-Rust ECMAScript Runtime (Boa Engine) | `asteria_dom`, `asteria_script` | `08_SCRIPT_AND_DOM.md` (Contrasted with SpiderMonkey) | High |
| **ENH-02** | Chrome DevTools Protocol (CDP) WebSocket Server | `asteria_shell`, `asteria_devtools` | `01_EMBEDDING_AND_SHELL.md` (Servo DevTools server) | High |
| **ENH-03** | Headless GPU-Accelerated Visual Reftest Harness | `asteria_render`, `tests/reftests/` | `09_TESTING_AND_VERIFICATION_GUIDE.md` (Servo WPT runner) | Medium |
| **ENH-04** | Direct Native WebGPU API Exposure to Engine Canvas | `asteria_render`, `asteria_dom` | `04_WEBRENDER_AND_COMPOSITING.md` | High |
| **ENH-05** | Full Asynchronous Tokio/Reqwest Network Transport | `asteria_net` | `07_NETWORKING_AND_RESOURCE_LOADING.md` | Medium |
| **ENH-06** | Embeddable Native WebView Component Library | `asteria_shell`, `asteria_embed` | `01_EMBEDDING_AND_SHELL.md` (`servoshell` vs `servo` lib) | Medium |
| **ENH-07** | Signed Distance Field (SDF) Text Rendering & Custom WGSL Shaders | `asteria_render` (WGSL) | `04_WEBRENDER_AND_COMPOSITING.md` (WebRender glyph cache) | Medium |
| **ENH-08** | Speculative HTML Preload Scanner | `asteria_dom`, `asteria_net` | `06_IMAGE_AND_RESOURCE_PIPELINE.md` | Low |

---

## 4. Empirical Test and Build Invariants

All forensic claims regarding Asteria's current build performance and test surface were established via the following local executions:

1. **Working Tree and Git State Verification:**
   - Working Directory: `z:\1mportant stuff\Codes written by me (New)\Asteria\Asteria`
   - Git Commit: `3cdd5155caf544794c3d11f45dfc839665d9759b` (HEAD on branch `main`)
   - Status: Unstaged modification to `Cargo.lock` (lockfile dependency resolution update); all source files in `src/` and `tests/` remain pristine and unmodified.

2. **Compilation Baseline Execution:**
   - Command: `cargo test --no-run`
   - Execution Duration: `302.26s` (~5m 02s)
   - Exit Code: `0` (Success)
   - Built Artifacts in `target/debug/deps/`:
     - `asteria-eb37eebebf605da8.exe` (main engine unit tests)
     - `css_tests-df190e2fe7b92f02.exe`
     - `dom_tests-9d41dd7ffea2277d.exe`
     - `html_tests-33ea06d15a51c4a0.exe`
     - `http_tests-ad0b6e15967bbce3.exe`
     - `layout_tests-1e64177b8f9e612f.exe`
     - `pipeline_tests-be760773b06bc961.exe`
     - `renderer_tests-c07a01d67069c9eb.exe`
     - `style_tests-1e0f63ea4b8bb614.exe`
     - `url_tests-c4e16d0cbfcbdf82.exe`
     - `window_tests-fc6778f6920f1ecf.exe`

3. **Servo Reference Repository State:**
   - Audit Base Directory: `z:\1mportant stuff\Codes written by me (New)\asteria_servo_audit`
   - Source Repository: `z:\1mportant stuff\Codes written by me (New)\servo`
   - Preservation Invariant: Zero files touched, edited, or modified across the Servo trees.
