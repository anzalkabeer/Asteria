# Layout Engine

> **Purpose:** Explain how Asteria computes the position and size of every element on the screen.
>
> **Audience:** Contributors, systems programmers, and anyone curious about 2D geometry.
>
> **Estimated reading time:** 18 minutes
>
> **Prerequisites:** [CSS Engine](06-css-engine.md)

---

## What is layout?

Layout is the process of turning styled elements into positioned rectangles. The style resolver tells us *what* each element looks like (colour, font size, margins). The layout engine tells us *where* each element goes and *how big* it is.

This is arguably the most complex stage of the rendering pipeline. A single CSS property change — like switching `display: block` to `display: flex` — can completely alter the geometry of an entire page.

**Source file:** `src/layout.rs`

---

## The CSS box model

Every element in CSS generates a rectangular box with four nested layers:

```
┌──────────────────────────── margin ─────────────────────────────┐
│                                                                 │
│   ┌────────────────────── border ──────────────────────────┐    │
│   │                                                        │    │
│   │   ┌──────────────── padding ──────────────────────┐    │    │
│   │   │                                               │    │    │
│   │   │            ┌── content ──┐                    │    │    │
│   │   │            │             │                    │    │    │
│   │   │            │  text, img  │                    │    │    │
│   │   │            │  children   │                    │    │    │
│   │   │            └─────────────┘                    │    │    │
│   │   │                                               │    │    │
│   │   └───────────────────────────────────────────────┘    │    │
│   │                                                        │    │
│   └────────────────────────────────────────────────────────┘    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

Asteria represents this as:

```rust
pub struct Dimensions {
    pub content: Rect,       // x, y, width, height of the content area
    pub padding: EdgeSizes,  // top, right, bottom, left
    pub border: EdgeSizes,   // top, right, bottom, left
    pub margin: EdgeSizes,   // top, right, bottom, left
}
```

From the content rect, you can compute larger containing rectangles:

- **Padding box** = content + padding
- **Border box** = content + padding + border
- **Margin box** = content + padding + border + margin

These rectangles are used by different parts of the engine:
- The paint engine draws backgrounds to the **padding box**
- Borders are drawn at the **border box**
- Adjacent elements are spaced using the **margin box**

---

## Formatting contexts

The layout engine uses different algorithms depending on the CSS `display` property of a container. These are called **formatting contexts**.

### Block formatting context

When a container has `display: block` (the default for `<div>`, `<p>`, `<h1>`, etc.), its children stack **vertically** — one below the other.

```
┌──────────────────────────┐
│ Container (block)        │
│ ┌──────────────────────┐ │
│ │ Child 1              │ │
│ └──────────────────────┘ │
│ ┌──────────────────────┐ │
│ │ Child 2              │ │
│ └──────────────────────┘ │
│ ┌──────────────────────┐ │
│ │ Child 3              │ │
│ └──────────────────────┘ │
└──────────────────────────┘
```

**Width:** Each block-level child expands to fill the full available width of its container (unless it has an explicit `width` set).

**Height:** The container's height is the sum of its children's margin boxes (plus its own padding).

**Margin centering:** If a block element has `width` set and `margin-left: auto; margin-right: auto`, the remaining space is split equally, centering the element.

**Algorithm:**

```
For each block child:
  1. Resolve width:
     - If explicit width set → use it
     - Otherwise → container_width - margins - padding - borders
  2. Position horizontally:
     - x = container.content.x + margin_left + border_left + padding_left
  3. Position vertically:
     - y = cursor_y + margin_top
     - cursor_y += child's margin box height
  4. Recurse into child's own children
  5. Set child height from either explicit height or sum of children
