# Asteria Commands Reference Guide

Welcome to the Asteria Browser Engine! Here is a handy reference guide for the various commands you can use to run, inspect, and test the project.

## Running the Browser Engine

### 1. Window Mode (GUI)
```bash
cargo run -- "<sample>" --window
```
**What it does**: This is the primary way to launch Asteria. It spawns a native OS window powered by `winit` and uses hardware-accelerated GPU rendering via `wgpu` to draw the webpage.
**What gets affected**: An actual application window opens. It dynamically builds the DOM, resolves CSS, performs block layout, and paints everything to the screen.

### 2. CLI Mode (Headless Diagnostics)
```bash
cargo run -- "<sample>" --cli
```
**What it does**: Runs Asteria entirely in the terminal without opening a GUI window.
**What gets affected**: Prints deep diagnostic information to the terminal, including:
- The tokenized HTML DOM Tree
- The resolved CSS rules
- The computed Styled DOM Tree
- 2D layout bounding boxes
- The Display List (visual draw commands)
- Performance metrics, timing, and energy diagnostics.
*(A `trace.json` file is also exported to the directory for Chrome trace profiling).*

*Note: You can replace `"<sample>"` with an actual path to a local HTML file or a web URL (e.g., `"https://example.com"`).*

---

## Testing the Engine

### 1. Run All Tests
```bash
cargo test
```
**What it does**: Runs the entire suite of 210+ unit tests and all integration tests.
**When to use**: After making any changes to the codebase to ensure nothing was broken.

### 2. Live Wikipedia Network Test
```bash
cargo test --test network_integration
```
**What it does**: Specifically tests the `StreamingHtmlProcessor` by performing a live HTTP fetch to a Wikipedia article. It tokenizes, parses, and builds the DOM chunk-by-chunk on a background thread.
**When to use**: To verify that the network layer and asynchronous streaming architecture are functioning correctly.

### 3. Edge Case Parser Tests
```bash
cargo test --test comprehensive_edge_cases
```
**What it does**: Tests the streaming parser against tricky HTML syntax like unquoted attributes, self-closing tags, and missing structural tags.
**When to use**: When you make changes to the `tokenizer.rs` or `parser.rs`.

### 4. Layout & Paint Integration Tests
```bash
cargo test --test layout_integration
cargo test --test paint_integration
```
**What it does**: Validates that inline flows, block widths, and coordinate accumulation work correctly, and that display lists emit the proper 2D render commands.

---

## Utility Commands

### Code Compilation Check
```bash
cargo check
```
**What it does**: Quickly verifies that your Rust code compiles without actually building the heavy binary or running it. Highly recommended to run frequently while writing code.
