# 05 Public APIs, Extension Points & Reusability Matrix

> **Evidence Status:** VERIFIED  
> **Source Commit:** `7ca9f80268a1d76e698560db82463e7960aa7845`  
> **Primary References:** `components/servo/lib.rs:5–150`, `components/servo/servo.rs:970–1171`, `components/servo/webview.rs:52–220`, `components/servo/examples/winit_minimal.rs:1–194`  
> **Audited Surface:** Servo Public Rust API (`servo = "=0.7.0"`) & C-ABI (`ffi/capi`)

---

## 1. Public Embedding Surface Overview

Servo's public embedding interface is exposed exclusively at the root of `components/servo/lib.rs`. It provides an event-loop agnostic, handle-driven API that enables applications to host web content while keeping engine internals fully encapsulated.

### Primary Abstraction Hierarchy:
- **`EventLoopWaker` Trait:** Allows Servo background threads to trigger wakeups in the host event loop.
- **`ServoBuilder` & `Servo`:** Manages global engine lifetime, shared caches, preferences, and actor threads.
- **`RenderingContext` Trait:** Bridges the host window surface (OpenGL, EGL, Surfman) or offscreen framebuffers to the engine.
- **`WebViewBuilder` & `WebView`:** Represents an individual browser tab or viewport. Owns navigation, sizing, and input event forwarding.
- **`ServoDelegate` & `WebViewDelegate`:** Trait-based callback hooks notifying the host of frame availability, navigation status, modal dialogs, and errors.

---

## 2. Comprehensive Reusability & Extension Matrix

| Capability / Interface | Source Evidence | Purpose & Responsibility | Dependencies & Prerequisites | Stability & Maintenance Assessment | Relevant Asteria Opportunity | Verification Strategy |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`ServoBuilder` / `Servo`** | `components/servo/servo.rs:970` | Engine bootstrap, actor thread orchestration, background I/O spawning. | Requires an `EventLoopWaker` implementation. | Stable; public API entry point. | Clean separation between engine backend and shell UI in Asteria. | Verified in `examples/winit_minimal.rs:78–82`. |
| **`WebView` Handle** | `components/servo/webview.rs:52` | Viewport abstraction for navigation, painting, and input forwarding. | Requires a `RenderingContext` and `Servo`. | Stable; reference-counted handle (`Rc<RefCell<WebViewInner>>`). | Multi-tab and multi-window viewport support in Asteria. | Verified in `examples/winit_minimal.rs:94–101`. |
| **`EventLoopWaker`** | `components/shared/embedder/lib.rs` | Cross-thread event loop signaling (`waker.wake()`). | Host event loop proxy (e.g. `winit::event_loop::EventLoopProxy`). | Highly stable; pure trait. | Decouples engine actor threads from Asteria's `winit` event loop. | Verified in `examples/winit_minimal.rs:168–190`. |
| **`RenderingContext`** | `components/shared/paint/rendering_context.rs` | Abstraction over OpenGL, EGL, and offscreen surfaces. | OpenGL ES / Desktop GL hardware context; `raw-window-handle`. | Stable; requires OpenGL-compatible swapchains. | Adapting Asteria's `wgpu` surface to an abstract rendering target. | Verified in `examples/winit_minimal.rs:71–76`. |
| **`WebViewDelegate`** | `components/servo/webview_delegate.rs` | Notifies host when a new frame is ready to paint, title changes, etc. | Host-provided callback struct. | Stable; clean callback contract. | Decouples presentation timing from internal layout ticks. | Verified in `examples/winit_minimal.rs:43–47`. |
| **`ProtocolRegistry`** | `components/net/protocols/mod.rs` | Custom URL scheme registration (`asset://`, `app://`). | Thread-safe scheme handlers implementing `ProtocolHandler`. | Extension point; avoids internal engine forks. | Custom local resource loading in Asteria (e.g. extension schemes). | Verified in `components/servo/lib.rs:136–148`. |
| **`UserContentManager`** | `components/servo/user_content_manager.rs` | Injects custom user scripts and stylesheets into pages. | Evaluated at document creation. | Stable; standard webview capability. | Allows Asteria to inject user scripts or devtools shims cleanly. | Verified in `components/servo/lib.rs:116`. |
| **`Taffy` Integration Pattern** | `components/layout/taffy/` | CSS Flexbox and Grid layout solver. | Pure Rust crate `taffy = "0.14.0"`. | Upstream crate; zero maintenance burden on browser core. | **DIRECT IMPORT:** Replaces Asteria's handwritten flex solver with battle-tested code. | Verified in `components/layout/taffy/mod.rs:30–60`. |
| **`AppUnits` (`Au`)** | `app_units = "0.7"` | Integer coordinate system ($1/60\text{th}$ px) eliminating rounding seams. | Pure Rust numeric crate. | Extremely stable; industry standard (used in Gecko and Servo). | **DIRECT IMPORT:** Fixes sub-pixel rounding and layout drift in Asteria. | Verified in `components/layout/lib.rs:37`. |
| **`keyboard-types`** | `keyboard-types = "0.8.3"` | Typed cross-platform keyboard codes, modifiers, and locations. | Pure Rust enum crate. | Standard W3C KeyboardEvent mapping. | **DIRECT IMPORT:** Standardizes Asteria's keyboard shortcuts and input handling. | Verified in `components/servo/lib.rs:66–68`. |

---

## 3. What Can Be Integrated Through Public APIs vs. Internal Access

### Documented Public Embedding Surface:
- Creating engines and browser viewports (`ServoBuilder`, `WebViewBuilder`).
- Navigating URLs (`WebView::load()`), back/forward traversal (`go_back()`, `go_forward()`).
- Pumping messages without blocking (`Servo::spin_event_loop()`).
- Forwarding user input events (`WebView::notify_input_event()`).
- Intercepting network requests (`WebViewDelegate::load_web_resource()`).
- Executing ad-hoc JavaScript (`WebView::evaluate_javascript()`).
- Registering custom URL protocols (`ProtocolRegistry`).

### Boundaries Requiring Internal Access (Forbidden / Encapsulated):
- **Direct DOM Tree Access:** Host embedders cannot directly traverse `Node` or `Element` pointers in memory; DOM nodes reside exclusively on the `ScriptThread`. Communication must pass through JavaScript evaluation or CDP DevTools protocols.
- **Layout Tree Interception:** The `BoxTree` and `FragmentTree` are private to `servo-layout`; only compiled `BuiltDisplayList` structures leave layout.
- **Synchronous Layout Invalidation:** External callers cannot trigger synchronous reflow on demand without going through the script thread's event loop.

---

## 4. Architectural & Maintenance Hazards

1. **Avoid Forking Internal Crates:** Forking `components/script` or `components/layout` directly into Asteria would import thousands of dependencies (including SpiderMonkey `mozjs` C++ build requirements, HarfBuzz C++ builds, and WebRender C dependencies).
2. **Prefer Clean External Crate Adoption:** Asteria should adopt the modular standalone crates that Servo helped incubate:
   - `taffy` for Flexbox/Grid layout.
   - `app_units` for integer coordinate stability.
   - `euclid` for strongly typed coordinate spaces (`CSSPixel` vs `DevicePixel`).
   - `keyboard-types` for W3C input event representations.
   - `string_cache` for interned string atoms.
3. **Graphics Context Differences:** Servo's public embedding API assumes an OpenGL/EGL surface for WebRender (`WindowRenderingContext`). Because Asteria is built natively on `wgpu 22.1`, embedding Servo wholesale would require maintaining an EGL/WGL context alongside Asteria's WGPU swapchains.
