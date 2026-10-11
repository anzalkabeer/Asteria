# AGENTS.md — Contributor & AI Agent Operating Instructions

> **Target Repository:** Asteria Browser Engine (`Asteria/Asteria`)  
> **Audience:** Antigravity, Anzal Kabeer, Keshav, downstream AI coding agents, and human contributors.  
> **Status:** Canonical Entry Point & Persistent Engineering Directive.  

---

## 1. Golden Rules of Engagement

1. **Check State Before Acting:** Always read [`docs/engineering/PROJECT_STATE.md`](docs/engineering/PROJECT_STATE.md) and [`docs/engineering/HANDOFF.md`](docs/engineering/HANDOFF.md) before starting or resuming any task. Do not rely on prior conversation transcripts or hidden agent memory.
2. **Consult Architecture First:** Consult [`docs/engineering/ARCHITECTURE.md`](docs/engineering/ARCHITECTURE.md) before modifying or crossing module/crate boundaries.
3. **Trace Issues to Specifications:** Every fix or refactor must reference a canonical ID in [`docs/engineering/ISSUES.md`](docs/engineering/ISSUES.md) and its corresponding specification in [`docs/engineering/specifications/`](docs/engineering/specifications/).
4. **Source Code is Ground Truth:** Inspect actual source files and line ranges. Never trust stale comments, old documentation, or previous LLM completions without verifying the current code.
5. **Verify Git Invariants:** Check `git status` and the active commit hash before and after edits. Do not modify or revert unrelated working-tree changes (e.g. modified `Cargo.lock`).
6. **Atomic & Reviewable Changes:** Implement changes in small, logically bounded steps. Avoid sprawling multi-file rewrites unless explicitly executing an authorized architecture refactor.
7. **Empirical Verification Required:** You must run and report actual test commands and results. Differentiate `FIXED` (code changed) from `VERIFIED` (passing tests and regression checks executed).
8. **Update Engineering Records:** Every completed task must update [`docs/engineering/IMPLEMENTATION_LOG.md`](docs/engineering/IMPLEMENTATION_LOG.md), update issue statuses in [`docs/engineering/ISSUES.md`](docs/engineering/ISSUES.md), and refresh [`docs/engineering/HANDOFF.md`](docs/engineering/HANDOFF.md).
9. **Document Architectural Decisions:** Record any new technical trade-offs or decisions in [`docs/engineering/decisions/`](docs/engineering/decisions/) (ADRs). Document rejected alternatives with explicit rationale.
10. **Preserve Distinctions:** Rigorously distinguish **Confirmed Facts** (verified in source/tests), **Inferences/Hypotheses** (suspected but unmeasured), and **Proposals** (planned architecture).

---

## 2. Documentation Hierarchy & Source of Truth

The documentation inside this repository is structured into distinct tiers. Do not create competing sources of truth.

```
Asteria/
├── AGENTS.md                              <-- THIS FILE: Universal contributor entry point
├── Cargo.toml / Cargo.lock                <-- Build system & dependency lock
├── src/ & tests/                          <-- Canonical source code
├── ASTERIA_IMPROVEMENT_AUDIT/             <-- Historical audit reference & comparative analysis
├── asteria_servo_audit/                   <-- Historical Servo reference audit
└── docs/
    ├── [00-20]-*.md                       <-- User & tutorial documentation (educational)
    └── engineering/                       <-- CANONICAL ENGINEERING MEMORY
        ├── PROJECT_STATE.md               <-- Current operational state & verified capabilities
        ├── ARCHITECTURE.md                <-- Reconstructed system architecture & dataflow
        ├── ISSUES.md                      <-- Master searchable issue index & collision mapping
        ├── ROADMAP.md                     <-- 5-stage dependency-aware implementation plan
        ├── IMPLEMENTATION_LOG.md          <-- Durable change log of completed engineering tasks
        ├── HANDOFF.md                     <-- Immediate task context for resuming work
        ├── TESTING.md                     <-- Verification commands, test harnesses, CI matrix
        ├── EVIDENCE_INDEX.md              <-- Cross-reference index linking IDs to source lines
        ├── decisions/                     <-- Architecture Decision Records (ADRs)
        ├── issues/                        <-- Individual detailed issue records (AST-*)
        └── specifications/                <-- Detailed technical remediation specs (SPEC-*)
```

---

## 3. Operational Protocols for Implementation Tasks

When assigned an implementation mission:

### Step 1: Pre-Flight Context Loading
- Read [`docs/engineering/HANDOFF.md`](docs/engineering/HANDOFF.md) to understand the current task.
- Read [`docs/engineering/PROJECT_STATE.md`](docs/engineering/PROJECT_STATE.md) to review compiler invariants and active limitations.
- Read the assigned issue in [`docs/engineering/issues/`](docs/engineering/issues/) and specification in [`docs/engineering/specifications/`](docs/engineering/specifications/).

### Step 2: Source Code Verification
- Locate the affected source files and lines using [`docs/engineering/EVIDENCE_INDEX.md`](docs/engineering/EVIDENCE_INDEX.md).
- Verify that the described defect or limitation still exists in the active code.

### Step 3: Targeted Implementation
- Make minimal, clean, idiomatic Rust modifications strictly addressing the specification.
- Follow Asteria's pure-Rust, zero-C++ dependency philosophy ([ADR-001](docs/engineering/decisions/ADR-001-pure-rust-over-spidermonkey-webrender.md)).

### Step 4: Verification & Test Execution
- Run relevant unit/integration tests as outlined in [`docs/engineering/TESTING.md`](docs/engineering/TESTING.md).
- Record exact terminal commands, exit codes, and durations.
- If visual rendering or GPU passes are touched, verify that no regressions occur.

### Step 5: Documentation Handoff
- Append an entry to [`docs/engineering/IMPLEMENTATION_LOG.md`](docs/engineering/IMPLEMENTATION_LOG.md).
- Update the issue state in [`docs/engineering/ISSUES.md`](docs/engineering/ISSUES.md) (e.g. `IN PROGRESS` -> `FIXED` -> `VERIFIED`).
- Update [`docs/engineering/HANDOFF.md`](docs/engineering/HANDOFF.md) with next steps.
