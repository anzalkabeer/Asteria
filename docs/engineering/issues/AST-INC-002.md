# ISSUE: AST-INC-002 — Incomplete CSS Specificity & Combinator Matching

- **Classification:** INCOMPLETE IMPLEMENTATION
- **Severity:** Medium
- **Priority:** P2
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Style Engine & CSS Parser
- **Specification Link:** [`../specifications/README.md`](../specifications/README.md)

---

## 1. Problem Statement
Asteria's selector engine (`src/css.rs:88-142`, `src/style.rs:215-320`) only matches simple tag, class, and ID selectors. It fails to match direct child combinators (`div > p`), adjacent sibling (`h1 + p`), general sibling (`h1 ~ p`), and pseudo-elements (`::before`, `::after`).

---

## 2. Evidence & Source Location
- **Location:** [`src/css.rs:88-142`](../../../src/css.rs#L88-L142) & [`src/style.rs:215-320`](../../../src/style.rs#L215-L320)
Compound selector matching does not recurse up parent or sibling pointers.

---

## 3. Root Cause
The selector matcher was implemented with a flat loop rather than a full CSS Selectors Level 4 tree walker.

---

## 4. Desired Behaviour
Extend selector matching to support child, sibling, and attribute selectors with full 4-tuple specificity `(inline, id, class, element)` per CSS specifications.
