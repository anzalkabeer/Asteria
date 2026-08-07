# CSS Engine

> **Purpose:** Explain how Asteria understands and applies CSS — from tokenization through cascade to computed styles.
>
> **Audience:** Contributors, CSS enthusiasts, and developers interested in style systems.
>
> **Estimated reading time:** 15 minutes
>
> **Prerequisites:** [How Asteria Works](02-how-asteria-works.md), [DOM](05-dom.md)

---

## Overview

The CSS engine is responsible for answering one question for every element on the page: **"What does this element look like?"**

It does this in three phases:

1. **Tokenization** — breaking CSS source text into tokens
2. **Parsing** — assembling tokens into a structured stylesheet (rules, selectors, declarations)
3. **Style resolution** — matching rules to DOM elements, resolving conflicts, and computing final property values

The output is a **styled tree** — a parallel structure to the DOM where every element carries a `ComputedStyle` with fully resolved CSS properties.

---

## Phase 1: CSS tokenization

**Source file:** `src/css_tokenizer.rs`

The CSS tokenizer reads CSS source text and produces a stream of tokens. CSS has its own syntax, distinct from HTML.

### Token types

| Token kind | Example | Description |
|---|---|---|
| `Ident` | `color`, `div`, `margin` | An identifier (property names, tag names) |
| `Hash` | `#header`, `#ff0000` | A hash token (IDs, hex colours) |
| `String` | `"Arial"`, `'sans-serif'` | A quoted string |
| `Number` | `16`, `1.5` | A numeric value |
| `Dimension` | `16px`, `2em` | A number with a unit identifier |
| `Percentage` | `50%`, `100%` | A percentage value |
| `Colon` | `:` | Property-value separator |

---

## Phase 2: CSS parsing

**Source file:** `src/css_parser.rs`

The parser reads the token stream and builds a `Stylesheet` — a structured representation of the CSS rules.

### Stylesheet structure

```
Stylesheet
  ├── Rule
  │     ├── Selectors: [h1, .title]
  │     └── Declarations:
  │           ├── color: blue
  │           └── font-size: 24px
  ├── Rule
  │     ├── Selectors: [.container > p]
  │     └── Declarations:
  │           └── margin: 16px
  └── MediaRule
        ├── Condition: (min-width: 768px)
        └── Rules: [...]
```

### Selectors

Asteria's selector model supports:

**Simple selectors** — the atomic building blocks:

| Type | Syntax | Example | Matches |
|---|---|---|---|
| Tag | `tagname` | `div` | All `<div>` elements |
| Class | `.classname` | `.main` | Elements with `class="main"` |
| ID | `#idname` | `#header` | The element with `id="header"` |
| Universal | `*` | `*` | Every element |
| Pseudo-class | `:name` | `:first-child` | Structural/state pseudo-classes |

**Compound selectors** — multiple simple selectors on the same element:

| Example | Meaning |
|---|---|
| `div.main` | A `<div>` with class `main` |
| `h1#title` | An `<h1>` with id `title` |
| `p.intro:first-child` | A `<p>` with class `intro` that is a first child |

**Complex selectors** — compound selectors connected by combinators:

| Combinator | Syntax | Example | Meaning |
|---|---|---|---|
| Descendant | ` ` (space) | `div p` | `<p>` anywhere inside a `<div>` |
| Child | `>` | `div > p` | `<p>` that is a direct child of `<div>` |
| Next sibling | `+` | `h1 + p` | `<p>` immediately after an `<h1>` |
| Subsequent sibling | `~` | `h1 ~ p` | Any `<p>` after an `<h1>` (same parent) |

**Grouped selectors** — multiple selectors sharing one rule:

```css
h1, h2, h3 {
    color: navy;
}
```

### Pseudo-classes

Currently supported pseudo-classes:

