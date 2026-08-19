# Asteria Rendering Engine — Codebase Flaws Report

> **Audit scope:** Every `src/` module, `src-tauri/` shell, and `tests/` suite.
> **Methodology:** Line-by-line code reading against CSS/HTML specs, Rust idioms, and the architecture described in `CLAUDE.md`.

---

## Table of Contents

1. [Critical Bugs](#1-critical-bugs)
2. [Logic Flaws](#2-logic-flaws)
3. [Specification Non-Compliance](#3-specification-non-compliance)
4. [Architecture-Level Flaws](#4-architecture-level-flaws)
5. [Code Quality & Maintainability](#5-code-quality--maintainability)
6. [Performance Issues](#6-performance-issues)
7. [Security Concerns](#7-security-concerns)
8. [Test Coverage Gaps](#8-test-coverage-gaps)

---

## 1. Critical Bugs

### 1.1 `parse_length()` — Percentage resolved against wrong base (`values.rs:454–456`)

```rust
if let Some(num) = s.strip_suffix('%') {
    return num.trim().parse::<f32>().unwrap_or(0.0) / 100.0 * em_base;
}
```

**Bug:** Percentage lengths are resolved against `em_base` (the element's font-size), but CSS `%` on `width`, `height`, `margin`, and `padding` should resolve against the **containing block's width** (or height for vertical properties). Only `font-size: 50%` should resolve against the parent font-size. This means `width: 50%` computes to `50% of font-size`, not `50% of parent width` — a fundamental layout calculation error that will make every percentage-based layout wrong.

**Impact:** All percentage-based widths, heights, margins, and paddings are computed incorrectly. A `width: 50%` on a child inside a 800px container computes to `8px` (50% of 16px font-size) instead of `400px`.

---

### 1.2 Shorthand expansion loses cascade priority (`style.rs:263–307`)

```rust
let mut expanded: HashMap<String, String> = HashMap::new();
for (prop, value) in &specified {
    if properties::is_shorthand(prop) {
        // ... expand ...
        if !specified.contains_key(longhand_name) {
            expanded.insert(longhand_name.to_string(), format!("{}px", px_val));
        }
    }
}
// Merge expanded shorthands (longhands take priority)
for (prop, value) in expanded {
    specified.entry(prop).or_insert(value);
}
```

**Bug:** The `specified` HashMap is iterated while building `expanded`, but `HashMap` iteration order is **non-deterministic**. If multiple shorthands or a shorthand and its longhands appear, the result depends on iteration order — which is undefined. Furthermore, the shorthand expansion pre-computes px values using `parent_style.font_size` before the element's own font-size has been resolved, creating a dependency ordering bug for `em`/`%`-based shorthand values.

**Impact:** Styles can resolve differently across runs. A `margin: 2em` declaration will be expanded using the parent's font-size instead of the element's (which hasn't been computed yet at expansion time).

---

### 1.3 `border` shorthand listed as shorthand but `expand_shorthand()` returns `None` for it (`properties.rs:205–226`)

```rust
pub fn is_shorthand(name: &str) -> bool {
    matches!(name, "margin" | "padding" | "border")
}

pub fn expand_shorthand(name: &str) -> Option<[PropertyId; 4]> {
    match name {
        "margin" => Some([...]),
        "padding" => Some([...]),
        _ => None,  // <-- "border" returns None here!
    }
}
```

**Bug:** `is_shorthand("border")` returns `true`, but `expand_shorthand("border")` returns `None`. In `style.rs`, the border shorthand has a special code path, but if anyone calls `expand_shorthand("border")` generically they get `None`. The `is_shorthand` / `expand_shorthand` API contract is inconsistent.

---

### 1.4 `property_from_name()` returns `None` for shorthands `margin` and `padding` (`properties.rs:199`)

```rust
"margin" | "padding" => None,
```

This means shorthand `margin` and `padding` declarations pass through the cascade step as raw strings but don't get a `PropertyId`. The expansion happens in `style.rs` but after the cascade `specified` map is built. If a longhand like `margin-top: 10px` and a shorthand `margin: 20px` both appear from different rules, the **cascade priority between them is lost** because shorthands have no `PropertyId` and are compared as raw strings, not as property-level overrides.

---

### 1.5 `render_layout_box` skips anonymous blocks (`paint.rs:98–99`)

```rust
fn render_layout_box(...) {
    if layout_box.styled_node.is_none() {
        return;  // <-- This early-returns for AnonymousBlock boxes
    }
    ...
}
```

**Bug:** Anonymous blocks (which wrap inline content inside block containers per CSS spec) have `styled_node = None`. The paint function returns immediately, meaning **all inline content wrapped in anonymous blocks is invisible** — it never generates display commands. This includes most text content in mixed inline/block layouts.

**Impact:** Text inside `<div><span>Hello</span><div>World</div></div>` — the "Hello" span gets wrapped in an anonymous block by `build_layout_tree`, which then gets skipped by the paint phase.

---

### 1.6 `layout_inline` ignores the containing block (`layout.rs:562–565`)

```rust
fn layout_inline(&mut self, _containing_block: Dimensions, dom: &Dom, source: &[u8]) {
    self.layout_block_children(dom, source);
}
```

**Bug:** The containing block is ignored (`_containing_block`). The inline node never computes its own width, position, margins, padding, or borders — it just recursively lays out children. This means inline elements have `Dimensions::default()` (all zeros) for their own box model, making them zero-width/zero-height invisible containers.

---

### 1.7 Inline layout double-layouts children (`layout.rs:339`)

```rust
// Recursively layout child's descendants
child.layout(child.dimensions, dom, source);
```

Inside the inline formatting context loop, each child is first manually positioned (lines 310–336), then `child.layout(child.dimensions, ...)` is called. This recursive call will re-enter `layout_block` or `layout_inline` and potentially **overwrite** the manually computed positions, since `calculate_block_position` computes position relative to the containing block's `content.y + content.height`, which is the child's own dimensions passed in.

---

## 2. Logic Flaws

### 2.1 `ComputedStyle::default()` sets `display: Inline` (`values.rs:255`)

```rust
display: Display::Inline,
```

**Flaw:** The CSS specification's initial value for `display` is indeed `inline`, but this means every element starts as inline and must be explicitly overridden. The UA stylesheet defaults in `style.rs` handle common block tags, but if a custom element or any unrecognized tag is used, it defaults to `inline` — which is correct per spec but leads to confusing behavior because the UA defaults are incomplete (missing `<blockquote>`, `<pre>`, `<figure>`, `<figcaption>`, `<details>`, `<summary>`, `<dl>`, `<dt>`, `<dd>`, `<table>`, `<thead>`, `<tbody>`, `<tfoot>`, `<caption>`, `<colgroup>`, `<col>`, `<address>`, `<fieldset>`, `<legend>`, `<hr>`, etc.).

---

### 2.2 `table`, `tr`, `td`, `th` are not in `is_default_block_tag()` (`style.rs:115–139`)

Table elements are missing from the UA stylesheet defaults. `<table>` should default to `display: table`, `<tr>` to `display: table-row`, `<td>`/`<th>` to `display: table-cell`. Currently they all default to `display: inline`, making all HTML tables render as inline text.

---

### 2.3 `hr` and `br` are not in `is_default_block_tag()` — `<hr>` should be block (`style.rs`)

`<hr>` should default to `display: block` with a default border. Currently defaults to inline.

---

### 2.4 `grid_gap` uses wrong edge values (`values.rs:323`, `layout.rs:445–446`)

```rust
PropertyId::GridGap => format!("{}px", self.grid_gap.top),  // values.rs:323
let gap_x = style.grid_gap.right;   // layout.rs:445
let gap_y = style.grid_gap.bottom;  // layout.rs:446
```

**Flaw:** The `gap` shorthand is parsed via `parse_edges()` (which fills top/right/bottom/left), but grid gap should just be `row-gap column-gap`. The display function shows `top`, layout reads `right` and `bottom`. This is internally inconsistent — nobody agrees on which edge stores which gap dimension.

---

### 2.5 `font-weight: lighter`/`bolder` use hardcoded values (`values.rs:619–620`)

```rust
"lighter" => 100.0,
"bolder" => 900.0,
```

Per CSS spec, `lighter` and `bolder` are **relative** to the inherited font-weight. `lighter` should subtract ~100 from the parent value, `bolder` should add ~100. Using fixed values means `lighter` on a 300-weight element still produces 100, and `bolder` on a 400-weight element jumps to 900 instead of 700.

---

### 2.6 `copy_property` copies non-inherited properties (`style.rs:542–545`)

```rust
PropertyId::Display => child.display = parent.display,
PropertyId::Position => child.position = parent.position,
PropertyId::Width => child.width = parent.width,
PropertyId::Height => child.height = parent.height,
```

**Flaw:** `copy_property` can copy `display`, `position`, `width`, `height`, `margin`, `padding`, and other non-inherited properties from parent to child. While this function is *called* only for properties where `is_inherited()` returns true, the `inherit` keyword case (line 382) calls it for ANY property. The function doesn't guard against misuse — and the `inherit` keyword on `margin-top` would incorrectly copy the parent's margin to the child.

Actually, `inherit` on non-inherited properties IS supposed to copy from parent per CSS spec. So the function is correct here — but the unconditional match arms for non-inherited properties could be confusing. This is a minor maintainability concern, not a bug.

---

### 2.7 Selector matching walks ancestors only for descendant combinators — `parts`-based path ignores child combinators (`style.rs:669–685`)

```rust
let mut current = dom.get(node_id).parent;
let mut part_idx = selector.parts.len() - 2;

loop {
    match current {
        None => return false,
        Some(ancestor_id) => {
            if compound_matches(&selector.parts[part_idx], ancestor_id, dom, source) {
                if part_idx == 0 { return true; }
                part_idx -= 1;
            }
            current = dom.get(ancestor_id).parent;
        }
    }
}
```

**Bug:** The `parts`-based matching path (lines 652–685, used when `steps` is empty) treats ALL multi-compound selectors as descendant selectors. It walks up the ancestor chain greedily. This legacy code path doesn't distinguish between `div p` (descendant) and `div > p` (child) because the `parts` structure doesn't encode combinators. The `steps`-based path (lines 688–756) correctly handles child, next-sibling, and subsequent-sibling combinators, but only if `selector.steps` is non-empty.

**Impact:** If any selector is constructed with `parts` but not `steps`, child selectors like `div > p` are treated as `div p`.

---

### 2.8 Flex gap is hardcoded to 16px (`layout.rs:385`)

```rust
let gap = 16.0;
```

The flex container's gap ignores the actual `grid_gap` style value and always uses 16px. CSS `gap` on flex containers should be respected.

---

### 2.9 Flex child default width is hardcoded to 200px (`layout.rs:391`)

```rust
let child_w = child.styled_node.and_then(|n| n.styles.width).unwrap_or(200.0);
```

**Flaw:** If a flex child has `width: auto`, it gets a hardcoded 200px instead of being sized by its content (intrinsic sizing) or flex grow/shrink factors. This breaks most flex layouts.

---

### 2.10 `var()` substitution can infinite-loop (`style.rs:326–346`)

```rust
while let Some(start) = result.find("var(") {
    // ... replace_range ...
}
```

If a CSS custom property value itself contains the literal string `var(` (e.g., `--x: "text containing var("`) or if there's a circular reference (`--a: var(--b)`, `--b: var(--a)`), this loop will never terminate. There's no recursion depth limit or cycle detection.

---

### 2.11 Scene graph `parent` is always `None` (`scene.rs:326, 385, etc.`)

```rust
parent: None,
```

Every scene node is created with `parent: None`. The `invalidate()` method walks up via `parent`, so it never actually propagates dirtiness. The entire incremental invalidation system (Pillar 3) is non-functional.

---

### 2.12 Host header omits port for HTTPS/443 (`http.rs:167–172`)

```rust
let host_header = if self.url.port == 80 {
    self.url.host.clone()
} else {
    self.url.host_port()
};
```

This only omits the port for HTTP/80. For HTTPS/443 (the default), the Host header will be `example.com:443` instead of just `example.com`. While technically valid, some servers reject this.

---

## 3. Specification Non-Compliance

### 3.1 `margin: auto` centering not supported

The block width algorithm (lines 177–191) only handles auto-expanding width and underflow distribution. CSS `margin-left: auto; margin-right: auto` centering works only if both margins happen to be 0.0 — it doesn't detect `margin: auto` as a keyword, because margin values are already parsed to px in `parse_length()` where `auto` would return 0.0.

---

### 3.2 No `!important` support

The cascade sorting (line 248–253) compares `origin → specificity → source_order` but doesn't handle `!important` declarations. Any `!important` in a stylesheet is silently treated as a normal declaration, breaking many real-world CSS layouts.

---

### 3.3 No `inherit` / `initial` / `unset` for shorthand properties

Shorthand expansion in `style.rs:264–303` doesn't handle `margin: inherit` or `padding: initial`. These are expanded via `parse_edges()` which will fail to parse the keyword and return `Edges::ZERO`.

---

### 3.4 No `currentColor` support (`values.rs`)

The CSS `currentColor` keyword is not handled anywhere. `color: currentColor` or `border-color: currentColor` will fall through to the `Color::BLACK` fallback.

---

### 3.5 CSS selector parsing doesn't handle attribute selectors with operators beyond `=` (`style.rs:816`)

```rust
return match op.as_str() {
    "=" => actual_val == val,
    _ => false,  // ~=, |=, ^=, $=, *= all return false
};
```

Only `=` (exact match) is supported. `~=` (word), `|=` (prefix-dash), `^=` (starts-with), `$=` (ends-with), and `*=` (contains) all silently fail.

---

### 3.6 No `box-sizing: border-box` support (`layout.rs`)

The block width algorithm always uses content-box sizing. `box-sizing: border-box` (used extensively in modern CSS via `* { box-sizing: border-box }`) is not supported, meaning padding and border are added outside the specified width.

---

### 3.7 No margin collapsing (`layout.rs:348–362`)

Adjacent vertical margins between block siblings should collapse (the larger wins). Currently both margins are fully applied, resulting in doubled spacing.

---

### 3.8 `background` shorthand mapped to `BackgroundColor` only (`properties.rs:181`)

```rust
"background-color" | "background" => Some(PropertyId::BackgroundColor),
```

The CSS `background` shorthand can contain `background-image`, `background-position`, `background-size`, `background-repeat`, etc. Mapping it directly to `BackgroundColor` means `background: url(img.png) no-repeat center` is treated as a color parse, producing `Color::BLACK`.

---

### 3.9 `<textarea>`, `<select>`, `<video>`, `<audio>`, `<canvas>`, `<iframe>` have no UA defaults

These elements have no special handling in the UA stylesheet section of `style.rs`, meaning they all default to `display: inline` with no intrinsic sizing.

---

## 4. Architecture-Level Flaws

### 4.1 String-heavy style resolution — interner exists but isn't used

The `interner.rs` module provides a well-designed `Symbol`-based string interner with pre-seeded CSS property names and HTML tags. However, the style resolution pipeline (`style.rs`) uses raw `String` for property names throughout:

```rust
let mut specified: HashMap<String, String> = HashMap::new();
```

Every style resolution allocates `String` objects for property names that could be `Symbol` lookups. The interner is defined but appears unused in the hot path — a significant wasted optimization opportunity.

---

### 4.2 Arena allocator exists but isn't used in the DOM

`arena.rs` defines a `FrameArena` bump allocator, but the DOM (`dom.rs`) uses `Vec<Node>` with index-based `NodeId`. The arena is defined but not integrated into the actual allocation strategy of the hot path.

---

### 4.3 `StyledNode` tree is a deep recursive owned structure

```rust
pub struct StyledNode {
    pub children: Vec<StyledNode>,
}
```

The styled tree is a recursively-owned `Vec<StyledNode>`. For large DOMs this means:
- Deep recursive `build_styled_node` calls that can stack overflow
- No shared references — every styled node owns its children
- Cannot share style data between siblings with identical styles (no style sharing cache)

---

### 4.4 `LayoutBox` lifetime ties it to `StyledNode`

```rust
pub struct LayoutBox<'a> {
    pub styled_node: Option<&'a StyledNode>,
}
```

The layout tree borrows the styled tree. This prevents any mutation of styles after layout starts and couples the two lifetimes, making incremental relayout difficult.

---

### 4.5 `HttpClient` code duplication between `send_request` and `stream` (`http.rs:292–449`)

The `send_request()` and `stream()` methods contain ~80 lines of duplicated DNS resolution, connection establishment, and TLS negotiation code. Any bug fix or feature change must be applied twice.

---

### 4.6 `ConnectionPool` doesn't expire idle connections

Connections are kept indefinitely until they fail a `peek()` liveness check. There's no idle timeout, max-connections-per-host limit, or max-total-connections limit. This can lead to resource exhaustion with many different hosts.

---

### 4.7 `ThreadedScheduler` has no graceful shutdown

```rust
pub fn shutdown(mut self) {
    drop(self.job_sender.take());
    for handle in self.workers.drain(..) {
        let _ = handle.join();
    }
}
```

Workers are joined but there's no timeout. If a worker is blocked on a network I/O operation, `shutdown()` blocks forever.

---

### 4.8 `AnimationManager` is an empty stub

```rust
pub struct AnimationManager {}
impl AnimationManager {
    pub fn tick(&mut self, _dt: f32) {
        // Handle animation progression
    }
}
```

The animation system defines `AnimationSpec`, keyframe types, timing functions, and easing curves, but the actual `AnimationManager` does nothing. No animations are ever executed. The `ComputedStyle` carries animation properties that are never consumed.

---

### 4.9 The display list flattens the tree but loses paint order for overlapping elements

The paint engine (`paint.rs`) generates a flat `DisplayList` with a fixed iteration order: background → borders → text → children. But for `position: absolute` or `position: fixed` elements, the CSS stacking context and `z-index` ordering should apply. Currently there's no stacking context handling at all.

---

## 5. Code Quality & Maintainability

### 5.1 Massive raw source indexing pattern repeated everywhere

The pattern `source[tag_start as usize..tag_end as usize]` with `std::str::from_utf8().unwrap_or("")` appears **dozens** of times across `style.rs`, `paint.rs`, `layout.rs`, and `parser.rs`. This is error-prone — any off-by-one in byte offsets causes panics or garbled text. This should be a single helper method on `Dom` or `Node`.

---

### 5.2 `property_id_to_name()` duplicates `property_from_name()` in reverse

Both functions manually list all 33 properties. If a new property is added, it must be updated in: `PropertyId` enum, `ALL_PROPERTIES` array, `is_inherited()`, `property_from_name()`, `property_id_to_name()`, `copy_property()`, `set_property()`, and `get_property_display()`. That's **8 places** that must stay in sync — a maintenance nightmare.

---

### 5.3 UA stylesheet defaults are scattered across `style.rs:401–496` as hardcoded conditions

The User-Agent stylesheet is implemented as a series of `if` statements checking tag names with hardcoded color values:

```rust
"body" => { computed.background_color = values::Color::rgb(248, 250, 252); }
"h1" => { computed.background_color = values::Color::rgb(240, 249, 255); }
```

This mixes rendering defaults into the cascade logic. These should be a proper UA stylesheet parsed through the same CSS engine.

---

### 5.4 Typos in scene.rs comments

```rust
//this state is from the NOdestate
//interactive visuall state of a oarticular node on ehihc the mous e is hovering
```

---

### 5.5 `#[allow(clippy::...)]` suppressions hiding issues

Several `#[allow(clippy::unnecessary_map_or)]` and `#[allow(clippy::collapsible_if)]` suppress lint warnings. These should be fixed, not suppressed — the lints usually point to genuinely simplifiable code.

---

### 5.6 Excessive `.clone()` in hot paths

`style.rs` clones `String` values liberally:
```rust
declarations.push(MatchedDeclaration {
    property: decl.property.clone(),
    value: decl.value.clone(),
    ...
});
```

For a page with 500 rules × 5 declarations × hundreds of elements, this creates millions of short-lived `String` clones during style resolution.

---

## 6. Performance Issues

### 6.1 O(elements × rules) selector matching with no indexing

Every element is matched against every CSS rule. There's no rule indexing by tag name, class, or ID. For a page with 1,000 elements and 500 rules, this is 500,000 selector match attempts. Production engines use hash-indexed rule maps to reduce this to ~O(elements × matching_rules).

---

### 6.2 `TaskScheduler::poll()` is O(n) linear scan for highest priority

```rust
pub fn poll(&mut self) -> Option<Task> {
    let mut best_idx = 0;
    for (i, task) in self.queue.iter().enumerate().skip(1) {
        if task.priority > best_priority { ... }
    }
    self.queue.remove(best_idx)
}
```

This linearly scans the entire queue and then calls `VecDeque::remove(best_idx)` which is O(n) for shifting elements. Should use a `BinaryHeap` for O(log n) priority queue operations.

---

### 6.3 `LruCache::insert()` is O(n) for eviction (`cache.rs:36–43`)

```rust
if let Some(lru_key) = self.map.iter()
    .min_by_key(|(_, (_, ts))| *ts)
    .map(|(k, _)| k.clone())
{
    self.map.remove(&lru_key);
}
```

Finding the LRU item requires a full scan of the HashMap. A proper LRU cache should use a doubly-linked list + HashMap for O(1) eviction.

---

### 6.4 `compute_intrinsic_inline_width` uses a rough `0.55 * font_size` character width estimate (`layout.rs:588`)

```rust
(trimmed_len * font_size * 0.55).max(0.0)
```

This produces wildly incorrect widths for non-Latin characters, proportional fonts, or even simple strings like "WWW" vs "iii". This heuristic makes inline layout inaccurate by default.

---

### 6.5 `read_response_headers` reads one byte at a time (`http.rs:468–476`)

```rust
loop {
    if stream.read_exact(&mut byte).is_err() { ... }
    header_buf.push(byte[0]);
    if header_buf.ends_with(b"\r\n\r\n") { break; }
}
```

Reading HTTP headers byte-by-byte is extremely slow — each `read_exact` is a syscall (or TLS record read). Should use a buffered reader with `BufRead::read_until` or at least read into a larger buffer.

---

### 6.6 Text node scene height hardcoded to `font_size * 1.2` (`scene.rs:376`)

```rust
height: *font_size * 1.2,
```

This doesn't account for multi-line text, line-height settings, or actual text measurement. Long text nodes get a single-line-height bounding box regardless of their actual rendered height.

---

## 7. Security Concerns

### 7.1 No URL sanitization on `<a href="">` link extraction (`paint.rs:127–131`)

```rust
if attr_name.eq_ignore_ascii_case("href") {
    return Some(
        std::str::from_utf8(&source[vs as usize..ve as usize])
            .unwrap_or("").to_string(),
    );
}
```

Link URLs are extracted and propagated without any sanitization. `javascript:`, `data:`, and `file:///` URLs are passed through unchanged. When clicked, these could execute arbitrary code or access local files.

---

### 7.2 No header injection protection in HTTP request serialization (`http.rs:142–186`)

User-controlled header values are written directly into the HTTP request without CRLF injection checking:
```rust
for (k, v) in &self.headers {
    req.push_str(&format!("{}: {}\r\n", k, v));
}
```

If an attacker controls header values (e.g., via `Referer` or custom headers), they could inject `\r\n` to add arbitrary headers or split the request.

---

### 7.3 DNS rebinding not mitigated

The DNS resolver caches results for 5 minutes, but doesn't pin resolved IPs for the duration of a page load. A malicious server could return different IPs on subsequent resolutions, enabling DNS rebinding attacks to access internal network resources.

---

### 7.4 No TLS certificate hostname verification audit

The `TlsConnector` uses `rustls` which should handle this, but the connection flow in `http.rs` doesn't validate that the TLS certificate matches the requested hostname at the application level — it relies entirely on `rustls`'s default configuration being correct.

---

### 7.5 `<img src>` URLs extracted without origin check (`paint.rs:227–231`)

Image `src` attributes are extracted and passed as `image_id` strings without URL validation. `file:///etc/passwd` or cross-origin URLs could be used to probe local files or track users.

---

## 8. Test Coverage Gaps

### 8.1 No integration tests for the full pipeline

The test suite tests individual modules (tokenizer, parser, CSS parser, style, layout, values) in isolation. There are no end-to-end tests that feed HTML+CSS through the entire `Parse → Style → Layout → Paint → Scene` pipeline and verify final pixel positions.

---

### 8.2 No tests for shorthand expansion correctness

There are no tests verifying that `margin: 10px 20px` correctly expands to the four longhand properties and that the cascade interaction between shorthands and longhands is correct.

---

### 8.3 No tests for `var()` substitution edge cases

CSS custom properties (`var(--x, fallback)`) are implemented but not tested for: nested `var()`, circular references, missing variables with fallbacks, or variables containing `var()` references.

---

### 8.4 No tests for anonymous block creation

The `build_layout_tree` function creates anonymous blocks for mixed inline/block content, but there are no tests verifying this behavior or that the anonymous blocks are correctly laid out.

---

### 8.5 No layout correctness tests against known-good reference values

Layout tests should verify that given a specific HTML+CSS input, the computed `x, y, width, height` of each box matches expected values. No such tests exist.

---

### 8.6 No tests for media query viewport matching

The `@media` rule parsing and viewport-width-based rule activation (`style.rs:189–195`) has no tests for boundary conditions (equal to min-width, equal to max-width, etc.).

---

### 8.7 No tests for HTTP chunked transfer encoding parsing

The `stream_chunked_body` function parses HTTP chunked encoding but has no unit tests, which is a complex protocol feature prone to off-by-one errors.

---

### 8.8 No tests for TLS connection handling

The TLS layer (`tls.rs`) has no tests. TLS errors, certificate failures, and protocol negotiation are untested.

---

### 8.9 No fuzz testing for the HTML tokenizer/parser

The HTML tokenizer is a complex state machine processing arbitrary byte input. There are no fuzz tests to catch panics, infinite loops, or out-of-bounds indexing on malformed HTML.

---

## Summary

| Category | Count | Severity |
|----------|-------|----------|
| Critical Bugs | 7 | 🔴 High |
| Logic Flaws | 12 | 🟠 Medium-High |
| Spec Non-Compliance | 9 | 🟡 Medium |
| Architecture Flaws | 9 | 🟠 Medium-High |
| Code Quality | 6 | 🟡 Medium |
| Performance Issues | 6 | 🟡 Medium |
| Security Concerns | 5 | 🔴 High |
| Test Coverage Gaps | 9 | 🟡 Medium |
| **Total** | **63** | |

The most impactful issues to fix first are:
1. **Percentage length resolution** (#1.1) — breaks all %-based layouts
2. **Anonymous block paint skipping** (#1.5) — makes text invisible
3. **Inline layout not computing own dimensions** (#1.6) — zero-size inline elements
4. **Scene graph parent always None** (#2.11) — breaks incremental rendering
5. **No margin collapsing** (#3.7) — doubled spacing everywhere
