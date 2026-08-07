# Painting

> **Purpose:** Explain how layout boxes become visual drawing instructions.
>
> **Audience:** Contributors, graphics programmers, and anyone curious about rendering.
>
> **Estimated reading time:** 8 minutes
>
> **Prerequisites:** [Layout Engine](07-layout-engine.md)

---

## What is painting?

Painting is the stage where the engine stops thinking about structure and starts thinking about pixels. The layout engine gives us a tree of precisely positioned boxes. The paint engine walks that tree and generates a flat, ordered list of **drawing instructions** — a **display list**.

Each instruction says something like "draw a blue rectangle here" or "render this text at that position." The display list doesn't care about the DOM, selectors, or formatting contexts. It's just a sequence of visual commands.

**Source file:** `src/paint.rs`

---

## Display commands

The display list is composed of four types of commands:

### SolidColor

```rust
SolidColor {
    color: Color,       // RGBA fill colour
    rect: Rect,         // Position and size
    link_url: Option<String>,
}
```

Used for element backgrounds. When an element has `background-color: #313244`, the paint engine emits a `SolidColor` command at the element's padding box.

### Border

```rust
Border {
    color: Color,       // Border colour
    rect: Rect,         // Border box rectangle
    border_width: EdgeSizes,  // Width per edge (top, right, bottom, left)
    link_url: Option<String>,
}
```

Draws the CSS border around an element. The border is drawn at the border box (content + padding + border area).

### Text

```rust
Text {
    text: String,       // The text content
    x: f32,             // Horizontal position
    y: f32,             // Vertical position
    target_width: f32,  // Available width for wrapping
    font_size: f32,     // Font size in pixels
    color: Color,       // Text colour
    link_url: Option<String>,
}
```

Draws a text fragment. The text, position, and styling are all resolved — no further lookups needed.

### Image

```rust
Image {
    image_id: String,   // Resource identifier
    x: f32,             // Horizontal position
    y: f32,             // Vertical position
    width: f32,         // Display width
    height: f32,        // Display height
    link_url: Option<String>,
}
```

Draws a decoded image at the specified position and size.

---

## Paint order

CSS specifies the order in which an element's visual components must be painted. Asteria follows this order for each layout box:

```
┌────────────────────────────────────────┐
│ 1. Background  (SolidColor)           │
│ 2. Borders     (Border)               │
│ 3. Content     (Text / Image)         │
│ 4. Children    (recurse into children)│
└────────────────────────────────────────┘
```

For a `<div>` with a background, border, and text content:

1. **Background** → `SolidColor` at the padding box
2. **Border** → `Border` at the border box
3. **Text** → `Text` commands for any text content
4. **Children** → recursively paint each child element

This ordering ensures that backgrounds are behind borders, borders are behind content, and parent elements are behind their children.

### Document-order stacking

Because the paint engine walks the layout tree recursively, the display list naturally follows document order — elements that appear later in the HTML appear later in the display list, and thus paint on top of earlier elements.

For proper z-index support (stacking contexts), elements would be grouped by stacking context and sorted by z-index before painting. Asteria tracks `z_order` values but doesn't yet implement full stacking context separation.

---

## The painting algorithm

```
function paint_layout_box(box, display_list):
    // 1. Background
    if box has background-color (not transparent):
        emit SolidColor(color, box.padding_box())

    // 2. Borders
    if box has border-width > 0:
        emit Border(color, box.border_box(), widths)

    // 3. Content
    if box is a text node:
        emit Text(text, position, font_size, color)
    if box is an image:
        emit Image(id, position, width, height)

    // 4. Children
    for each child in box.children:
        paint_layout_box(child, display_list)
```

The function is called once with the root layout box, and it recursively visits every box in the tree, appending commands to the display list.

---

## Link tracking

Every display command can optionally carry a `link_url`. When painting elements inside an `<a>` tag, the URL from the `href` attribute is propagated down to all commands generated for that subtree. This allows the GPU renderer to perform hit-testing — when a user clicks, the renderer can check which display command was under the cursor and navigate to the link's URL.

---

## The display list

The final output is a `DisplayList`:

```rust
pub struct DisplayList {
    pub commands: Vec<DisplayCommand>,
}
```

This is a flat vector — no tree structure, no hierarchy. The commands are in paint order, ready to be consumed by the scene graph builder.

### Example display list

For a simple page with a heading and paragraph:

```
[0] SolidColor { color: #1e1e2e, rect: (0,0,800,600) }      ← body background
[1] SolidColor { color: #313244, rect: (0,0,800,50) }        ← header background
[2] Text { "Hello World", x: 0, y: 5, size: 24, color: #89b4fa }  ← h1 text
[3] Text { "Welcome to Asteria", x: 0, y: 55, size: 16, color: #a6adc8 }  ← p text
```

The GPU renderer processes these commands in order, drawing each one on top of the previous.

---

## Performance considerations

The display list is designed to be lightweight:

- **Flat structure** — no pointer chasing, cache-friendly sequential access
- **Minimal data** — each command stores only what's needed for drawing
- **No duplicates** — each visual element generates exactly one command
- **Ready to batch** — the scene graph can group commands by type for GPU batching

For a page with 100 visible elements, the display list typically contains 200–400 commands (backgrounds + borders + text for each element).

---

## Current limitations

| Feature | Status |
|---|---|
| Full stacking context separation | 🔜 |
| `z-index` sorting | Tracked, not fully sorted |
| `opacity` | 🔜 |
| `transform` (rotate, scale) | 🔜 |
| `clip-path` / clipping | 🔜 |
| `box-shadow` | 🔜 |
| `border-radius` (rounded corners) | 🔜 |
| CSS gradients | 🔜 |

---

## Related documents

- [Layout Engine](07-layout-engine.md) — produces the layout tree that painting walks
- [GPU Renderer](09-gpu-renderer.md) — consumes the display list (via scene graph)
- [Glossary](17-glossary.md) — painting and display list terminology