```

### Inline formatting context

When a container's children are inline-level (`<span>`, text nodes, `<a>`, `<em>`), they flow **horizontally** — left to right, wrapping to the next line when they reach the container's edge.

```
┌─────────────────────────────────────┐
│ Container                           │
│ ┌─────┐ ┌──────┐ ┌───┐ ┌────────┐  │
│ │word1│ │ word2│ │ w3│ │ word4  │  │
│ └─────┘ └──────┘ └───┘ └────────┘  │
│ ┌──────┐ ┌────┐                     │
│ │word5 │ │ w6 │                     │
│ └──────┘ └────┘                     │
└─────────────────────────────────────┘
```

**Line wrapping:** Text is measured character by character. When a word would overflow the container width, it wraps to a new line.

**Character metrics:** Layout estimates inline text width using a font-size ratio approximation (`font_size * 0.55`). The GPU renderer subsequently rasterises glyphs using `glyphon` font metrics. This known architectural simplification creates a slight mismatch between layout text bounding boxes and GPU glyph placement.

**Algorithm:**

```
cursor_x = 0
cursor_y = 0
line_height = max font_size on current line

For each inline child:
  Measure the child's width
  If cursor_x + width > container_width:
    // Line break
    cursor_x = 0
    cursor_y += line_height
  Position child at (cursor_x, cursor_y)
  cursor_x += width
```

### Flex formatting context

When a container has `display: flex`, its children are arranged along a horizontal main axis with explicit widths.

```
┌───────────────────────────────────────────┐
│ Flex container                            │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐   │
│ │  Item 1   │ │  Item 2   │ │  Item 3   │   │
│ │           │ │           │ │           │   │
│ └──────────┘ └──────────┘ └──────────┘   │
└───────────────────────────────────────────┘
```

**Asteria's flex implementation currently supports:**

| Feature | Status |
|---|---|
| `flex-direction: row` (horizontal) | ✅ |
| Explicit widths on flex children | ✅ |
| Whitespace-node filtering | ✅ |
| Container-clamped sizing | ✅ |
| `flex-direction: column` | 🔜 |
| `flex-wrap` | 🔜 |
| `justify-content` | 🔜 |
| `align-items` | 🔜 |
| `flex-grow` / `flex-shrink` | 🔜 |

**Algorithm:**

```
cursor_x = container.content.x
For each non-whitespace flex child:
  child.x = cursor_x
  width = child's explicit width (clamped to container boundaries)
  cursor_x += width
```

---

## Box types

Every node in the layout tree has a box type:

| BoxType | Created for | Description |
|---|---|---|
| `BlockNode` | Block-level elements | Participates in block formatting |
| `InlineNode` | Inline-level elements | Participates in inline formatting |
| `FlexNode` | Flex items | Participates in flex formatting |
| `AnonymousBlock` | Mixed children | Wrapper for inline children inside a block container |

### Anonymous blocks

When a block container has a mix of block and inline children, the inline children are wrapped in an **anonymous block** — an invisible container that establishes an inline formatting context:

```html
<div>
  Some text          <!-- inline -->
  <p>Paragraph</p>   <!-- block -->
  More text          <!-- inline -->
</div>
```

Becomes:

```
LayoutBox [BlockNode: div]
  ├── LayoutBox [AnonymousBlock]      ← wraps the inline text
  │     └── LayoutBox [InlineNode: "Some text"]
  ├── LayoutBox [BlockNode: p]
  │     └── LayoutBox [InlineNode: "Paragraph"]
  └── LayoutBox [AnonymousBlock]      ← wraps the inline text
        └── LayoutBox [InlineNode: "More text"]
```

This ensures every formatting context is unambiguous — a block container's direct children are either *all* block-level or *all* inline-level.

---

## The layout tree

The output of the layout engine is a `LayoutTree` — a tree of `LayoutBox` nodes, each carrying computed `Dimensions`:

```rust
pub struct LayoutBox<'a> {
    pub dimensions: Dimensions,
    pub box_type: BoxType,
    pub styled_node: Option<&'a StyledNode>,
    pub children: Vec<LayoutBox<'a>>,
}
```

Example output for a simple page:

```
LayoutBox [BlockNode: body] @ (0, 0) 800×400
  ├── LayoutBox [BlockNode: h1] @ (0, 0) 800×30
  │     margin: top=0 right=0 bottom=10 left=0
  │     padding: top=0 right=0 bottom=0 left=0
  │     content: 800×30
  │     └── LayoutBox [InlineNode: "Hello World"] @ (0, 0) 144×24
  └── LayoutBox [BlockNode: div] @ (0, 40) 800×200
        padding: top=16 right=16 bottom=16 left=16
        content: 768×168
        └── LayoutBox [InlineNode: "Content..."] @ (16, 56) 600×16
