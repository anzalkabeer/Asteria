# COMPARATIVE ANALYSIS: SERVO LAYOUT 2020 VS. ASTERIA LAYOUT

## 1. Architectural Overview

| Dimension | Servo (Layout 2020) | Asteria (Current Layout Engine) |
| :--- | :--- | :--- |
| **Concurrency Model** | Multi-threaded work-stealing parallel layout (Rayon) | Single-threaded sequential pass |
| **Formatting Contexts** | Block, Inline, Flexbox, Grid, Table, Multicol | Partial Block, Naive Single-Line Flexbox |
| **Text Line Breaking** | Unified with shaped glyph metrics from HarfBuzz | `estimate_char_width_ratio` hardcoded heuristics |
| **Incremental Layout** | Fine-grained dirty bit propagation | Total document relayout from root |
| **Code Size** | ~80,000 LOC across 5 dedicated crates | ~3,240 LOC in a single file (`layout.rs`) |

---

## 2. In-Depth Comparative Assessment

### What Servo Does Well
Servo’s Layout 2020 is designed to scale across all available CPU cores. Its parallel inline and block formatting passes achieve remarkable benchmark numbers on large, complex HTML documents. Furthermore, it accurately models CSS Grid and multi-line Flexbox.

### Why Layout 2020 Cannot Be Cloned Directly
1. **Coupling with Stylo:** Layout 2020 is intimately coupled with Firefox’s C++ Stylo (Servo's CSS engine) and Servo's script runtime DOM pointers.
2. **Extreme Complexity:** The codebase is over 80,000 lines of complex unsafe Rust, ARC pointers, and Rayon parallel scopes.
3. **Overkill for Asteria:** Asteria's current bottlenecks are GPU buffer thrashing and immediate-mode scrolling lag, not multi-core layout parallelization.

### The Winning Strategy for Asteria
1. **Delegate Flexbox & Grid to Taffy (`SPEC-10`):** Taffy is a lightweight, battle-tested Rust implementation of CSS Flexbox and Grid. It gives Asteria full modern CSS layout compliance without writing tens of thousands of lines of custom layout code.
2. **Integrate True Font Metrics (`SPEC-06`):** Query font ascent, descent, and glyph advances directly from `FontContext` during inline text layout to ensure layout line breaks match rendered GPU pixels.
