# Testing

> **Purpose:** Explain Asteria's testing strategy — what's tested, how, and why.
>
> **Audience:** Contributors who want to write tests or understand the test suite.
>
> **Estimated reading time:** 7 minutes
>
> **Prerequisites:** [Project Architecture](03-project-architecture.md)

---

## Testing philosophy

Browser engines are notoriously hard to test because correctness is often visual — "does this page look right?" Asteria uses a layered testing approach:

1. **Unit tests** — verify individual functions and data structures
2. **Integration tests** — verify that pipeline stages work together correctly
3. **Visual fixtures** — HTML/CSS pages that exercise specific rendering features
4. **Observability tests** — verify profiling, tracing, and diagnostics

---

## Running tests

```bash
# Run the full test suite
cargo test

# Run tests with output visible
cargo test -- --nocapture

# Run a specific test file
cargo test --test style_integration
cargo test --test layout_integration
cargo test --test paint_integration
cargo test --test image_integration
cargo test --test network_integration
cargo test --test renderer_integration
cargo test --test observability_trace

# Run tests matching a name pattern
cargo test specificity
cargo test margin_centering
```

---

## Unit tests

Unit tests live inside each source module, in a `#[cfg(test)] mod tests` block. They test individual functions in isolation.

### Examples of what unit tests cover

| Module | What's tested |
|---|---|
| `tokenizer.rs` | Token emission for various HTML constructs |
| `parser.rs` | DOM tree shape for specific HTML inputs |
| `dom.rs` | Node creation, child insertion, attribute access |
| `css_tokenizer.rs` | CSS token types for various CSS syntax |
| `css_parser.rs` | Selector parsing, declaration parsing, media queries |
| `style.rs` | Specificity calculation, selector matching, inheritance |
| `values.rs` | Colour parsing, length unit conversion, style computation |
| `properties.rs` | Property ID lookup, inheritance flags, shorthand rules |
| `layout.rs` | Box model computation, margin centering, flex layout |
| `paint.rs` | Display command generation, paint ordering |
| `scene.rs` | Scene node creation, z-ordering, dirty flags |
| `interner.rs` | String interning, symbol lookup, pre-seeded values |
| `arena.rs` | Bump allocation, typed allocation, reset |
| `cache.rs` | LRU eviction, hit/miss behaviour |
| `net/dns.rs` | DNS resolution, TTL caching |
| `net/http.rs` | URL parsing, request formatting, response parsing |

---

## Integration tests

Integration tests live in the `tests/` directory. Each file exercises a complete subsystem or cross-subsystem flow.

### `style_integration.rs`

Tests the CSS pipeline end-to-end:
- Parse HTML → DOM
- Parse CSS → Stylesheet
- Resolve styles → Computed styles on each element
- Verify that specificity, inheritance, and cascade produce correct results

**Example scenarios:**
- ID selector overrides class selector
- Inline `style=""` beats stylesheet rules
- Inherited `color` flows from parent to child
- `@media` rules activate/deactivate based on viewport width

### `layout_integration.rs`

Tests the layout engine:
- Verify box dimensions for block elements
- Verify margin centering with `margin: auto`
- Verify inline text wrapping at container boundaries
- Verify flex layout positioning
- Verify anonymous block generation for mixed children

### `paint_integration.rs`

Tests display list generation:
- Verify correct paint order (backgrounds before borders before text)
- Verify that commands carry correct positions and sizes
- Verify link URL propagation through display commands

### `image_integration.rs`

Tests image format detection:
- PNG magic byte detection
- JPEG magic byte detection
- BMP, GIF, WebP, TIFF, SVG detection
- Unknown format handling

### `network_integration.rs`

Tests the networking stack:
- URL parsing (scheme, host, port, path)
- Local HTTP client parsing, TLS handshakes, and response structure verification (deterministic local flows avoiding remote network dependencies in default CI runs)

### `renderer_integration.rs`

Tests the GPU rendering pipeline:
- Scene graph construction from display lists
- Batch builder output
- Render command generation
- Pass coordination

### `observability_trace.rs`

Tests the devtools and profiling infrastructure:
- Trace event recording
- Chrome Trace JSON export format
- Engine snapshot construction
- Memory metrics tracking

### `comprehensive_edge_cases.rs`

Covers boundary conditions across multiple subsystems:
- Empty documents
- Deeply nested elements
- Large attribute counts
- Malformed HTML
- Edge cases in CSS parsing

---

## Visual test fixtures

**Directory:** `tests/fixtures/`

These are real HTML/CSS pages that exercise specific rendering features. They're not automated — you run them manually and visually verify the output.

### `blog.html`

A blog article layout testing:
- Navigation bar with dark background
- Article with styled headings and paragraphs
- Callout box with left border accent
- Footer with separator border
- CSS inheritance and border-left styling

```bash
cargo run -- tests/fixtures/blog.html
```

### `gallery.html`

A flex card gallery testing:
- CSS `display: flex` row layout
- Multiple cards with fixed widths
- Image placeholder frames
- Background colours and borders
- Text within flex items

```bash
cargo run -- tests/fixtures/gallery.html
```

### `hello.html`

A minimal test page for basic rendering verification:

```bash
cargo run -- tests/fixtures/hello.html
```

---

## CI pipeline

The GitHub Actions CI pipeline (`ci.yml`) runs on every push and pull request:

```yaml
steps:
  - cargo check --all-targets --verbose    # Compilation check
  - cargo fmt --all -- --check             # Formatting check
  - cargo clippy --all-targets -- -D warnings  # Lint check
  - cargo test --verbose                   # Full test suite
```

All four checks must pass for a PR to be merged.

---

## Writing new tests

### Where to put tests

| Type | Location |
|---|---|
| Unit tests for `src/foo.rs` | Inside `src/foo.rs` in a `#[cfg(test)] mod tests` block |
| Integration tests | `tests/foo_integration.rs` |
| Visual test pages | `tests/fixtures/foo.html` |

### Test naming

Use descriptive, behaviour-oriented names:

```rust
#[test]
fn id_selector_beats_class_selector_in_specificity() { ... }

#[test]
fn block_element_expands_to_full_container_width() { ... }

#[test]
fn inline_text_wraps_at_container_boundary() { ... }
```

### Test structure

Follow the Arrange-Act-Assert pattern:

```rust
#[test]
fn margin_auto_centers_element() {
    // Arrange — set up input HTML bytes and parse DOM/stylesheet
    let bytes = b"<!DOCTYPE html><html><body><div style='width:200px;margin:0 auto'>content</div></body></html>";
    let mut processor = StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();
    let stylesheet = Stylesheet::parse(b"");

    // Act — run style resolution and 2D layout calculation
    let styled = resolve_styles(&dom, &stylesheet, bytes);
    let layout = layout_document(&styled, &dom, bytes, 800.0, 600.0).unwrap();

    // Assert — verify layout tree box generation
    assert!(layout.box_count() > 0);
}
```

---

## Future testing improvements

| Improvement | Description |
|---|---|
| Screenshot comparison tests | Render a page, capture a screenshot, compare to a reference image |
| Web Platform Tests (WPT) | Run the W3C/WHATWG standard test suite against Asteria |
| Fuzz testing | Use `cargo-fuzz` to test parsers against random input |
| Performance benchmarks | `cargo bench` benchmarks for hot paths |
| Layout comparison | Compare Asteria's layout output against Chrome DevTools layout data |

---

## Related documents

- [Contributing](14-contributing.md) — how to submit changes
- [Project Architecture](03-project-architecture.md) — where test files live
- [Known Limitations](20-known-limitations.md) — known failures and edge cases
