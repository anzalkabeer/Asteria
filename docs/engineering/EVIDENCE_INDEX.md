# EVIDENCE_INDEX.md — Forensic Evidence & Provenance Directory

> **Repository Baseline:** Asteria Git commit `3cdd5155caf544794c3d11f45dfc839665d9759b` on `main`.  
> **Link Convention:** All links are relative to the repository root for full GitHub browsing compatibility.

---

## 1. Master Forensic Finding Evidence Cross-Reference

| Issue ID | Subsystem | Asteria Source Path & Lines | Historical Audit Finding | Servo Reference Audit Document | Verification Status |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **AST-DEF-001** | Shell / Render | [`src/renderer/window/window.rs:747-794`](../../src/renderer/window/window.rs#L747-L794) | `AST-DEF-001`, `AST-005` | [`04_WEBRENDER_AND_COMPOSITING.md`](../../asteria_servo_audit/subsystems/04_WEBRENDER_AND_COMPOSITING.md) | **CONFIRMED** via event loop inspection; redraw triggers CPU rebuild. |
| **AST-DEF-002** | Media / Render | [`src/image.rs:97-100`](../../src/image.rs#L97-L100)<br>[`src/renderer/command_builder.rs:97-128`](../../src/renderer/command_builder.rs#L97-L128)<br>[`src/renderer/gpu/renderer.rs:248-275`](../../src/renderer/gpu/renderer.rs#L248-L275) | `AST-DEF-002` | [`06_IMAGE_AND_RESOURCE_PIPELINE.md`](../../asteria_servo_audit/subsystems/06_IMAGE_AND_RESOURCE_PIPELINE.md) | **CONFIRMED** in source; `ImagePass` uncalled, dummy bytes emitted. |
| **AST-DEF-003** | Networking | [`src/net/http.rs:47-70`](../../src/net/http.rs#L47-L70)<br>[`src/net/url.rs:48-96`](../../src/net/url.rs#L48-L96) | `AST-DEF-003`, `NET-BUG-01` | [`07_NETWORKING_AND_RESOURCE_LOADING.md`](../../asteria_servo_audit/subsystems/07_NETWORKING_AND_RESOURCE_LOADING.md) | **CONFIRMED** in source; `#` fragment included in HTTP request line. |
| **AST-DEF-004** | Networking | [`src/net/http.rs:163-195`](../../src/net/http.rs#L163-L195)<br>[`src/net/fetch.rs:104-142`](../../src/net/fetch.rs#L104-L142) | `AST-DEF-004`, `NET-BUG-02` | [`07_NETWORKING_AND_RESOURCE_LOADING.md`](../../asteria_servo_audit/subsystems/07_NETWORKING_AND_RESOURCE_LOADING.md) | **CONFIRMED** in source; unbounded `Vec<u8>` read buffer. |
| **AST-ARC-001** | Workspace | [`Cargo.toml:1-48`](../../Cargo.toml#L1-L48) | `AST-ARC-001`, `AST-009` | [`02_CRATE_CATALOGUE.md`](../../asteria_servo_audit/02_CRATE_CATALOGUE.md) | **CONFIRMED** in manifest; single monolithic crate, 33 modules in `src/`. |
| **AST-ARC-002** | Text / Layout | [`src/layout.rs:2534-2558`](../../src/layout.rs#L2534-L2558)<br>[`src/text.rs:188-232`](../../src/text.rs#L188-L232) | `AST-ARC-002`, `AST-003` | [`05_LAYOUT_2020.md`](../../asteria_servo_audit/subsystems/05_LAYOUT_2020.md) | **CONFIRMED** in source; `estimate_char_width_ratio` hardcodes ratios. |
| **AST-ARC-003** | Layout Engine | [`src/layout.rs:1350-1685`](../../src/layout.rs#L1350-L1685) | `AST-ARC-003`, `AST-002` | [`05_LAYOUT_2020.md`](../../asteria_servo_audit/subsystems/05_LAYOUT_2020.md) | **CONFIRMED** in source; single-line row flexbox only, no grid. |
| **AST-ARC-004** | Scene Graph | [`src/scene.rs:360-372`](../../src/scene.rs#L360-L372) | `AST-ARC-004` | [`04_WEBRENDER_AND_COMPOSITING.md`](../../asteria_servo_audit/subsystems/04_WEBRENDER_AND_COMPOSITING.md) | **CONFIRMED** in source; parent-child inferred via `contains()`. |
| **AST-ARC-005** | Concurrency | [`src/scheduler.rs:26-107`](../../src/scheduler.rs#L26-L107)<br>[`src/scheduler.rs:142-198`](../../src/scheduler.rs#L142-L198) | `AST-ARC-005` | [`03_RUNTIME_FLOWS.md`](../../asteria_servo_audit/03_RUNTIME_FLOWS.md) | **CONFIRMED** in source; `TaskScheduler` priority queue uncalled. |
| **AST-INC-001** | Memory System | [`src/arena.rs:1-120`](../../src/arena.rs#L1-L120)<br>[`src/pool.rs:1-110`](../../src/pool.rs#L1-L110)<br>[`src/intern.rs:1-180`](../../src/intern.rs#L1-L180) | `AST-INC-001`, `AST-007` | [`10_SECURITY_AND_PERFORMANCE_AUDIT.md`](../../asteria_servo_audit/10_SECURITY_AND_PERFORMANCE_AUDIT.md) | **CONFIRMED** in source; zero call-sites outside unit test modules. |
| **AST-INC-002** | Style Engine | [`src/css.rs:88-142`](../../src/css.rs#L88-L142)<br>[`src/style.rs:215-320`](../../src/style.rs#L215-L320) | `AST-INC-002` | [`05_LAYOUT_2020.md`](../../asteria_servo_audit/subsystems/05_LAYOUT_2020.md) | **CONFIRMED** in source; simplified selector specificity. |
| **AST-INC-003** | Scripting | [`src/dom.rs:1-420`](../../src/dom.rs#L1-L420) | `AST-INC-003`, `AST-010` | [`08_SCRIPT_AND_DOM.md`](../../asteria_servo_audit/subsystems/08_SCRIPT_AND_DOM.md) | **CONFIRMED** in source; static read-only DOM, no script runtime. |
| **AST-PERF-001** | GPU Renderer | [`src/renderer/gpu/batch_builder.rs:45-120`](../../src/renderer/gpu/batch_builder.rs#L45-L120)<br>[`src/renderer/gpu/renderer.rs:310-380`](../../src/renderer/gpu/renderer.rs#L310-L380) | `AST-PERF-001` | [`04_WEBRENDER_AND_COMPOSITING.md`](../../asteria_servo_audit/subsystems/04_WEBRENDER_AND_COMPOSITING.md) | **CONFIRMED** in source; `create_buffer_init` called every frame. |
| **AST-PERF-002** | Networking | [`src/net/fetch.rs:45-98`](../../src/net/fetch.rs#L45-L98) | `AST-PERF-002`, `AST-011` | [`07_NETWORKING_AND_RESOURCE_LOADING.md`](../../asteria_servo_audit/subsystems/07_NETWORKING_AND_RESOURCE_LOADING.md) | **CONFIRMED** in source; synchronous TCP socket blocking. |
| **AST-PERF-003** | Style Engine | [`src/style.rs:110-185`](../../src/style.rs#L110-L185) | `AST-PERF-003`, `AST-004` | [`05_LAYOUT_2020.md`](../../asteria_servo_audit/subsystems/05_LAYOUT_2020.md) | **CONFIRMED** in source; $O(N \times R)$ linear rule matching. |
| **AST-PERF-004** | Layout Engine | [`src/renderer/window/window.rs:810-850`](../../src/renderer/window/window.rs#L810-L850) | `AST-PERF-004` | [`05_LAYOUT_2020.md`](../../asteria_servo_audit/subsystems/05_LAYOUT_2020.md) | **CONFIRMED** in source; full relayout from root on mutation. |
| **AST-SEC-001** | Security / Net | [`src/net/fetch.rs:145-180`](../../src/net/fetch.rs#L145-L180) | `AST-SEC-001` | [`10_SECURITY_AND_PERFORMANCE_AUDIT.md`](../../asteria_servo_audit/10_SECURITY_AND_PERFORMANCE_AUDIT.md) | **CONFIRMED** in source; unconfined `file://` reading from disk. |
| **AST-SEC-002** | Web Security | [`src/html.rs:40-110`](../../src/html.rs#L40-L110)<br>[`src/net/fetch.rs:80-105`](../../src/net/fetch.rs#L80-L105) | `AST-SEC-002` | [`07_NETWORKING_AND_RESOURCE_LOADING.md`](../../asteria_servo_audit/subsystems/07_NETWORKING_AND_RESOURCE_LOADING.md) | **CONFIRMED** in source; zero CSP headers or origin validation. |
| **AST-ENG-001** | Quality Assurance| [`tests/`](../../tests/) | `AST-ENG-001` | [`09_TESTING_AND_VERIFICATION_GUIDE.md`](../../asteria_servo_audit/09_TESTING_AND_VERIFICATION_GUIDE.md) | **CONFIRMED** in tree; zero automated pixel reftests exist. |

---

## 2. Empirical Compilation Baseline Evidence

- **Execution Command:** `cargo test --no-run`
- **Executed In:** `Asteria/Asteria/`
- **Git Commit:** `3cdd5155caf544794c3d11f45dfc839665d9759b`
- **Exit Code:** `0` (Success)
- **Duration:** 302.26s (~5m 02s)
- **Detailed Build Log:** [`ASTERIA_IMPROVEMENT_AUDIT/evidence/CARGO_TEST_BASELINE.md`](../../ASTERIA_IMPROVEMENT_AUDIT/evidence/CARGO_TEST_BASELINE.md)
