# 04 Servo Comparative Engineering Analysis

> **Document Identifier:** `AST-DOC-004`  
> **Audited Repository:** Asteria Browser Engine (`Asteria/Asteria/`)  
> **Comparative Engineering Reference:** Servo Browser Engine (`servo/`, Git `7ca9f80268a1d76e698560db82463e7960aa7845`) via Completed Forensic Audit (`asteria_servo_audit/`)  
> **Objective:** Evaluate Asteria's architecture against Servo's battle-tested patterns, establish which differences are defects, which are justified simplifications, and which represent transformative improvement opportunities.  

---

## 1. Master Subsystem Comparative Matrix

| Subsystem Dimension | Asteria Current Approach | Servo Audit Reference | Servo Engineering Approach | Architectural Verdict & Recommended Outcome |
| :--- | :--- | :--- | :--- | :--- |
| **Workspace & Crates** | Monolithic single crate (`asteria 0.1.0`) with 31 internal modules (`Cargo.toml:1-28`). | `02_CRATE_CATALOG.md:20-65`, `03_SYSTEM_ARCHITECTURE.md:77` | 77 modular crates across 11 tiers; strict trait isolation (`components/shared/*`). | **Refactor existing implementation** into a 7-crate Cargo workspace (`AST-ARC-001`, `AST-009`). |
| **Concurrency & Scheduling** | `ThreadedScheduler` (4 workers) over FIFO `mpsc`; detached from `TaskScheduler` (`src/scheduler.rs`). | `03_SYSTEM_ARCHITECTURE.md:78`, `subsystems/02_CONSTELLATION_SUPERVISOR.md` | Actor model with DAG deadlock prevention (`Can-Block-On`); Constellation supervisor. | **Adapt relevant Servo pattern**: Unify schedulers with bounded priority channels; avoid complex multi-process IPC. |
| **Coordinate Math** | Floating-point `f32` coordinates across geometry and layout (`src/layout.rs:18-78`). | `subsystems/05_LAYOUT_ENGINE_FLOW_TREE.md:40-52`, `AST-001` | Fixed-point `app_units::Au` integer coordinates ($1\text{px} = 60\text{Au}$). | **Integrate appropriate public interface**: Adopt `app_units = "0.7"` to eliminate sub-pixel rounding drift. |
| **Flexbox & Grid Layout** | Handwritten row flex solver (~400 LOC) and basic grid in `src/layout.rs`. | `subsystems/06_TAFFY_FLEX_GRID_DELEGATION.md:12-85`, `AST-002` | Delegates all flexbox and grid calculations to `taffy 0.14`. | **Replace implementation**: Delegate flex and grid calculations to pure Rust `taffy 0.14` (`AST-002`). |
| **Text Sizing & Shaping** | Hardcoded character-ratio lookup table `estimate_char_width_ratio` (`src/layout.rs:2534`). | `subsystems/10_TEXT_SHAPING_FONT_SERVICE.md:15-60` | Full text shaping via HarfBuzz (`harfbuzz-sys`) and FreeType font metrics. | **Correct identified defect**: Integrate `glyphon::FontSystem` into layout to measure actual typeface advances (`AST-DEF-003`). |
| **Style Cascade & Memory** | Monolithic `ComputedStyle` with 7 heap allocations; string-heavy cascade in `style.rs`. | `subsystems/04_STYLO_PARALLEL_CSS.md:30-90`, `AST-004` | Stylo segmented structs (`BoxData`, `ColorData`); interned `Atom`s; rule tree sharing. | **Refactor existing implementation**: Segment `ComputedStyle`; use `Arc<HashMap<Symbol, String>>` for custom properties. |
| **String Interning** | `src/interner.rs` exists with 140+ pre-seeded `Symbol` constants, but is **100% unused**. | `subsystems/04_STYLO_PARALLEL_CSS.md:45`, `AST-007` | Global string atom pool (`string_cache::Atom`) for tag names and CSS property IDs. | **Refactor existing implementation**: Activate existing `Symbol` interner across DOM and Style engines. |
| **Display List & GPU Redraw**| Immediate-mode vertex buffer rebuilding and CPU iteration on every redraw (`window.rs:747`). | `subsystems/07_WEBRENDER_GPU_COMPOSITOR.md:20-80`, `AST-005` | Retained display list; WebRender compositor updates spatial node uniforms on GPU during scroll. | **Adapt relevant Servo pattern**: Retain vertex buffers; pass `scroll_offset` as a GPU uniform to shaders (`AST-PERF-001`). |
| **Graphics Backend** | Modern `wgpu 22.1` hardware abstraction over Vulkan/Metal/DirectX 12 (`src/renderer/`). | `subsystems/07_WEBRENDER_GPU_COMPOSITOR.md:90-110` | WebRender on raw OpenGL / Glow via `surfman`. | **Retain Asteria's existing approach**: Preserve `wgpu` backend; do NOT adopt WebRender. |
| **DOM Tree Structure** | Flat `Vec<Node>` arena with zero-copy byte offsets into input slice (`src/dom.rs`). | `subsystems/03_SCRIPT_THREAD_DOM_ARENA.md:20-75`, `AST-012` | Pointer-based `MutNullableDom` nodes with SpiderMonkey GC tracing. | **Refactor existing implementation**: Retain index arena, but store owned `Symbol` tag names and text to enable DOM mutation. |
| **JavaScript Integration** | Completely missing (`FNOP.md` Phase 2 target). | `subsystems/03_SCRIPT_THREAD_DOM_ARENA.md:80-120` | Full Mozilla SpiderMonkey C++ engine (`mozjs`). | **Do not copy Servo**: Avoid C++ SpiderMonkey; adopt pure-Rust `boa_engine` or embedded QuickJS (`rquickjs`). |
| **Browser Shell UI** | Immediate-mode UI chrome drawn directly into webpage vertex buffers (`shell_ui.rs`). | `subsystems/01_EMBEDDING_AND_SHELL.md:30-70`, `AST-006` | Decoupled `egui` shell rendering into an isolated overlay pass over the web view. | **Refactor existing implementation**: Decouple chrome into a separate render pass or adopt `egui` (`AST-006`). |
| **Networking & HTTP** | Synchronous blocking HTTP/1.1 client with custom DNS/TCP/TLS (`src/net/`). | `subsystems/08_RESOURCE_THREADS_NETWORKING.md:20-85`, `AST-011` | Asynchronous streaming network pipeline on Tokio and Hyper (`ResourceThreads`). | **Adapt relevant Servo pattern**: Run streaming fetches on background threads; stream chunks into parser. |

