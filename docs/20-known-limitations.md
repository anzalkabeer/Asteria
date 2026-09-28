# Known Limitations

> **Purpose:** Be honest about what Asteria can and can't do today.
>
> **Audience:** Everyone — users, contributors, and evaluators.
>
> **Estimated reading time:** 5 minutes
>
> **Prerequisites:** None

---

## HTML limitations

| Feature | Status | Notes |
|---|---|---|
| Standard elements (`div`, `p`, `h1`, `a`, etc.) | ✅ | Supported |
| Attributes (`class`, `id`, `style`, `href`, etc.) | ✅ | Supported |
| Character entities (`&amp;`, `&lt;`, `&#x1F600;`) | ✅ | Supported via entity decoding |
| `<template>` elements | ❌ | Not supported |
| `<script>` execution | ❌ | Parsed but not executed |
| `<form>` elements | ❌ | No form handling |
| `<canvas>` | ❌ | Not supported |
| `<video>` / `<audio>` | ❌ | Not supported |
| `<iframe>` | ❌ | Not supported |
| `<svg>` rendering | ❌ | Detected as image format but not rendered |
| Full HTML5 spec compliance | ❌ | Adoption agency algorithm, foster parenting, etc. not implemented |

---

## CSS limitations

| Feature | Status | Notes |
|---|---|---|
| Tag, class, ID selectors | ✅ | Fully supported |
| Compound selectors (`div.main`) | ✅ | Fully supported |
| Descendant, child, sibling combinators | ✅ | All four supported |
| `:first-child`, `:last-child`, `:hover` | ✅ | Supported |
| Specificity and cascade | ✅ | Correct (ID, class, tag) scoring |
| Inheritance | ✅ | `color`, `font-size`, and other inherited properties |
| Shorthand expansion (`margin`, `padding`, `border`) | ✅ | Supported |
| `@media` viewport queries | ✅ | `min-width`, `max-width` |
| `!important` | ✅ | Supported with declaration priority sorting |
| `@import` | ✅ | Supported for external stylesheets |
| `@keyframes` / CSS animations | ❌ | Not supported |
| CSS transitions | ❌ | Not supported |
| `var()` / custom properties | ✅ | Supported with variable substitution |
| `::before` / `::after` pseudo-elements | ❌ | Not supported |
| `:nth-child()`, `:not()` pseudo-classes | ❌ | Not supported |
| Attribute selectors (`[type="text"]`) | ✅ | Supported with attribute presence and operators |
| `calc()` | ✅ | Supported for mathematical expressions |
| CSS Grid | ✅ | Supported via grid formatting context |
| `opacity` | ✅ | Fully supported with hierarchical cascade and alpha modulation |
| `transform` | ✅ | 2D transforms (translate, rotate, scale, skew, matrix) supported |
| `box-shadow` | ✅ | Multiple shadows, blur, spread, color, and inset |
| `border-radius` | ✅ | 1-4 value shorthand parsing and rounded rect commands |
| `text-decoration` | ❌ | Not supported |
| `overflow` | ✅ | `overflow: hidden` clipping supported via PushClip/PopClip |
| `z-index` (full stacking contexts) | Partial | Values tracked but not fully sorted |
| CSS gradients | ❌ | Not supported |
| `float` | ❌ | Not supported |
| `position: absolute / fixed / sticky` | ✅ | absolute and fixed positioning supported out-of-flow |

---

## Layout limitations

| Feature | Status | Notes |
|---|---|---|
| Block formatting context | ✅ | Vertical stacking, auto-width, margin centering |
| Inline formatting context | ✅ | Horizontal flow with line wrapping |
| Flex row layout (`display: flex`) | ✅ | Horizontal row with explicit widths |
| `flex-direction: column` | ✅ | Supported in flex layout |
| `flex-wrap` | ✅ | Supported for multi-line flex containers |
| `justify-content` / `align-items` | ❌ | Not yet implemented |
| `flex-grow` / `flex-shrink` | ❌ | Not yet implemented |
| CSS Grid | ✅ | Supported via grid formatting context |
| `min-width` / `max-width` | ❌ | Not supported |
| `min-height` / `max-height` | ❌ | Not supported |
| `box-sizing: border-box` | ✅ | Supported across layout box sizing |
| Percentage heights | ❌ | Not resolved |
| Positioned elements | ✅ | absolute and fixed positioned elements supported |
| Float layout | ❌ | Not supported |
| Incremental layout | ❌ | Full reflow on every change |
| Table layout | ✅ | Multi-pass table formatting context, column sizing, row sizing, colspan, rowspan, vertical alignment |

