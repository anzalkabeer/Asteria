# ISSUE: AST-ARC-003 — Ad-Hoc Flexbox Without Grid or Multi-Line Wrap

- **Classification:** ARCHITECTURAL WEAKNESS / INCOMPLETE
- **Severity:** High
- **Priority:** P1
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Layout Engine
- **Specification Link:** [`../specifications/SPEC-10.md`](../specifications/SPEC-10.md)

---

## 1. Problem Statement
Asteria's layout engine (`src/layout.rs:1350-1685`) contains an ad-hoc, handwritten flexbox solver (~400 LOC) that only handles basic single-line row flow. It lacks multi-line wrapping (`flex-wrap: wrap`), `flex-grow` and `flex-shrink` proportion scaling, baseline alignment, and CSS Grid layout is completely missing.

---

## 2. Evidence & Source Location
- **Location:** [`src/layout.rs:1350-1685`](../../../src/layout.rs#L1350-L1685)
Handwritten solver operates only on linear children sequences and lacks cross-axis wrapping calculations.

---

## 3. Root Cause
Implementing full W3C Flexbox and Grid specifications by hand requires thousands of lines of specialized geometry solving algorithms.

---

## 4. Desired Behaviour
Delegate Flexbox and CSS Grid container calculations to the battle-tested, pure-Rust **Taffy 0.14** engine via a thin adapter module in `asteria_layout`.
