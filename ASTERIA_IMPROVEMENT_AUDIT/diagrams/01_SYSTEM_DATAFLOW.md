# ASTERIA ARCHITECTURAL DIAGRAMS: SYSTEM DATAFLOW

## 1. Current Asteria Frame Pipeline (Immediate-Mode Thrashing)

The diagram below illustrates the current data flow from user scroll events to GPU presentation, highlighting the bottleneck at the BatchBuilder stage (`AST-DEF-001`, `AST-PERF-001`).

```mermaid
sequenceDiagram
    autonumber
    actor User as User / OS
    participant Win as Window & EventLoop
    participant Pipe as Pipeline
    participant Scene as Scene Tree
    participant BB as BatchBuilder
    participant GPU as WGPU Device
    participant Screen as Surface / Display

    User->>Win: Scroll Wheel Event (delta_y)
    Note over Win: Viewport Offset Updated
    Win->>Pipe: rebuild_display_commands(scene, offset)
    Note over Pipe,Scene: Linear walk of entire scene tree
    Pipe->>BB: BatchBuilder::new() + build()
    Note over BB: CPU recalculates all vertex positions<br/>Allocates Vec<Vertex> and Vec<u16>
    BB->>GPU: device.create_buffer_init(vertices)
    BB->>GPU: device.create_buffer_init(indices)
    Note over GPU: Allocates fresh GPU VRAM buffers every frame!
    Win->>GPU: render_pass.draw_indexed(...)
    GPU->>Screen: surface.present()
```

---

## 2. Remediated Frame Pipeline (Retained GPU Uniform Offsets)

The diagram below illustrates the optimized flow specified in `SPEC-08` and `SPEC-01`, eliminating CPU rebuilding during scrolling and achieving 120 FPS performance.

```mermaid
sequenceDiagram
    autonumber
    actor User as User / OS
    participant Win as Window & EventLoop
    participant GPU as WGPU Queue & Device
    participant Screen as Surface / Display

    Note over GPU: Retained Vertex & Index Buffers cached in VRAM
    User->>Win: Scroll Wheel Event (delta_y)
    Note over Win: Clamp viewport_offset_y
    Win->>GPU: queue.write_buffer(&u_viewport, offset)
    Note over GPU: Updates 16-byte uniform buffer (Zero CPU vertex work!)
    Win->>GPU: render_pass.draw_indexed(cached_buffers)
    GPU->>Screen: surface.present() (120 FPS / 60 FPS)
```
