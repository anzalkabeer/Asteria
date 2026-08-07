# DOM

> **Purpose:** Explain Asteria's Document Object Model — the tree structure at the heart of the engine.
>
> **Audience:** Contributors and systems programmers interested in data structure design.
>
> **Estimated reading time:** 10 minutes
>
> **Prerequisites:** [HTML Engine](04-html-engine.md)

---

## What is the DOM?

The DOM (Document Object Model) is the browser's internal representation of a web page. After the HTML parser finishes, the raw text has been transformed into a tree of nodes — elements, text, and comments — connected by parent-child relationships.

Every subsequent stage of the engine works on this tree. The style resolver walks it to match CSS selectors. The layout engine traverses it to compute positions. The paint engine reads it to extract text content. The DOM is the single source of truth for what the page contains.

---

## Arena allocation

**Source file:** `src/dom.rs`

Most browser engines store DOM nodes as heap-allocated objects linked by pointers (or smart pointers like `Rc`, `Arc`, or `Box`). This is flexible but has costs: each node is a separate allocation, reference counting adds overhead, and pointer chasing hurts cache performance.

Asteria takes a different approach: **arena allocation**.

All DOM nodes live in a single, contiguous `Vec<Node>`:

```rust
pub struct Dom {
    pub nodes: Vec<Node>,
}
```

Instead of pointers, nodes reference each other using lightweight **index handles**:

```rust
pub struct NodeId(pub u32);
```

A `NodeId` is just a 32-bit integer — the index of a node in the `Vec`. To get a node, you index into the vector:

```rust
// Get a reference to node #5
let node = dom.get(NodeId(5));
```

### Why this matters

| Aspect                 | Pointer-based DOM       | Arena DOM                                                   |
| ---------------------- | ----------------------- | ----------------------------------------------------------- |
| **Node size**          | 8+ bytes per pointer    | 4 bytes per `NodeId`                                        |
| **Cache locality**     | Nodes scattered in heap | Nodes packed contiguously                                   |
| **Allocation**         | One `malloc` per node   | One `Vec::push` per node                                    |
| **Deallocation**       | Per-node destructor     | Drop the entire `Vec`                                       |
| **Reference counting** | Required (Rc/Arc)       | Not needed                                                  |
| **Memory safety**      | Runtime borrow checks   | Safe Rust bounds checks (checked at runtime via `Dom::get`) |

For a page with 500 elements, the arena approach means 500 nodes packed tightly in memory, accessed by simple array indexing. The CPU cache can prefetch the next nodes efficiently, and there's no reference-counting overhead on every access.

### How other engines compare

| Engine      | DOM storage                                      | Trade-off                        |
| ----------- | ------------------------------------------------ | -------------------------------- |
| **Blink**   | C++ objects with raw pointers, garbage collected | Flexible but complex GC          |
| **Gecko**   | C++ objects with cycle-collected pointers        | Handles cycles but adds overhead |
| **Servo**   | Rust objects with custom prevent-rc mechanism    | Safe but still heap-allocated    |
| **Asteria** | Arena `Vec<Node>` with `NodeId(u32)` handles     | Fast access, simple memory model |

---

## Node structure

Every node in the DOM has the following fields:

```rust
pub struct Node {
    pub kind: NodeKind,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub attributes: Vec<(u32, u32, u32, u32)>,
    pub flags: NodeFlags,
}
```

### Node kinds

```rust
pub enum NodeKind {
    Document,
    Element { tag_start: u32, tag_end: u32 },
    Text { start: u32, end: u32 },
    Comment { start: u32, end: u32 },
}
```

| Kind       | Description                               | Offset meaning                                       |
| ---------- | ----------------------------------------- | ---------------------------------------------------- |
| `Document` | The root node — every DOM has exactly one | None                                                 |
| `Element`  | An HTML element like `<div>` or `<h1>`    | `tag_start..tag_end` → tag name in the source buffer |
| `Text`     | Raw text between tags                     | `start..end` → text content in the source buffer     |
| `Comment`  | An HTML comment                           | `start..end` → comment content in the source buffer  |

Notice that `Element`, `Text`, and `Comment` all store byte offset pairs — not strings. This is the zero-copy design from the [tokenizer](04-html-engine.md). To get the actual tag name of an element, you slice into the original HTML byte buffer:

```rust
// Get the tag name of an element node
if let NodeKind::Element { tag_start, tag_end } = node.kind {
    let tag_name = &source_bytes[tag_start as usize..tag_end as usize];
    // tag_name is now a &[u8] like b"div"
}
```

