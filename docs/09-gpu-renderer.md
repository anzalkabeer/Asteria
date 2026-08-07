# GPU Renderer

> **Purpose:** Explain how Asteria turns display lists into GPU commands and renders them on screen.
>
> **Audience:** Graphics programmers, Rust developers, and contributors.
>
> **Estimated reading time:** 12 minutes
>
> **Prerequisites:** [Painting](08-painting.md)

---

## Overview

The GPU renderer is the final stage of Asteria's pipeline. It takes the scene graph — a flat, data-oriented representation of everything that needs to be drawn — and renders it to the screen using hardware-accelerated graphics.

Asteria uses [wgpu](https://wgpu.rs/) for GPU access, which implements the WebGPU standard and works across Vulkan (Linux/Windows), Metal (macOS/iOS), DX12 (Windows), and a software fallback. Windowing and input events are handled by [winit](https://docs.rs/winit/).

---

## The rendering architecture

```
Scene Graph
     │
     ▼
┌────────────────┐
│ Batch Builder  │ ← Groups nodes by type (rects, text, images)
└───────┬────────┘
        │
        ▼
┌────────────────┐
│Command Builder │ ← Converts batches into GPU draw commands
└───────┬────────┘
        │
        ▼
┌────────────────┐
│ Render Graph   │ ← Coordinates render passes
└───────┬────────┘
        │
    ┌───┴────────────────┐
    │                    │
    ▼                    ▼
┌──────────┐    ┌──────────┐    ┌──────────┐
│ Rect     │    │ Image    │    │ Text     │
│ Pass     │    │ Pass     │    │ Pass     │
└──────────┘    └──────────┘    └──────────┘
    │                │                │
    └────────────────┼────────────────┘
                     │
                     ▼
              ┌──────────────┐
              │  wgpu Device │
              │  + Surface   │
              └──────┬───────┘
                     │
                     ▼
                  Screen
```

---

## The scene graph

**Source file:** `src/scene.rs`

Before reaching the GPU, the display list is converted into a **scene graph** — a flat, cache-friendly representation optimised for rendering:

```rust
pub struct SceneNode {
    pub rect: Rect,              // Bounding box in document coordinates
    pub kind: SceneNodeKind,     // SolidRect, Border, Text, Image, Container
    pub parent: Option<SceneNodeId>,
    pub z_order: u32,            // Paint stacking order
    pub segment_id: u16,         // GPU tile segment
    pub dirty: bool,             // Needs re-rendering?
    pub state: NodeState,        // Normal, Hovered, Active
    pub link_url: Option<String>,
}
```

All scene nodes live in a contiguous `Vec<SceneNode>` — no pointer chasing, maximum cache locality. This is the same data-oriented philosophy used in the DOM arena.

Parallel arrays store additional data:
- **Colour array** — RGBA values for each coloured node
- **Text runs** — text content, font size, and target width for each text node

---

## wgpu backend

**Source file:** `src/renderer/backend/wgpu_backend.rs`

The `WgpuBackend` initialises the GPU and creates the rendering surface:

```
┌─────────────────┐
│ wgpu::Instance  │ ← Handle to the GPU API
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ wgpu::Adapter   │ ← Specific GPU hardware
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌─────────────────┐
│ wgpu::Device    │     │ wgpu::Queue     │
│ (GPU handle)    │     │ (command queue)  │
└────────┬────────┘     └────────┬────────┘
         │                       │
         ▼                       ▼
┌─────────────────────────────────────────┐
│ wgpu::Surface (backed by OS window)    │
└─────────────────────────────────────────┘
```

Key choices:
- **Power preference:** `HighPerformance` — use the discrete GPU when available
- **Present mode:** `AutoVsync` — synchronise with the display refresh rate
- **Surface format:** sRGB preferred for correct colour rendering

The backend is created once at startup. On window resize, the surface is reconfigured with the new dimensions.

---

## Render passes

Rendering is split into passes, each responsible for one type of visual primitive.

### Rect pass

**Source file:** `src/renderer/passes/rect_pass.rs`

Draws all solid-colour rectangles and borders. This includes element backgrounds, borders, and any other filled regions.

**How it works:**

1. For each `SolidRect` and `Border` scene node, generate 4 vertices (a quad)
2. Each vertex carries position (x, y) and colour (r, g, b, a)
3. Pack all vertices into a single vertex buffer
4. Upload the buffer to the GPU
5. Execute one draw call with the rect shader pipeline

**Vertex format:**

```rust
struct Vertex {
    position: [f32; 2],  // Screen-space x, y
    color: [f32; 4],     // RGBA
}
```

### Image pass

**Source file:** `src/renderer/passes/image_pass.rs`

Draws decoded images as textured quads.

1. Upload image data as GPU textures
2. Create textured quads mapping to the image's position and size
3. Execute with the image shader pipeline

### Text pass

**Source file:** `src/renderer/passes/text_pass.rs`

Renders text using [glyphon](https://github.com/grovesNL/glyphon) — a GPU text rendering engine for wgpu.

1. For each `Text` scene node, create a text buffer with the text content, font size, and colour
2. Glyphon handles glyph shaping, atlas packing, and GPU rendering
3. Text is rendered with proper font metrics and subpixel positioning

---

## Shaders

Asteria uses WGSL (WebGPU Shading Language) for its shader programs.

### Rect shader (`shader.wgsl`)

```wgsl
// Vertex shader — positions quads in normalised device coordinates
@vertex fn vs_main(@location(0) position: vec2<f32>,
                   @location(1) color: vec4<f32>) -> ... {
    // Convert pixel coordinates to NDC (-1..1)
    let ndc_x = (position.x / viewport.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (position.y / viewport.y) * 2.0;
    ...
}

// Fragment shader — outputs the interpolated colour
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
```

### Image shader (`image_shader.wgsl`)

Similar to the rect shader, but samples from a texture:

```wgsl
@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}
```

---

## Batching and optimization

**Source files:** `src/renderer/commands/batch_builder.rs`, `src/renderer/scheduler/batching.rs`

Rather than issuing one GPU draw call per element (which would be extremely slow), Asteria **batches** similar drawing operations together:

```
Without batching:              With batching:
  draw(rect_1)                  draw(all_rects)    ← 1 draw call
  draw(rect_2)                  draw(all_images)   ← 1 draw call
  draw(rect_3)                  draw(all_text)     ← 1 draw call
  draw(image_1)
  draw(image_2)
  draw(text_1)
  draw(text_2)
  = 7 draw calls               = 3 draw calls
```

The batch builder groups scene nodes by type, packs their vertex data into shared buffers, and hands them to the render graph for execution.

### Pipeline reuse

GPU render pipelines (shaders + state configuration) are created **once at startup**. Only the vertex data changes per frame. This eliminates the overhead of pipeline creation during rendering.

---

## The render graph

**Source file:** `src/renderer/graph/render_graph.rs`

The render graph coordinates the execution of render passes:

1. Begin a render pass on the current surface texture
2. Execute the **rect pass** (backgrounds and borders)
3. Execute the **image pass** (decoded images)
4. Execute the **text pass** (glyphon text rendering)
5. Submit the command buffer to the GPU queue
6. Present the surface texture to the screen

Pass order matters — rects are drawn first (background layer), images next, and text last (foreground layer).

---

## The window event loop

**Source file:** `src/renderer/window/window.rs`

The `AsteriaWindow` hosts the winit event loop, which runs for the lifetime of the application:

```
Loop forever:
  WindowEvent::Resized → resize GPU surface, re-run layout, rebuild scene
  WindowEvent::CursorMoved → hit-test scene nodes, update hover state
  WindowEvent::MouseInput → hit-test, activate element, follow links
  WindowEvent::MouseWheel → update scroll offset, rebuild batches
  WindowEvent::KeyboardInput → handle shortcuts (Ctrl+T, Ctrl+W, etc.)
  WindowEvent::RedrawRequested → prepare passes, render, present
```

### Hit testing

When the mouse moves or clicks, the renderer needs to determine which scene node is under the cursor. It iterates through scene nodes in reverse z-order (front to back), checking if the cursor position falls within each node's bounding rectangle. The first match is the "hit" element.

This enables hover effects (changing `NodeState` to `Hovered`) and link navigation (reading the `link_url` from the hit node).

---

## Segment-based invalidation

**Source file:** `src/segment.rs`

The viewport is divided into rectangular **segments** (tiles) for region-based GPU caching. When only a small part of the page changes (like a hover effect), only the affected segments need to be re-rendered.

```
┌────────┬────────┬────────┐
│ Seg 0  │ Seg 1  │ Seg 2  │
│        │ DIRTY  │        │  ← Only segment 1 needs re-rendering
├────────┼────────┼────────┤
│ Seg 3  │ Seg 4  │ Seg 5  │
│        │        │        │
└────────┴────────┴────────┘
```

Each scene node is assigned to a segment based on its position. When a node is marked dirty, its segment is flagged for re-rendering. Clean segments can skip GPU submission entirely.

---

## Current limitations

| Feature | Status |
|---|---|
| Layer compositing | 🔜 |
| Texture atlasing | 🔜 |
| Subpixel text antialiasing | Depends on glyphon |
| `border-radius` (rounded rects) | 🔜 |
| `box-shadow` | 🔜 |
| CSS `transform` (GPU transforms) | 🔜 |
| `opacity` blending | 🔜 |
| Multi-surface compositing | 🔜 |

---

## Related documents

- [Painting](08-painting.md) — produces the display list fed to the scene graph
- [Browser Shell](11-browser-shell.md) — manages the window and tabs
- [Performance](12-performance.md) — batching and optimisation philosophy
- [Design Decisions](18-design-decisions.md) — why wgpu, why this architecture
- [Glossary](17-glossary.md) — GPU and rendering terminology
