# Subsystem Report: 01 Embedding, WebView & Shell Architecture

> **Finding ID Prefix:** `SUB-EMB-`  
> **Target Subsystem:** Embedding, WebView Lifecycle, Window Presentation & Shell Integration  
> **Audited Crates:** `components/servo` (`servo`), `components/shared/embedder` (`servo-embedder-traits`), `ports/servoshell` (`servoshell`)  
> **Evidence Status:** VERIFIED against source tree (`7ca9f80268a1d76e698560db82463e7960aa7845`)  
> **Relevance to Asteria:** CRITICAL (Re-architecting Asteria's monolithic shell into an embeddable engine)

---

## A. Purpose and Design

The Embedding Subsystem decouples Servo's internal multi-threaded actor pipeline from external host applications (such as desktop windowing managers, native mobile shells, automated headless harnesses, or foreign C ABI consumers).

### Architectural Invariants:
1. **Engine as a Passive Event Consumer:** `Servo` does not own the host OS thread loop. It integrates via an `EventLoopWaker` trait, allowing the host application to control scheduling while calling `Servo::spin_event_loop()` when wakeups occur.
2. **Handle-Based WebViews:** A `WebView` represents a discrete browser tab or iframe viewport. It is exposed as an `Rc<RefCell<WebViewInner>>` handle. Cloning produces another handle; dropping the final handle triggers automated teardown of all backend pipelines.
3. **Decoupled Presentation:** Rendering is abstracted through a `RenderingContext` trait. Painting occurs only when the host responds to `WebViewDelegate::notify_new_frame_ready()`, allowing embedders to synchronize presentation with native UI frameworks (`egui`, Qt, Cocoa, Android SurfaceView).

---

## B. Source Map & File Responsibilities

| File Path | Primary Responsibilities |
| :--- | :--- |
| `components/servo/lib.rs` | Public export surface of the engine; re-exports embedding types and delegates. |
| `components/servo/servo.rs` | `ServoBuilder`, `Servo`, `ServoInner`, `spin_event_loop()`, `EmbedderMessageSelector`. |
| `components/servo/webview.rs` | `WebViewBuilder`, `WebView`, `WebViewInner`, viewport geometry, input event routing. |
| `components/servo/webview_delegate.rs` | Callbacks trait `WebViewDelegate` (navigation, dialogs, title, cursor, repaint signals). |
| `components/servo/servo_delegate.rs` | Global callbacks trait `ServoDelegate` (engine errors, shutdown, DevTools server status). |
| `components/shared/embedder/lib.rs` | Decoupled trait `EventLoopWaker`, message types `EmbedderMsg`, `Cursor`, `Theme`. |
| `ports/servoshell/main.rs` | Binary entry point; handles Windows console attachment (`AttachConsole`). |
| `ports/servoshell/desktop/app.rs` | Reference host event loop implementing `winit 0.30` `ApplicationHandler`. |
| `ports/servoshell/desktop/headed_window.rs` | Manages native window surfaces, HiDPI scaling, and GL swapchains. |
| `ports/servoshell/desktop/gui.rs` | Browser chrome UI rendered via `egui 0.34` (URL bar, tab management, back/forward). |

---

## C. Important Types and Public APIs

### 1. `ServoBuilder` & `Servo` (`components/servo/servo.rs:970–1171`)
- **`ServoBuilder`:** Builder for configuring global preferences, cache paths, proxy settings, user content managers, and the mandatory `EventLoopWaker`.
- **`Servo`:** Owns the engine background threads. Exposes:
  - `Servo::spin_event_loop(&self)`: Pumps inbound channels across Embedder, Net, Constellation, and Paint without blocking.
  - `Servo::setup_logging(&self)`: Routes internal log messages to constellation crash reporters.
  - `Servo::forward_accessibility_action(&self, action: ActionRequest)`: AccessKit integration.

### 2. `WebViewBuilder` & `WebView` (`components/servo/webview.rs:52–220`)
- **`WebViewBuilder::new(rendering_context, servo)`:** Binds a viewport to a graphics context.
- **`WebView` Methods:**
  - `load(&self, url: ServoUrl)`: Initiates top-level navigation.
  - `paint(&self)`: Invokes WebRender to paint into the associated `RenderingContext`.
  - `notify_input_event(&self, event: InputEvent)`: Forwards keyboard, mouse, touch, and scroll events.
  - `set_viewport_details(&self, details: ViewportDetails)`: Updates dimensions and HiDPI scale factor.
  - `go_back(&self)` / `go_forward(&self)`: Traverses session history.

### 3. `RenderingContext` (`components/shared/paint/rendering_context.rs`)
Abstracts surface buffers:
- `WindowRenderingContext`: Manages native OpenGL/EGL swapchain buffers.
- `OffscreenRenderingContext`: Headless FBO buffers for offscreen rasterization and screenshots.
- `SoftwareRenderingContext`: CPU-side pixel buffer fallback.

### 4. `WebViewDelegate` (`components/servo/webview_delegate.rs`)
Host callbacks:
- `notify_new_frame_ready(webview)`: Signals that layout/paint has produced a frame ready to swap.
- `notify_title_changed(webview, title)`: Document `<title>` update.
- `notify_url_changed(webview, url)`: Navigation commit update.
- `notify_cursor_changed(webview, cursor)`: CSS `cursor` style update.
- `show_embedder_control(webview, control)`: Alert, confirm, prompt, file picker, or color picker requests.

---

## D. Internal Execution: WebView Creation to Presentation

```mermaid
sequenceDiagram
    autonumber
    actor Host as Host Shell (winit/App)
    participant SB as ServoBuilder
    participant S as ServoInner
    participant WB as WebViewBuilder
    participant WV as WebView (Inner)
    participant C as Constellation
    participant P as Paint Subsystem
    participant GL as RenderingContext (GL)

    Host->>SB: build(event_loop_waker)
    SB->>S: Spawns Constellation & Net Threads
    SB-->>Host: Servo Instance

    Host->>WB: new(RenderingContext, Servo)
    WB->>P: register_rendering_context() -> PainterId
    WB->>WV: WebView::new(WebViewId)
    WV->>P: add_webview(viewport_details)
    WV->>C: send(NewWebViewDetails)
    WB-->>Host: WebView Handle

    Host->>WV: load("https://example.com")
    WV->>C: send(LoadUrl)
    Note over C,P: Constellation starts pipeline, Script parses DOM, Layout builds display list
    P->>Host: waker.wake() -> wakes host event loop
    Host->>S: Servo::spin_event_loop()
    S->>P: perform_updates()
    S->>Host: delegate.notify_new_frame_ready(WebView)

    Host->>WV: WebView::paint()
    WV->>P: paint_webview() -> WebRender render
    Host->>GL: RenderingContext::present() (SwapBuffers)
```

---

## E. Cross-Component Relationships & IPC

1. **Weak Handle Map:** `ServoInner` maintains `RefCell<FxHashMap<WebViewId, Weak<RefCell<WebViewInner>>>>`. The engine does not keep strong references to WebViews; when the embedder drops its handle, `WebViewInner::drop` informs Constellation and Paint, automatically garbage-collecting the pipeline forest.
2. **Selector Multiplexing:** `EmbedderMessageSelector` uses `crossbeam_channel::Select` across 3 unbounded channels:
   - `embedder_receiver`: Dialog responses, focus shifts.
   - `net_embedder_receiver`: File picker and auth requests from network stack.
   - `constellation_embedder_receiver`: Title changes, frame ready, URL navigations.

---

## F. Engineering Characteristics & Hazards

1. **Re-Entrant Borrow Prevention:** Calling embedder callbacks while an internal `RefCell` is borrowed causes panics. Servo mitigates this in `servo.rs:366–380` by collecting strong handles into a temporary `Vec<WebView>` and dropping internal collection borrows before dispatching delegate calls.
2. **Zero-Lock Event Pumping:** `spin_event_loop()` uses `try_recv()` and non-blocking selects; the embedder main thread is guaranteed never to sleep or freeze inside engine code.

---

## G. Asteria Engineering Implications

### Comparative Diagnosis of Asteria:
- **Current Asteria Defect:** Asteria mixes windowing (`winit 0.29`), chrome UI (`shell_ui.rs`), and engine reflow inside a single monolithic `Shell` struct in `src/shell.rs`.
- **Architectural Solution for Asteria:**
  1. Extract `WebView` and `WebViewBuilder` into an isolated abstraction.
  2. Implement an `EventLoopWaker` pattern so Asteria's engine does not control `winit`'s event loop directly.
  3. Replace Asteria's handwritten UI chrome rendering in `shell_ui.rs` with an immediate-mode UI crate (`egui`), rendering chrome above the webview surface exactly as demonstrated in `servoshell/desktop/gui.rs`.

---

## H. Confidence and Coverage

- **Inspected:** `components/servo/lib.rs`, `components/servo/servo.rs`, `components/servo/webview.rs`, `ports/servoshell/desktop/app.rs`, `ports/servoshell/desktop/headed_window.rs`.
- **Verified:** Handle lifecycle, delegate dispatching, `spin_event_loop` multiplexing, and presentation sequencing.
- **Audit Status:** Complete (100% verified against code).
