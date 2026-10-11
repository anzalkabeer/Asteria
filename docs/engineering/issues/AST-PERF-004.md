# ISSUE: AST-PERF-004 — Full Document Relayout on DOM Mutation

- **Classification:** PERFORMANCE CONCERN
- **Severity:** Medium
- **Priority:** P2
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Layout Engine & Pipeline
- **Specification Link:** [`../specifications/README.md`](../specifications/README.md)

---

## 1. Problem Statement
Whenever a DOM attribute or text content changes, `src/renderer/window/window.rs:810-850` invalidates the entire document, discards the existing `LayoutBox` tree, and recalculates layout box coordinates from the document root.

---

## 2. Evidence & Source Location
- **Location:** [`src/renderer/window/window.rs:810-850`](../../../src/renderer/window/window.rs#L810-L850)
Full pipeline rebuild invoked unconditionally on any state change.

---

## 3. Root Cause
The layout tree lacks dirty bits and damage propagation tracking.

---

## 4. Desired Behaviour
Introduce dirty flags (`DIRTY_LAYOUT`, `DIRTY_STYLE`) on DOM nodes and layout boxes, enabling incremental reflow of affected subtrees without re-laying out the entire page.
