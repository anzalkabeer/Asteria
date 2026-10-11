# ISSUE: AST-INC-003 — Missing JavaScript Execution Runtime

- **Classification:** INCOMPLETE IMPLEMENTATION
- **Severity:** High
- **Priority:** P3
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Scripting & DOM Runtime
- **Enhancement Link:** [`../../../ASTERIA_IMPROVEMENT_AUDIT/06_ENHANCEMENT_OPPORTUNITIES.md#enh-01-embedded-pure-rust-ecmascript-runtime-boa-engine`](../../../ASTERIA_IMPROVEMENT_AUDIT/06_ENHANCEMENT_OPPORTUNITIES.md#enh-01-embedded-pure-rust-ecmascript-runtime-boa-engine)

---

## 1. Problem Statement
Asteria currently cannot execute JavaScript. The DOM tree is completely static and read-only post-parse, `<script>` tags are skipped during HTML parsing, and no event target bindings exist for DOM manipulation or user interaction scripts.

---

## 2. Evidence & Source Location
- **Location:** [`src/dom.rs:1-420`](../../../src/dom.rs#L1-L420) & [`src/html.rs`](../../../src/html.rs)
DOM lacks mutation methods (`appendChild`, `setAttribute`); script tags are discarded.

---

## 3. Root Cause
Asteria prioritized static document layout and GPU rendering before introducing an ECMAScript engine.

---

## 4. Desired Behaviour
Embed the pure-Rust **Boa ECMAScript Engine** behind an isolated script thread actor, binding DOM node manipulation, event listeners, and timers to Boa native objects.
