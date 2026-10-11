# Asteria Forensic Audit of Servo: Persistent Progress & State Checkpoint

> **Audit Identifier:** ASTERIA_SERVO_AUDIT_2026_10  
> **Target Engine (Audit Subject):** Servo (`components/`, `ports/servoshell`, `ffi/capi`)  
> **Beneficiary Engine:** Asteria (`Asteria/Asteria`)  
> **Last Updated:** 2026-10-10  
> **Active Auditor:** Senior Browser-Engine Architect / Reverse Engineer Agent

---

## 1. Environment & Baseline Invariants

| Attribute | Servo (Audit Subject) | Asteria (Beneficiary Project) |
| :--- | :--- | :--- |
| **Workspace Root** | `z:\1mportant stuff\Codes written by me (New)\servo` | `z:\1mportant stuff\Codes written by me (New)\Asteria\Asteria` |
| **Git Revision / Commit** | `7ca9f80268a1d76e698560db82463e7960aa7845` | `3cdd5155caf544794c3d11f45dfc839665d9759b` |
| **Git Branch** | `main` (tracking `origin/main`) | `main` (tracking `origin/main`) |
| **Working Tree Status** | Clean (`## main...origin/main`) | Working (`M Cargo.lock`) |
| **Rust Edition & Channel** | Rust 2024 (channel `1.97.1` pin; host `1.96.1`) | Rust 2024 (host `1.96.1`) |
| **Build & Package Tooling** | Cargo Workspace + `mach` Python driver (`uv`) | Pure Cargo (`Cargo.toml`) |
| **Crate Composition** | 77 workspace packages (modular multi-process) | Monolithic single crate (`lib.rs` + `main.rs`) |
| **Audit Output Location** | `z:\1mportant stuff\Codes written by me (New)\asteria_servo_audit` (isolated) | Same workspace |

---

## 2. Audit Execution Phases

- [x] **Phase 0: Environment Establishment & Tooling Inventory** *(Completed)*
  - Verified local paths, clean Git status of Servo, Asteria repository presence, Rust compiler versions, and tool capabilities.
  - Initialized isolated audit directory structure: `findings/`, `subsystems/`, `diagrams/`, `evidence/`.
  - Extracted deterministic `cargo metadata` into raw and CSV evidence tables.
- [x] **Phase 1: Repository Census & Crate Catalog** *(Completed)*
  - [x] Deliver `01_REPOSITORY_MAP.md` (Directory structure, workspace configuration, build system, externals).
  - [x] Deliver `02_CRATE_CATALOG.md` (Detailed census of all 77 crates, APIs, dependencies, consumers, Asteria relevance).
- [x] **Phase 2: System Architecture & Concurrency Topology** *(Completed)*
  - [x] Deliver `03_SYSTEM_ARCHITECTURE.md` (Multi-process IPC, thread models, Constellation, Script, Layout, Paint, Net).
- [x] **Phase 3: Subsystem Deep-Dive Audits** *(Completed)*
  - [x] `subsystems/01_EMBEDDING_AND_SHELL.md` (`components/servo`, `servo-embedder-traits`, `servoshell`)
  - [x] `subsystems/02_CONSTELLATION_AND_LIFECYCLE.md` (`servo-constellation`, `servo-constellation-traits`)
  - [x] `subsystems/03_SCRIPT_AND_DOM.md` (`servo-script`, `servo-script-traits`, DOM, mozjs)
  - [x] `subsystems/04_NETWORKING_AND_RESOURCE_LOADING.md` (`servo-net`, `servo-net-traits`, fetch)
  - [x] `subsystems/05_STYLING_AND_SELECTOR_MATCHING.md` (Stylo, selectors, style resolution)
  - [x] `subsystems/06_LAYOUT_AND_FORMATTING.md` (`servo-layout`, `servo-layout-api`, Taffy, flow tree)
  - [x] `subsystems/07_PAINT_AND_GRAPHICS.md` (`servo-paint`, `servo-paint-api`, WebRender, display lists)
  - [x] `subsystems/08_FONTS_AND_TEXT_SHAPING.md` (`servo-fonts`, `servo-fonts-traits`, HarfBuzz, system fonts)
  - [x] `subsystems/09_INPUT_AND_EVENT_DISPATCH.md` (Event routing from winit to DOM)
  - [x] `subsystems/10_STORAGE_MEDIA_SERVICES.md` (`servo-storage`, WebGL/WebGPU, media)
