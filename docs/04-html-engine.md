# HTML Engine

> **Purpose:** Explain how Asteria reads raw HTML and constructs a document tree.
>
> **Audience:** Contributors and developers interested in parsing.
>
> **Estimated reading time:** 10 minutes
>
> **Prerequisites:** [How Asteria Works](02-how-asteria-works.md)

---

## Overview

The HTML engine is the entry point of Asteria's pipeline. It takes raw bytes — from a local file or a network response — and produces a structured DOM tree that every subsequent stage depends on.

The process has two phases:

1. **Tokenization** — breaking raw bytes into meaningful chunks (tokens)
2. **Parsing** — assembling tokens into a tree of DOM nodes

These phases run in sequence, though Asteria supports a streaming mode where they interleave with network data arrival.

---

## Tokenization

**Source file:** `src/tokenizer.rs`

The tokenizer is a **state machine** that reads HTML one byte at a time and emits tokens. It maintains an internal state that determines how each byte is interpreted.

### States

The tokenizer has the following states:

| State | Active when... |
|---|---|
| `Data` | Reading text content between tags |
| `TagOpen` | Just saw `<` — deciding if this is a start tag, end tag, or comment |
| `TagName` | Reading the name of a tag (e.g., `div`, `p`, `h1`) |
| `EndTagOpen` | Inside `</` — an end tag |
| `BeforeAttributeName` | Between the tag name and the first attribute |
| `AttributeName` | Reading an attribute name (e.g., `class`, `id`) |
| `AfterAttributeName` | Between the attribute name and `=` |
| `BeforeAttributeValue` | Just saw `=` — about to read the value |
| `AttributeValueDoubleQuoted` | Inside `"..."` |
| `AttributeValueSingleQuoted` | Inside `'...'` |
| `AttributeValueUnquoted` | Unquoted attribute value |
| `AfterAttributeValue` | Just finished an attribute value |
| `SelfClosingStartTag` | Saw `/` inside a tag — self-closing like `<br />` |
| `Comment` | Inside `<!-- ... -->` |
| `Doctype` | Inside `<!DOCTYPE ...>` |

### How it works

Consider this HTML:

```html
<div class="main">Hello</div>
```

The tokenizer processes it like this:

```
Byte: <     → State: Data → TagOpen
Byte: d     → State: TagOpen → TagName (tag_name_start = 1)
Byte: i     → State: TagName (continue)
Byte: v     → State: TagName (continue)
Byte: (spc) → State: TagName → BeforeAttributeName (tag_name_end = 4)
Byte: c     → State: BeforeAttributeName → AttributeName (attr_name_start)
...
Byte: >     → Emit StartTag token
Byte: H     → State: Data (token_start for text)
...
Byte: <     → Emit Text token, State: TagOpen
Byte: /     → State: EndTagOpen
...
Byte: >     → Emit EndTag token
```

### Token types

The tokenizer produces these token kinds:

| Kind | Example | Description |
|---|---|---|
| `StartTag` | `<div class="main">` | Opening tag with name and attributes |
| `EndTag` | `</div>` | Closing tag |
| `Text` | `Hello` | Text content between tags |
| `Comment` | `<!-- note -->` | HTML comment |
| `Doctype` | `<!DOCTYPE html>` | Document type declaration |
| `Eof` | *(end of input)* | End of file marker |

### Zero-copy design

This is a critical design choice. The tokenizer never allocates strings for tag names, attribute names, or attribute values. Instead, it records byte offset pairs — `(start, end)` — into the original input buffer.

For example, given the input `<div class="main">`:
- Tag name: bytes 1..4 → `"div"` (looked up from the source buffer when needed)
- Attribute name: bytes 5..10 → `"class"`
- Attribute value: bytes 12..16 → `"main"`

This eliminates thousands of small heap allocations on a typical page and keeps the tokenizer's memory footprint minimal.

---

## Parsing

**Source file:** `src/parser.rs`

The parser consumes tokens from the tokenizer and builds the DOM tree. It maintains a stack of open elements to track where in the tree new nodes should be inserted.

### The parsing algorithm

