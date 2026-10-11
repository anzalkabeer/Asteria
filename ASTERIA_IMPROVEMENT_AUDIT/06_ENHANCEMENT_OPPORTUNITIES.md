# 06 Strategic Engineering Enhancement Opportunities

> **Document Identifier:** `AST-DOC-006`  
> **Audited Repository:** Asteria Browser Engine (`Asteria/Asteria/`)  
> **Focus:** Value-multiplying enhancements, capability expansions, developer experience, and architectural advancements beyond defect correction.  

---

## 1. Enhancement Opportunity Portfolio

| Opportunity ID | Capability Domain | Practical Value & Strategic Impact | Prerequisite Architecture | Dependencies & Complexity |
| :--- | :--- | :--- | :--- | :--- |
| **`ENH-01`** | Scripting & Web Platform | **Pure-Rust JS Integration (`boa_engine`):** Enables dynamic DOM interaction, event listeners, and timers without C++ build dependencies. | `SPEC-05` (Modular crates), `AST-ARC-005` (Mutable DOM). | `boa_engine = "0.19"`; Complexity: High. |
| **`ENH-02`** | Observability & Tooling | **Chrome DevTools Protocol (CDP) WebSocket Server:** Connect standard Google Chrome DevTools (`chrome://inspect`) to inspect Asteria's live DOM and styles. | `asteria_net` or `asteria_shell`. | `tokio-tungstenite = "0.21"`; Complexity: Medium. |
| **`ENH-03`** | Quality Assurance | **Headless GPU Reftest & Golden Image Harness:** Automated pixel-diff testing against PNG reference fixtures using headless `wgpu` texture readback. | `asteria_render`. | `image = "0.25"`; Complexity: Low. |
| **`ENH-04`** | Web Graphics Platform | **Native W3C WebGPU API Exposure to Web Content:** Expose Asteria's existing `wgpu` context to `<canvas>` via HTML/JS bindings. | `ENH-01` (JS engine), `asteria_render`. | Reuses existing `wgpu = "22.1"`; Complexity: Medium. |
| **`ENH-05`** | Networking & Performance | **Asynchronous Subresource Fetching on Tokio:** Background parallel fetching of external stylesheets, web fonts, and images. | `asteria_net`. | `tokio = "1.38"`, `reqwest = "0.12"`; Complexity: Medium. |
| **`ENH-06`** | Embeddability & Ecosystem | **Public `WebView` & `EventLoopWaker` Embedding API:** Enable third-party Rust apps to embed Asteria as an HTML/CSS UI view (like Servo or Tauri). | `SPEC-05` (Workspace). | Trait definitions in `asteria_traits`; Complexity: Medium. |
| **`ENH-07`** | Visual Rendering Fidelity | **Signed Distance Field (SDF) Antialiased Border-Radius & Shadow Shaders:** Crisp, anti-aliased rounded rectangles and Gaussian box shadows in WGSL. | `RectPass` in `asteria_render`. | WGSL shader math; Complexity: Low. |
| **`ENH-08`** | Page Load Latency | **Speculative HTML Preload Scanner:** Lightweight token scanner that discovers and fetches external `<link>` and `<img>` URLs before full layout. | `asteria_dom`, `asteria_net`. | Pure Rust state machine; Complexity: Low. |

---

## 2. Detailed Technical Enhancement Proposals

---

### ENH-01: Pure-Rust ECMAScript Engine Integration via `boa_engine`

- **Current Limitation:**
  Asteria cannot execute JavaScript (`FNOP.md` Phase 2 target). Pages are completely static once parsed. User events cannot trigger script callbacks, and the DOM cannot be modified after initial layout.
- **Why It Matters to Asteria:**
  Transitioning Asteria from a document renderer to a real web platform requires scripting. However, adopting C++ engines (SpiderMonkey or V8) would compromise Asteria's pure-Rust build reproducibility and create extreme build fragility on Windows.
- **Related Servo Mechanism:**
  Servo embeds Mozilla's SpiderMonkey (`components/script/`), which requires hundreds of megabytes of C++ sources and LLVM Clang. Asteria should deliberately avoid this trap.
- **Implementation Strategy:**
  1. Add `boa_engine = "0.19"` to `crates/asteria_shell`.
  2. Implement a dedicated `ScriptThread` actor that runs Boa in an isolated execution loop.
  3. Expose minimal DOM bindings: `console.log`, `document.getElementById`, `element.addEventListener`, `setTimeout`.
  4. Script mutations emit `DomMutation` events across an `mpsc` channel to trigger layout dirty flags.
