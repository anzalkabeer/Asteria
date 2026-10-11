# ADR-005: 7-Crate Workspace Modularization Architecture

- **Status:** **PROPOSED**
- **Date:** October 2026
- **Proposers:** Principal Architect & Antigravity AI Pair
- **Related Issues:** [`AST-ARC-001`](../issues/AST-ARC-001.md)
- **Specification:** [`SPEC-05`](../specifications/SPEC-05.md)

---

## 1. Context & Problem Statement
Asteria currently exists as a single monolithic Cargo package with 33 submodules in `src/`. Clean builds take over 5 minutes (`302s` baseline). Internal modules can access arbitrary internal structures without compiler enforcement, and testing non-graphical modules (e.g. `dom.rs` or `css.rs`) requires linking the entire WGPU graphics and windowing pipeline.

---

## 2. Decision Drivers
1. **Compilation Speed:** Leverage Cargo's parallel crate compilation to reduce clean build times.
2. **Strict Architectural Boundaries:** Enforce acyclic dependency flow at compile time.
3. **Headless Unit Testing:** Enable non-graphical crates to be tested independently in headless CI environments.

---

## 3. Considered Options
- **Option A (Status Quo):** Maintain the single monolithic package.
- **Option B (Extreme Decomposition - Servo Style):** Decompose into 77 micro-crates.
- **Option C (Selected Proposal):** Partition into a balanced 7-crate workspace:
  - `crates/asteria_dom`
  - `crates/asteria_style`
  - `crates/asteria_layout`
  - `crates/asteria_paint`
  - `crates/asteria_render`
  - `crates/asteria_net`
  - `crates/asteria_shell`

---

## 4. Decision & Rationale
Adopt Option C. A 7-crate workspace cleanly reflects Asteria's real pipeline stages while avoiding the dependency governance overhead of dozens of micro-crates.

---

## 5. Consequences & Trade-Offs
- **Positive:**
  - Parallel compilation reduces build times by ~60%.
  - Zero possibility of circular dependencies between renderer and DOM.
  - Headless tests run in seconds without WGPU dependencies.
- **Negative:**
  - Requires moving source files into `crates/` subdirectories and setting up workspace `Cargo.toml`.
