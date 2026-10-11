# ADR-001: Pure Rust Architecture Over C++ Browser Runtimes

- **Status:** **ACCEPTED**
- **Date:** August 2026 (Foundational Invariant)
- **Deciders:** Keshav, Anzal Kabeer
- **Consulted:** `collab-model.md`, `CLAUDE.md`, Servo Comparative Audit

---

## 1. Context & Problem Statement
Traditional browser engines (Chromium/Blink, Gecko, WebKit, and Servo) rely extensively on legacy C and C++ runtimes for critical components such as JavaScript execution (V8, SpiderMonkey, JavaScriptCore), text shaping and rasterization (HarfBuzz, FreeType, Skia), and graphics compositing (WebRender, Angle, EGL). Integrating these components into Asteria requires complex multi-language build systems (CMake, Python `mach`, Ninja), introduces memory safety risks, and severely harms developer ergonomics on Windows.

---

## 2. Decision Drivers
1. **Developer Velocity & Portability:** Asteria must build cleanly with a standard `cargo build` command without requiring external C++ compilers, MSYS2, or Python build scripts.
2. **Memory Safety:** Eliminate buffer overflows, use-after-free, and type confusion vulnerabilities inherent in legacy C++ browser subsystems.
3. **Auditability:** Maintain a compact, intelligible codebase where the entire pipeline can be understood and audited by a small engineering team.

---

## 3. Considered Options
- **Option A:** Embed Servo's SpiderMonkey and WebRender stacks.
- **Option B:** Embed Blink/V8 or WebKit runtime components via C bindings.
- **Option C (Selected):** Build an entirely 100% pure-Rust rendering engine, adopting modern pure-Rust ecosystem crates (WGPU, Taffy, Boa, Swash, Rustls).

---

## 4. Decision & Rationale
Asteria strictly adheres to a **100% Pure Rust Architecture**. All core rendering, parsing, styling, networking, and layout subsystems are written in safe, idiomatic Rust. SpiderMonkey and WebRender are explicitly rejected. When ECMAScript support is introduced, it will use pure-Rust engines such as Boa.

---

## 5. Consequences & Trade-Offs
- **Positive:**
  - Build succeeds with plain `cargo build` in under 5 minutes.
  - Zero C/C++ memory safety vulnerabilities in engine core.
  - Portable across Windows, Linux, and macOS without platform C toolchains.
- **Negative:**
  - Cannot leverage 20 years of C++ browser optimization in SpiderMonkey/V8.
  - ECMAScript execution speed in Boa will be slower than JIT-compiled V8/SpiderMonkey.
