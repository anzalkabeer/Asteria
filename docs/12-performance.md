# Performance

> **Purpose:** Explain Asteria's approach to performance — the design choices, optimisation strategies, and profiling tools.
>
> **Audience:** Systems programmers, performance-oriented contributors.
>
> **Estimated reading time:** 10 minutes
>
> **Prerequisites:** [How Asteria Works](02-how-asteria-works.md), [Project Architecture](03-project-architecture.md)

---

## Philosophy

Performance in a browser engine isn't about micro-optimising hot loops (though that matters eventually). It's about making the right architectural choices so that the engine is fast *by default*.

Asteria's performance strategy can be summarised as:

1. **Choose the right data structures** — flat arrays over pointer trees, arenas over per-object allocation
2. **Minimise allocation** — zero-copy parsing, bump allocation, object pooling
3. **Batch GPU work** — fewer draw calls, more data per call
4. **Measure everything** — microsecond-precision profiling at every pipeline stage

---

## Arena allocation

The DOM is stored in a `Vec<Node>` with `NodeId(u32)` handles. The scene graph uses `Vec<SceneNode>` with `SceneNodeId(u32)` handles. This isn't a coincidence — it's a deliberate architectural pattern.

### Why arenas win

| Operation | Heap-allocated tree | Arena |
|---|---|---|
| Allocate a node | `malloc()` → system call, fragmentation | `vec.push()` → amortised O(1) |
| Access a node | Pointer dereference (cache miss likely) | Array index (cache-friendly) |
| Traverse N nodes | N pointer chases | Sequential memory access |
| Free all nodes | N individual destructors | Drop one Vec |
| Memory overhead | Per-object header + alignment padding | Zero overhead (packed) |

For a DOM with 500 nodes, the arena approach means:
- **1 allocation** (the Vec itself) vs. **500 allocations** (one per node)
- **Contiguous memory** (prefetch-friendly) vs. **scattered heap** (cache-hostile)
- **Instant cleanup** (drop the Vec) vs. **500 destructor calls**

### Frame arena

**Source file:** `src/arena.rs`

The `FrameArena` is a bump allocator for per-frame temporary data:

```rust
pub struct FrameArena {
    buffer: Vec<u8>,
    offset: usize,
    capacity: usize,
}
```

Allocation is a pointer bump — move the offset forward by the requested size. Deallocation is a single reset — set the offset back to zero. No individual frees, no fragmentation, no overhead.

```
Allocate 100 bytes: offset 0 → 100    (instant)
Allocate 200 bytes: offset 100 → 300  (instant)
Allocate 50 bytes:  offset 300 → 350  (instant)
Reset frame:        offset 350 → 0     (instant)
```

The frame arena also supports typed allocation with proper memory alignment via `alloc_typed<T>()`.

---

## Cache locality

Modern CPUs are orders of magnitude faster than main memory. The gap is bridged by caches — small, fast memory close to the CPU. Data that's stored contiguously in memory is loaded into cache together, making subsequent accesses near-instant.

Asteria's data-oriented design exploits this:

| Data structure | Layout | Cache behaviour |
|---|---|---|
| DOM nodes | Contiguous `Vec<Node>` | Sequential traversal stays in cache |
| Scene nodes | Contiguous `Vec<SceneNode>` | Batching iterates sequentially |
| Colour data | Parallel array `Vec<[f32; 4]>` | GPU prep reads colours linearly |
| Text data | Parallel array `Vec<TextRun>` | Text rendering reads sequentially |

Compare this to a pointer-heavy tree where each node might live in a different cache line, causing a cache miss on every access.

---

## String interning

**Source file:** `src/interner.rs`

In a browser engine, the same strings appear thousands of times: `"div"`, `"class"`, `"color"`, `"margin"`. The string interner maps each unique string to a 4-byte `Symbol(u32)`:

```
"div"    → Symbol(3)
"color"  → Symbol(42)
"margin" → Symbol(48)
```

Benefits:
- **Comparison** — `Symbol == Symbol` is a single integer comparison vs. byte-by-byte string comparison
- **Memory** — one copy of each string, shared via `Rc<str>`, vs. thousands of duplicates
- **Hashing** — hash a u32 vs. hash an entire string
- **Pre-seeding** — 67 common HTML tags, attributes, and CSS properties are assigned deterministic Symbol values at startup, enabling fast `match` statements

---

## Zero-copy parsing

The HTML tokenizer stores byte offset pairs `(start, end)` into the original input buffer rather than copying text into new strings:

```
Input: "<div class="main">Hello</div>"

Token: StartTag { tag_start: 1, tag_end: 4 }  ← points to "div"
       Attr { name: (5,10), value: (12,16) }   ← points to "class", "main"
Token: Text { start: 18, end: 23 }              ← points to "Hello"
Token: EndTag { tag_start: 25, tag_end: 28 }    ← points to "div"
```

