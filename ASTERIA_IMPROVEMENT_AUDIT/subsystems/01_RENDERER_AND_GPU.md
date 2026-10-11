# ASTERIA SUBSYSTEM AUDIT: RENDERER AND GPU

## 1. Subsystem Scope & Files
- **Primary Source Modules:**
  - [`src/renderer/gpu/renderer.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/gpu/renderer.rs)
  - [`src/renderer/gpu/batch_builder.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/gpu/batch_builder.rs)
  - [`src/renderer/gpu/pipeline.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/gpu/pipeline.rs)
  - [`src/renderer/gpu/shader.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/gpu/shader.rs)
  - [`src/renderer/gpu/texture.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/gpu/texture.rs)
  - [`src/renderer/glyph_cache.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/glyph_cache.rs)
  - [`src/renderer/font.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/font.rs)

---

## 2. Architectural Analysis

### Strengths
1. **Modern Hardware Abstraction:** The renderer is built on WGPU 0.20, enabling cross-platform portability across Vulkan, Metal, and DirectX 12 without platform-specific rendering backends.
2. **Text Atlas Caching:** `GlyphCache` provides a texture atlas caching mechanism for rasterized font glyphs, preventing redundant CPU glyph rasterization on repeated text characters.
3. **Clean Command Separation:** The separation between `RenderCommand` generation in `command_builder.rs` and GPU execution in `renderer.rs` provides a clean display-list abstraction.

### Deficiencies Identified
1. **Per-Frame GPU Buffer Reallocation (`AST-PERF-001`):** `BatchBuilder` allocates fresh GPU vertex and index buffers every single frame via `create_buffer_init` instead of maintaining ring buffers or persistent geometry pools.
2. **Immediate-Mode Redraw on Scroll (`AST-DEF-001`):** Scrolling and hovering force a complete rebuild of display commands and GPU vertex buffers on every tick.
3. **Unexecuted Image Pipeline (`AST-DEF-002`):** `ImagePass` pipeline is declared in `renderer.rs` but never bound or invoked in the main render pass.

---

## 3. Comparison with Servo (WebRender)
Servo delegates rendering to WebRender (`asteria_servo_audit/subsystems/04_WEBRENDER_AND_COMPOSITING.md`). While WebRender provides advanced hardware clipping, subpixel anti-aliasing, and asynchronous scene building, it incurs an immense dependency tree (~150 crates) and complex C++ dependencies.

**Decision:** Asteria should **preserve its native lightweight WGPU architecture** rather than adopting WebRender, but adopt WebRender's retained spatial transform tree patterns (`SPEC-08`) and retained vertex buffers (`SPEC-01`).
