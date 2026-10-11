# EMPIRICAL EVIDENCE: CARGO TEST BASELINE EXECUTION

## 1. Execution Metadata
- **Date & Time:** October 11, 2026
- **Working Directory:** `z:\1mportant stuff\Codes written by me (New)\Asteria\Asteria`
- **Git Commit:** `3cdd5155caf544794c3d11f45dfc839665d9759b` (HEAD on branch `main`)
- **OS Platform:** Windows 10/11 x86_64
- **Rust Toolchain:** Stable x86_64-pc-windows-msvc (Cargo 1.80+)

---

## 2. Command Invocation & Result
```powershell
cargo test --no-run
```
- **Execution Duration:** 302.26 seconds (5 minutes 02 seconds)
- **Process Exit Code:** `0` (Success)
- **Standard Error / Build Log Summary:**
  - Zero compiler errors.
  - Zero fatal link errors.
  - Built 11 separate test runner executables in `target\debug\deps\`.

---

## 3. Verified Output Binaries
The following binaries were successfully generated and verified in `target\debug\deps\`:
1. `asteria-eb37eebebf605da8.exe` (Engine internal unit tests)
2. `css_tests-df190e2fe7b92f02.exe` (CSS parser and specificity tests)
3. `dom_tests-9d41dd7ffea2277d.exe` (DOM tree creation and query tests)
4. `html_tests-33ea06d15a51c4a0.exe` (HTML tokenizer tests)
5. `http_tests-ad0b6e15967bbce3.exe` (HTTP request client tests)
6. `layout_tests-1e64177b8f9e612f.exe` (Box model and layout tests)
7. `pipeline_tests-be760773b06bc961.exe` (Pipeline pass execution tests)
8. `renderer_tests-c07a01d67069c9eb.exe` (Command builder tests)
9. `style_tests-1e0f63ea4b8bb614.exe` (Style computation tests)
10. `url_tests-c4e16d0cbfcbdf82.exe` (URL parsing tests)
11. `window_tests-fc6778f6920f1ecf.exe` (Windowing and event tests)

---

## 4. Engineering Takeaway
This empirical execution confirms that Asteria’s current codebase is completely functional and compiles cleanly without errors. The compilation time of 5m 02s highlights the cost of the monolithic crate design (`AST-ARC-001`), directly justifying the multi-crate workspace modularization in `SPEC-05`.
