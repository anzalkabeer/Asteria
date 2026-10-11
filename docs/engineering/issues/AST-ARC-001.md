# ISSUE: AST-ARC-001 — Monolithic Single-Crate Cargo Architecture

- **Classification:** ARCHITECTURAL WEAKNESS
- **Severity:** High
- **Priority:** P1
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Build System & Workspace
- **Specification Link:** [`../specifications/SPEC-05.md`](../specifications/SPEC-05.md)

---

## 1. Problem Statement
All 33 engine modules reside in a single monolithic Cargo package (`asteria 0.1.0`). Clean compilation takes over 5 minutes (302s baseline). Unit testing of parsing and layout requires compiling the entire WGPU graphics stack, and internal compiler boundaries cannot prevent circular dependencies between subsystems.

---

## 2. Evidence & Source Location
- **Location:** [`Cargo.toml:1-48`](../../../Cargo.toml#L1-L48)
```toml
[package]
name = "asteria"
version = "0.1.0"
# Single crate contains all 33 modules in src/
```

---

## 3. Root Cause
Asteria began as a flat experimental prototype and was never partitioned into a multi-crate Cargo workspace as the codebase expanded.

---

## 4. Desired Behaviour
Partition Asteria into a 7-crate workspace (`asteria_dom`, `asteria_style`, `asteria_layout`, `asteria_paint`, `asteria_render`, `asteria_net`, `asteria_shell`) enforcing strict compile-time dependency isolation.
