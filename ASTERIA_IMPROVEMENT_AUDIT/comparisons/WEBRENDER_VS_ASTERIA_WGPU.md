# COMPARATIVE ANALYSIS: SERVO WEBRENDER VS. ASTERIA WGPU

## 1. Architectural Overview

| Dimension | Servo (WebRender) | Asteria (Native WGPU) |
| :--- | :--- | :--- |
| **Backend Technology** | Custom OpenGL / Direct3D / Vulkan renderer | WGPU 0.20 (Vulkan, Metal, DX12, WebGPU) |
| **Scene Representation** | Retained Spatial Node Tree (`SpatialNodeTree`) | Heuristic Scene Graph (`src/scene.rs`) |
| **Scrolling Model** | Retained transform scroll nodes | Immediate rebuild of vertex buffers on scroll |
| **Text Rendering** | WebRender GPU Glyph Cache + HarfBuzz/Swash | Swash / FreeType + CPU Texture Atlas Cache |
| **Dependency Weight** | ~150 dependent crates, C++ build dependencies | ~35 pure/clean crates, 100% Rust |
| **Binary Overhead** | ~40-60 MB | ~15-20 MB |

---

## 2. In-Depth Comparative Assessment

### What Servo Does Well
Servo’s WebRender separates **Scene Building** from **Frame Rendering**. Once a display list is converted into WebRender primitives, user scrolling merely modifies a transformation matrix inside a GPU uniform buffer. Zero CPU vertex rebuilding occurs during 120 FPS scrolling.

### What Asteria Does Right
Asteria chose **WGPU**, which is significantly cleaner, safer, and more future-proof than maintaining legacy OpenGL contexts. It supports modern WebGPU shaders (WGSL) and runs portably across Windows, Linux, and macOS.

### Why WebRender Must NOT Be Imported into Asteria
Importing WebRender into Asteria would be an engineering disaster:
1. WebRender brings massive C++ toolchain requirements (FreeType, Harfbuzz, Angle, EGL).
2. It breaks Asteria’s clean, modern WGSL shader architecture.
3. It would increase Asteria’s clean build time from 5 minutes to over 25 minutes.

### The Winning Strategy for Asteria
Preserve Asteria’s native WGPU backend, but **adopt WebRender's architectural principles**:
- Retain GPU vertex buffers across frames (`SPEC-01`).
- Implement spatial transform uniform offsets in the WGSL vertex shader to achieve 120 FPS scrolling without CPU redraws (`SPEC-08`).