### Attributes

Attributes are stored the same way — as four byte offsets:

```
(name_start, name_end, value_start, value_end)
```

For `<div class="main">`, the attribute would be stored as:

- `name_start..name_end` → points to `"class"` in the source buffer
- `value_start..value_end` → points to `"main"` in the source buffer

This means the DOM doesn't own any strings at all. The original input buffer is the single source of text data.

### Node flags

```rust
pub struct NodeFlags {
    pub needs_style: bool,
    pub needs_layout: bool,
    pub needs_paint: bool,
}
```

These flags track invalidation status. The DOM is **structurally immutable** after parsing (its hierarchy, node types, attributes, and text boundaries do not change), but metadata flags (`NodeFlags`) can be updated when interactive events (such as window resizing or hover state transitions) mark specific nodes for partial re-processing.

---

## Tree relationships

Every node (except the root) has a `parent: Option<NodeId>` pointing to its parent. Every node has a `children: Vec<NodeId>` listing its child nodes.

```
NodeId(0): Document
  └── NodeId(1): Element <html>
        ├── NodeId(2): Element <head>
        │     └── NodeId(3): Element <title>
        │           └── NodeId(4): Text "Example"
        └── NodeId(5): Element <body>
              ├── NodeId(6): Element <h1>
              │     └── NodeId(7): Text "Hello"
              └── NodeId(8): Element <p>
                    └── NodeId(9): Text "World"
```

### Traversal

Walking the tree is straightforward:

```rust
// Depth-first traversal
fn visit(dom: &Dom, node_id: NodeId) {
    let node = dom.get(node_id);
    // process this node...

    for &child_id in &node.children {
        visit(dom, child_id);
    }
}

// Start from the root
visit(&dom, NodeId(0));
```

Because nodes are stored contiguously and accessed by index, this traversal has excellent cache performance — far better than chasing heap pointers.

---

## Ownership model

The DOM is **structurally immutable after construction**. The parser builds the tree topology, node kinds, and attribute offsets once. Subsequent pipeline stages do not alter the tree structure.

Instead, each stage creates its own parallel data structure:

```
DOM (structurally immutable)
  │
  ├── StyledTree (style resolver creates this)
  │
  ├── LayoutTree (layout engine creates this)
  │
  └── DisplayList (paint engine creates this)
```

This design has several benefits:

- **No structural borrow conflicts** — tree hierarchy and node data can be shared safely across read-only pipeline passes
- **Clean separation** — each stage's output is self-contained
- **Easy debugging** — you can inspect the DOM at any point without worrying about structural mutations
- **Parallelism potential** — immutable structural data can be safely shared across threads, while invalidation metadata flags (`NodeFlags`) track dirty states for incremental passes

---

## DOM construction

The DOM is constructed by the parser during tree-building. The parser calls methods on the `Dom`:

```rust
// Create a new element node as a child of parent
let node_id = dom.add_element(parent_id, tag_start, tag_end);

// Add attributes to the element
dom.set_attributes(node_id, attributes);

// Create a text node as a child of parent
let text_id = dom.add_text(parent_id, start, end);
```

After construction, the DOM provides read-only access:

```rust
// Get a node by ID
let node: &Node = dom.get(node_id);

// Get the tag name of an element
let name: Option<&str> = dom.tag_name(node_id, source_bytes);

// Get an attribute value
let value: Option<&str> = dom.attribute_value(node_id, "class", source_bytes);

// Pretty-print the tree
dom.print_tree(source_bytes);
```

---

## Memory characteristics

For a page with 500 nodes:

| Metric             | Approximate value                      |
| ------------------ | -------------------------------------- |
| Node storage       | ~500 × ~80 bytes = ~40 KB (contiguous) |
| NodeId overhead    | 4 bytes per reference                  |
| String allocations | Zero (zero-copy offsets)               |
| Reference counting | None                                   |
| Deallocation       | Instant (drop the Vec)                 |

Compare this to a traditional heap-allocated DOM where each node might be a 200+ byte object scattered across the heap, linked by 8-byte pointers, with reference counting on every access.

---

## Related documents

- [HTML Engine](04-html-engine.md) — how the parser builds this tree
- [CSS Engine](06-css-engine.md) — how styles are matched to DOM nodes
- [Layout Engine](07-layout-engine.md) — how the DOM drives layout computation
- [Design Decisions](18-design-decisions.md) — why arena allocation was chosen
- [Glossary](17-glossary.md) — DOM terminology
