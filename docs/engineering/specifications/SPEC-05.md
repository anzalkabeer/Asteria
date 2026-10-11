# SPEC-05: 7-Crate Workspace Modularization Architecture

- **Related Issue:** [`../issues/AST-ARC-001.md`](../issues/AST-ARC-001.md)
- **Target Subsystem:** Build System & Workspace (`Cargo.toml`, `crates/*`)
- **Stage:** Stage 2 (P1)
- **ADR Reference:** [`../decisions/ADR-005-workspace-modularization.md`](../decisions/ADR-005-workspace-modularization.md)

---

## 1. Problem Statement
Asteria currently exists as a single monolithic crate with 33 submodules in `src/`. Clean builds take over 5 minutes (`302s` baseline), and internal module boundaries are not enforced by the compiler.

---

## 2. Root Cause
Monolithic Cargo package layout without workspace crate partitioning.

---

## 3. Recommended Solution
Restructure Asteria into a 7-crate Cargo workspace:
- `crates/asteria_dom`: HTML5 tokenizer, DOM node tree, generic tree data structures.
- `crates/asteria_style`: CSS tokenizer, property parsing, cascade, specificity, computed style.
- `crates/asteria_layout`: Geometry, box tree, BFC, IFC, and Taffy layout adapters.
- `crates/asteria_paint`: Display list, command builder, font context, glyph cache.
- `crates/asteria_render`: WGPU device, WGSL shaders, vertex batching, texture passes.
- `crates/asteria_net`: URL parser, HTTP client, resource loader, disk cache.
- `crates/asteria_shell`: Winit event loop, window management, browser chrome, CLI.

### Workspace Root `Cargo.toml`:
```toml
[workspace]
members = [
    "crates/asteria_dom",
    "crates/asteria_style",
    "crates/asteria_layout",
    "crates/asteria_paint",
    "crates/asteria_render",
    "crates/asteria_net",
    "crates/asteria_shell",
]
resolver = "2"
```

---

## 4. Affected Source Locations
- `Cargo.toml`
- Movement of `src/*` modules into `crates/*`

---

## 5. Verification & Acceptance Criteria
1. `cargo check --workspace` completes successfully across all 7 crates.
2. `cargo test -p asteria_dom` runs cleanly in under 5 seconds without linking WGPU or graphics drivers.
