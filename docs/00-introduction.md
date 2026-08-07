# Introduction to Asteria

> **Purpose:** Introduce the Asteria project — what it is, why it exists, and where it's going.
>
> **Audience:** Everyone — no technical background required.
>
> **Estimated reading time:** 5 minutes
>
> **Prerequisites:** None

---

## What is Asteria?

Asteria is a web browser engine. It's the software that takes a web page — its HTML, CSS, images, and other resources — and turns all of that into the visual result you see on your screen.

When you open a website in Chrome, Firefox, or Safari, there's an engine underneath doing the heavy lifting. Chrome uses an engine called Blink. Firefox uses Gecko. Safari uses WebKit. These are enormous, decades-old codebases maintained by hundreds of engineers.

Asteria is a new engine being built from scratch in the [Rust](https://www.rust-lang.org/) programming language. It doesn't borrow code from any existing browser. Every component — the HTML parser, the CSS engine, the layout solver, the GPU renderer, the networking stack — is hand-written.

---

## Why do browser engines exist?

The web is built on a deceptively simple idea: text files describe what a page should look like, and a program on your computer figures out how to display it.

But that "figuring out" part is staggeringly complex. A browser engine has to:

1. **Download** resources over the network (HTML, CSS, images, scripts)
2. **Parse** raw text into structured data
3. **Build a tree** of every element on the page (the DOM)
4. **Apply styles** from CSS rules, deciding what color, size, and font every element gets
5. **Calculate layout** — where every box, line of text, and image goes on screen
6. **Paint** the visual result into a list of drawing instructions
7. **Render** those instructions on the GPU at 60+ frames per second
8. **Handle interaction** — scrolling, clicking, hovering, typing

All of this happens in milliseconds, every time you load a page. Browser engines are among the most complex pieces of consumer software ever built.

---

## Why build another one?

A few reasons.

**To learn.** The best way to understand something deeply is to build it yourself. There are no shortcuts. Reading documentation about how layout works is one thing; implementing a layout engine that correctly positions elements on screen is something else entirely. Asteria exists because we wanted that deeper understanding.

**To explore.** Existing engines carry decades of legacy decisions, compatibility constraints, and accumulated complexity. Starting fresh gives us the freedom to explore different architectural choices — arena allocation instead of reference counting, data-oriented design instead of deep object hierarchies, GPU-first rendering from day one.

**To contribute.** The web is too important to be controlled by only a few engine implementations. Projects like [Servo](https://servo.org/) and [Ladybird](https://ladybird.dev/) have shown that new engines can push the ecosystem forward. Asteria aims to be part of that conversation.

---

## Project philosophy

Asteria is guided by a few core principles:

### Build everything from scratch

The core web engine — parsing, styling, layout, painting — uses no third-party browser crates. We use external libraries only for things outside the web engine itself: GPU access (wgpu), windowing (winit), TLS (rustls), and font rendering (glyphon). Everything that processes web content is ours.

### Data-oriented design

Instead of deep class hierarchies, Asteria favours flat arrays, integer handles, and cache-friendly memory layouts. The DOM is stored in a contiguous arena. Scene nodes live in a flat vector. This isn't an academic exercise — these choices directly affect performance on real hardware.

### Pipeline clarity

The engine is structured as a clean, linear pipeline. Each stage has a well-defined input and output. HTML bytes go in one end; rendered pixels come out the other. No stage reaches back into a previous one. This makes the engine easier to reason about, test, and extend.

### Explain everything

This documentation is a first-class part of the project. We believe that if you can't explain how something works, you don't fully understand it yet. Every design decision, trade-off, and architectural choice should be recorded and explained.

---

## Long-term vision

Asteria is a long-term project. Right now, it can render styled HTML pages with flexbox layouts on the GPU, load resources over HTTPS, and manage multiple browser tabs.

Where it's heading:

- **Broader CSS support** — animations, transitions, grid layout, positioned elements
- **JavaScript execution** — a script engine for dynamic web pages
- **Standards compliance** — progressive alignment with W3C and WHATWG specifications
- **A usable browser** — address bar, bookmarks, settings, and a polished interface
- **Cross-platform builds** — native performance on Windows, macOS, and Linux

The goal isn't to replace Chrome or Firefox. The goal is to build a browser engine that is well-understood, well-documented, and architecturally clean — and to make the knowledge we gain along the way available to anyone who's curious.

---

## Where to go from here

| If you want to... | Read... |
|---|---|
| Understand how browsers work in general | [How a Browser Works](01-how-a-browser-works.md) |
| See how Asteria's pipeline fits together | [How Asteria Works](02-how-asteria-works.md) |
| Explore the codebase | [Project Architecture](03-project-architecture.md) |
| Learn about a specific subsystem | [HTML Engine](04-html-engine.md), [DOM](05-dom.md), [CSS Engine](06-css-engine.md), [Layout](07-layout-engine.md), [Painting](08-painting.md), [GPU Renderer](09-gpu-renderer.md) |
| Contribute | [Contributing](14-contributing.md) |
| See the project roadmap | [Roadmap](13-roadmap.md) |
| Look up a term | [Glossary](17-glossary.md) |