```

---

## Sizing rules

### Width computation

1. If the element has an explicit `width` CSS property → use it
2. Otherwise → `container_content_width - margin_left - margin_right - border_left - border_right - padding_left - padding_right`
3. This is the **content-box** model (W3C default) — `width` sets the content area, not the border box

### Height computation

1. If the element has an explicit `height` CSS property → use it
2. Otherwise → sum of children's margin box heights (for block) or line heights (for inline)

### Auto margins

`margin: auto` distributes remaining space. For horizontal centering:

```
remaining = container_width - element_width
margin_left = remaining / 2
margin_right = remaining / 2
```

---

## Scrolling and overflow

When content is taller than the viewport, the page becomes scrollable. Asteria tracks a `scroll_offset` in the window event loop and translates the rendered scene by this offset, effectively shifting which part of the layout tree is visible.

```
┌──────────────────────┐ ← viewport top
│  visible content     │
│                      │
│                      │
└──────────────────────┘ ← viewport bottom
│  content below fold  │  ← revealed by scrolling
│  (still laid out,    │
│   just off-screen)   │
└──────────────────────┘
```

The layout engine always computes positions for the entire document, regardless of viewport size. Scrolling only changes what portion is rendered.

---

## Resize and reflow

When the browser window is resized, the layout engine re-runs with new viewport dimensions. This is called **reflow**.

The entire layout tree is rebuilt from the styled tree with the new `viewport_width` and `viewport_height`. This ensures all width calculations (percentage widths, auto widths, line wrapping) are updated to reflect the new available space.

```
Window resized (1200px → 800px)
  → Layout engine runs again with new width
  → Block widths shrink
  → Text lines wrap differently
  → Content height changes
  → New layout tree produced
  → Scene graph rebuilt
  → GPU re-renders
```

In the future, incremental layout will allow re-computing only the affected subtrees rather than the entire document.

---

## Layout entry point

The top-level function that drives all of this:

```rust
pub fn layout_document(
    styled: &StyledTree,
    dom: &Dom,
    source: &[u8],
    viewport_width: f32,
    viewport_height: f32,
) -> Option<LayoutTree>
```

It creates a root `LayoutBox` for the `<body>` element, sets its available width to the viewport width, and recursively lays out all children according to their formatting contexts.

---

## How other engines compare

| Engine | Layout model | Approach |
|---|---|---|
| **Blink** | LayoutNG | Constraint-based, fragment tree |
| **Gecko** | Layout 2020 rewrite (in progress) | Frame tree with reflow |
| **Servo** | Layout 2020 | Parallel layout with Rayon |
| **Asteria** | Recursive tree walk | Direct recursion, block/inline/flex contexts |

Asteria's current approach is simpler — direct recursive traversal — which trades some performance on very large pages for clarity and ease of extension.

---

## Current limitations

| Feature | Status |
|---|---|
| CSS Grid | 🔜 |
| `position: absolute / fixed / sticky` | 🔜 |
| `float: left / right` | 🔜 |
| `overflow: hidden / scroll` | 🔜 |
| `min-width` / `max-width` | 🔜 |
| `box-sizing: border-box` | 🔜 |
| Percentage heights | 🔜 |
| Incremental reflow | 🔜 |

---

## Related documents

- [CSS Engine](06-css-engine.md) — produces the styled tree consumed by layout
- [Painting](08-painting.md) — converts the layout tree into drawing instructions
- [Design Decisions](18-design-decisions.md) — layout architecture choices
- [Glossary](17-glossary.md) — layout terminology
