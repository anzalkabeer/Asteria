# FINDING RECORD: AST-DEF-002

## 1. Classification & Identity
- **Finding ID:** `AST-DEF-002`
- **Classification:** CONFIRMED DEFECT
- **Severity:** High
- **Subsystem:** Media, Resource Pipeline, and GPU Renderer
- **Confidence Level:** High (Directly verified in source and verified non-functional in renderer)

---

## 2. Location & Evidence
- **Source Locations:**
  - [`src/image.rs:97-100`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/image.rs#L97-L100)
  - [`src/renderer/command_builder.rs:97-128`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/command_builder.rs#L97-L128)
  - [`src/renderer/gpu/renderer.rs:248-275`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/gpu/renderer.rs#L248-L275)

- **Source Code Verification:**
  - In `src/image.rs:97-100`:
    ```rust
    pub fn decode_image(data: &[u8]) -> Result<DecodedImage, ImageError> {
        // MOCK: Generates placeholder gradient bytes instead of actual PNG/JPEG decoding
        let width = 100;
        let height = 100;
        let pixels = vec![128u8; (width * height * 4) as usize];
        Ok(DecodedImage { width, height, pixels })
    }
    ```
  - In `src/renderer/command_builder.rs:97-128`:
    ```rust
    // For <img> tags, instead of emitting an ImageCommand with a texture ID:
    commands.push(RenderCommand::DrawRect {
        rect: node.rect,
        color: Color::from_rgb(200, 200, 200),
    });
    commands.push(RenderCommand::DrawText {
        text: "[IMG]".to_string(),
        pos: node.rect.center(),
        ...
    });
    ```
  - In `src/renderer/gpu/renderer.rs:248-275`:
    An `ImagePass` pipeline and bind group layout is defined in the WGPU setup, but the active render pass loop in `render()` never binds or executes `self.image_pass`.

---

## 3. Root Cause Analysis
The image decoding pipeline was left as a scaffold during early engine development. Real image decoding crates (such as `image-rs`) were never wired up to the networking fetch output, and the display list command builder was never updated to generate textured quad primitives.

---

## 4. Servo Architectural Reference
- Reference: `asteria_servo_audit/subsystems/06_IMAGE_AND_RESOURCE_PIPELINE.md`
- Mechanism: Servo uses a dedicated resource cache thread that decodes images asynchronously using `image` and `png`/`jpeg-decoder`. Decoded RGBA frames are assigned a unique WebRender `ImageKey` and pushed to the GPU texture cache.

---

## 5. Remediation Plan
Implemented via `SPEC-02`:
1. Integrate the `image` crate (v0.25) in `src/image.rs` to decode PNG, JPEG, WEBP, and GIF formats into RGBA8 buffers.
2. Update `src/renderer/command_builder.rs` to emit `RenderCommand::DrawImage { rect, image_id }`.
3. In `src/renderer/gpu/renderer.rs`, upload decoded images to WGPU 2D textures and execute the textured quad pipeline in `render()`.