- **Prerequisites:**
  `AST-ARC-005` (DOM must use `Symbol` handles rather than static source buffer offsets).
- **Complexity & Costs:**
  High complexity. Boa adds ~15s to clean build times, but preserves 100% pure-Rust reproducibility.
- **Acceptance Criteria:**
  A test fixture with `<button onclick="document.body.style.backgroundColor = 'blue'">` dynamically updates the background color on click.

---

### ENH-02: Chrome DevTools Protocol (CDP) WebSocket Server

- **Current Limitation:**
  Asteria currently exports an offline `trace.json` file. Debugging DOM tree state, computed styles, and layout geometry requires looking at terminal printouts or static trace files.
- **Why It Matters to Asteria:**
  Allows developers to debug websites running inside Asteria using standard Chrome DevTools: inspecting live DOM elements, toggling CSS properties, viewing console logs, and measuring layout boxes.
- **Related Servo Mechanism:**
  Servo includes a devtools server in `components/devtools/` that speaks CDP JSON-RPC over WebSockets.
- **Implementation Strategy:**
  1. Spin up a lightweight local WebSocket server on `127.0.0.1:9222` using `tokio-tungstenite`.
  2. Implement CDP domains:
     - `DOM.getDocument`, `DOM.requestChildNodes`
     - `CSS.getComputedStyleForNode`, `CSS.getMatchedStylesForNode`
     - `Page.reload`, `Page.navigate`
  3. Developers open Chrome, navigate to `chrome://inspect`, and connect directly to Asteria.
- **Prerequisites:**
  `asteria_shell` modularization.
- **Complexity & Costs:**
  Medium. Low maintenance cost once JSON-RPC schema types are defined.
- **Acceptance Criteria:**
  Chrome DevTools connects to `127.0.0.1:9222`, displays Asteria's live DOM tree, and highlights element bounding boxes on hover.

---

### ENH-03: Headless GPU Reftest & Golden Image Harness

- **Current Limitation:**
  Tests in `tests/` rely on manual numeric assertions (e.g. `assert_eq!(box.width, 200.0)`). Visual regressions in text line-wrapping, border radius rendering, or flexbox alignment go completely undetected.
- **Why It Matters to Asteria:**
  Guarantees pixel-perfect rendering stability across commits. Allows the engine to run automated regression checks in CI without opening an interactive window.
- **Related Servo Mechanism:**
  Servo runs automated WPT reftests using headless WebRender texture capture (`mach test-wpt`).
- **Implementation Strategy:**
  1. Add a headless rendering mode to `WgpuBackend` using a standard `wgpu::Texture` instead of an OS swapchain surface.
  2. Implement a `render_to_image(&self, scene: &SceneGraph, width: u32, height: u32) -> image::RgbaImage` helper using `wgpu::BufferView` readback.
  3. Add test runner in `tests/reftests.rs` that renders `tests/fixtures/*.html`, compares the pixel output against `tests/fixtures/golden/*.png`, and asserts that pixel difference is below 0.1%.
- **Prerequisites:**
  `asteria_render` crate.
- **Complexity & Costs:**
  Low. High leverage for development confidence.
- **Acceptance Criteria:**
  Running `cargo test --test reftests` renders test fixtures headlessly and verifies pixel fidelity against golden PNG baselines.

---

### ENH-04: Native W3C WebGPU API Exposure for Web Content

- **Current Limitation:**
  Webpages cannot access hardware acceleration or `<canvas>` compute shaders.
- **Why It Matters to Asteria:**
  Asteria is built directly on `wgpu 22.1`! While other engines must build complex GPU abstractions from scratch, Asteria already owns an initialized `wgpu::Instance`, `Adapter`, `Device`, and `Queue`. Exposing this context to web content gives Asteria a massive competitive advantage.
- **Related Servo Mechanism:**
  Servo implements WebGPU in `components/webgpu/` by wrapping `wgpu-core`.
- **Implementation Strategy:**
  1. Bind `<canvas id="gpu">` to an offscreen `wgpu::Texture`.
  2. Expose `navigator.gpu` via `boa_engine` bindings, passing calls through to Asteria's existing `wgpu::Device`.
  3. Composite the canvas texture into the render graph via `ImagePass`.
