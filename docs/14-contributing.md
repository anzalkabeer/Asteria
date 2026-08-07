# Contributing

> **Purpose:** Guide new contributors through the process of working on Asteria.
>
> **Audience:** Anyone who wants to contribute code, docs, tests, or ideas.
>
> **Estimated reading time:** 7 minutes
>
> **Prerequisites:** Familiarity with Rust and Git

---

## Getting started

### Prerequisites

- [Rust](https://www.rust-lang.org/) (2024 edition)
- A GPU-capable system with Vulkan, Metal, or DX12 support (for rendering)
- Git

### Setup

```bash
# Clone the repository
git clone https://github.com/anzalkabeer/Asteria.git
cd Asteria/Asteria

# Verify everything compiles
cargo check

# Run the full test suite
cargo test

# Run the engine with a test fixture
cargo run -- tests/fixtures/blog.html
```

If compilation succeeds and tests pass, you're ready to go.

---

## Code style

### Formatting

All code must pass `cargo fmt`. This is enforced by CI. Run it before committing:

```bash
cargo fmt --all
```

### Linting

All code must pass `cargo clippy` with no warnings:

```bash
cargo clippy --all-targets -- -D warnings
```

### Comments

Asteria uses a specific comment style:

- **Section headers** use the format `// ─── Section Title ───────────────`
- **Module-level documentation** at the top of each file explains what the module does and why
- **Public API items** have `///` doc comments
- **Internal logic** has `//` comments explaining *why*, not just *what*

### Naming

- Types: `PascalCase` (e.g., `LayoutBox`, `SceneNode`, `ComputedStyle`)
- Functions: `snake_case` (e.g., `resolve_styles`, `build_display_list`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `SYM_DIV`, `RECT_PASS_INDEX`)
- Private fields: no prefix (Rust convention)

---

## Architecture expectations

Before making changes, understand these architectural rules:

### The DOM is immutable after construction

The parser builds the DOM, and then it's done. No later stage should mutate DOM nodes. If you need to attach data to DOM nodes, create a parallel data structure indexed by `NodeId`.

### Each pipeline stage has a distinct output

| Stage | Input | Output |
|---|---|---|
| Tokenizer | `&[u8]` | `Vec<Token>` |
| Parser | `Vec<Token>` | `Dom` |
| CSS Parser | `&[u8]` | `Stylesheet` |
| Style Resolver | `Dom` + `Stylesheet` | `StyledTree` |
| Layout | `StyledTree` | `LayoutTree` |
| Paint | `LayoutTree` | `DisplayList` |
| Scene | `DisplayList` | `SceneGraph` |
| Renderer | `SceneGraph` | Pixels on screen |

Don't mix stages. Don't have the layout engine modify the DOM. Don't have the paint engine query the stylesheet directly.

### No new dependencies without approval

Asteria's core engine has no third-party browser crates. If you need to add a dependency, discuss it first. External crates are used only for concerns outside the web engine: GPU (wgpu), windowing (winit), TLS (rustls), fonts (glyphon).

### Tag names are case-insensitive

Always compare HTML tag names using `.eq_ignore_ascii_case()` or `.to_ascii_lowercase()`. Never compare with plain `==` on raw strings.

---

## Commit style

Write clear, descriptive commit messages:

```
feat(layout): implement CSS flex-direction column

Add support for vertical flex layouts. When a flex container has
flex-direction: column, children are stacked vertically instead
of horizontally.

Extends the flex layout algorithm in layout.rs to check the
computed flex-direction property and switch between row and
column positioning logic.

Closes #42
```

**Prefixes:**

| Prefix | Usage |
|---|---|
| `feat` | New feature |
| `fix` | Bug fix |
| `refactor` | Code restructuring without behaviour change |
| `docs` | Documentation changes |
| `test` | Adding or modifying tests |
| `perf` | Performance improvement |
| `chore` | Build, CI, tooling changes |

---

## Pull requests

1. **Create a branch** from `main` with a descriptive name:
   ```bash
   git checkout -b feat/flex-column-layout
   ```

2. **Make your changes** following the code style and architecture rules above

3. **Add tests** — every new feature should have unit tests or integration tests

4. **Run the full check** before opening a PR:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```

5. **Open a PR** with:
   - A clear description of what changed and why
   - Before/after comparisons for visual changes
   - References to any related issues

6. **Respond to review feedback** — all PRs require review before merging

---

## Testing

### Unit tests

Add unit tests in the same file as the code they test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specificity_id_beats_class() {
        // ...
    }
}
```

### Integration tests

Integration tests live in `tests/`:

```
tests/
├── fixtures/              ← HTML/CSS test pages
│   ├── blog.html          ← Article layout test
│   ├── gallery.html       ← Flex + image card test
│   └── hello.html         ← Minimal test
├── style_integration.rs   ← CSS cascade and selector tests
├── layout_integration.rs  ← Box model and positioning tests
├── paint_integration.rs   ← Display list generation tests
├── image_integration.rs   ← Image format detection tests
├── network_integration.rs ← HTTP/DNS tests
├── renderer_integration.rs ← GPU pipeline tests
└── observability_trace.rs ← Devtools trace tests
```

### Visual testing

For rendering changes, test with the HTML fixtures:

```bash
# Test flex layouts and images
cargo run -- tests/fixtures/gallery.html

# Test article layout and styling
cargo run -- tests/fixtures/blog.html
```

Compare the visual output to expected rendering. Take screenshots for PR descriptions when relevant.

---

## Areas where help is most needed

| Area | Difficulty | Impact |
|---|---|---|
| Implement additional CSS properties | Medium | High |
| CSS Grid layout | Hard | High |
| Rounded corners (`border-radius`) | Medium | High |
| Improve test coverage | Easy-Medium | High |
| Documentation improvements | Easy | Medium |
| Visual test fixtures | Easy | Medium |
| Performance profiling on real pages | Medium | High |
| Accessibility infrastructure | Hard | High |

---

## Related documents

- [Project Architecture](03-project-architecture.md) — codebase map
- [Testing](15-testing.md) — testing strategy in detail
- [Roadmap](13-roadmap.md) — what's being worked on
- [Design Decisions](18-design-decisions.md) — understand why the code is the way it is
