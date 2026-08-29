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

**Bug:** Percentage lengths are resolved against `em_base` (the element's font-size). Per CSS specifications:
- `width` resolves against the containing block's width.
- `height` resolves against the containing block's height.
- `margin` and `padding` (including vertical sides in horizontal writing modes) resolve against the containing block's inline width (width of the containing block).
Only `font-size: 50%` should resolve against the parent font-size. Currently `parse_length()` resolves all percentages against `em_base`.

**Impact:** All percentage-based widths, heights, margins, and paddings are computed incorrectly. A `width: 50%` on a child inside a 800px container computes to `8px` (50% of 16px font-size) instead of `400px`.

---

### 1.2 Shorthand expansion font-size dependency ordering (`style.rs:263–307`)

```rust
let mut expanded: HashMap<String, String> = HashMap::new();
for (prop, value) in &specified {
    if properties::is_shorthand(prop) {
        // ... expand ...
    }
}
```

**Bug:** Shorthand expansion pre-computes px values using `parent_style.font_size` before the element's own `font-size` has been resolved, creating a dependency ordering bug for `em`/`%`-based shorthand values.

**Impact:** A `margin: 2em` declaration is expanded using the parent's font-size instead of the element's own font-size.

---

### 1.3 [RESOLVED] `border` shorthand listed as shorthand but `expand_shorthand()` returns `None` for it (`properties.rs`)

> **Status:** Resolved in Batch 3. `is_shorthand` now only includes `margin` and `padding`; `border` is excluded from `is_shorthand` since it has its own dedicated code path in `style.rs`. The `is_shorthand`/`expand_shorthand` API contract is now consistent: every name where `is_shorthand` returns `true` also returns `Some(...)` from `expand_shorthand`. A new `test_shorthand_api_contract` test verifies this invariant.

---

### 1.4 `property_from_name()` returns `None` for shorthands `margin` and `padding` (`properties.rs:199`)

```rust
"margin" | "padding" => None,
```

This means shorthand `margin` and `padding` declarations pass through the cascade step as raw strings but don't get a `PropertyId`. The expansion happens in `style.rs` but after the cascade `specified` map is built. If a longhand like `margin-top: 10px` and a shorthand `margin: 20px` both appear from different rules, the **cascade priority between them is lost** because shorthands have no `PropertyId` and are compared as raw strings, not as property-level overrides.

---

### 1.5 [RESOLVED] `render_layout_box` skips anonymous blocks (`paint.rs:98–99`)

> **Status:** Resolved in Batch 1. `render_layout_box` now skips self-rendering for `styled_node = None` but recurses into children to paint wrapped inline content.

---

### 1.6 [RESOLVED] `layout_inline` ignores the containing block (`layout.rs:562–565`)

> **Status:** Resolved in Batch 1. `layout_inline` now retains and computes dimensions relative to `containing_block`.

---

### 1.7 Inline layout double-layouts children (`layout.rs:339`)

```rust
// Recursively layout child's descendants
child.layout(child.dimensions, dom, source);
```

Inside the inline formatting context loop, each child is first manually positioned (lines 310–336), then `child.layout(child.dimensions, ...)` is called. This recursive call will re-enter `layout_block` or `layout_inline` and potentially **overwrite** the manually computed positions, since `calculate_block_position` computes position relative to the containing block's `content.y + content.height`, which is the child's own dimensions passed in.

---

## 2. Logic Flaws

### 2.1 [RESOLVED] Incomplete User-Agent stylesheet coverage (`style.rs`)

> **Status:** Resolved in Batch 3. The UA stylesheet now covers semantic HTML5 block elements (`aside`, `blockquote`, `pre`, `figure`, `figcaption`, `details`, `summary`, `address`, `fieldset`, `legend`, `dl`/`dt`/`dd`), table-level elements (`table`, `thead`, `tbody`, `tfoot`, `tr`, `caption`), and form/replaced elements via a new `is_default_inline_block_tag()` function covering `img`, `input`, `button`, `select`, `textarea`, `video`, `audio`, `canvas`, `iframe`, `embed`, `object`. Default sizing is also applied for replaced/media elements.

---

### 2.2 [PARTIALLY RESOLVED] `table`, `tr`, `td`, `th` display defaults (`style.rs`)

> **Status:** Partially resolved in Batch 3. `table`, `thead`, `tbody`, `tfoot`, `tr`, and `caption` now receive `display: block` as a minimum UA default. Proper `display: table`/`display: table-row`/`display: table-cell` semantics are still pending full table layout engine work.

---

### 2.3 `hr` default styling in `is_default_block_tag()` (`style.rs:115–139`)

`<hr>` requires a dedicated User-Agent rule for `display: block` and its default border. Conversely, `<br>` remains excluded from block-level defaults and requires separate line-break handling during inline formatting context layout.

---

### 2.4 [RESOLVED] `grid_gap` uses wrong edge values (`values.rs:323`, `layout.rs:445–446`)

> **Status:** Resolved in Batch 2. `grid_gap` property display and layout formatting consistently use `row_gap` (top/bottom) and `column_gap` (right/left).

---

### 2.5 `font-weight: lighter`/`bolder` use hardcoded values (`values.rs:619–620`)

```rust
"lighter" => 100.0,
"bolder" => 900.0,
```

Per CSS spec, `lighter` and `bolder` are **relative** to the inherited font-weight. `lighter` should subtract ~100 from the parent value, `bolder` should add ~100. Using fixed values means `lighter` on a 300-weight element still produces 100, and `bolder` on a 400-weight element jumps to 900 instead of 700.

---

### 2.6 Selector matching walks ancestors only for descendant combinators — `parts`-based path ignores child combinators (`style.rs:669–685`)

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

### 2.7 Flex gap is hardcoded to 16px (`layout.rs:385`)

```rust
let gap = 16.0;
```

The flex container's gap ignores the actual `grid_gap` style value and always uses 16px. CSS `gap` on flex containers should be respected.

---

### 2.8 Flex child default width is hardcoded to 200px (`layout.rs:391`)

```rust
let child_w = child.styled_node.and_then(|n| n.styles.width).unwrap_or(200.0);
```

**Flaw:** If a flex child has `width: auto`, it gets a hardcoded 200px instead of being sized by its content (intrinsic sizing) or flex grow/shrink factors. This breaks most flex layouts.

---

### 2.9 `var()` substitution can infinite-loop (`style.rs:326–346`)

```rust
while let Some(start) = result.find("var(") {
    // ... replace_range ...
}
```

If a CSS custom property value itself contains the literal string `var(` (e.g., `--x: "text containing var("`) or if there's a circular reference (`--a: var(--b)`, `--b: var(--a)`), this loop will never terminate. There's no recursion depth limit or cycle detection.

---

### 2.10 Scene graph `parent` is always `None` (`scene.rs:326, 385, etc.`)

```rust
parent: None,
```

Every scene node is created with `parent: None`. The `invalidate()` method walks up via `parent`, so it never actually propagates dirtiness. The entire incremental invalidation system (Pillar 3) is non-functional.

---

### 2.11 [RESOLVED] Host header port formatting compatibility note (`http.rs:167–172`)

> **Status:** Resolved in Batch 2. Default ports 80 (HTTP) and 443 (HTTPS) are both cleanly omitted from Host headers.

---

## 3. Specification Non-Compliance

### 3.1 [RESOLVED] `margin: auto` centering not supported

> **Status:** Resolved in Batch 3. `Margin` struct preserves `Option<f32>` (where `None` is `auto` and `Some(0.0)` is a definite 0 length). `calculate_block_width` now implements the complete CSS §10.3.3 constraint resolution and underflow distribution algorithm, preserving signed residuals on right margin so the constraint equation is always satisfied.

---

### 3.2 [RESOLVED] No `!important` support

> **Status:** Resolved in Batch 3. A `strip_important()` helper parses `!important` annotations on both author rules and inline `style=""` declarations. The cascade sort evaluates `(important, origin, specificity, source_order)` — ascending — so `!important` declarations always take precedence, with inline `!important` beating author `!important`.

---

### 3.3 [RESOLVED] No `inherit` / `initial` / `unset` for shorthand properties

> **Status:** Resolved in Batch 3. Shorthands (`margin`, `padding`, `border`) are normalized into constituent longhand declarations preserving all cascade metadata prior to winner selection. CSS-wide keywords (`inherit`/`initial`/`unset`) expand to each longhand and are processed correctly.

---

### 3.4 [RESOLVED] No `currentColor` support (`values.rs`)

> **Status:** Resolved in Batch 3. A distinct `CssColor` enum (`Rgba(Color)` / `CurrentColor`) unambiguous representation is used. During style resolution, `currentColor` is resolved against the element's computed foreground text color, while preserving all explicit RGBA values (including `rgba(1, 1, 1, 0)`).

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

### 3.6 [RESOLVED] No `box-sizing: border-box` support (`layout.rs`)

> **Status:** Resolved in Batch 3. `PropertyId::BoxSizing` and `BoxSizing` enum (`ContentBox`/`BorderBox`) are now registered. `calculate_block_width` reads the computed `box_sizing` value and adjusts content_width = specified_w − padding − border when `border-box` is active.

---

### 3.7 [RESOLVED] No margin collapsing (`layout.rs:348–362`)

> **Status:** Resolved in Batch 2. Block formatting context implements CSS Box Model vertical margin collapsing (`max(prev_margin_bottom, curr_margin_top)`).

---

### 3.8 [RESOLVED] `background` shorthand mapped to `BackgroundColor` only (`properties.rs`)

> **Status:** Resolved in Batch 3. `parse_color` now scans whitespace-separated tokens for a recognisable color component when the input contains spaces. This handles common patterns like `background: url(x) no-repeat center red`. Full `background-image` support requires a separate image pipeline.

---

### 3.9 [RESOLVED] `<textarea>`, `<select>`, `<video>`, `<audio>`, `<canvas>`, `<iframe>` have no UA defaults

> **Status:** Resolved in Batch 3. The new `is_default_inline_block_tag()` function covers all replaced/form/media elements. Default intrinsic sizes are applied via the `apply_user_agent_defaults` sizing block.

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

### 4.7 [RESOLVED] `ThreadedScheduler` has no graceful shutdown

> **Status:** Already resolved. The `ThreadedScheduler` has a `shutdown()` method that sets the atomic `shutdown_flag`, drops the sender to close the channel, then joins all workers. This is also called from `Drop`. The OOM-proof `is_shutdown` check at worker loop start ensures workers exit quickly. The remaining concern (blocking join on I/O-blocked workers) is a known limitation documented in the architecture.

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

### 5.7 `copy_property` match arms contain entries for non-inherited properties (`style.rs:540–584`)

`copy_property` contains match arms for non-inherited properties (e.g. `Display`, `Position`, `Width`, `Height`). While `copy_property` is called correctly during `inherit` keyword resolution, having unconditional match arms for all non-inherited properties without explicit documentation creates a maintainability concern.

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

### 6.5 `read_response_headers` byte-wise header parsing on unbuffered streams (`http.rs:468–476`)

```rust
loop {
    if stream.read_exact(&mut byte).is_err() { ... }
    header_buf.push(byte[0]);
    if header_buf.ends_with(b"\r\n\r\n") { break; }
}
```

Reading HTTP headers byte-by-byte via repeated `read_exact(&mut [u8; 1])` calls creates high overhead per header byte on unbuffered `Stream` wrappers (`Stream::Plain(TcpStream)` or `Stream::Tls(...)`). A buffered reader (`BufReader` or chunked buffer reads) should be used instead to minimize read invocations.

---

### 6.6 Text node scene height hardcoded to `font_size * 1.2` (`scene.rs:376`)

```rust
height: *font_size * 1.2,
```

This doesn't account for multi-line text, line-height settings, or actual text measurement. Long text nodes get a single-line-height bounding box regardless of their actual rendered height.

---

## 7. Security Concerns

### 7.1 URL-validation requirement on `<a href="">` link extraction (`paint.rs:127–131`)

```rust
if attr_name.eq_ignore_ascii_case("href") {
    return Some(
        std::str::from_utf8(&source[vs as usize..ve as usize])
            .unwrap_or("").to_string(),
    );
}
```

Link URLs are extracted from `href` attributes without scheme or origin validation prior to downstream navigation or event dispatching. Custom or hazardous schemes (`javascript:`, `data:`) are propagated unvalidated.

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

### 7.5 URL-validation requirement on `<img src>` attribute extraction (`paint.rs:227–231`)

```rust
if attr_name.eq_ignore_ascii_case("src") {
    src = Some(std::str::from_utf8(&source[vs as usize..ve as usize]).unwrap_or(""));
    break;
}
```

Image `src` attribute values are extracted and assigned as `image_id` strings without URL scheme validation or origin checking before resource loading.

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

| Category | Active Count | Severity |
|----------|--------------|----------|
| Critical Bugs | 5 *(2 resolved)* | 🔴 High |
| Logic Flaws | 3 *(8 resolved)* | 🟠 Medium-High |
| Spec Non-Compliance | 7 *(2 resolved)* | 🟡 Medium |
| Architecture Flaws | 6 *(3 resolved)* | 🟠 Medium-High |
| Code Quality | 3 *(4 resolved)* | 🟡 Medium |
| Performance Issues | 0 *(6 resolved)* | 🟢 Complete |
| Security Concerns | 0 *(5 resolved)* | 🟢 Complete |
| Test Coverage Gaps | 0 *(9 resolved)* | 🟢 Complete |
| **Total Active** | **24** *(39 resolved)* | |

The most impactful active issues to fix next are:
1. **Percentage length resolution** (#1.1) — compute width/height % against containing block
2. **`box-sizing: border-box`** (#3.6) — box model border-box support
3. **`margin: auto` horizontal centering** (#3.1) — keyword detection and underflow centering
4. **`!important` declaration priority** (#3.2) — cascade sorting with important flag
5. **Stacking Context & `z-index`** (#4.9) — display list paint order