- [x] **Phase 4: End-to-End Traced Runtime Flows** *(Completed)*
  - [x] Deliver `04_RUNTIME_FLOWS.md` (Startup, WebView creation, Navigation, Parsing->DOM, Style->Layout->Paint, Input dispatch, Teardown).
- [x] **Phase 5: Public APIs, Extension Points & Embedding** *(Completed)*
  - [x] Deliver `05_APIS_AND_EXTENSION_POINTS.md` (ServoBuilder, WebView, RenderingContext, EventLoopWaker, protocol handlers).
- [x] **Phase 6: Servo-to-Asteria Engineering Guide & Comparative Analysis** *(Completed)*
  - [x] Deliver `06_ASTERIA_INTEGRATION_GUIDE.md` (Direct gap analysis vs `Asteria/Asteria/src`, architecture transition plan).
- [x] **Phase 7: Security, Reliability, Performance & Build/Test Tooling** *(Completed)*
  - [x] Deliver `07_SECURITY_RELIABILITY_PERFORMANCE.md` (Sandbox, memory safety, malloc_size_of, profiler).
  - [x] Deliver `08_TESTING_BUILD_AND_DEBUG_GUIDE.md` (Mach commands, WPT testing, unit tests, tracing).
- [x] **Phase 8: Central Source Evidence Index** *(Completed)*
  - [x] Deliver `09_SOURCE_EVIDENCE_INDEX.md` (Stable finding IDs: `ARCH-xxx`, `FLOW-xxx`, `API-xxx`, `AST-xxx`, etc.).
- [ ] **Phase 9: Final Synthesis & Coverage Verification** *(In Progress)*
  - [ ] Deliver `00_EXECUTIVE_SUMMARY.md`
  - [ ] Deliver `10_AUDIT_COVERAGE_AND_REMAINING_WORK.md`

---

## 3. Key Architectural Discoveries to Date

1. **Strict Core/Traits Boundary:** Servo enforces clean decoupled communication using a paired crate architecture (`components/<name>` vs `components/shared/<name>`). Every major subsystem has a dedicated traits/API crate (e.g., `servo-constellation-traits`, `servo-script-traits`, `servo-net-traits`, `servo-layout-api`, `servo-paint-api`, `servo-embedder-traits`).
2. **Actor & IPC Concurrency:** Subsystems run as independent threads or separate OS processes communicating via crossbeam channels (in-process) and `ipc-channel` (cross-process). The `Constellation` actor coordinates all pipeline instances.
3. **Stylo & WebRender Engine Pipeline:** Rather than custom rendering and CSS parsing, Servo offloads CSS styling to Stylo (shared with Firefox) and 2D/3D GPU scene decomposition to WebRender (retained display list GPU engine), with layout managed via `servo-layout` and `taffy`.
4. **Asteria vs. Servo Contrast:** Asteria is a monolithic immediate-style browser engine built atop `wgpu` and handwritten recursive layout/CSS parsers. Asteria's pain points (identified in `FLAWS.md`—percentage handling, flexbox layout stability, UA styling cascade, async networking coordination) directly mirror problems Servo solved through its pipeline decoupling and trait abstractions.

---

## 4. Current Workstream & Exact Next Steps

- **Active Task:** Authoring `01_REPOSITORY_MAP.md` and `02_CRATE_CATALOG.md` grounded in deterministic manifest analysis and source verification.
- **Next Steps:**
  1. Complete and write `01_REPOSITORY_MAP.md`.
  2. Complete and write `02_CRATE_CATALOG.md` categorizing all 77 packages into architectural tiers.
  3. Synthesize `03_SYSTEM_ARCHITECTURE.md` mapping Servo's pipeline actor model.
