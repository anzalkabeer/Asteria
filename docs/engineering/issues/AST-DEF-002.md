# ISSUE: AST-DEF-002 — Mock Image Pipeline & Uncalled ImagePass

- **Classification:** CONFIRMED DEFECT
- **Severity:** High
- **Priority:** P0
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Media, Resource Loading & GPU Renderer
- **Specification Link:** [`../specifications/SPEC-02.md`](../specifications/SPEC-02.md)

---

## 1. Problem Statement
Asteria cannot display bitmap images (`<img>` tags). The image decoding function generates mock grey pixels, the command builder emits grey placeholder boxes with `[IMG]` labels, and the GPU renderer never executes the `ImagePass` pipeline.

---

## 2. Evidence & Source Location
- **Decoding Mock:** [`src/image.rs:97-100`](../../../src/image.rs#L97-L100) generates fixed dummy pixel arrays.
- **Placeholder Command:** [`src/renderer/command_builder.rs:97-128`](../../../src/renderer/command_builder.rs#L97-L128) draws `DrawRect` and text `"[IMG]"`.
- **Uncalled Pipeline:** [`src/renderer/gpu/renderer.rs:248-275`](../../../src/renderer/gpu/renderer.rs#L248-L275) instantiates `ImagePass` but never binds or draws it in `render()`.

---

## 3. Root Cause
The image decoding pipeline was stubbed out during initial scaffolding and was never wired up to real image decoding crates or GPU texture bind groups.

---

## 4. Desired Behaviour
Integrate `image = "0.25"` to decode PNG/JPEG/WEBP bytes, upload textures to WGPU 2D textures, and render textured quads via `ImagePass`.
