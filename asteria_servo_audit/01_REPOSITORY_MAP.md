# 01 Repository Map: Servo Engine Architecture & Topology

> **Evidence Status:** VERIFIED  
> **Git Revision:** `7ca9f80268a1d76e698560db82463e7960aa7845` (Branch: `main`)  
> **Target Package Version:** Servo `0.7.0` (Rust Edition 2024, pinned to toolchain `1.97.1`)  
> **Host Audit Environment:** Windows 11 / Rust 1.96.1 / Cargo 1.96.1 / Git 2.52.0  
> **Audit Output Root:** `z:\1mportant stuff\Codes written by me (New)\asteria_servo_audit`

---

## 1. Top-Level Repository Structure

The local Servo checkout (`z:\1mportant stuff\Codes written by me (New)\servo`) is organized into distinct functional tiers that separate engine implementation, cross-component interface abstractions, browser shells, C-ABI bindings, build tooling, and test suites.

```
servo/
├── .cargo/                     # Cargo runner & target-specific linker configurations
├── .config/                    # Tool configurations (e.g., git-cliff, nextest)
├── .github/                    # CI/CD workflows, issue templates, dependabot rules
├── components/                 # Core engine implementations and trait abstractions
│   ├── shared/                 # 16 interface/traits crates decoupling major subsystems
│   ├── media/                  # Audio/video pipelines, GStreamer, and media thread backends
│   ├── config/                 # Command-line options and runtime preferences
│   ├── script/                 # Script execution, DOM tree, HTML parsers, WebIDL
│   ├── script_bindings/        # WebIDL code generation pipeline and JS reflection
│   ├── layout/                 # Flow tree construction, block/inline layout, Taffy flex/grid
│   ├── paint/                  # WebRender display list generation and render tasks
│   ├── net/                    # HTTP/1.1 & HTTP/2 networking, Fetch spec, caching, cookies
│   ├── constellation/          # Central orchestrator: pipelines, frame trees, tab lifecycles
│   ├── fonts/                  # Font caching, discovery, HarfBuzz text shaping
│   ├── canvas/                 # Canvas 2D and WebGL rendering thread
│   ├── webgl/                  # WebGL state machine and GL bindings
│   ├── webgpu/                 # WebGPU implementation on wgpu-core
│   ├── webxr/                  # WebXR device API and OpenXR/mock backends
│   ├── devtools/               # Chrome DevTools Protocol (CDP) server implementation
│   ├── storage/                # LocalStorage, SessionStorage, SQLite persistence
│   ├── servo/                  # Public Rust embedding API (ServoBuilder, WebView, delegates)
│   └── ...                     # System utilities: allocator, timers, metrics, geometry
├── ports/                      # Application shells embedding Servo
│   └── servoshell/             # Default binary: desktop (winit + egui) & mobile (EGL/Android/OHOS)
├── ffi/                        # Foreign Function Interfaces
│   └── capi/                   # `servo-capi`: C dynamic library (`servo_capi.dll` / `libservo_capi.so`)
├── resources/                  # Bundled assets: UA stylesheets, error HTML, media controls, shaders
├── python/                     # Python-driven build orchestrator (`mach`) & WPT test runners
├── support/                    # Linter tooling (`crown` AST plugin), packaging helpers
├── tests/                      # Conformance & unit testing
│   ├── unit/                   # Unit test crates (malloc_size_of, profile, script, style)
│   ├── capi/                   # C-API integration tests
│   └── wpt/                    # Web Platform Tests integration harness
├── Cargo.toml                  # Workspace root manifest (Resolver 2, workspace dependencies)
├── Cargo.lock                  # Pinned dependency graph (307 KB lockfile)
├── rust-toolchain.toml         # Toolchain pin: channel `1.97.1` with llvm-tools and rustc-dev
└── pyproject.toml / uv.lock    # Python workspace tooling powered by Astral `uv`
```

---

## 2. Cargo Workspace Configuration

