# TESTING.md — Verification & Quality Assurance Knowledge Base

> **Target Repository:** Asteria Browser Engine (`Asteria/Asteria`)  
> **Audience:** Contributors, CI runners, and AI testing agents.  
> **Last Verified Toolchain:** Rust Stable (x86_64-pc-windows-msvc) on commit `3cdd5155caf544794c3d11f45dfc839665d9759b`.

---

## 1. Build Prerequisites & Environment

- **Rust Version:** Rust 1.80+ (2024 edition target specified in `Cargo.toml`).
- **Graphics Backend:** Hardware supporting Vulkan, DirectX 12, or Metal for WGPU 0.20 rendering.
- **Headless CI Fallback:** In headless CI environments (Linux/Windows containers), software Vulkan rasterizers (Mesa Lavapipe on Linux, WARP on Windows) must be available for tests that initialize WGPU device adapters.

---

## 2. Verified Build & Test Commands

The following commands have been empirically executed and verified in this repository:

### 2.1 Compilation & Test Build Check (Verified Baseline)
```powershell
# Compiles all library code and produces all 11 external test binaries without running them
cargo test --no-run
```
- **Observed Result:** Exit code `0` (Success). Duration: 302.26 seconds (~5m 02s).
- **Verified Output Binaries in `target/debug/deps/`:**
  - `asteria-*.exe` (Internal unit tests in `src/`)
  - `css_tests-*.exe` (`tests/css_tests.rs`)
  - `dom_tests-*.exe` (`tests/dom_tests.rs`)
  - `html_tests-*.exe` (`tests/html_tests.rs`)
  - `http_tests-*.exe` (`tests/http_tests.rs`)
  - `layout_tests-*.exe` (`tests/layout_tests.rs`)
  - `pipeline_tests-*.exe` (`tests/pipeline_tests.rs`)
  - `renderer_tests-*.exe` (`tests/renderer_tests.rs`)
  - `style_tests-*.exe` (`tests/style_tests.rs`)
  - `url_tests-*.exe` (`tests/url_tests.rs`)
  - `window_tests-*.exe` (`tests/window_tests.rs`)

### 2.2 Running Targeted Subsystem Tests
To execute specific test suites quickly without running the entire monolithic package:
```powershell
# Fast non-graphical unit tests (Recommended for rapid iteration)
cargo test --test url_tests
cargo test --test http_tests
cargo test --test html_tests
cargo test --test dom_tests
cargo test --test css_tests
cargo test --test style_tests
cargo test --test layout_tests
```

### 2.3 Running the Application
```powershell
# Launch Asteria with default welcome page
cargo run

# Launch Asteria with specific URL or local file
cargo run -- https://example.com
cargo run -- file:///path/to/test.html
```

---

## 3. Formatting, Linting & Static Analysis (Unverified Baseline)

The following commands are available but have not been formally baselined during this audit:
```powershell
# Check code formatting
cargo fmt --check

# Run static analysis and lints
cargo clippy --all-targets -- -D warnings
```
*(Note: Running `cargo clippy` on the existing monolithic crate is expected to flag dead code warnings for unused memory utilities `src/arena.rs`, `src/pool.rs`, and `src/intern.rs` as identified in [`AST-INC-001`](issues/AST-INC-001.md)).*

---

## 4. Current Testing Limitations & Gaps

1. **Missing Visual Pixel Regression Tests ([`AST-ENG-001`](issues/AST-ENG-001.md)):**
   - The test suite contains zero automated pixel reftests (comparing rendered GPU output against reference PNG images).
   - Visual correctness of rendering passes currently relies on manual inspection in the browser window.
2. **Window Tests Environment Dependency:**
   - Tests in `tests/window_tests.rs` and `src/renderer/window/window.rs` require an active windowing surface handle from `winit`, which may fail on headless virtual displays unless headless backends are configured ([`ENH-03`](../../ASTERIA_IMPROVEMENT_AUDIT/06_ENHANCEMENT_OPPORTUNITIES.md#enh-03-headless-gpu-accelerated-visual-reftest-harness)).
3. **Monolithic Compile Time:**
   - Any test modification requires recompiling the single monolith crate, resulting in slow 30-60s incremental test cycle times. This will be resolved when the workspace is modularized into 7 independent crates in Stage 2 ([`SPEC-05`](specifications/SPEC-05.md)).
