# Design Decisions

> **Purpose:** Record and explain every major architectural choice in Asteria — a permanent engineering journal.
>
> **Audience:** Contributors, systems programmers, and anyone interested in *why* the architecture looks this way.
>
> **Estimated reading time:** 15 minutes
>
> **Prerequisites:** [How Asteria Works](02-how-asteria-works.md), [Project Architecture](03-project-architecture.md)

---

## Why Rust?

**Decision:** Build the engine in Rust.

**Alternatives considered:** C++, Zig, Go, TypeScript

**Rationale:**

Browser engines are historically plagued by memory safety bugs. Chromium alone has disclosed thousands of CVEs, with a majority traced to memory errors — use-after-free, buffer overflows, dangling pointers. C++ doesn't prevent these at compile time.

Rust eliminates entire categories of memory bugs through its ownership system, borrow checker, and lifetime annotations — all at compile time, with no runtime overhead. You get C++-level performance with memory safety guarantees that would require a garbage collector in other languages.

Additional factors:
- Servo proved Rust is viable for browser engines at scale
- Cargo provides a unified build system, package manager, and test runner
- The Rust ecosystem has mature GPU (wgpu), windowing (winit), and TLS (rustls) libraries
- Pattern matching and algebraic types make parser state machines cleaner than C++ switch statements

**Trade-offs:**
- Steeper learning curve for contributors unfamiliar with Rust's ownership model
- Some algorithms (tree mutation, graph traversal) require more careful design to satisfy the borrow checker
- The Rust ecosystem, while growing, is smaller than C++'s

---

## Why arena allocation for the DOM?

**Decision:** Store all DOM nodes in a contiguous `Vec<Node>` with `NodeId(u32)` index handles.

**Alternatives considered:** `Box<Node>` pointers, `Rc<RefCell<Node>>` reference counting, `Arc<RwLock<Node>>` for thread safety

**Rationale:**

The DOM is the most-traversed data structure in the engine. Every pipeline stage — style resolution, layout, painting — walks the entire tree. Cache performance during traversal is critical.

| Approach | Allocation | Access | Traversal | Deallocation |
|---|---|---|---|---|
| `Box<Node>` | Per-node malloc | Pointer deref (cache miss) | Pointer chasing | Per-node free |
| `Rc<RefCell<Node>>` | Per-node + refcount | Pointer deref + borrow check | Pointer chasing | Refcount check |
| Arena `Vec<Node>` | One Vec push | Array index (cache hit) | Sequential memory | Drop entire Vec |

The arena approach gives us:
- **O(1) allocation** — `vec.push()` is amortised constant time
- **Sequential memory access** — nodes are packed contiguously, prefetch-friendly
- **Zero reference counting** — NodeId is a plain integer, no overhead
- **Instant cleanup** — dropping the Vec frees everything at once

**Trade-offs:**
- Nodes can't be individually freed (the arena grows monotonically). This is fine because the DOM is immutable after construction — we never delete individual nodes.
- `NodeId` values are only meaningful within their arena. Mixing IDs from different DOMs would be a bug (but Rust's type system doesn't prevent this).
- The `children: Vec<NodeId>` still allocates per-node. A more extreme data-oriented approach would use a flat child list with offsets, but the current design is a good balance.

**How other engines handle this:**
- Blink (Chrome): C++ objects with raw pointers, garbage collected via Oilpan
- Gecko (Firefox): C++ objects with cycle-collected pointers
- Servo: Rust objects with custom prevent-RC mechanism
- Ladybird: C++ objects with traditional allocation

Asteria's approach is closest to ECS (Entity Component System) patterns used in game engines — flat storage, integer handles, data-oriented traversal.

---

## Why zero-copy tokenization?

**Decision:** Store byte offset pairs `(start, end)` in tokens and DOM nodes instead of copied strings.

**Alternatives considered:** Allocating `String` for each token, using `Cow<str>`

**Rationale:**

A typical HTML page contains hundreds of tokens — tag names, attribute names, attribute values, text content. If each one allocates a `String`, you get hundreds of small heap allocations during tokenization alone.

By storing byte offsets into the original input buffer, the tokenizer allocates zero strings. The DOM inherits this — element tag names and text content are offsets, not owned strings. The original byte buffer is kept alive alongside the DOM.