- **Prerequisites:**
  `ENH-01` (JS engine), `SPEC-11` (Textured quads in `ImagePass`).
- **Complexity & Costs:**
  Medium complexity, exceptional strategic synergy with Asteria's existing stack.
- **Acceptance Criteria:**
  A web page running a WebGPU triangle demo renders hardware-accelerated triangles inside Asteria's canvas element.

---

### ENH-05: Asynchronous Subresource Fetching on Tokio

- **Current Limitation:**
  `ResourceLoader` blocks the executing thread synchronously on HTTP requests. External `<link rel="stylesheet">` files are skipped over HTTP, and image subresources are not loaded.
- **Why It Matters to Asteria:**
  Allows Asteria to load real-world web pages with external CSS files, custom web fonts, and multiple image assets in parallel without blocking rendering.
- **Related Servo Mechanism:**
  Servo's `ResourceThreads` processes subresources asynchronously using Tokio and hyper channels (`components/net/resource_thread.rs`).
- **Implementation Strategy:**
  1. Add `tokio = { version = "1.38", features = ["rt-multi-thread"] }` and `reqwest = "0.12"` to `crates/asteria_net`.
  2. When the HTML parser or preload scanner encounters `<link rel="stylesheet">` or `<img src="...">`, dispatch an async fetch task to Tokio.
  3. As CSS files arrive, incrementally re-run style resolution. As images arrive, upload to `GpuTextureCache` and trigger a repaint.
- **Prerequisites:**
  `asteria_net` crate modularization.
- **Complexity & Costs:**
  Medium. Standard modern Rust networking pattern.
- **Acceptance Criteria:**
  Navigating to an external URL loads the HTML, fetches all external CSS files in parallel, and renders the fully-styled page.

---

### ENH-06: Public `WebView` and `EventLoopWaker` Embedding API Surface

- **Current Limitation:**
  Asteria can only run as its own standalone desktop application. It cannot be embedded as a web view inside other Rust desktop applications (e.g. game UIs, productivity tools, Tauri-like shells).
- **Why It Matters to Asteria:**
  Enables Asteria to become an embeddable alternative to Chromium/CEF and WebKit for the Rust ecosystem.
- **Related Servo Mechanism:**
  Servo exposes `ServoBuilder`, `WebView`, `EventLoopWaker`, and `RenderingContext` traits (`components/servo/lib.rs`).
- **Implementation Strategy:**
  1. Create `crates/asteria_traits` defining `WebViewDelegate`, `EventLoopWaker`, and `RenderSurface`.
  2. Encapsulate browser instance in `pub struct AsteriaEngine`:
     ```rust
     pub struct AsteriaEngine {
         pub fn create_webview(&mut self, width: u32, height: u32) -> WebViewHandle;
         pub fn handle_event(&mut self, event: InputEvent);
         pub fn render(&mut self, surface: &wgpu::TextureView);
     }
     ```
  3. `asteria_shell` becomes a simple consumer of this public embedding API.
- **Prerequisites:**
  `SPEC-05` (7-crate workspace).
- **Complexity & Costs:**
  Medium. Significantly expands the project's utility and adoption.
- **Acceptance Criteria:**
  A minimal sample application in `examples/embedded_view.rs` embeds Asteria inside an arbitrary window.

---

### ENH-07: Signed Distance Field (SDF) Antialiased Border-Radius & Shadow Shaders

- **Current Limitation:**
  `RoundedRect` rendering in `RectPass` relies on simple quad geometry without sub-pixel edge smoothing, leading to aliased, pixelated rounded corners on high-DPI displays.
- **Why It Matters to Asteria:**
  Delivers premium, modern visual aesthetics matching Safari and Chrome with zero CPU cost.
- **Implementation Strategy:**
  1. Update `shader.wgsl` to compute the signed distance to the rounded rectangle boundary in the fragment shader:
     $$\text{dist} = \text{length}(\max(|p| - (b - r), 0)) - r$$
  2. Compute alpha using `smoothstep(0.0, 1.0, -dist)`.
  3. Render all rounded corners, pill buttons, and soft Gaussian box shadows with hardware-accelerated anti-aliasing in a single draw pass.
- **Prerequisites:**
  `RectPass` in `asteria_render`.
- **Complexity & Costs:**
  Low. Pure shader math improvement with zero new dependencies.
- **Acceptance Criteria:**
  Rounded buttons with `border-radius: 16px` display smooth, anti-aliased borders under 200% zoom.