---

## 2. In-Depth Comparative Analyses by Subsystem

### 2.1 Coordinate Mathematics & Precision (f32 vs. AppUnits)

#### Asteria's Current Approach
Asteria represents all dimensions, margins, paddings, borders, and positions as 32-bit floating-point numbers (`f32`) throughout `src/layout.rs:18-78` and `src/values.rs`.
```rust
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
```

#### Servo's Mechanism (`06_ASTERIA_INTEGRATION_GUIDE.md:67-77`, `AST-001`)
Servo represents all layout coordinates in **AppUnits** (`app_units::Au`), where $1\text{px} = 60\text{Au}$. Coordinates are stored as signed 32-bit integers (`i32`).
- Addition, subtraction, and margin collapsing are exact integer operations.
- Fractions such as $1/2\text{px}$ ($30\text{Au}$), $1/3\text{px}$ ($20\text{Au}$), $1/4\text{px}$ ($15\text{Au}$), and $1/5\text{px}$ ($12\text{Au}$) are represented with zero precision loss.
- Conversion to floating-point pixels occurs only at the final display list serialization boundary when generating vertex coordinates.

#### Engineering Assessment
- **Problem in Asteria:** Cumulative floating-point rounding errors cause sub-pixel hairline cracks between adjacent layout boxes, borders disappearing at certain viewport widths, and flex item underflows.
- **Applicability to Asteria:** Directly applicable with near-zero overhead. `app_units = "0.7"` is a tiny standalone crate with no external dependencies.
- **Recommendation:** **Integrate appropriate public interface (`app_units = "0.7"`).** Replace `f32` with `Au` in `src/layout.rs`. Convert `Au` to `f32` only when emitting `DisplayCommand` in `src/paint.rs`.

---

### 2.2 Multidimensional Layout: Flexbox and Grid Delegation

#### Asteria's Current Approach
Asteria implements custom handwritten solvers for Flexbox (`src/layout.rs:430-520`) and Grid (`src/layout.rs:1050-1280`).
- Flexbox only supports horizontal row flow (`flex-direction: row`).
- Lacks proper CSS Flexbox §9 multi-line flex item cross-axis sizing, baseline alignment, flex-grow normalization under min/max constraints, and intrinsic aspect ratio preservation.
- Flex container `gap` was previously hardcoded (`FLAWS.md:2.7`), and auto-width children fell back to hardcoded 200px (`FLAWS.md:2.8`).