| Pseudo-class | Meaning |
|---|---|
| `:first-child` | Element is the first child of its parent |
| `:last-child` | Element is the last child of its parent |
| `:hover` | Element is under the mouse cursor |

### Media queries

Asteria supports `@media` rules with viewport-based conditions:

```css
@media (min-width: 768px) {
    .sidebar { display: flex; }
}

@media (max-width: 480px) {
    .nav { display: none; }
}
```

Media queries are evaluated against the current viewport dimensions. When the window is resized, applicable `@media` rules are re-evaluated.

### Declarations and properties

A declaration is a property-value pair:

```css
color: #38bdf8;
font-size: 24px;
margin: 16px 8px;
background-color: rgb(15, 23, 42);
```

The parser preserves property names and raw value strings. Value interpretation (parsing `#38bdf8` as a colour, `24px` as a length) happens later during style resolution.

---

## Phase 3: Style resolution

**Source file:** `src/style.rs`

Style resolution is the most complex part of the CSS engine. It bridges the DOM and the stylesheet, producing computed styles for every element.

### The algorithm

For each element in the DOM:

```
┌───────────────────────────────────────────────┐
│ 1. Collect all matching rules from stylesheet │
│    (selector matching)                        │
├───────────────────────────────────────────────┤
│ 2. Calculate specificity for each match       │
│    (ID count, class count, tag count)         │
├───────────────────────────────────────────────┤
│ 3. Add inline style="" declarations           │
│    (highest specificity)                      │
├───────────────────────────────────────────────┤
│ 4. Sort all declarations by cascade priority  │
│    (origin → specificity → source order)      │
├───────────────────────────────────────────────┤
│ 5. For each property, pick the winning value  │
├───────────────────────────────────────────────┤
│ 6. Expand shorthand properties               │
│    (margin → margin-top/right/bottom/left)    │
├───────────────────────────────────────────────┤
│ 7. Inherit from parent where applicable       │
│    (color, font-size inherit; margin doesn't) │
├───────────────────────────────────────────────┤
│ 8. Compute absolute values                    │
│    (em → px, percentages → pixels)            │
└───────────────────────────────────────────────┘
```

### Selector matching

To check if a selector matches an element, Asteria walks the selector from right to left (most selectors are checked this way in production engines because the rightmost part is the most specific):

For `div.container > p.intro`:

1. Is this element a `<p>` with class `intro`? If not, no match.
2. Is its direct parent a `<div>` with class `container`? If not, no match.
3. Both match → the rule applies.

Combinator matching works by traversing the DOM:

| Combinator | Traversal |
|---|---|
| Descendant (space) | Walk up through all ancestors |
| Child (`>`) | Check only the direct parent |
| Next sibling (`+`) | Check the immediately preceding sibling |
| Subsequent sibling (`~`) | Check all preceding siblings |

### Specificity

Specificity is a three-component score: `(ID count, class count, tag count)`.

| Selector | IDs | Classes | Tags | Score |
|---|---|---|---|---|
| `p` | 0 | 0 | 1 | (0, 0, 1) |
| `.main` | 0 | 1 | 0 | (0, 1, 0) |
| `#header` | 1 | 0 | 0 | (1, 0, 0) |
| `div.main p` | 0 | 1 | 2 | (0, 1, 2) |
| `#nav .link:hover` | 1 | 2 | 0 | (1, 2, 0) |

Scores are compared lexicographically: IDs beat classes, classes beat tags. Inline `style=""` attributes always beat stylesheet rules.

### The cascade

When multiple declarations compete for the same property on the same element, the cascade resolves the conflict:

```
Priority (highest → lowest):
  1. Inline style=""  (origin = Inline)
  2. Higher specificity (IDs > classes > tags)
  3. Later source order (last rule wins)
```

### Inheritance

**Source file:** `src/properties.rs`

Some CSS properties are inherited — they flow down from parent to child unless explicitly overridden:

