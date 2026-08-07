# FAQ

> **Purpose:** Answer the questions people ask most about Asteria.
>
> **Audience:** Everyone.
>
> **Estimated reading time:** 5 minutes
>
> **Prerequisites:** None

---

### Why not just use Chromium?

Chromium is an incredible engineering achievement. It's also over 35 million lines of code, with decades of accumulated complexity, compatibility workarounds, and legacy architectural decisions.

We're not competing with Chromium. We're building something from scratch to understand *how* it works and to explore whether different design choices lead to a cleaner, more maintainable engine. You learn more by building a house than by renovating someone else's.

---

### Why Rust?

Rust gives us:

- **Memory safety in safe Rust** — eliminates use-after-free, data races, and segfaults in safe code without garbage collection (though unsafe code and FFI must still be audited). Browser engines are notorious for memory safety bugs; Rust eliminates entire categories of them at compile time.
- **Performance comparable to C++** — zero-cost abstractions, no runtime overhead, direct control over memory layout.
- **Modern tooling** — Cargo (build system + package manager), rustfmt (formatting), Clippy (linting), and a thriving ecosystem.
- **Expressive type system** — enums with data, pattern matching, and traits make complex state machines (like tokenizers) clean and safe to write.

Servo (Mozilla's experimental engine) proved that Rust is a viable choice for browser engines. We agree.

---

### Why build another browser engine?

The web is the world's largest software platform, yet it's powered by only three major engine families: Blink (Chrome/Edge/Opera), Gecko (Firefox), and WebKit (Safari, which descended from KHTML in the early 2000s).

More implementations improve the ecosystem:
- They find spec bugs and ambiguities
- They push for cleaner standards
- They explore architectural alternatives
- They prevent monoculture

Projects like Servo and Ladybird demonstrate that this matters. Asteria adds another voice to the conversation.

---

### How does this compare to Servo?

[Servo](https://servo.org/) is Mozilla's experimental engine, written in Rust. It pioneered parallel layout and CSS processing.

Asteria is different in several ways:
- **Scope** — Servo aims for production-level web compatibility; Asteria currently focuses on clean architecture and progressive feature implementation
- **Dependencies** — Servo uses `html5ever`, `cssparser`, and other mature crates; Asteria builds everything from scratch
- **Architecture** — Asteria uses arena allocation for the DOM and a data-oriented scene graph; Servo uses traditional heap-allocated DOM objects
- **Team size** — Servo has had hundreds of contributors over a decade; Asteria is built by two people

We respect Servo enormously. Different projects make different trade-offs.

---

### How does this compare to Ladybird?

[Ladybird](https://ladybird.dev/) (originally part of SerenityOS) is a browser engine written in C++ from scratch. It has similar "build everything from scratch" philosophy.

Key differences:
- **Language** — Ladybird uses C++; Asteria uses Rust
- **GPU rendering** — Asteria uses wgpu (WebGPU standard) from day one; Ladybird started with software rendering
- **DOM architecture** — Asteria uses arena allocation; Ladybird uses a more traditional object model
- **Project maturity** — Ladybird has been in development longer and has broader HTML/CSS coverage

Ladybird is an inspiration. It proves that building a browser from scratch is not only possible but valuable.

---

### Is this a learning project?

It started as one. It's becoming more than that.

Yes, one of the primary motivations is deep understanding — we wanted to know exactly how browsers work by building one. But the engine is real. It renders real HTML and CSS with GPU acceleration. It handles network loading, tabbed browsing, and interactive events.

The line between "learning project" and "real project" is where you draw it. We draw it at the point where someone can load a web page and see it rendered correctly on their screen. Asteria does that.

---

### Will JavaScript be supported?

Eventually, yes. JavaScript execution is on the roadmap.

This is one of the largest engineering challenges ahead. A JavaScript engine involves lexing, parsing, bytecode compilation (or interpretation), a runtime with garbage collection, and bindings to the DOM API.

Options include:
- Integrating an existing engine (V8, SpiderMonkey, QuickJS)
- Building a lightweight interpreter from scratch

No decision has been made yet. When it happens, it will be documented in the [Design Decisions](18-design-decisions.md) document.

---

### Can I use Asteria as my daily browser?

Not yet. Asteria can render styled web pages, but it doesn't support JavaScript, most CSS properties, forms, or many HTML features that modern websites depend on.

Think of it as a rendering engine that can display static web content. A full-featured browser experience is a long-term goal — see the [Roadmap](13-roadmap.md).

---

### What platforms does Asteria run on?

Asteria should work on any platform supported by wgpu and winit:
- **Windows** (Vulkan or DX12)
- **macOS** (Metal)
- **Linux** (Vulkan)

In practice, development and testing have focused on Windows and macOS. If you encounter platform-specific issues, please open an issue.

---

### How can I contribute?

See the [Contributing](14-contributing.md) guide. Short version:

1. Clone the repo
2. Pick an area from the [Roadmap](13-roadmap.md)
3. Write code, add tests, submit a PR

We're particularly looking for help with CSS property implementation, layout features, and visual test fixtures.

---

### What license is Asteria under?

MIT License. You can use, modify, and distribute the code freely.

---

## Related documents

- [Introduction](00-introduction.md) — project overview
- [Roadmap](13-roadmap.md) — where the project is heading
- [Contributing](14-contributing.md) — how to help
- [Design Decisions](18-design-decisions.md) — why things are the way they are
