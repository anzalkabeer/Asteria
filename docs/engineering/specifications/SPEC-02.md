# SPEC-02: Real PNG/JPEG Image Decoding Pipeline & GPU Texture Pass

- **Related Issue:** [`../issues/AST-DEF-002.md`](../issues/AST-DEF-002.md)
- **Target Subsystem:** Media & GPU Renderer (`src/image.rs`, `src/renderer/gpu/renderer.rs`, `src/renderer/command_builder.rs`)
- **Stage:** Stage 1 (P0)

---

## 1. Problem Statement
`src/image.rs:97-100` generates mock gradient bytes instead of decoding real image formats, `command_builder.rs:97-128` renders `<img>` tags as grey boxes with `[IMG]`, and `ImagePass` in `renderer.rs` is never bound or drawn in the render pass loop.

---

## 2. Root Cause
Scaffolded image decoding and GPU texture binding code was never connected to real decoding crates or render pipelines.

---

## 3. Recommended Solution
1. Add `image = { version = "0.25", default-features = false, features = ["png", "jpeg", "webp"] }` to `Cargo.toml`.
2. Implement real decoding in `src/image.rs`:
```rust
pub fn decode_image(bytes: &[u8]) -> Result<DecodedImage, ImageError> {
    let reader = image::io::Reader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| ImageError::Io(e))?;
    let dynamic_img = reader.decode().map_err(|e| ImageError::Decode(e.to_string()))?;
    let rgba = dynamic_img.to_rgba8();
    Ok(DecodedImage {
        width: rgba.width(),
        height: rgba.height(),
        pixels: rgba.into_raw(),
    })
}
```
3. In `src/renderer/command_builder.rs`, emit `RenderCommand::DrawImage { rect, image_id }`.
4. In `src/renderer/gpu/renderer.rs`, upload decoded RGBA pixels to WGPU 2D textures, bind the texture view in `ImagePass`, and draw textured quads.

---

## 4. Affected Source Locations
- `src/image.rs`
- `src/renderer/command_builder.rs`
- `src/renderer/gpu/renderer.rs`

---

## 5. Verification & Acceptance Criteria
1. `test_png_image_decode_dimensions`: Successfully decodes a 64x64 PNG and asserts correct dimensions and byte lengths.
2. Rendered output displays the actual bitmap image rather than a grey box.