---

## Rendering limitations

| Feature | Status | Notes |
|---|---|---|
| Solid colour backgrounds | ✅ | Fully supported |
| Borders (solid) | ✅ | Per-edge widths supported |
| Text rendering (basic) | ✅ | Via glyphon |
| Image placeholders | ✅ | Format detection and decode pipeline |
| Rounded corners | ✅ | Emits RoundedRect display commands |
| Box shadows | ✅ | Emits BoxShadow commands before element background |
| Alpha blending / opacity | ✅ | Cascading opacity modulating color alpha channels |
| CSS transforms | ✅ | 2D transforms (translate, rotate, scale, skew, matrix) with transform-origin, stacking context creation, and affine vertex tessellation |
| Layer compositing | ✅ | GPU layer separation with CompositorLayer, layer promotion for transforms/stacking contexts/translucent subtrees, viewport culling, and batch compositing |
| Subpixel text antialiasing | ✅ | Supported via glyphon surface format and font hinting defaults |
| Custom fonts | ✅ | font-family cascade & mapping to system fonts (serif, sans-serif, monospace, cursive, fantasy, named fonts) + font-weight mapping |
| Text selection | ✅ | Interactive click-and-drag selection across text nodes, select-all (Ctrl+A / Cmd+A), visual highlight rects, and clipboard copy (Ctrl+C / Cmd+C) |

---

## Networking limitations

| Feature | Status | Notes |
|---|---|---|
| HTTP/1.1 GET | ✅ | Supported |
| HTTPS (TLS 1.2/1.3) | ✅ | Via rustls |
| DNS with caching | ✅ | TTL-based |
| Redirects | ✅ | Followed up to a limit |
| HTTP/2 | ❌ | Not supported |
| HTTP/3 (QUIC) | ❌ | Not supported |
| POST / PUT / DELETE | ❌ | Only GET supported |
| Cookies | ❌ | Not supported |
| Cache-Control headers | ❌ | Not honoured |
| WebSockets | ❌ | Not supported |
| Service workers | ❌ | Not supported |
| Parallel resource fetching | ❌ | Resources loaded sequentially |

---

## Browser limitations

| Feature | Status | Notes |
|---|---|---|
| Multiple tabs | ✅ | Via keyboard shortcuts |
| Navigation history | ✅ | Per-tab back/forward/reload |
| Scrolling | ✅ | Mouse wheel |
| Link clicking | ✅ | Hit testing + navigation |
| Window resize reflow | ✅ | Live content reflow |
| Address bar | ✅ | Interactive omnibox URL input UI with navigation |
| Tab bar UI | ✅ | Clickable tab bar UI with tab switching and closing |
| Bookmarks | ❌ | Not supported |
| Settings | ❌ | Not supported |
| Find in page | ❌ | Not supported |
| Text selection and copy | ✅ | Interactive drag selection, Ctrl+A select-all, visual highlight, and clipboard integration via arboard |
| Right-click context menu | ❌ | Not supported |
| Print | ❌ | Not supported |
| Developer tools panel | ❌ | CLI-only devtools |

---

## JavaScript

JavaScript is not yet supported. There is no script engine, no DOM API bindings, and no event handling from JavaScript. This is the single largest feature gap and is on the long-term roadmap.

---

## Performance considerations

- **Full reflow on every resize** — the entire layout tree is rebuilt when the window is resized. Incremental layout is planned.
- **Sequential resource loading** — CSS and linked resources are loaded one at a time. Parallel fetching is planned.
- **No style sharing cache** — elements with identical styles are computed independently. A sharing cache would deduplicate this work.
- **No selector indexing** — selector matching is O(elements × rules). An index would speed this up for large stylesheets.
- **Single-threaded pipeline** — the main pipeline runs on one thread. The task scheduler supports parallelism, but layout and style are not yet parallelised.

---

## What this means

Asteria can correctly render styled HTML pages with block, inline, and flex layouts using GPU acceleration. It handles tabbed browsing, navigation, scrolling, and network loading.

It cannot render most modern websites because they depend on JavaScript, hundreds of CSS properties, form handling, and other features not yet implemented.

The project is in active development. Each limitation listed here is a potential contribution opportunity — see the [Roadmap](13-roadmap.md) and [Contributing](14-contributing.md).

---

## Related documents

- [Roadmap](13-roadmap.md) — when these limitations will be addressed
- [FAQ](16-faq.md) — "Can I use Asteria as my daily browser?"
- [Contributing](14-contributing.md) — help fix these limitations