```
Initialize: create Document root, push to open stack

For each token:
  StartTag →
    1. Create a new Element node in the DOM
    2. Set it as a child of the current top of the stack
    3. Push it onto the open stack (unless self-closing or void)

  EndTag →
    1. Find the matching open element on the stack
    2. Pop the stack back to that element

  Text →
    1. Create a Text node
    2. Add it as a child of the current top of the stack

  Comment →
    1. Create a Comment node
    2. Add it as a child of the current top of the stack

  Eof →
    Parsing complete
```

### Implicit closing

HTML has rules about which elements can contain which other elements. For example, a `<p>` tag is implicitly closed when another block-level element is encountered:

```html
<p>First paragraph
<p>Second paragraph
```

The parser handles this by checking whether the current open element should be auto-closed before inserting a new element.

### Void elements

Some HTML elements can never have children: `<br>`, `<img>`, `<input>`, `<hr>`, `<meta>`, `<link>`. The parser recognises these and never pushes them onto the open stack.

### Error recovery

HTML parsing is intentionally forgiving. Browsers don't reject malformed HTML — they try to make sense of it. Asteria follows this philosophy:

- Mismatched end tags are ignored
- Unclosed tags are implicitly closed at end of input
- Unknown tags are treated as generic elements

---

## Streaming mode

**Source file:** `src/streaming_parser.rs`

For network-loaded pages, waiting for the entire HTML document before parsing would add unnecessary latency. Asteria's `StreamingHtmlProcessor` processes HTML chunks as they arrive from the network:

```
┌────────────────┐     ┌──────────────────┐     ┌──────────┐
│ Network chunks │────►│ StreamingHtml    │────►│   DOM    │
│ (partial HTML) │     │ Processor        │     │  (grows) │
└────────────────┘     └──────────────────┘     └──────────┘
```

Each call to `receive_network_chunk()` appends incoming bytes to an internal contiguous source buffer retained by the processor. Because token and DOM node offsets (`u32` start/end pairs) index directly into this buffer, retaining the complete byte buffer across chunk receptions ensures all slice references remain valid during subsequent style resolution, layout, and paint passes. When the final chunk arrives (marked with `is_eof = true`), the processor finalises the tree.

This means the browser can begin style resolution and layout on the accumulated DOM snapshot as chunks arrive, rather than waiting for the complete document download.

---

## Supported HTML

Asteria currently supports:

| Feature | Status |
|---|---|
| Standard elements (`div`, `p`, `h1`-`h6`, `span`, `a`, etc.) | ✅ |
| Attributes (`class`, `id`, `style`, `href`, `src`, etc.) | ✅ |
| Text content | ✅ |
| Comments | ✅ |
| DOCTYPE declarations | ✅ |
| Self-closing tags (`<br />`, `<img />`) | ✅ |
| Void elements (no closing tag needed) | ✅ |
| Implicit tag closing | ✅ |
| Nested elements | ✅ |
| Streaming/chunked parsing | ✅ |
| `<style>` tag content extraction | ✅ |
| `<link>` stylesheet discovery | ✅ |
| Character entities (`&amp;`, `&lt;`, etc.) | 🔜 |
| `<template>` elements | 🔜 |
| `<script>` execution | 🔜 |

---

## How other engines compare

| Engine | Tokenizer | Parser | DOM storage |
|---|---|---|---|
| **Blink (Chrome)** | State machine, C++ | Tree builder with adoption agency | Heap-allocated C++ objects |
| **Gecko (Firefox)** | State machine, C++ | nsHtml5TreeBuilder | Heap-allocated with cycle collector |
| **Servo** | html5ever (Rust) | html5ever tree builder | DOM objects with prevent_rc cycles |
| **Asteria** | State machine, Rust, zero-copy | Custom tree builder | Arena-allocated `Vec<Node>` |

Asteria's approach trades full HTML5 spec compliance (for now) for simplicity and performance. The zero-copy tokenizer and arena DOM keep allocation overhead extremely low.

---

## Related documents

- [DOM](05-dom.md) — the tree structure produced by the parser
- [Project Architecture](03-project-architecture.md) — where these files live
- [CSS Engine](06-css-engine.md) — the next stage in the pipeline
- [Glossary](17-glossary.md) — definitions of parsing terms
