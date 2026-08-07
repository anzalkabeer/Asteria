# Glossary

> **Purpose:** Define every technical term used across the Asteria documentation.
>
> **Audience:** Everyone — look up any unfamiliar term.
>
> **Estimated reading time:** Reference document (not meant to be read end-to-end)
>
> **Prerequisites:** None

---

## A

**Arena allocation**
A memory management strategy where all objects are stored in a single contiguous buffer (typically a `Vec`). Objects are referenced by index rather than pointers. Deallocation is instant — drop the entire buffer. See [DOM](05-dom.md).

**Attribute**
A name-value pair on an HTML element. In `<div class="main">`, `class="main"` is an attribute with name `class` and value `main`.

---

## B

**Batching**
The practice of combining multiple GPU draw calls into a single call by packing vertex data into shared buffers. Reduces GPU state transitions. See [GPU Renderer](09-gpu-renderer.md).

**Block formatting context (BFC)**
The layout environment in which block boxes are laid out and formatted. In CSS, a BFC is established by the document root, floats, positioned elements, inline-blocks, flex items, or `display: flow-root` (and managed via block formatting contexts in Asteria's layout solver). See [Layout Engine](07-layout-engine.md).

**Border box**
The rectangle enclosing an element's content, padding, and border. See [Layout Engine](07-layout-engine.md).

**Box model**
The CSS model where every element is a rectangular box with four layers: content, padding, border, and margin. See [Layout Engine](07-layout-engine.md).

**Bump allocator**
An allocator that "bumps" a pointer forward for each allocation. Extremely fast (O(1) per allocation). Deallocation is all-or-nothing — reset the pointer. Asteria's `FrameArena` is a bump allocator. See [Performance](12-performance.md).

---

## C

**Cache locality**
The tendency for data stored contiguously in memory to be accessed faster because CPU caches load memory in blocks (cache lines). Flat arrays have better cache locality than pointer-linked trees.

**Cascade**
The CSS algorithm that resolves conflicts when multiple rules set the same property on the same element. Priority is determined by origin, specificity, and source order. See [CSS Engine](06-css-engine.md).

**Combinator**
In CSS, the symbol connecting parts of a complex selector. Descendant (space), child (`>`), next sibling (`+`), subsequent sibling (`~`). See [CSS Engine](06-css-engine.md).

**Compositing**
The process of combining independently rendered layers into a final image. Enables efficient scrolling and animations by re-compositing layers without re-rendering. Not yet implemented in Asteria.

**Computed style**
The set of resolved CSS property values for an element used by the layout engine. Absolute units (such as `px` or resolved `em` values) are computed into concrete values, while relative percentages and keywords are resolved against their respective layout bases. See [CSS Engine](06-css-engine.md).

**Content box**
The innermost rectangle of the CSS box model — the area containing the element's actual content (text, images, children). Width and height CSS properties set the content box size by default.

**CSSOM**
CSS Object Model — the structured representation of parsed CSS rules. Asteria's `Stylesheet` is its CSSOM.

---

## D

**Declaration**
A CSS property-value pair. In `color: blue;`, the property is `color` and the value is `blue`.

**Display list**
A flat, ordered list of drawing instructions produced by the paint engine. Each instruction is a `DisplayCommand` — solid colour, border, text, or image. See [Painting](08-painting.md).

**DNS**
Domain Name System — the internet's phone book. Translates domain names (like `example.com`) into IP addresses (like `93.184.216.34`).

**DOM**
Document Object Model — a tree representation of an HTML document. Every element, text node, and comment is a node in the tree. See [DOM](05-dom.md).

---

## E

**Edge sizes**
The four directional values (top, right, bottom, left) used for margins, padding, and borders.

---

## F

**Flex layout / Flexbox**
A CSS layout mode (`display: flex`) where children are arranged along a main axis (horizontal or vertical) with flexible sizing. See [Layout Engine](07-layout-engine.md).

**Formatting context**
The layout algorithm used by a container to position its children. Block formatting contexts stack children vertically; inline formatting contexts flow them horizontally. See [Layout Engine](07-layout-engine.md).

**Fragment shader**
A GPU program that determines the colour of each pixel within a rendered primitive. Runs once per pixel. See [GPU Renderer](09-gpu-renderer.md).

---

## G

**Glyphon**
A GPU text rendering library for wgpu. Handles glyph shaping, atlas management, and text rasterisation. Used by Asteria's text pass.

**GPU**
Graphics Processing Unit — specialised hardware designed for parallel computation and rendering. Asteria uses the GPU for all visual rendering via wgpu.

**GPU pipeline**
A configuration object on the GPU that defines how vertices are processed and pixels are coloured. Includes shaders, vertex format, and render state. Created once and reused per frame.

---

## H

**Hit testing**
The process of determining which element (scene node) is at a given screen coordinate. Used to handle mouse hover and click events.

**HTML**
HyperText Markup Language — the standard language for describing web page structure. Tags like `<div>`, `<p>`, `<h1>` define elements.

---

## I

**Inheritance (CSS)**
The mechanism by which certain CSS properties (like `color` and `font-size`) flow from parent elements to their children unless explicitly overridden. See [CSS Engine](06-css-engine.md).

**Inline formatting context**
A layout mode where children flow horizontally, left to right, wrapping to the next line when they reach the container's edge. See [Layout Engine](07-layout-engine.md).

**Interner / String interner**
A data structure that maps strings to compact integer handles (`Symbol`). Two equal strings always map to the same handle, enabling O(1) comparison. See [Performance](12-performance.md).

---

## L

**Layout tree**
A tree of positioned boxes produced by the layout engine. Each node carries `Dimensions` (content rect, padding, border, margin) in document coordinates. See [Layout Engine](07-layout-engine.md).

**LRU cache**
Least Recently Used cache — a cache that evicts the oldest (least recently accessed) entries when it reaches capacity.

---

## M

**Margin box**
The outermost rectangle of the CSS box model — the border box plus margins. Used for spacing between adjacent elements.

**Margin centering**
When `margin-left` and `margin-right` are both `auto`, the remaining space is split equally, centering the element horizontally.

**Media query**
A CSS feature (`@media`) that conditionally applies rules based on device characteristics. Asteria supports viewport-width-based queries (`min-width`, `max-width`).

---

## N

**NDC**
Normalised Device Coordinates — the coordinate system used by GPUs. Ranges from (-1, -1) at the bottom-left to (1, 1) at the top-right. Pixel coordinates are converted to NDC in the vertex shader.

**Node (DOM)**
A single element in the DOM tree — an element, text node, comment, or the document root. See [DOM](05-dom.md).

**NodeId**
A 32-bit integer handle (`NodeId(u32)`) that identifies a node in the DOM arena. An index into `Vec<Node>`.

---

## P

**Padding box**
The rectangle enclosing an element's content and padding (but not border or margin).

**Paint order**
The CSS-defined order in which an element's visual components are drawn: background → border → content → children. See [Painting](08-painting.md).

**Parser**
The component that reads a stream of tokens and builds a structured tree (DOM for HTML, stylesheet for CSS).

---

## R

**Rasterisation**
The process of converting vector shapes (rectangles, text outlines) into pixel values. The GPU performs rasterisation during rendering.

**Reflow**
Re-running the layout engine when the viewport dimensions change (e.g., window resize). See [Layout Engine](07-layout-engine.md).

**Render graph**
A coordination system that manages the execution order of GPU render passes. See [GPU Renderer](09-gpu-renderer.md).

**Render pass**
A single phase of GPU rendering that draws one category of visual primitives (rectangles, text, or images).

---

## S

**Scene graph**
A flat, data-oriented representation of all visual primitives to be rendered. Optimised for GPU submission with contiguous storage, z-ordering, and segment assignment. See [GPU Renderer](09-gpu-renderer.md).

**SceneNodeId**
A 32-bit integer handle identifying a node in the scene graph. An index into `Vec<SceneNode>`.

**Selector**
The part of a CSS rule that describes which elements it applies to. `div.main > p` is a selector. See [CSS Engine](06-css-engine.md).

**Shorthand (CSS)**
A CSS property that sets multiple related properties at once. `margin: 16px` sets `margin-top`, `margin-right`, `margin-bottom`, and `margin-left`.

**Specificity**
A three-component score (ID count, class count, tag count) used to resolve conflicts between CSS rules. Higher specificity wins. See [CSS Engine](06-css-engine.md).

**Stacking context**
A conceptual layer in which elements are painted together with a shared z-ordering. Created by positioned elements, opacity, transforms, etc.

**State machine**
An algorithm that transitions between named states based on input. The HTML tokenizer is a state machine with states like `Data`, `TagOpen`, `TagName`, etc.

**Styled tree**
A tree mirroring the DOM structure, where each node carries a `ComputedStyle` — the fully resolved CSS properties for that element.

**Symbol**
A 4-byte integer handle (`Symbol(u32)`) representing an interned string. See [Performance](12-performance.md).

---

## T

**TLS**
Transport Layer Security — the cryptographic protocol that secures HTTPS connections. Asteria uses rustls for TLS.

**Token**
A meaningful chunk of text identified by the tokenizer. HTML tokens include start tags, end tags, text, and comments. CSS tokens include identifiers, numbers, strings, and symbols.

**Tokenizer**
The component that reads raw text and breaks it into tokens. The first step of parsing.

---

## V

**Vertex**
A point in space with associated data (position, colour, texture coordinates). GPU rendering works with vertices assembled into triangles and quads.

**Vertex buffer**
A block of GPU memory containing vertex data. The renderer packs all vertices for a draw call into a single buffer.

**Vertex shader**
A GPU program that processes each vertex — typically transforming positions from pixel coordinates to normalised device coordinates (NDC).

**Viewport**
The visible area of the browser window where content is rendered.

**Void element**
An HTML element that cannot have children: `<br>`, `<img>`, `<input>`, `<hr>`, `<meta>`, `<link>`. The parser never pushes these onto the open element stack.

---

## W

**wgpu**
A cross-platform GPU API for Rust, implementing the WebGPU standard. Works on Vulkan, Metal, DX12, and WebGL. Asteria's GPU backend.

**WGSL**
WebGPU Shading Language — the shader language used by wgpu. Asteria's rect and image shaders are written in WGSL.

**winit**
A cross-platform window creation and event handling library for Rust. Provides the OS window, keyboard/mouse input, and event loop.

---

## Z

**z-index / z-order**
The front-to-back ordering of elements. Higher z-order values are painted on top of lower values. Controls which elements appear "in front" when they overlap.

**Zero-copy**
A technique where data is referenced by position (byte offset pairs) rather than copied into new allocations. Asteria's HTML tokenizer is zero-copy — it stores offsets into the original input buffer.