No string allocations during tokenization. The DOM nodes also store offsets, not strings. The original byte buffer is kept alive alongside the DOM, and text is only materialised when needed (e.g., for display or comparison).

---

## GPU batching

Instead of issuing one GPU draw call per visual element, the renderer groups elements by type and draws them in bulk:

```
Naive approach:                  Asteria's approach:
  100 rect draw calls             1 rect draw call (100 quads batched)
  50 text draw calls              1 text draw call (50 text runs batched)
  = 150 GPU state changes         = 2 GPU state changes
```

Each render pass (rect, image, text) collects all relevant scene nodes, packs their vertex data into a single buffer, and issues one draw call. This dramatically reduces GPU state transitions, which are the primary bottleneck in real-time rendering.

### Pipeline reuse

GPU render pipelines (shader programs + render state) are created once at startup and reused every frame. Only the vertex data changes. This avoids the expensive pipeline creation cost during rendering.

---

## Object pool

**Source file:** `src/pool.rs`

The object pool pre-allocates reusable buffers to reduce allocation pressure during rendering. Instead of allocating new `Vec`s for vertex data each frame, the pool hands out pre-allocated buffers that are returned and reused.

---

## LRU cache

**Source file:** `src/cache.rs`

The `LruCache` bounds memory usage by evicting the least-recently-used entries when the cache reaches capacity. This is used for:
- Loaded resources (avoiding redundant file reads or network fetches)
- Decoded images (avoiding repeated decoding)

---

## Profiling

**Source file:** `src/profiler.rs`

The `EngineProfiler` measures every pipeline stage with microsecond precision:

```
── Asteria Engine Performance Profile ──────────────────
Total Pipeline Duration : 2.847 ms (351.2 FPS)
DOM Nodes / Layout / Commands: 15 / 12 / 28
────────────────────────────────────────────────────────
  HTML Parsing    : 0.234 ms   (  8.2%)
  CSS Parsing     : 0.156 ms   (  5.5%)
  Style Resolution: 0.412 ms   ( 14.5%)
  2D Layout Engine: 0.523 ms   ( 18.4%)
  Display List    : 0.189 ms   (  6.6%)
  GPU Render Pass : 1.333 ms   ( 46.8%)
────────────────────────────────────────────────────────
```

### RAII stage guards

Timing is done with `StageGuard` — an RAII scope guard that starts a timer on creation and records the elapsed time when dropped:

```rust
{
    let _guard = profiler.stage_guard(EngineStage::Layout);
    // ... layout computation happens here ...
} // guard dropped → duration recorded automatically
```

This pattern ensures timing is always accurate, even if the code panics.

### Devtools and tracing

**Source directory:** `src/devtools/`

The Asteria Observability Framework (AOF) provides:

- **Chrome Trace export** — pipeline events are recorded and exported to `trace.json`, which can be loaded in Chrome's `chrome://tracing` viewer
- **Memory inspector** — tracks bytes allocated and GPU VRAM used
- **Energy diagnostics** — estimates the computational "energy impact" of a frame based on allocation count and GPU uploads
- **Engine snapshots** — point-in-time captures of DOM, style, layout, and scene state for debugging

---

## Optimisation roadmap

| Optimisation | Status | Impact |
|---|---|---|
| Arena DOM | ✅ | Eliminates per-node allocation |
| Zero-copy tokenizer | ✅ | Eliminates string allocation during parsing |
| String interning | ✅ | Eliminates duplicate strings, fast comparison |
| GPU batching | ✅ | Reduces draw calls by 10-100x |
| Pipeline reuse | ✅ | Avoids per-frame GPU pipeline creation |
| Frame arena | ✅ | Zero-cost per-frame allocation |
| LRU resource cache | ✅ | Avoids redundant loads |
| Object pool | ✅ | Reduces allocation churn |
| Incremental layout | 🔜 | Re-compute only changed subtrees |
| Selector indexing | 🔜 | Speed up CSS matching for large stylesheets |
| Style sharing cache | 🔜 | Reuse computed styles for similar elements |
| Layer compositing | 🔜 | Avoid re-rendering unchanged layers |
| Parallel layout | 🔜 | Use multiple threads for independent subtrees |
| Texture atlasing | 🔜 | Reduce texture switches during rendering |

---

## Related documents

- [DOM](05-dom.md) — arena allocation in detail
- [GPU Renderer](09-gpu-renderer.md) — batching and rendering strategy
- [Design Decisions](18-design-decisions.md) — why these choices were made
- [Glossary](17-glossary.md) — performance terminology
