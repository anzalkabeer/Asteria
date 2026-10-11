# ISSUE: AST-PERF-003 — Naive O(N) Linear Style Cascade Matching

- **Classification:** PERFORMANCE CONCERN
- **Severity:** Medium
- **Priority:** P2
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Style Engine
- **Specification Link:** [`../specifications/README.md`](../specifications/README.md)

---

## 1. Problem Statement
In `src/style.rs:110-185`, computing the styles for a document involves an exhaustive linear loop over every CSS rule in every stylesheet for every DOM node in the tree. On realistic web pages with thousands of DOM nodes and thousands of CSS rules, this $O(N \times R)$ algorithm consumes hundreds of milliseconds of CPU time.

---

## 2. Evidence & Source Location
- **Location:** [`src/style.rs:110-185`](../../../src/style.rs#L110-L185)
Exhaustive linear iteration with no rule index or bloom filter.

---

## 3. Root Cause
The style cascade was built as a baseline reference implementation without rule bucketing.

---

## 4. Desired Behaviour
Introduce a rule sharing cache and bucket CSS rules by tag name, class name, and ID into hash tables, reducing selector matching complexity from $O(N \times R)$ to $O(N \times \log R)$.
