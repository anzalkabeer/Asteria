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
| Character entities (`&amp;`, `&lt;`, `&#x1F600;`) | ❌ | Not decoded; rendered as raw text |
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
| `!important` | ❌ | Not yet implemented |
| `@import` | ❌ | External stylesheet inclusion not supported |
| `@keyframes` / CSS animations | ❌ | Not supported |
| CSS transitions | ❌ | Not supported |
| `var()` / custom properties | ❌ | Not supported |
| `::before` / `::after` pseudo-elements | ❌ | Not supported |
| `:nth-child()`, `:not()` pseudo-classes | ❌ | Not supported |
| Attribute selectors (`[type="text"]`) | ❌ | Not supported |
| `calc()` | ❌ | Not supported |
| CSS Grid | ❌ | Not supported |
| `opacity` | ❌ | Not supported |
| `transform` | ❌ | Not supported |
| `box-shadow` | ❌ | Not supported |
| `border-radius` | ❌ | Not supported |
| `text-decoration` | ❌ | Not supported |
| `overflow` | ❌ | Content always visible |
| `z-index` (full stacking contexts) | Partial | Values tracked but not fully sorted |
| CSS gradients | ❌ | Not supported |
| `float` | ❌ | Not supported |
| `position: absolute / fixed / sticky` | ❌ | Not supported |

---

## Layout limitations

| Feature | Status | Notes |
|---|---|---|
| Block formatting context | ✅ | Vertical stacking, auto-width, margin centering |
| Inline formatting context | ✅ | Horizontal flow with line wrapping |
| Flex row layout (`display: flex`) | ✅ | Horizontal row with explicit widths |
| `flex-direction: column` | ❌ | Not yet implemented |
| `flex-wrap` | ❌ | Not yet implemented |
| `justify-content` / `align-items` | ❌ | Not yet implemented |
| `flex-grow` / `flex-shrink` | ❌ | Not yet implemented |
| CSS Grid | ❌ | Not supported |
| `min-width` / `max-width` | ❌ | Not supported |
| `min-height` / `max-height` | ❌ | Not supported |
| `box-sizing: border-box` | ❌ | Always content-box |
| Percentage heights | ❌ | Not resolved |
| Positioned elements | ❌ | `absolute`, `fixed`, `sticky` not supported |
| Float layout | ❌ | Not supported |
| Incremental layout | ❌ | Full reflow on every change |
| Table layout | ❌ | Tables parsed but not laid out as tables |

---

## Rendering limitations

| Feature | Status | Notes |
|---|---|---|
| Solid colour backgrounds | ✅ | Fully supported |
| Borders (solid) | ✅ | Per-edge widths supported |
| Text rendering (basic) | ✅ | Via glyphon |
| Image placeholders | ✅ | Format detection and decode pipeline |
| Rounded corners | ❌ | Not supported |
| Box shadows | ❌ | Not supported |
| Alpha blending / opacity | ❌ | Not supported |
| CSS transforms | ❌ | Not supported |
| Layer compositing | ❌ | No GPU layer separation |
| Subpixel text antialiasing | Partial | Depends on glyphon and GPU driver |
| Custom fonts | ❌ | System default font only |
| Text selection | ❌ | Not supported |

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
| Address bar | ❌ | No URL input UI |
| Tab bar UI | ❌ | Tabs managed via keyboard only |
| Bookmarks | ❌ | Not supported |
| Settings | ❌ | Not supported |
| Find in page | ❌ | Not supported |
| Text selection and copy | ❌ | Not supported |
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