#### Servo's Mechanism (`subsystems/06_TAFFY_FLEX_GRID_DELEGATION.md`, `AST-002`)
Servo delegates all Flexbox and CSS Grid layout algorithms to `taffy 0.14`.
- Servo's layout engine maps Stylo's computed style properties directly to Taffy's `Style` struct.
- Servo defines an adapter implementing `taffy::TraversePartialTree` and `taffy::LayoutPartialTree` (`components/layout/taffy/stylo_taffy/mod.rs:45-120`).
- Taffy solves the full 2D constraint equations (including nested flex containers, complex grid line spans, and auto tracks) and returns pixel rectangles for all children.

#### Engineering Assessment
- **Problem in Asteria:** CSS Flexbox and Grid specifications span hundreds of pages of complex edge cases. Maintaining a proprietary handwritten solver consumes massive engineering effort while remaining perpetually non-compliant.
- **Applicability to Asteria:** Ideal. Taffy is written in 100% safe Rust, has minimal dependencies, is actively maintained, and is the industry-standard layout solver for both Servo and Bevy Engine.
- **Recommendation:** **Replace implementation with `taffy 0.14` (`AST-002`).** Map Asteria's `ComputedStyle` to `taffy::style::Style`, let Taffy compute child layout rectangles in `layout_flex` and `layout_grid`, and remove over 1,200 lines of fragile handwritten solver code.

---

### 2.3 Style Resolution & Computed Style Memory Footprint

#### Asteria's Current Approach
- In `src/values.rs:700-771`, `ComputedStyle` is an enormous monolithic struct (over 350 bytes) containing 45 fields.
- 7 fields are heap-allocated: `font_family: Vec<String>`, `grid_template_columns: Vec<GridTrack>`, `grid_template_rows: Vec<GridTrack>`, `animation_name: String`, `box_shadow: Vec<BoxShadow>`, `transform: Vec<TransformFunction>`, and `variables: HashMap<String, String>`.
- In `src/style.rs:828`, every DOM node clones the entire CSS variable map: `computed.variables = current_variables.clone()`.
- Sibling elements with identical styles perform redundant selector matching and property calculations.

#### Servo's Mechanism (`subsystems/04_STYLO_PARALLEL_CSS.md`, `AST-004`)
- Stylo partitions computed styles into distinct logical groups wrapped in reference-counted pointers (`Arc<StyleBoxData>`, `Arc<StyleColorData>`, `Arc<StylePositionData>`, `Arc<StyleFontData>`).
- If an element does not override positioning properties, it shares the exact same `Arc<StylePositionData>` instance as its parent or default style.
- Stylo implements a **Style Sharing Cache**: if element $B$ has the same tag, classes, and parent style as element $A$, Stylo reuses element $A$'s computed style handle in $O(1)$ time without running selector matching.

#### Engineering Assessment
- **Problem in Asteria:** Unbounded memory consumption and heap allocations during style resolution on large documents. Deep-copying `ComputedStyle` on every DOM node produces extreme cache churn.
- **Applicability to Asteria:** Partitioning `ComputedStyle` and introducing copy-on-write `Arc` sharing for `variables` is straightforward and high-impact.
- **Recommendation:** **Refactor existing implementation (`AST-004`, `AST-PERF-002`).**
  1. Wrap `variables` in `Arc<HashMap<Symbol, String>>`.
  2. Partition `ComputedStyle` into logical sub-structs (`BoxStyle`, `FontStyle`, `ColorStyle`).
  3. Introduce a lightweight style sharing cache for sibling elements.

---

### 2.4 Graphics Presentation & Compositing (wgpu vs. WebRender)

#### Asteria's Current Approach
- Asteria renders hardware-accelerated graphics using `wgpu 22.1` (`src/renderer/`).
- Operates in immediate mode: every scroll event or hover state change triggers `build_rect_batch()` and `populate_text_pass()`, regenerating CPU vertex arrays and re-uploading vertex/index buffers to the GPU.

#### Servo's Mechanism (`subsystems/07_WEBRENDER_GPU_COMPOSITOR.md`, `AST-005`)
- Servo emits a `DisplayList` uploaded to WebRender.
- WebRender decomposes display items into a retained GPU scene graph.
- Scrolling, zooming, and CSS transforms update spatial node uniform matrices on the GPU compositor thread without touching the layout tree or regenerating geometry batches.

