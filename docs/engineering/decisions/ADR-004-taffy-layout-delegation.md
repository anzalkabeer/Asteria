# ADR-004: Delegation of Flexbox & Grid Layout to Taffy

- **Status:** **PROPOSED**
- **Date:** October 2026
- **Proposers:** Principal Architect & Antigravity AI Pair
- **Related Issues:** [`AST-ARC-003`](../issues/AST-ARC-003.md)
- **Specification:** [`SPEC-10`](../specifications/SPEC-10.md)

---

## 1. Context & Problem Statement
Asteria's layout engine (`src/layout.rs`) contains a ~400-line handwritten flexbox solver that only supports single-line row flow. It lacks multi-line flex wrapping, flex-grow/shrink factors, baseline alignment, and CSS Grid layout is completely missing. Writing a full W3C-compliant Flexbox and Grid solver from scratch requires thousands of lines of complex geometric code.

---

## 2. Decision Drivers
1. **Standards Compliance:** Support modern web layouts (flex-wrap, flex-grow, flex-shrink, CSS Grid template areas and fractions).
2. **Maintenance Simplicity:** Avoid maintaining thousands of lines of fragile layout algorithm code in-tree.
3. **Pure Rust Ecosystem:** Rely on well-tested pure Rust crates without C++ dependencies.

---

## 3. Considered Options
- **Option A (Status Quo):** Continue handcrafting flexbox and grid solvers in `src/layout.rs`.
- **Option B (Servo Layout 2020):** Port Servo's parallel Layout 2020 engine (80,000 LOC, coupled to Stylo and Rayon).
- **Option C (Selected Proposal):** Delegate Flexbox and Grid computation to **Taffy 0.14** via a lightweight adapter module.

---

## 4. Decision & Rationale
Adopt Option C. Taffy is the industry-standard pure-Rust implementation of CSS Flexbox and CSS Grid (used by Servo, Bevy, and Dioxus). Asteria retains its custom Block (BFC) and Inline (IFC) formatting engines while delegating flex and grid containers to Taffy.

---

## 5. Consequences & Trade-Offs
- **Positive:** Immediately gains full CSS Flexbox and Grid support with zero custom algorithm maintenance; battle-tested WPT compliance.
- **Negative:** Adds `taffy = "0.14"` dependency (~200 KB binary size impact); requires translating Asteria `ComputedStyle` into Taffy style structs.
