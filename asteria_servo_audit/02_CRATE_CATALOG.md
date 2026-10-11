# 02 Complete Crate Catalogue: Servo Engine & Trait Graph

> **Evidence Status:** VERIFIED  
> **Total Workspace Packages:** 77 Packages  
> **Source Revision:** `7ca9f80268a1d76e698560db82463e7960aa7845`  
> **Classification Framework:** 11 Functional Tiers  
> **Audit Status:** Grounded in deterministic Cargo metadata and source manifests.

---

## Tier Index
- [Tier 1: Public Engine, Embedding & Shells (4 crates)](#tier-1-public-engine-embedding--shells)
- [Tier 2: System Orchestration, Lifecycle & IPC (5 crates)](#tier-2-system-orchestration-lifecycle--ipc)
- [Tier 3: Script, DOM, WebIDL & JS Reflection (8 crates)](#tier-3-script-dom-webidl--js-reflection)
- [Tier 4: Layout & Box Formatting (2 crates)](#tier-4-layout--box-formatting)
- [Tier 5: Rendering, Painting & Display Lists (6 crates)](#tier-5-rendering-painting--display-lists)
- [Tier 6: Fonts, Text Shaping & Glyphs (2 crates)](#tier-6-fonts-text-shaping--glyphs)
- [Tier 7: Networking, Protocols & Cache (4 crates)](#tier-7-networking-protocols--cache)
- [Tier 8: Storage, Web Services & DevTools (5 crates)](#tier-8-storage-web-services--devtools)
- [Tier 9: Media Subsystem, WebGL, WebGPU & XR (22 crates)](#tier-9-media-subsystem-webgl-webgpu--xr)
- [Tier 10: System Support, Allocators, Hang Monitors & Profiling (14 crates)](#tier-10-system-support-allocators-hang-monitors--profiling)
- [Tier 11: Integration & Unit Test Packages (5 crates)](#tier-11-integration--unit-test-packages)

---

## Tier 1: Public Engine, Embedding & Shells

### 1. `servo`
- **Path:** `components/servo` (`Cargo.toml`)
- **Kind:** `rlib` (Workspace dependency: `=0.7.0`)
- **Purpose [VERIFIED]:** Serves as the primary public embedding facade for Rust consumers. Wires together Constellation, Script, Net, Layout, and Paint subsystems behind high-level `Servo`, `ServoBuilder`, and `WebView` abstractions.
- **Public Interfaces:** `Servo`, `ServoBuilder`, `WebView`, `WebViewBuilder`, `ServoDelegate`, `WebViewDelegate`, `EventLoopWaker`, `RenderingContext`.
- **Major Modules:** `servo`, `webview`, `servo_delegate`, `webview_delegate`, `network_manager`, `site_data_manager`, `user_content_manager`, `proxies`, `responders`.
- **Key Dependencies:** `servo-constellation`, `servo-embedder-traits`, `servo-net`, `servo-script`, `servo-paint`, `servo-paint-api`, `servo-layout-api`, `servo-config`.
- **Key Consumers:** `servoshell`, `servo-capi`.
- **Features:** `default = ["bundled", "clipboard", "js_jit", "multiprocess"]`, `default_web_features`, `brotli-compression-stream`, `gamepad`, `webgpu`, `webgl`, `webxr`.
- **Relevance to Asteria [RECOMMENDATION]:** Critical architectural reference. Asteria currently exposes a monolithic shell tightly coupled to `winit` and `wgpu`. Adopting Servo's decoupled `ServoBuilder` + `WebView` model would allow Asteria to support headless testing, tabbed views, and multiple window embeddings cleanly.
- **Audit Status:** VERIFIED.

### 2. `servo-embedder-traits`
- **Path:** `components/shared/embedder` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Defines the contract between the engine and the host environment without requiring the embedder to depend on engine internals.
- **Public Interfaces:** Traits `EmbedderMethods`, `EventLoopWaker`; structs `EmbedderMsg`, `Cursor`, `MediaSessionActionType`, `FilterPattern`, `UserScript`.
- **Major Modules:** `lib.rs`, `resources.rs`, `user_contents.rs`.
- **Key Dependencies:** `servo-url`, `servo-geometry`, `webrender_api`, `keyboard-types`, `ipc-channel`.
- **Key Consumers:** `servo`, `servo-constellation`, `servo-constellation-traits`, `servo-script`, `servoshell`.
- **Features:** `gamepad`, `webxr`.
- **Relevance to Asteria [RECOMMENDATION]:** High. Asteria lacks an embedder abstraction trait layer; adding an equivalent traits crate will allow headless test harnesses, native UI shells, and mobile targets to share the core Asteria rendering pipeline.
- **Audit Status:** VERIFIED.

### 3. `servoshell`
- **Path:** `ports/servoshell` (`Cargo.toml`)
- **Kind:** `bin` + `lib`
- **Purpose [VERIFIED]:** The reference browser application. Implements window management via `winit 0.30`, UI chrome (URL bar, tabs, devtools toggle) via `egui 0.34`, and GL surface presentation.
- **Public Interfaces:** `servoshell::main()`, `desktop::app::App`, `desktop::headed_window::HeadedWindow`.
- **Major Modules:** `desktop/app.rs`, `desktop/headed_window.rs`, `desktop/gui.rs`, `desktop/event_loop.rs`, `desktop/keyutils.rs`, `egl/android`, `egl/ohos`.
- **Key Dependencies:** `servo`, `winit`, `egui`, `egui-winit`, `egui_glow`, `glow`, `surfman`, `rustls`.
- **Key Consumers:** Root binary entry point.
- **Features:** `default = ["default_without_allocator", "servo-allocator/use-jemalloc"]`, `bundled`, `gamepad`, `webgpu`, `webgl`, `webxr`.
- **Relevance to Asteria [RECOMMENDATION]:** High. Asteria uses handwritten immediate-mode rendering for UI chrome in `shell_ui.rs`. Examining servoshell's clean separation between browser UI (`egui`) and web content surface (`WebView::paint`) provides a roadmap for modernizing Asteria's shell.
- **Audit Status:** VERIFIED.

### 4. `servo-capi`
- **Path:** `ffi/capi` (`Cargo.toml`)
- **Kind:** `cdylib`
- **Purpose [VERIFIED]:** Exposes a stable C ABI (`servo_capi.h`) for non-Rust embedding (Python, C++, Qt, GTK, Android NDK).
- **Public Interfaces:** `CServo`, `CWebView`, `servo_init()`, `servo_webview_create()`, `servo_webview_load_url()`.
- **Major Modules:** `lib.rs`.
- **Key Dependencies:** `servo`, `libc`, `dpi`, `url`.
- **Key Consumers:** External foreign consumers; tested via `tests/capi`.
- **Features:** `capi`, `no-wgl`.
- **Relevance to Asteria [RECOMMENDATION]:** Medium/Long-term. Asteria is currently pure Rust; building a C API is not an immediate priority, but Servo's memory management across FFI boundaries is informative.
- **Audit Status:** VERIFIED.

---

## Tier 2: System Orchestration, Lifecycle & IPC

### 5. `servo-constellation`
- **Path:** `components/constellation` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** The central brain of the browser engine. Owns the forest of frame trees, active pipelines, browsing contexts, session history (traversal, bfcache), and coordinates work across Script, Layout, and Paint threads/processes.
- **Public Interfaces:** `Constellation`, `Constellation<MessageFilter>`.
- **Major Modules:** `constellation.rs`, `pipeline.rs`, `browsing_context.rs`, `session_history.rs`, `sandboxing.rs`.
- **Key Dependencies:** `servo-constellation-traits`, `servo-embedder-traits`, `servo-script-traits`, `servo-layout-api`, `servo-paint-api`, `servo-net-traits`, `ipc-channel`.
- **Key Consumers:** `servo`.
- **Features:** `bluetooth`, `gamepad`.
- **Relevance to Asteria [RECOMMENDATION]:** Foundational. Asteria currently coordinates state inside a monolithic `Shell` struct. Servo's Constellation demonstrates how to decouple navigation history, tab state, and frame hierarchies from rendering loops.
- **Audit Status:** VERIFIED.

### 6. `servo-constellation-traits`
- **Path:** `components/shared/constellation` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Message definitions and channel wrappers for communicating with the Constellation actor.
- **Public Interfaces:** `ConstellationMsg`, `Key`, `PipelineId`, `BrowsingContextId`, `TopLevelBrowsingContextId`.
- **Major Modules:** `lib.rs`.
- **Key Dependencies:** `servo-base`, `servo-url`, `servo-embedder-traits`, `servo-script-traits`, `servo-paint-api`, `ipc-channel`.
- **Key Consumers:** `servo-constellation`, `servo-script`, `servo-paint`, `servo-net`, `servo`.
- **Features:** `bluetooth`.
- **Relevance to Asteria [RECOMMENDATION]:** High. Defines the core messaging vocabulary for tabs and pipelines.
- **Audit Status:** VERIFIED.

### 7. `servo-base`
- **Path:** `components/shared/base` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Fundamental types shared across all engine boundaries, specifically globally unique typed identifiers and cross-thread/cross-process communication channels.
- **Public Interfaces:** `PipelineId`, `BrowsingContextId`, `WebViewId`, `GenericSender<T>`, `GenericReceiver<T>`.
- **Major Modules:** `id.rs`, `generic_channel.rs`.
- **Key Dependencies:** `crossbeam-channel`, `ipc-channel`, `serde`, `uuid`.
- **Key Consumers:** Almost every internal crate (50+ consumers).
- **Features:** None.
- **Relevance to Asteria [RECOMMENDATION]:** Immediate value. Asteria uses integer indices or raw IDs without strong typing. `GenericSender<T>` seamlessly unifies multi-threaded in-memory channels with multi-process IPC channels.
- **Audit Status:** VERIFIED.

### 8. `servo-config`
- **Path:** `components/config` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Global configuration, command-line flags (`opts`), and dynamic runtime preferences (`prefs`).
- **Public Interfaces:** `Opts`, `Preferences`, `pref!`, `pref_map!`.
- **Major Modules:** `opts.rs`, `prefs.rs`.
- **Key Dependencies:** `servo-config-macro`, `serde`, `bpaf`, `rustc-hash`.
- **Key Consumers:** `servo`, `servoshell`, `servo-script`, `servo-layout`, `servo-net`, `servo-paint`.
- **Features:** None.
- **Relevance to Asteria [RECOMMENDATION]:** High. Asteria hardcodes layout flags and debug settings; adopting a structured preferences registry simplifies experimentation.
- **Audit Status:** VERIFIED.

### 9. `servo-config-macro`
- **Path:** `components/config/macro` (`Cargo.toml`)
- **Kind:** `proc-macro`
- **Purpose [VERIFIED]:** Procedural macro emitting compile-time preference lookups and type conversions for `servo-config`.
- **Key Dependencies:** `proc-macro2`, `quote`, `syn`.
- **Key Consumers:** `servo-config`.
- **Audit Status:** VERIFIED.

---

## Tier 3: Script, DOM, WebIDL & JS Reflection

### 10. `servo-script`
- **Path:** `components/script` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** The JavaScript execution and DOM manipulation runtime. Implements the DOM tree arena, HTML parsing hookup (`html5ever`), microtask queuing, event dispatching, and Web API bindings.
- **Public Interfaces:** `ScriptThread`, `Document`, `Node`, `Element`, `Window`, `DOMRef<T>`.
- **Major Modules:** `dom/`, `script_thread.rs`, `script_runtime.rs`, `htmlparser.rs`, `microtask.rs`, `textinput.rs`.
- **Key Dependencies:** `servo-script-traits`, `servo-script-bindings`, `mozjs`, `html5ever`, `stylo`, `servo-layout-api`, `servo-net-traits`.
- **Key Consumers:** `servo`, `tests/unit/script`.
- **Features:** `brotli-compression-stream`, `bluetooth`, `crown`, `debugmozjs`, `gamepad`, `js_jit`.
- **Relevance to Asteria [RECOMMENDATION]:** Fundamental architectural blueprint. Asteria currently has no JavaScript engine. Understanding how Servo separates DOM storage (`dom/`) from JS reflection via SpiderMonkey wrappers is vital for Asteria's future script integration.
- **Audit Status:** VERIFIED.

### 11. `servo-script-traits`
- **Path:** `components/shared/script` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Interface boundary for sending messages to and receiving callbacks from the Script thread.
- **Public Interfaces:** `ScriptMsg`, `LayoutMsg`, `WindowSizeData`, `UntrustedNodeAddress`.
- **Major Modules:** `lib.rs`.
- **Key Dependencies:** `servo-base`, `servo-url`, `servo-geometry`, `webrender_api`, `ipc-channel`.
- **Key Consumers:** `servo-script`, `servo-constellation`, `servo-layout`, `servo-paint`.
- **Features:** `bluetooth`.
- **Audit Status:** VERIFIED.

### 12. `servo-script-bindings`
- **Path:** `components/script_bindings` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** The bridge between Rust DOM types and SpiderMonkey JS objects. Runs `codegen.py` during `build.rs` to generate WebIDL bindings.
- **Public Interfaces:** `JS<T>`, `Root<T>`, `DomReflector`, `ToJSValConvertible`, `FromJSValConvertible`.
- **Major Modules:** `codegen/`, `webidls/`, `reflector.rs`, `root.rs`, `conversions.rs`, `proxyhandler.rs`.
- **Key Dependencies:** `mozjs`, `servo-dom-struct`, `servo-jstraceable-derive`.
- **Key Consumers:** `servo-script`.
- **Audit Status:** VERIFIED.

### 13. `servo-dom-struct`
- **Path:** `components/dom_struct` (`Cargo.toml`)
- **Kind:** `proc-macro`
- **Purpose [VERIFIED]:** Attribute macro `#[dom_struct]` ensuring DOM nodes satisfy memory layout, tracing, and finalize invariants.
- **Key Consumers:** `servo-script`.
- **Audit Status:** VERIFIED.

### 14. `servo-jstraceable-derive`
- **Path:** `components/jstraceable_derive` (`Cargo.toml`)
- **Kind:** `proc-macro`
- **Purpose [VERIFIED]:** Derives `JSTraceable` trait to allow SpiderMonkey GC to accurately trace references held in Rust memory.
- **Key Consumers:** `servo-script`, `servo-script-bindings`.
- **Audit Status:** VERIFIED.

### 15. `servo-xpath`
- **Path:** `components/xpath` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Evaluates XPath 1.0 expressions over DOM trees.
- **Key Consumers:** `servo-script`.
- **Audit Status:** VERIFIED.

### 16. `servo-svg`
- **Path:** `components/svg` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Parses and calculates geometry for SVG path elements and shapes (`resvg`, `svgtypes`, `kurbo`).
- **Key Consumers:** `servo-script`.
- **Relevance to Asteria [RECOMMENDATION]:** Asteria notes in `README.md:70`: "SVG detected but not rendered". `servo-svg` shows how to cleanly parse SVG paths into vector primitives.
- **Audit Status:** VERIFIED.

### 17. `servo-webvtt`
- **Path:** `components/webvtt` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** WebVTT subtitle and caption parser.
- **Key Consumers:** `servo-script`.
- **Audit Status:** VERIFIED.

---

## Tier 4: Layout & Box Formatting

### 18. `servo-layout`
- **Path:** `components/layout` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Transforms styled DOM nodes into a tree of layout boxes/fragments. Resolves widths, heights, margins, padding, floats, positioned items, and line breaks. Integrates `taffy` for CSS Flexbox and Grid.
- **Public Interfaces:** `Layout`, `Flow`, `Box`, `Fragment`, `LayoutQuery`.
- **Major Modules:** `flow/`, `fragment/`, `inline/`, `block.rs`, `flex.rs`, `grid.rs`, `table.rs`, `text.rs`.
- **Key Dependencies:** `servo-layout-api`, `stylo`, `taffy`, `servo-fonts`, `servo-geometry`.
- **Key Consumers:** `servo`, `servo-script`, `servo-paint`.
- **Features:** None.
- **Relevance to Asteria [RECOMMENDATION]:** Highest architectural relevance. Asteria has struggled with containing block calculations, percentage resolution (`FLAWS.md:1.1`), flexbox stability (`FLAWS.md:2.4`), and inline formatting double-layout (`FLAWS.md:1.7`). Servo's Flow tree and Taffy integration offer direct solutions.
- **Audit Status:** VERIFIED.

### 19. `servo-layout-api`
- **Path:** `components/shared/layout` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Interface crate exposing layout requests, hit-testing queries, and restyle notifications.
- **Public Interfaces:** `LayoutRPC`, `LayoutFactory`, `ReflowGoal`, `HitTestResult`.
- **Major Modules:** `lib.rs`.
- **Key Dependencies:** `servo-base`, `servo-geometry`, `webrender_api`, `style_traits`.
- **Key Consumers:** `servo-layout`, `servo-script`, `servo-constellation`, `servo-paint`.
- **Audit Status:** VERIFIED.

---

## Tier 5: Rendering, Painting & Display Lists

### 20. `servo-paint`
- **Path:** `components/paint` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Receives layout fragments and emits WebRender display items (rectangles, borders, text runs, shadows, gradients, image keys).
- **Public Interfaces:** `PaintThread`, `PaintTask`, `WebRenderDebugOption`.
- **Major Modules:** `paint_thread.rs`, `display_list_builder.rs`, `border.rs`, `text.rs`.
- **Key Dependencies:** `servo-paint-api`, `webrender`, `webrender_api`, `glow`, `surfman`.
- **Key Consumers:** `servo`, `servo-constellation`.
- **Features:** None.
- **Relevance to Asteria [RECOMMENDATION]:** High. Asteria uses immediate `wgpu` vertex batching in `paint.rs`. Servo's separation between display list generation and GPU rasterization eliminates layout-paint contention.
- **Audit Status:** VERIFIED.

### 21. `servo-paint-api`
- **Path:** `components/shared/paint` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Defines painting commands, rendering contexts, and presentation interfaces.
- **Public Interfaces:** `PaintMsg`, `RenderingContext`, `WindowRenderingContext`, `OffscreenRenderingContext`, `SoftwareRenderingContext`.
- **Major Modules:** `lib.rs`, `rendering_context.rs`.
- **Key Dependencies:** `webrender_api`, `glow`, `surfman`, `raw-window-handle`.
- **Key Consumers:** `servo-paint`, `servo`, `servoshell`.
- **Audit Status:** VERIFIED.

### 22. `servo-canvas`
- **Path:** `components/canvas` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Owns the dedicated background thread responsible for HTML5 2D `<canvas>` rasterization (`vello` / `vello_cpu`).
- **Key Dependencies:** `servo-canvas-traits`, `vello`, `peniko`, `kurbo`.
- **Key Consumers:** `servo-script`, `servo-constellation`.
- **Audit Status:** VERIFIED.

### 23. `servo-canvas-traits`
- **Path:** `components/shared/canvas` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** IPC messages and canvas drawing command buffers between DOM and Canvas thread.
- **Key Consumers:** `servo-canvas`, `servo-script`.
- **Audit Status:** VERIFIED.

### 24. `servo-pixels`
- **Path:** `components/pixels` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Low-level RGBA pixel buffer representation, color space conversions, and image buffer arithmetic.
- **Key Consumers:** `servo-paint`, `servo-canvas`, `servo-net`.
- **Audit Status:** VERIFIED.

### 25. `servo-geometry`
- **Path:** `components/geometry` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Strongly-typed 2D coordinate spaces using `euclid` (device pixels, CSS pixels, device-independent pixels).
- **Public Interfaces:** `DeviceIndependentPixel`, `DeviceIndependentIntRect`, `convert_rect_to_css_pixel()`.
- **Key Consumers:** All rendering and layout crates.
- **Relevance to Asteria [RECOMMENDATION]:** Immediate value. Eliminates bugs where physical device pixel units and logical CSS pixel units are accidentally mixed.
- **Audit Status:** VERIFIED.

---

## Tier 6: Fonts, Text Shaping & Glyphs

### 26. `servo-fonts`
- **Path:** `components/fonts` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Cross-platform font discovery, font cache management, OpenType feature selection, and HarfBuzz shaping.
- **Public Interfaces:** `FontContext`, `FontGroup`, `ShapedGlyphRun`.
- **Major Modules:** `font_context.rs`, `font_group.rs`, `shaper.rs`, `platform/`.
- **Key Dependencies:** `servo-fonts-traits`, `harfbuzz-sys`, `read-fonts`, `skrifa`, `font-types`, `fontsan`.
- **Key Consumers:** `servo-layout`, `servo-paint`.
- **Features:** `bundled_freetype`.
- **Relevance to Asteria [RECOMMENDATION]:** High. Asteria uses `glyphon` for basic glyph layout, but lacks complex HarfBuzz script shaping (Arabic, Indic, ligatures) and system font fallback.
- **Audit Status:** VERIFIED.

### 27. `servo-fonts-traits`
- **Path:** `components/shared/fonts` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Font matching queries and font family descriptor types.
- **Key Consumers:** `servo-fonts`, `servo-layout`.
- **Audit Status:** VERIFIED.

---

## Tier 7: Networking, Protocols & Cache

### 28. `servo-net`
- **Path:** `components/net` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** The network engine. Implements WHATWG Fetch specification, HTTP/1.1 and HTTP/2 connection pooling (`hyper`, `hyper-rustls`), response streaming, cookie jar, and memory/disk caching.
- **Public Interfaces:** `FetchContext`, `ImageCache`, `FileManagerThread`.
- **Major Modules:** `fetch/`, `http_loader.rs`, `image_cache.rs`, `cookie_storage.rs`, `filemanager_thread.rs`, `protocols/`.
- **Key Dependencies:** `servo-net-traits`, `hyper`, `hyper-util`, `rustls`, `tokio`, `cookie`.
- **Key Consumers:** `servo`, `servo-script`, `servo-constellation`.
- **Features:** None.
- **Relevance to Asteria [RECOMMENDATION]:** Critical. Asteria has a basic custom HTTP client in `net/` with limited caching and redirect support. `servo-net` demonstrates how to handle asynchronous streaming responses into parser tokenizers.
- **Audit Status:** VERIFIED.

### 29. `servo-net-traits`
- **Path:** `components/shared/net` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Network message types, Fetch requests, responses, HTTP headers, CORS policies, and cookie tokens.
- **Public Interfaces:** `Request`, `Response`, `ResponseBody`, `FetchResponseListener`, `NetworkError`.
- **Key Consumers:** `servo-net`, `servo-script`, `servo-constellation`, `servo`.
- **Audit Status:** VERIFIED.

### 30. `servo-url`
- **Path:** `components/url` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Safe, validated WHATWG URL wrapper enforcing invariant checks and origin equality.
- **Public Interfaces:** `ServoUrl`, `ImmutableOrigin`.
- **Key Consumers:** Used universally across almost all crates (45+ consumers).
- **Audit Status:** VERIFIED.

### 31. `servo-hyper-serde`
- **Path:** `components/hyper_serde` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Serde serialization wrappers for `http` and `hyper` types to allow cross-process IPC transmission.
- **Key Consumers:** `servo-net`, `servo-net-traits`.
- **Audit Status:** VERIFIED.

---

## Tier 8: Storage, Web Services & DevTools

### 32. `servo-storage`
- **Path:** `components/storage` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Manages web storage (localStorage, sessionStorage) backed by SQLite (`rusqlite`).
- **Key Dependencies:** `servo-storage-traits`, `rusqlite`, `sea-query`.
- **Key Consumers:** `servo-constellation`.
- **Audit Status:** VERIFIED.

### 33. `servo-storage-traits`
- **Path:** `components/shared/storage` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Storage IPC requests and origin storage quota descriptors.
- **Key Consumers:** `servo-storage`, `servo-script`, `servo-constellation`.
- **Audit Status:** VERIFIED.

### 34. `servo-devtools`
- **Path:** `components/devtools` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Implements a Chrome DevTools Protocol (CDP) WebSocket server allowing remote inspection of DOM trees, styles, and console output.
- **Key Dependencies:** `servo-devtools-traits`, `warp`, `async-tungstenite`.
- **Key Consumers:** `servo-constellation`.
- **Relevance to Asteria [RECOMMENDATION]:** High. Asteria has a basic prototype devtools module in `src/devtools/`; CDP compliance allows using standard Chrome DevTools frontend.
- **Audit Status:** VERIFIED.

### 35. `servo-devtools-traits`
- **Path:** `components/shared/devtools` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** Trait definitions for reporting page events, console messages, and timeline markers to DevTools.
- **Key Consumers:** `servo-devtools`, `servo-script`.
- **Audit Status:** VERIFIED.

### 36. `servo-webdriver-server`
- **Path:** `components/webdriver_server` (`Cargo.toml`)
- **Kind:** `lib`
- **Purpose [VERIFIED]:** W3C WebDriver HTTP endpoint for automated browser test execution.
- **Key Dependencies:** `webdriver`, `warp`.
- **Key Consumers:** `servoshell`.
- **Audit Status:** VERIFIED.

---

## Tier 9: Media Subsystem, WebGL, WebGPU & XR

### 37–52. `servo-media` & Backends (16 crates)
- **Crater Range:** `servo-media`, `servo-media-audio`, `servo-media-auto`, `servo-media-derive`, `servo-media-dummy`, `servo-media-examples`, `servo-media-gstreamer`, `servo-media-gstreamer-render`, `servo-media-gstreamer-render-android`, `servo-media-gstreamer-render-unix`, `servo-media-ohos`, `servo-media-player`, `servo-media-streams`, `servo-media-thread`, `servo-media-traits`, `servo-media-webrtc`.
- **Purpose [VERIFIED]:** Hardware-accelerated audio/video decoding, pipeline streaming, WebRTC, and media player playback. Default backends use GStreamer on Linux/Windows/macOS or OpenHarmony media APIs.
- **Key Consumers:** `servo-script`, `servo`.
- **Audit Status:** VERIFIED.

### 53. `servo-bluetooth` & 54. `servo-bluetooth-traits`
- **Paths:** `components/bluetooth`, `components/shared/bluetooth`
- **Purpose [VERIFIED]:** Web Bluetooth API support using `btleplug`.
- **Audit Status:** VERIFIED.

### 55. `servo-webgl`
- **Path:** `components/webgl` (`Cargo.toml`)
- **Purpose [VERIFIED]:** WebGL 1 & 2 state management and GPU texture upload.
- **Key Dependencies:** `gleam`, `glow`.
- **Audit Status:** VERIFIED.

### 56. `servo-webgpu`, 57. `servo-webgpu-traits`, 58. `servo-script-webgpu`
- **Paths:** `components/webgpu`, `components/shared/webgpu`, `components/script_webgpu`
- **Purpose [VERIFIED]:** W3C WebGPU specification implementation backed by `wgpu-core`.
- **Relevance to Asteria [RECOMMENDATION]:** Asteria is built entirely on `wgpu 22.1`. These crates illustrate how to expose `wgpu` capabilities safely to untrusted Web APIs.
- **Audit Status:** VERIFIED.

### 59. `servo-webxr` & 60. `servo-webxr-api`
- **Paths:** `components/webxr`, `components/shared/webxr`
- **Purpose [VERIFIED]:** WebXR Device API integration with OpenXR on Windows/Linux.
- **Audit Status:** VERIFIED.

---

## Tier 10: System Support, Allocators, Hang Monitors & Profiling

### 61. `servo-allocator`
- **Path:** `components/allocator` (`Cargo.toml`)
- **Purpose [VERIFIED]:** Configures `tikv-jemallocator` as the global memory allocator.
- **Audit Status:** VERIFIED.

### 62. `servo-background-hang-monitor` & 63. `...-api`
- **Paths:** `components/background_hang_monitor`, `components/shared/background_hang_monitor`
- **Purpose [VERIFIED]:** Heartbeat monitor thread that flags script thread lockups or infinite loops.
- **Audit Status:** VERIFIED.

### 64. `servo-default-resources`
- **Path:** `components/default-resources` (`Cargo.toml`)
- **Purpose [VERIFIED]:** Bundles embedded resources (UA CSS, icons) directly into the binary at link time.
- **Audit Status:** VERIFIED.

### 65. `servo-deny-public-fields`
- **Path:** `components/deny_public_fields` (`Cargo.toml`)
- **Purpose [VERIFIED]:** Compiler lint ensuring structs retain private fields for encapsulation.
- **Audit Status:** VERIFIED.

### 66. `servo-malloc-size-of`
- **Path:** `components/malloc_size_of` (`Cargo.toml`)
- **Purpose [VERIFIED]:** Heap memory measurement trait (`MallocSizeOf`) calculating live memory footprint of complex DOM and style trees.
- **Relevance to Asteria [RECOMMENDATION]:** Essential for identifying memory leaks and unbounded caches in Asteria.
- **Audit Status:** VERIFIED.

### 67. `servo-metrics`
- **Path:** `components/metrics` (`Cargo.toml`)
- **Purpose [VERIFIED]:** Performance metrics collector for page load times, layout ticks, and frame rates.
- **Audit Status:** VERIFIED.

### 68. `servo-profile` & 69. `servo-profile-traits`
- **Paths:** `components/profile`, `components/shared/profile`
- **Purpose [VERIFIED]:** Microsecond-precision stage timer and profiler across thread boundaries.
- **Audit Status:** VERIFIED.

### 70. `servo-timers`
- **Path:** `components/timers` (`Cargo.toml`)
- **Purpose [VERIFIED]:** High-precision thread timer queue for `setTimeout` and `setInterval`.
- **Audit Status:** VERIFIED.

### 71. `servo-tracing`
- **Path:** `components/servo_tracing` (`Cargo.toml`)
- **Purpose [VERIFIED]:** Tracing attribute macros emitting structured Perfetto traces.
- **Audit Status:** VERIFIED.

### 72. `servo-wakelock`
- **Path:** `components/wakelock` (`Cargo.toml`)
- **Purpose [VERIFIED]:** Screen wakelock support for media playback.
- **Audit Status:** VERIFIED.

---

## Tier 11: Integration & Unit Test Packages

### 73. `malloc_size_of_tests` (`tests/unit/malloc_size_of`)
- **Purpose [VERIFIED]:** Validates accurate heap allocation reporting for complex nested structures.

### 74. `profile_tests` (`tests/unit/profile`)
- **Purpose [VERIFIED]:** Validates profiler measurement accuracy and serialization.

### 75. `script_tests` (`tests/unit/script`)
- **Purpose [VERIFIED]:** Unit tests for DOM node operations, events, and script bindings.

### 76. `style_tests` (`tests/unit/style`)
- **Purpose [VERIFIED]:** Unit tests for Stylo integration and CSS selector matching.

### 77. `servo-capi-tests` (`tests/capi`)
- **Purpose [VERIFIED]:** Tests for C ABI library loading, engine initialization, and headless rendering.