#### Engineering Assessment
- **Servo's Trap to Avoid:** **Do NOT adopt WebRender.** WebRender is an enormous C++/Rust engine tightly bound to raw OpenGL/Glow and `surfman`. Porting WebRender to Asteria would destroy Asteria's clean, modern `wgpu` architecture.
- **Opportunity for Asteria:** Asteria can achieve the benefits of retained-mode GPU scrolling entirely within its existing `wgpu` pipeline!
- **Recommendation:** **Adapt relevant Servo pattern independently (`AST-005`, `AST-PERF-001`).**
  - Add a uniform buffer containing `scroll_offset: [f32; 2]` and `viewport_size: [f32; 2]` to `RectPass` and `TextPass`.
  - In WGSL vertex shaders (`shader.wgsl`), offset vertex positions by `uniforms.scroll_offset` on the GPU.
  - Rebuild vertex buffers only when DOM, styles, or document layout change—never during pure scrolling.

---

### 2.5 JavaScript & Scripting Engine (SpiderMonkey vs. Pure Rust JS)

#### Asteria's Current Approach
Asteria currently has **no JavaScript engine** (`FNOP.md` Phase 2 roadmap target).

#### Servo's Mechanism (`subsystems/03_SCRIPT_THREAD_DOM_ARENA.md`)
Servo integrates Mozilla's **SpiderMonkey** (`mozjs`):
- SpiderMonkey is a high-performance JIT-compiled C++ JS engine.
- Requires building Mozilla's massive C++ source tree via `mach`, LLVM Clang, and Python build drivers.
- Memory management requires complex rooted GC pointers (`JS<T>`, `Trace`, SpiderMonkey garbage collection hooks) entangled across the entire DOM tree.

#### Engineering Assessment
- **Servo's Trap to Avoid:** **Under no circumstances should Asteria adopt SpiderMonkey.** Adopting `mozjs` would instantly eliminate Asteria's pure-Rust build reproducibility, introduce massive C++ compile times, and create severe cross-platform build fragility on Windows.
- **Alternative for Asteria:** Use a pure Rust ECMAScript engine:
  - **`boa_engine`:** 100% pure Rust, zero C++ dependencies, implements modern ECMAScript standards, clean Rust garbage collector.
  - **`rquickjs`:** Embedded QuickJS wrapper with near-instant compile times and full ES2023 compliance.
- **Recommendation:** **Independent implementation using pure Rust (`boa_engine`).** Prepare Asteria's DOM with string-atom handles (`AST-ARC-005`), then introduce `boa_engine` behind an isolated script thread actor.

---

### 2.6 Browser Shell & Chrome Decoupling

#### Asteria's Current Approach
In `src/shell_ui.rs` and `src/renderer/window/window.rs`, browser UI chrome (tabs, omnibox, back/forward buttons, status bar) is drawn as immediate-mode rectangles and text directly into the webpage's vertex buffers.

#### Servo's Mechanism (`subsystems/01_EMBEDDING_AND_SHELL.md`, `AST-006`)
In `servoshell`, browser chrome is completely isolated:
- `servoshell` uses `egui` (`egui 0.34` + `egui-winit`) to render UI chrome.
- Servo's web content renders into an isolated texture or sub-rectangle (`WebView`).
- Changes to browser chrome do not trigger webpage re-layout or re-paint.

#### Engineering Assessment
- **Problem in Asteria:** Hundreds of lines of tedious rectangle and text drawing code in `shell_ui.rs` clutter the rendering pipeline and cause unnecessary redraws.
- **Recommendation:** **Refactor existing implementation (`AST-006`).** Either separate Asteria's render graph into distinct `ContentPass` and `ChromePass` pipelines or adopt `egui` for the browser chrome, reserving the inner viewport for Asteria's web layout.

---

## 3. What Asteria Must NOT Import from Servo

| Servo Subsystem | Why Asteria Must Reject It | Preferred Asteria Architecture |
| :--- | :--- | :--- |
| **SpiderMonkey (`mozjs`)** | Requires massive C++ compilation, Python build drivers, and GC rooting macros. | Pure-Rust `boa_engine` or `rquickjs`. |
| **WebRender Compositor** | Enormous legacy C++/Rust codebase tied to raw OpenGL and Surfman. | Retain Asteria's modern `wgpu 22.1` backend; add uniform-based GPU scroll offsets. |
| **Gaol Sandboxing & Multi-Process IPC** | Extreme architectural complexity; heavy IPC serialization overhead via `ipc-channel`. | Retain multi-threaded single-process actor topology with `mpsc`/`crossbeam` channels. |
| **Mach Build Driver** | Proprietary Python-based build orchestration script. | Standard `cargo build` and `cargo test`. |
| **C++ HarfBuzz (`harfbuzz-sys`)** | C/C++ compilation dependency for font shaping. | Pure-Rust text shaping via `cosmic-text` and `glyphon::FontSystem`. |
