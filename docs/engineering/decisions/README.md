# Architecture Decision Records (ADRs)

> **Convention:** Lightweight, durable records of major architectural choices, trade-offs, and rejected alternatives.  
> **Status Lifecycle:** `PROPOSED` → `ACCEPTED` → `SUPERSEDED` | `REJECTED`.  
> **Rule:** An item recommended in an audit is `PROPOSED` until formally ratified and authorized by project leads.

---

## Decision Records Index

| ADR ID | Title | Status | Date | Primary Topics | Related Issues |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **[ADR-001](ADR-001-pure-rust-over-spidermonkey-webrender.md)** | Pure Rust Architecture Over C++ Runtimes | **ACCEPTED** | Aug 2026 | Pure Rust, No C++, No SpiderMonkey | Core Philosophy |
| **[ADR-002](ADR-002-wgpu-graphics-abstraction.md)** | Native WGPU WebGPU Graphics Abstraction | **ACCEPTED** | Aug 2026 | WGPU 0.20, WGSL, Vulkan/DX12/Metal | Core Graphics |
| **[ADR-003](ADR-003-retained-gpu-transform-scrolling.md)** | Retained GPU Spatial Transform Scrolling | **PROPOSED** | Oct 2026 | WGSL Uniforms, 120 FPS Scroll | `AST-DEF-001`, `SPEC-08` |
| **[ADR-004](ADR-004-taffy-layout-delegation.md)** | Delegation of Flexbox & Grid to Taffy | **PROPOSED** | Oct 2026 | Taffy 0.14, CSS Grid, Flexbox Wrap | `AST-ARC-003`, `SPEC-10` |
| **[ADR-005](ADR-005-workspace-modularization.md)** | 7-Crate Workspace Partitioning | **PROPOSED** | Oct 2026 | Cargo Workspace, Boundary Enforcement | `AST-ARC-001`, `SPEC-05` |

---

## ADR Template

Each ADR must follow this standardized schema:

```markdown
# ADR-XXX: Title

- **Status:** [PROPOSED | ACCEPTED | SUPERSEDED | REJECTED]
- **Date:** YYYY-MM-DD
- **Deciders:** [Contributors / AI Agents]
- **Consulted:** [Audit Documents, References]

## 1. Context & Problem Statement
[Describe the problem requiring an architectural decision.]

## 2. Decision Drivers
[List the architectural constraints, performance requirements, or safety properties.]

## 3. Considered Options
[Detail all evaluated alternatives, including status quo.]

## 4. Decision & Rationale
[Explain the selected choice and why it was chosen over the alternatives.]

## 5. Consequences & Trade-Offs
- Positive impacts
- Negative impacts / new maintenance burdens

## 6. Related Issues & Source References
[Link canonical issue IDs and source files.]
```
