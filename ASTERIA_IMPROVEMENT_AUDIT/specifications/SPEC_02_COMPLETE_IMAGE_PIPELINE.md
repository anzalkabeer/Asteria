# TECHNICAL SPECIFICATION: SPEC-02
## Full Image Decoding Pipeline & GPU Texture Pass

---

## 1. Problem Statement & Motivation
Currently in `src/image.rs:97-100` and `src/renderer/command_builder.rs:97-128`, Asteria cannot display images (`AST-DEF-002`). `decode_image` produces dummy mock data, `ImagePass` in `renderer.rs` is never bound in `render()`, and images are drawn as grey boxes with `[IMG]`.

---

## 2. Root Cause
The image decoding pipeline was left as an incomplete scaffold when Asteria was first created and was never integrated with real image decoders or the GPU command queue.

---

## 3. Desired Architecture
1. **Decoders:** Add `image = { version = "0.25", default-features = false, features = ["png", "jpeg", "webp"] }`.
2. **Decoding Engine:**
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
3. **Texture Upload & ImagePass Execution:**
In `src/renderer/gpu/renderer.rs`:
- Allocate a 2D texture via `device.create_texture(...)` and write pixel data via `queue.write_texture(...)`.
- Create a texture bind group combining the texture view and sampler.
- During `render()`, execute `render_pass.set_pipeline(&self.image_pipeline)`, bind the image texture bind group, and draw the quad.

---

## 4. Affected Source Locations
- `src/image.rs`
- `src/renderer/command_builder.rs`
- `src/renderer/gpu/renderer.rs`
- Target crates: `crates/asteria_paint`, `crates/asteria_render`

---

## 5. Verification & Acceptance Criteria
1. `test_png_decoding_and_texture_creation`: Decodes a test 64x64 PNG and asserts dimensions, RGBA byte size, and texture creation success.
2. An HTML document containing `<img src="...">` renders the bitmap image onto the screen surface rather than a placeholder box.
