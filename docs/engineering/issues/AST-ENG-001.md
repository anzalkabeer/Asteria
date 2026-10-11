# ISSUE: AST-ENG-001 — Missing Visual Reftests & Automated Pixel CI

- **Classification:** ENGINEERING PRACTICE GAP
- **Severity:** High
- **Priority:** P2
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Quality Assurance & Testing Infrastructure
- **Enhancement Link:** [`../../../ASTERIA_IMPROVEMENT_AUDIT/06_ENHANCEMENT_OPPORTUNITIES.md#enh-03-headless-gpu-accelerated-visual-reftest-harness`](../../../ASTERIA_IMPROVEMENT_AUDIT/06_ENHANCEMENT_OPPORTUNITIES.md#enh-03-headless-gpu-accelerated-visual-reftest-harness)

---

## 1. Problem Statement
Asteria's test suite consists exclusively of unit tests asserting data structure properties. There are zero automated visual regression tests (reftests) comparing rendered GPU output against reference HTML or baseline PNG images. Visual regressions in border radius, font line heights, or flexbox layouts can go undetected.

---

## 2. Evidence & Source Location
- **Location:** [`tests/`](../../../tests/) directory
All 11 test suites test AST tokens, layout rect coordinates, or HTTP strings; none inspect rendered framebuffer pixels.

---

## 3. Root Cause
Asteria lacks a headless offscreen WGPU render harness capable of dumping framebuffer textures to PNG files without spawning a physical desktop window.

---

## 4. Desired Behaviour
Implement a headless GPU reftest harness that renders test HTML documents to offscreen textures, computes pixel-diff ratios against reference images, and fails CI on visual deviations.