**Trade-offs:**
- The original input buffer must outlive the DOM (it's borrowed, not owned)
- Looking up actual text requires slicing into the source buffer and converting to `&str`
- String comparison requires the source buffer to be available

These are manageable constraints. The performance benefit is significant on large documents.

---

## Why wgpu for GPU rendering?

**Decision:** Use wgpu as the GPU backend.

**Alternatives considered:** OpenGL (via glow), Vulkan directly, software rendering

**Rationale:**

wgpu implements the WebGPU standard and provides a modern, safe GPU API that works across all major backends:
- Vulkan (Windows/Linux)
- Metal (macOS/iOS)
- DX12 (Windows)
- WebGL/WebGPU (browser, future)

Compared to raw Vulkan, wgpu handles device selection, surface management, and resource lifecycle automatically. Compared to OpenGL, wgpu is designed for modern GPU architectures and doesn't carry legacy baggage.

The Rust API is safe — no `unsafe` needed for basic rendering. And the wgpu ecosystem includes glyphon for text rendering, which integrates cleanly.

**Trade-offs:**
- wgpu is an abstraction layer, so we can't access backend-specific features
- The WebGPU API is still evolving (though wgpu is stable for our use case)
- Some platforms (older GPUs, embedded systems) may not support wgpu's minimum requirements

---

## Why a custom networking stack?

**Decision:** Build HTTP/1.1, DNS, TCP, and TLS handling from scratch instead of using `reqwest` or `hyper`.

**Alternatives considered:** `reqwest` (high-level HTTP client), `hyper` (low-level HTTP library)

**Rationale:**

Asteria's philosophy is to understand every layer. The networking stack is a core part of a browser engine — connection pooling, DNS caching, streaming responses, and TLS negotiation all directly affect page load performance.

Building it ourselves gives us:
- Full control over connection lifecycle and reuse
- Custom DNS caching with TTL management
- Streaming integration with the HTML parser (chunks arriving progressively)
- Understanding of the HTTP protocol at the byte level

We still use `rustls` for TLS and `webpki-roots` for certificate verification — cryptography is one area where using a well-audited library is the responsible choice.

**Trade-offs:**
- Our HTTP client is HTTP/1.1 only (no HTTP/2 or HTTP/3 yet)
- No cookie handling, redirect policies, or other high-level HTTP features
- More code to maintain compared to using a mature library

---

## Why immutable DOM?

**Decision:** The DOM is immutable after construction. No pipeline stage modifies it.

**Alternatives considered:** Mutable DOM with change notifications, copy-on-write DOM

**Rationale:**

If the DOM is mutable, every stage that reads it must account for the possibility that a later stage has modified it. This creates complex ordering dependencies, requires synchronisation for parallel processing, and makes debugging harder.

By making the DOM immutable after parsing:
- **Style resolution** reads the DOM freely, producing its own `StyledTree`
- **Layout** reads the styled tree, producing its own `LayoutTree`
- **Painting** reads the layout tree, producing its own `DisplayList`
- No stage's output depends on another stage's mutations

This clean separation makes the pipeline easier to test (each stage is pure function-like), debug (inspect any stage's output independently), and eventually parallelise (immutable data can be shared across threads without locks).

**Trade-offs:**
- Memory: we maintain multiple parallel data structures (DOM, styled tree, layout tree)
- When JavaScript eventually modifies the DOM, the immutability constraint will need to be relaxed for mutation operations, though the pipeline can still treat each "version" of the DOM as immutable during a render pass

---

## Why a flat scene graph?

**Decision:** Store scene nodes in a flat `Vec<SceneNode>` instead of a hierarchical tree.

**Alternatives considered:** Tree-based scene graph with parent-child pointers, ECS-style separate component arrays

**Rationale:**

The scene graph exists for one purpose: efficient GPU rendering. GPU rendering doesn't need tree structure — it needs flat lists of vertices grouped by draw call. A flat scene graph is:

- Cache-friendly for sequential iteration (batch building)
- Easy to sort by z-order
- Easy to segment into viewport tiles
- Simple to mark dirty flags for incremental re-rendering

**Trade-offs:**
- Parent-child relationships are stored as `Option<SceneNodeId>` rather than tree edges — traversal up the tree requires iteration
- Not as natural for hierarchical operations (clip regions, transform stacking) — when these are needed, additional data structures may be required

---

## Why separate render passes?

**Decision:** Split GPU rendering into three passes: rect, image, text.

**Alternatives considered:** Single unified pass, per-element rendering

**Rationale:**

GPU rendering is fastest when you minimise state changes. Each render pass uses a different pipeline (different shaders, vertex formats, and possibly textures). By grouping all operations of the same type:

- One pipeline bind per pass (3 total) instead of per element (potentially hundreds)
- Vertex data is packed into contiguous type-specific buffers
- The GPU can process each batch without pipeline switching

The three-pass architecture (`rects → images → text`) also matches the visual layering: backgrounds behind images behind text.

**Trade-offs:**
- Correct z-ordering across types requires the passes to be ordered correctly. Currently, all rects render behind all images, which render behind all text. Per-element z-index interleaving across types is not yet supported.

---

## Why a string interner?

**Decision:** Intern common strings into `Symbol(u32)` handles with pre-seeded values.

**Alternatives considered:** Using `String` everywhere, using `&str` with lifetime tracking

**Rationale:**

Tag names (`"div"`, `"p"`, `"span"`) and CSS property names (`"color"`, `"margin"`, `"display"`) appear thousands of times in a typical page. Without interning, each occurrence is a separate string allocation, and every comparison is byte-by-byte.

The interner ensures:
- Each unique string is stored once in memory
- Comparison is a single integer `==` operation
- Common strings have deterministic, known `Symbol` values (enabling fast `match` expressions)
- Total memory for tag names is bounded regardless of page size

Pre-seeding 67 common HTML tags, attributes, and CSS properties means the most frequent lookups are constant-time with no hash table probe.

**Trade-offs:**
- Symbols are only meaningful within their interner — passing symbols between interners is a bug
- Resolving a symbol back to a string requires the interner (for debugging/display)
- The pre-seeded list must be kept in sync with what the engine actually uses

---

## Future decisions to document

As Asteria grows, the following decisions will be documented here:

- **JavaScript engine choice** — integrate V8/SpiderMonkey or build from scratch?
- **CSS Grid implementation** — constraint-based or grid-template-based algorithm?
- **Layer compositing** — when to promote elements to GPU layers?
- **Incremental layout** — subtree invalidation strategy?
- **Font loading** — custom font discovery and rendering pipeline?
- **Accessibility** — accessibility tree design?

---

## Related documents

- [How Asteria Works](02-how-asteria-works.md) — the pipeline these decisions shape
- [DOM](05-dom.md) — arena allocation in practice
- [GPU Renderer](09-gpu-renderer.md) — wgpu and render passes in practice
- [Performance](12-performance.md) — how these decisions affect performance
- [FAQ](16-faq.md) — high-level questions about project direction