| Inherited ✅ | Not inherited ❌ |
|---|---|
| `color` | `margin` |
| `font-size` | `padding` |
| `font-family` | `border` |
| `line-height` | `background-color` |
| `text-align` | `width` |
| `visibility` | `height` |
| `cursor` | `display` |

If an element doesn't have a rule setting `color`, it inherits its parent's colour. If it doesn't have a rule setting `margin`, it gets the initial value (typically `0`).

### Shorthand expansion

CSS shorthands are expanded into their individual longhand properties:

| Shorthand | Expands to |
|---|---|
| `margin: 16px` | `margin-top: 16px`, `margin-right: 16px`, `margin-bottom: 16px`, `margin-left: 16px` |
| `margin: 8px 16px` | `margin-top: 8px`, `margin-right: 16px`, `margin-bottom: 8px`, `margin-left: 16px` |
| `padding: 10px 20px 30px` | `padding-top: 10px`, `padding-right: 20px`, `padding-bottom: 30px`, `padding-left: 20px` |
| `border: 1px solid black` | `border-width`, `border-style`, `border-color` per edge |

### Computed styles

**Source file:** `src/values.rs`

After resolution, every element gets a `ComputedStyle` — a struct of fully resolved, absolute property values:

```rust
pub struct ComputedStyle {
    pub display: Display,          // Block, Inline, Flex, None
    pub color: Color,              // RGBA
    pub background_color: Color,   // RGBA
    pub font_size: f32,            // Absolute pixels
    pub margin: Edges,             // Top, right, bottom, left in pixels
    pub padding: Edges,            // Top, right, bottom, left in pixels
    pub border_width: Edges,       // Border thickness per edge
    pub border_color: Color,       // Border colour
    pub width: Option<f32>,        // Explicit width or None (auto)
    pub height: Option<f32>,       // Explicit height or None (auto)
    // ... more properties
}
```

Relative values are resolved during style resolution and layout:
- `2em` → multiplied by parent's font-size → absolute pixels
- `50%` → resolved against property-specific reference bases (e.g. width/margins resolve against containing block content width; font-size percentage resolves against parent font-size). In Asteria's current layout solver, percentages on width and horizontal margins compute against the containing block's available width as a baseline simplification.
- `inherit` → copied from parent's computed value
- Named colours (`red`, `blue`) → RGBA values

---

## The styled tree

The output of style resolution is a `StyledTree` — a tree that mirrors the DOM structure but carries `ComputedStyle` on each node:

```
StyledNode(0) [Document]
  └── StyledNode(1) [html] { display: block, ... }
        ├── StyledNode(2) [head] { display: none, ... }
        └── StyledNode(3) [body] { background: #1e1e2e, color: #cdd6f4, ... }
              ├── StyledNode(4) [h1] { color: #89b4fa, font-size: 24px, ... }
              │     └── StyledNode(5) [text] { inherited color: #89b4fa, ... }
              └── StyledNode(6) [p] { color: #a6adc8, font-size: 16px, ... }
                    └── StyledNode(7) [text] { inherited color: #a6adc8, ... }
```

This tree is what the layout engine consumes to compute geometry.

---

## Current limitations

| Feature | Status |
|---|---|
| `!important` declarations | 🔜 |
| `@import` external stylesheets | 🔜 |
| `@keyframes` animations | 🔜 |
| CSS custom properties (`var()`) | 🔜 |
| `::before` / `::after` pseudo-elements | 🔜 |
| Attribute selectors (`[type="text"]`) | 🔜 |
| `:nth-child()` pseudo-class | 🔜 |

---

## Related documents

- [HTML Engine](04-html-engine.md) — produces the DOM that style resolution works on
- [DOM](05-dom.md) — the tree structure CSS selectors match against
- [Layout Engine](07-layout-engine.md) — consumes the styled tree
- [Design Decisions](18-design-decisions.md) — cascade architecture choices
- [Glossary](17-glossary.md) — CSS terminology
