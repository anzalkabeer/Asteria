# ADR-002: Native WGPU WebGPU Graphics Abstraction

- **Status:** **ACCEPTED**
- **Date:** August 2026 (Foundational Invariant)
- **Deciders:** Keshav, Anzal Kabeer
- **Consulted:** `docs/09-gpu-renderer.md`, `src/renderer/gpu/renderer.rs`

---

## 1. Context & Problem Statement
A web browser requires high-throughput, low-latency 2D and text rasterization on modern GPUs. Legacy browser engines rely on OpenGL/EGL or Skia, which suffer from driver overhead, deprecated APIs on macOS (Metal), and driver instability on Windows.

---

## 2. Decision Drivers
1. **Cross-Platform Modern Backends:** Native support for Vulkan (Linux/Android), Metal (macOS/iOS), and DirectX 12 (Windows).
2. **Safe Graphics API:** Memory safety at the GPU resource boundary without raw OpenGL pointer arithmetic.
3. **Standards-Aligned Shaders:** Use standard WebGPU Shading Language (WGSL) rather than managing divergent GLSL, HLSL, and MSL codebases.

---

## 3. Considered Options
- **Option A:** Raw OpenGL bindings via `glow` or `glutin`.
- **Option B:** CPU software rasterizer (tiny-skia / raqote).
- **Option C (Selected):** Modern GPU abstraction via `wgpu` 0.20 and WGSL shaders.

---

## 4. Decision & Rationale
Asteria chose **WGPU 0.20** as its unified graphics hardware abstraction layer. WGPU translates standard WGSL shader pipelines into native Vulkan, DirectX 12, or Metal calls automatically, providing maximum GPU performance while maintaining pure-Rust safety.

---

## 5. Consequences & Trade-Offs
- **Positive:** Single WGSL shader codebase targets all major OS platforms; native GPU acceleration for rects and glyph text runs.
- **Negative:** Headless CI environments require software Vulkan adapters (Mesa Lavapipe or WARP) to execute GPU tests.
