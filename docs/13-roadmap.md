# Roadmap

> **Purpose:** Describe where Asteria has been and where it's going.
>
> **Audience:** Everyone — potential contributors, curious observers, users.
>
> **Estimated reading time:** 5 minutes
>
> **Prerequisites:** None

---

## Current state

Asteria is a working browser engine that can load web pages from disk or the network, parse HTML and CSS, compute layout, and render the result on the GPU with interactive tabbed browsing.

As of August 2026, the engine can:

- Parse HTML into an arena-allocated DOM
- Parse CSS with selectors, specificity, and media queries
- Resolve styles with cascade, inheritance, and shorthand expansion
- Compute block, inline, and flex layouts with the CSS box model
- Paint to a display list and render on the GPU via wgpu
- Load pages over HTTP/HTTPS with DNS caching and TLS
- Manage multiple tabs with navigation history
- Handle scrolling, hover, clicks, and window resizing
- Profile every pipeline stage with microsecond precision

---

## Completed milestones

| Phase | Milestone | Description |
|---|---|---|
| **1** | HTML Engine | Zero-copy tokenizer, parser, and arena DOM |
| **2** | CSS Engine | Tokenizer, parser, selectors (tag, class, ID, compound, combinators, pseudo-classes) |
| **3** | Style Resolution | Specificity, cascade, inheritance, shorthand expansion, `@media`, computed styles |
| **4** | Layout Engine | Block, inline, and flex formatting contexts with CSS box model |
| **5** | Paint Engine | Display list generator with CSS paint ordering |
| **6** | GPU Renderer | wgpu hardware rendering with WGSL shaders, batched passes, and winit windowing |
| **7** | Networking | Custom HTTP/1.1 + TLS client, DNS resolver, connection pool, streaming resource bus |
| **8** | Infrastructure | Browser shell, tab manager, multi-threaded scheduler, string interner, profiler, devtools |
| **9** | Interactive Browser | Keyboard shortcuts, scrolling, hover effects, link navigation, window resize reflow |

---

## Near-term goals

These are actively being worked on or planned for the near future:

### CSS improvements

- [ ] `!important` declaration support
- [ ] `@import` for external stylesheet inclusion
- [ ] `@keyframes` and CSS animations
- [ ] CSS transitions
- [ ] Additional property support (more box-model properties, text properties)
- [ ] Custom properties (`var()`)

### Layout improvements

- [ ] `flex-direction: column`
- [ ] `flex-wrap` and `justify-content` / `align-items`
- [ ] `flex-grow` and `flex-shrink`
- [ ] CSS Grid layout
- [ ] `position: absolute` / `fixed` / `sticky`
- [ ] `float` support

### Rendering improvements

- [ ] `border-radius` (rounded corners)
- [ ] `box-shadow`
- [ ] `opacity` and alpha blending
- [ ] CSS `transform` (GPU-accelerated transforms)
- [ ] Layer compositing for smoother scrolling
- [ ] Incremental layout (re-compute only changed subtrees)

### Browser improvements

- [ ] Address bar with URL input
- [ ] Tab bar UI
- [ ] Bookmarks
- [ ] Settings / preferences panel
- [ ] Proper text selection and copy
- [ ] Context menu (right-click)

---

## Long-term goals

These represent major engineering milestones:

### JavaScript engine

A script execution engine that can:
- Parse and execute JavaScript
- Provide DOM API bindings (`document.getElementById`, `element.style`, etc.)
- Handle events (`onclick`, `addEventListener`)
- Enable dynamic page modification

This is a massive undertaking. It may involve integrating an existing JS engine (like V8 or SpiderMonkey) or building a lightweight interpreter.

### Standards compliance

Progressive alignment with W3C and WHATWG specifications:
- HTML5 parsing spec compliance (adoption agency algorithm, etc.)
- CSS 2.1 and CSS 3 property coverage
- Web Compatibility Test suites (WPT)

### Networking evolution

- HTTP/2 multiplexed connections
- WebSocket support
- Fetch API semantics
- Cache-Control / ETag header support
- Service workers

### Cross-platform distribution

- Pre-built release binaries for Windows, macOS, and Linux
- Package manager distribution (Homebrew, winget, APT)
- CI/CD release pipeline

### Performance at scale

- Selector indexing for large stylesheets
- Style sharing cache
- Parallel layout on independent subtrees
- Texture atlas for GPU rendering
- Incremental rendering and compositing

---

## How you can help

| Area | What's needed |
|---|---|
| CSS properties | Implement additional CSS properties and test against WPT |
| Layout | Extend flex support, implement grid layout |
| Rendering | Add visual features (rounded corners, shadows, gradients) |
| Testing | Write test fixtures, run rendering comparisons |
| Documentation | Improve existing docs, add examples, record demos |
| Performance | Profile real pages, identify bottlenecks, optimise hot paths |

See [Contributing](14-contributing.md) for how to get started.

---

## Related documents

- [How Asteria Works](02-how-asteria-works.md) — the current pipeline
- [Known Limitations](20-known-limitations.md) — what's missing today
- [Contributing](14-contributing.md) — how to help