### 2.1 Workspace Manifest Attributes
Verified in `Cargo.toml:1–32`:
- **Resolver:** Version `"2"` (per-feature unification decoupled across normal/build dependencies).
- **Workspace Package Version:** `0.7.0`.
- **Rust Edition:** `2024`.
- **Minimum Supported Rust Version (MSRV):** `1.88.0` (`rust-version = "1.88.0"`, explicitly skipping 1.90 due to rust-lang/rust#147265).
- **Toolchain Pin:** `rust-toolchain.toml` specifies channel `"1.97.1"` with components `clippy`, `rustfmt`, `rust-src`, and compiler internal crates `rustc-dev` and `llvm-tools` (used for `support/crown`).

### 2.2 Workspace Members
The workspace defines a virtual root that delegates to 77 member packages:
```toml
# Cargo.toml:3-13
members = [
    "components/media/examples",
    "components/svg",
    "components/xpath",
    "ffi/capi",
    "ports/servoshell",
    "tests/capi",
    "tests/unit/*",
]
default-members = ["ports/servoshell"]
exclude = [".cargo", "support/crown"]
```
*Note on Workspace Discovery:* All 39 crates in `components/` and 16 crates in `components/shared/` are linked into the workspace dependency graph as path-referenced dependencies under `[workspace.dependencies]` (lines 306–378), ensuring atomic workspace builds and version locking.

### 2.3 Compilation Profiles
Verified in `Cargo.toml:392–447`:
- `dev`: Optimized specifically for heavy components. `crypto-bigint` is compiled at `opt-level = 3` to prevent Web Platform Test timeouts during RSA keygen. `servo-script` limits `codegen-units = 16` to circumvent Windows 4GB `.rlib` limits (rust-lang/rust#151184).
- `checked-release`: Release optimizations (`opt-level = 3`) with `debug-assertions = true` and `overflow-checks = true` for CI performance testing.
- `medium`: Fast incremental rebuild profile with `opt-level = 2` and `debug = "line-tables-only"`.
- `production`: Fully stripped, LTO (`lto = true`), single codegen unit (`codegen-units = 1`), optimized for size (`opt-level = "s"`).

---

## 3. Major Directory Responsibilities

### 3.1 `components/` vs `components/shared/` (The Traits Isolation Architecture)
The fundamental architectural discipline in Servo is **trait isolation**. Concrete engine subsystems do not depend directly on concrete implementations of other subsystems. Instead:
- Every major subsystem has a crate in `components/shared/<subsystem>` exporting traits, identifiers, IPC channels, and message types.
- The heavy implementation resides in `components/<subsystem>`.

| Subsystem | Concrete Implementation Crate | Interface / Traits Crate (`components/shared/`) | Primary Responsibility |
| :--- | :--- | :--- | :--- |
| **Constellation** | `components/constellation` (`servo-constellation`) | `components/shared/constellation` (`servo-constellation-traits`) | Manages the forest of frame trees, pipelines, navigation history, and process coordination. |
| **Script & DOM** | `components/script` (`servo-script`) | `components/shared/script` (`servo-script-traits`) | Houses DOM arena, JS engine integration, HTML5 event loop, microtasks, Web APIs. |
| **Networking** | `components/net` (`servo-net`) | `components/shared/net` (`servo-net-traits`) | Implements WHATWG Fetch, resource caches, cookies, HTTP connections. |
| **Layout** | `components/layout` (`servo-layout`) | `components/shared/layout` (`servo-layout-api`) | Builds layout box tree, performs text/inline line breaking, block layout, Taffy flexbox/grid. |
| **Paint** | `components/paint` (`servo-paint`) | `components/shared/paint` (`servo-paint-api`) | Translates layout boxes to WebRender display list items; dispatches render commands. |
| **Embedder** | *Implemented by embedders* | `components/shared/embedder` (`servo-embedder-traits`) | Defines embedder callbacks (`WebViewDelegate`, `ServoDelegate`, window methods). |
| **Fonts** | `components/fonts` (`servo-fonts`) | `components/shared/fonts` (`servo-fonts-traits`) | Font matching, fallback font lists, HarfBuzz shaping wrappers. |
| **DevTools** | `components/devtools` (`servo-devtools`) | `components/shared/devtools` (`servo-devtools-traits`) | CDP server communicating DOM inspection, CSS inspection, and console logs. |
| **Storage** | `components/storage` (`servo-storage`) | `components/shared/storage` (`servo-storage-traits`) | LocalStorage / SessionStorage backends backed by SQLite (`rusqlite`). |
| **Hang Monitor** | `components/background_hang_monitor` | `components/shared/background_hang_monitor` | Thread heartbeat monitoring to detect uncooperative script loops. |

### 3.2 `ports/servoshell` (The Reference Browser Shell)
Located in `ports/servoshell`, this is a fully functional browser application demonstrating how to embed Servo:
- Uses `winit 0.30` (`winit::application::ApplicationHandler`) for cross-platform windowing and native event loops.
- Uses `egui 0.34` (`egui-winit`, `egui_glow`) for rendering browser chrome (address bar, back/forward buttons, tab bar, status overlay).
- Handles headless mode (`desktop/headless_window.rs`) for automated CLI rendering and screenshots.
- Houses mobile entry points for Android (`egl/android`) and OpenHarmony (`egl/ohos`).

### 3.3 `ffi/capi` (C Foreign Function Interface)
- Compiles into a C-compatible dynamic library (`servo_capi.dll` / `libservo_capi.so`).
- Generates C headers via `cargo-c` (`package.metadata.capi.header`).
- Exposes C ABI handles for creating engines, WebViews, sending URLs, injecting input events, and receiving pixel buffers.

### 3.4 `resources/` (Embedded Web Engine Assets)
Contains static assets mandatory for web standards compliance:
- `user-agent.css`: Baseline WHATWG user agent default styles for all HTML elements.
- `quirks-mode.css`: Quirks mode CSS overrides for legacy pages.
- `presentational-hints.css`: Mapping HTML attributes (`width`, `align`, `bgcolor`) to CSS properties.
- `scrollbars.css`: Default CSS styling for engine-drawn scrollbars.
- `error.html`: Net error page templates.
- `media-controls.css` & `media-controls.js`: Built-in HTML5 `<video>` / `<audio>` controls UI.

---

## 4. Binary and Library Entry Points

| Entry Point | Source Path | Target Kind | Primary Role |
| :--- | :--- | :--- | :--- |
| **`servoshell` Binary** | `ports/servoshell/main.rs` | `bin` | Shell entry point: initializes Windows console, invokes `servoshell::desktop::cli::main()`. |
| **`servoshell` Library** | `ports/servoshell/lib.rs` | `lib` | Shared shell logic, crypto initialization (`rustls::crypto::aws_lc_rs`), tracing setup. |
| **`servo` Engine Library** | `components/servo/lib.rs` | `rlib` | The primary public embedding interface for Rust consumers (`Servo`, `WebView`, `ServoBuilder`). |
| **`servo_capi` Shared Library** | `ffi/capi/lib.rs` | `cdylib` | C ABI interface exporting `extern "C"` functions for embedding from C, C++, Python, or C#. |
| **Content Process Entry Point** | `components/servo/servo.rs:run_content_process()` | Function | Spawned worker entry point when `multiprocess` feature is enabled. |

---

## 5. Public APIs and Embedding Surfaces

The primary Rust embedding interface is exported directly from `components/servo/lib.rs:60–123`:
1. **Engine Lifecycle:**
   - [`ServoBuilder`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/servo/components/servo/servo.rs): Configures global preferences, cache directories, and user content scripts.
   - [`Servo`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/servo/components/servo/servo.rs): The main engine handle; owns background threads and event dispatching.
   - [`EventLoopWaker`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/servo/components/shared/embedder/lib.rs): Host trait enabling Servo to wake the host event loop across thread boundaries.
   - `Servo::spin_event_loop(&mut self)`: Called by the host event loop to pump incoming IPC messages and wakeups.
2. **WebView Representation:**
   - [`WebViewBuilder`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/servo/components/servo/webview.rs): Configures an individual browser viewport, attaching a `RenderingContext` and `WebViewDelegate`.
   - [`WebView`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/servo/components/servo/webview.rs): Handle for navigating (`load()`), resizing, painting (`paint()`), and forwarding events (`notify_input_event()`).
3. **Graphics & Presentation Context:**
   - [`RenderingContext`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/servo/components/shared/paint/rendering_context.rs): Trait abstracting swapchains and frame presentation.
   - Pre-packaged implementations: `WindowRenderingContext` (OpenGL/EGL surfaces), `OffscreenRenderingContext` (headless GPU buffers), `SoftwareRenderingContext` (CPU pixel buffers).
4. **Callbacks & Delegation:**
   - [`ServoDelegate`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/servo/components/servo/servo_delegate.rs): Global notifications (crashes, permission requests, hang reports).
   - [`WebViewDelegate`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/servo/components/servo/webview_delegate.rs): Viewport-specific callbacks (`notify_new_frame_ready`, title change, URL change, cursor change, alert/prompt dialogs).

---

## 6. Feature Flags and Conditional Compilation

Verified in `components/servo/Cargo.toml:18–80` and `ports/servoshell/Cargo.toml:43–90`:

| Feature Name | Default in Servo | Default in Shell | Purpose & Effect |
| :--- | :--- | :--- | :--- |
| **`multiprocess`** | Yes | Yes | Enables process isolation between Constellation (UI) and Script/Layout (Content processes). Uses `ipc-channel`. |
| **`js_jit`** | Yes | Yes | Enables IonMonkey / Baseline JIT compilers in SpiderMonkey (`mozjs`). |
| **`bundled`** | Yes | Yes | Bundles `baked-in-resources` (UA css, error pages) and `bundled_freetype` directly into binary. |
| **`webgpu`** | In `default_web_features` | Yes | Exposes W3C WebGPU API in DOM via `wgpu-core`. |
| **`webgl`** | In `default_web_features` | Yes | Exposes WebGL 1.0 & 2.0 via `gleam` and OpenGL ES. |
| **`webxr`** | In `default_web_features` | Yes | Exposes WebXR device API; links OpenXR or headless VR mocks. |
| **`clipboard`** | Yes | Yes | System clipboard integration via `arboard`. |
| **`media-gstreamer`**| No | Platform-specific | Hardware-accelerated media decoding via GStreamer 1.18+. |
| **`crown`** | No | No | Enables the custom AST linter plugin `crown` during build checks. |
| **`no-wgl`** | Target-specific | Windows target | Disables legacy Windows WGL in favor of ANGLE EGL on Windows CI. |

---

## 7. Major External Engine Dependencies

Servo delegates several large, spec-heavy domains to dedicated external projects:

1. **Stylo (`https://github.com/servo/stylo` rev `0cb50925b0259e53b953d72d5def3a24caa9ab58`):**
   - The parallel CSS engine co-developed with Mozilla for Firefox.
   - Handles CSS tokenization, `@media` queries, selector matching (`selectors` crate), cascade calculation, and computed style structs.
2. **WebRender (`webrender = "0.70"`):**
   - Retained-mode GPU display list renderer developed for Firefox.
   - Accepts display lists containing transformed rectangles, text runs, images, border radii, and box shadows, rendering them via instanced GPU shaders.
3. **SpiderMonkey (`mozjs = "=0.26.7"`):**
   - Mozilla's C++ JavaScript runtime engine.
   - Integrated via raw C++ bindings, garbage collection roots, and traced GC pointers (`JS<T>`, `Root<T>`).
4. **Taffy (`taffy = "0.14.0"`):**
   - CSS Flexbox and CSS Grid layout algorithms written in pure Rust.
   - Used by `servo-layout` to calculate complex flex item constraints and grid template tracks.
5. **Surfman (`surfman = "0.14.0"`):**
   - Cross-platform hardware graphics surface management (WGL on Windows, EGL on Linux/Android, CGL on macOS).
6. **HarfBuzz (`harfbuzz-sys = "0.8"`) & Skrifa (`0.46.2`):**
   - OpenType text shaping, bidirectional script reordering, and glyph metric querying.

---

## 8. Build System and Code Generation Pipeline

Servo utilizes a dual-layer build system:
1. **Python Driver (`mach`):**
   - Orchestrated via `mach.ps1` (Windows), `mach.bat`, and `python/mach_bootstrap.py`.
   - Uses Astral `uv` (`uv.lock`) to maintain an isolated virtual environment for Python tools without polluting host packages.
   - Commands include: `mach build`, `mach test-wpt`, `mach test-unit`, `mach package`.
2. **Cargo Engine:**
   - Underneath `mach`, standard `cargo build` executes the Rust compilation.
3. **WebIDL Code Generator (`components/script_bindings`):**
   - WHATWG standard WebIDL files located in `components/script_bindings/webidls/` describe all browser DOM interfaces (`Document.webidl`, `Element.webidl`, `Window.webidl`).
   - `components/script_bindings/codegen/codegen.py` parses these IDL files at build time (`build.rs`) and emits Rust structs, JS reflectors, callback stubs, and argument converters.
   - Generates tens of thousands of lines of boilerplate bridging mozjs values to Rust types.

---

## 9. Test Suite Organization

| Test Directory | Framework / Manifest | Scope & Purpose |
| :--- | :--- | :--- |
| **`tests/unit/`** | Standard `cargo test` | Targeted unit test crates for `script`, `style`, `profile`, and `malloc_size_of`. |
| **`tests/capi/`** | `tests/capi/Cargo.toml` | Tests C dynamic library initialization, webview creation, and evaluation stubs. |
| **`tests/wpt/`** | Web Platform Tests harness | Upstream W3C test repository running thousands of standard web compliance tests. |
| **`tests/html/`** | Visual reftests | HTML pages loaded by servoshell to verify layout rendering against expected png snapshots. |

---

## 10. Platform Abstraction Matrix

| Platform | Windowing & Shell | Surface / Graphics Context | Text / Font Rasterization | Media Backend |
| :--- | :--- | :--- | :--- | :--- |
| **Windows** | `winit` (`ports/servoshell/desktop`) | `surfman` (WGL or ANGLE EGL) + `glow` | DirectWrite (`dwrote`), FreeType | GStreamer (optional) |
| **Linux (X11/Wayland)** | `winit` (`ports/servoshell/desktop`) | `surfman` (EGL / GLX) + `glow` | Fontconfig (`yeslogic-fontconfig-sys`), FreeType | GStreamer |
| **macOS** | `winit` + Cocoa integration | `surfman` (CGL) + `glow` | CoreText (`objc2-core-text`), FreeType | AVFoundation / GStreamer |
| **Android** | Android Activity / JNI | Native EGL (`ports/servoshell/egl/android`) | FreeType (`bundled_freetype`) | GStreamer Android |
| **OpenHarmony (OHOS)** | NAPI / XComponent (`ports/servoshell/egl/ohos`)| Native EGL + OHOS VSync (`ohos-vsync`) | FreeType | OHOS Media (`ohos-media-sys`)|
