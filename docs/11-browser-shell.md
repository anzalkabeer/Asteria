# Browser Shell

> **Purpose:** Describe the browser application layer — tabs, windows, navigation, and the rendering loop.
>
> **Audience:** Contributors and anyone interested in browser UI architecture.
>
> **Estimated reading time:** 8 minutes
>
> **Prerequisites:** [How Asteria Works](02-how-asteria-works.md)

---

## Overview

The browser shell is the layer between the user and the rendering engine. While the engine understands how to turn HTML/CSS into pixels, the shell manages the *browser experience* — tabs, navigation, keyboard shortcuts, scrolling, and window management.

**Source file:** `src/shell.rs` (tab management), `src/renderer/window/window.rs` (window and event loop)

---

## Tab management

### TabManager

The `TabManager` is the top-level browser state manager. It holds all open tabs and tracks which one is active:

```
TabManager
  ├── Tab 0 (active)
  │     ├── URL: "https://example.com"
  │     ├── Title: "Example Page"
  │     ├── NavigationHistory
  │     ├── PageResources (HTML + CSS)
  │     ├── DOM
  │     └── Stylesheet
  ├── Tab 1
  │     ├── URL: "file:///blog.html"
  │     └── ...
  └── Tab 2
        ├── URL: "about:blank"
        └── ...
```

### Tab

Each `Tab` stores its complete engine state:

```rust
pub struct Tab {
    pub id: TabId,                      // Unique 64-bit identifier
    pub url: String,                    // Current URL
    pub title: String,                  // Page title
    pub history: NavigationHistory,     // Back/forward stack
    pub page_resources: Option<PageResources>,  // Loaded HTML + CSS
    pub dom: Option<Dom>,              // Parsed DOM tree
    pub stylesheet: Option<Stylesheet>, // Parsed CSS
}
```

When you switch tabs, the engine switches to that tab's DOM, stylesheet, and rendering state.

### Navigation history

Each tab maintains its own independent history stack:

```
History: [page_A, page_B, page_C]
                          ↑
                    current_index = 2

Go back → current_index = 1 → reload page_B
Go forward → current_index = 2 → reload page_C
Navigate to page_D → truncate forward, push → [page_A, page_B, page_C, page_D]
```

---

## Shell events

The browser shell communicates through a `ShellEvent` system:

| Event | Action |
|---|---|
| `NavigateTo(url)` | Load a new URL in the active tab |
| `NewTab` | Create a new blank tab |
| `CloseTab(id)` | Close a specific tab |
| `SwitchTab(id)` | Make a different tab active |
| `GoBack` | Navigate back in the active tab's history |
| `GoForward` | Navigate forward in the active tab's history |
| `Reload` | Re-fetch and re-render the current page |

---

## Keyboard shortcuts

The window event loop translates keyboard input into shell events:

| Shortcut | Action |
|---|---|
| `Ctrl+T` | Open a new tab |
| `Ctrl+W` | Close the current tab |
| `Alt+←` (Left Arrow) | Navigate back |
| `Alt+→` (Right Arrow) | Navigate forward |
| `Ctrl+R` | Reload the current page |
| `F5` | Reload the current page |

Modifier state (Ctrl, Alt, Shift) is tracked continuously so that key combinations are detected correctly.

---

## The rendering loop

The heart of the browser is the winit event loop in `AsteriaWindow`. It runs for the lifetime of the application:

```
┌─────────────────────────────────────────────────────┐
│                    Event Loop                       │
│                                                     │
│  ┌──────────────────────────────────────────────┐   │
│  │ WindowEvent::Resized(width, height)          │   │
│  │  → Resize GPU surface                        │   │
│  │  → Re-run layout with new dimensions         │   │
│  │  → Rebuild scene graph                       │   │
│  │  → Request redraw                            │   │
│  └──────────────────────────────────────────────┘   │
│                                                     │
│  ┌──────────────────────────────────────────────┐   │
│  │ WindowEvent::CursorMoved(x, y)              │   │
│  │  → Hit-test against scene nodes              │   │
│  │  → Update hovered node state                 │   │
│  │  → Update cursor style                       │   │
│  │  → Request redraw if state changed           │   │
│  └──────────────────────────────────────────────┘   │
│                                                     │
│  ┌──────────────────────────────────────────────┐   │
│  │ WindowEvent::MouseInput(Pressed)             │   │
│  │  → Hit-test for clicked element              │   │
│  │  → If link → navigate to URL                 │   │
│  │  → Update active node state                  │   │
│  └──────────────────────────────────────────────┘   │
│                                                     │
│  ┌──────────────────────────────────────────────┐   │
│  │ WindowEvent::MouseWheel(delta)               │   │
│  │  → Update scroll_offset                      │   │
│  │  → Rebuild scene batches with offset         │   │
│  │  → Request redraw                            │   │
│  └──────────────────────────────────────────────┘   │
│                                                     │
│  ┌──────────────────────────────────────────────┐   │
│  │ WindowEvent::KeyboardInput(key, modifiers)   │   │
│  │  → Match against shortcuts table             │   │
│  │  → Dispatch shell event                      │   │
│  └──────────────────────────────────────────────┘   │
│                                                     │
│  ┌──────────────────────────────────────────────┐   │
│  │ WindowEvent::RedrawRequested                 │   │
│  │  → Prepare render passes                     │   │
│  │  → Execute GPU rendering                     │   │
│  │  → Present frame to screen                   │   │
│  └──────────────────────────────────────────────┘   │
│                                                     │
└─────────────────────────────────────────────────────┘
```

---

## Scrolling

When content extends beyond the viewport, the user can scroll to reveal more content. Asteria tracks a vertical `scroll_offset` value that shifts the rendered scene:

```
Document height: 2000px
Viewport height:  600px
Scroll offset:    400px

Visible region: y=400 to y=1000
```

The scroll offset is applied when building GPU vertex buffers — each element's y-coordinate is adjusted by subtracting the scroll offset. Elements above or below the visible region are still in the scene graph but produce vertices outside the viewport, which the GPU clips automatically.

---

## Navigation flow

When a user navigates to a new URL (by clicking a link or entering a URL):

```
User action → ShellEvent::NavigateTo(url)
  │
  ▼
TabManager.handle_event()
  │
  ├── Update navigation history
  ├── Resource loader fetches HTML + CSS
  ├── Parse HTML → DOM
  ├── Parse CSS → Stylesheet
  ├── Store in Tab
  │
  ▼
Window event loop
  │
  ├── Re-run style resolution
  ├── Re-run layout
  ├── Rebuild paint → scene graph
  ├── Rebuild GPU batches
  └── Render new page
```

---

## Link click handling

When the user clicks on a link (`<a href="...">`):

1. **Hit test** — find the scene node under the cursor
2. **Check for link_url** — if the node has a `link_url`, extract it
3. **Navigate** — dispatch `ShellEvent::NavigateTo(url)` with the link's URL
4. **Re-render** — the full pipeline runs for the new page

This is why the paint engine propagates `link_url` through display commands and scene nodes — it enables interactive navigation.

---

## Window configuration

The `AsteriaWindow` creates a native OS window via winit:

```rust
WindowBuilder::new()
    .with_title("Asteria Engine Browser Shell")
    .with_inner_size(LogicalSize::new(800, 600))
    .build(event_loop)
```

The window title, initial size, and behaviour are all configurable. The event loop integrates with the OS event system, so the browser responds naturally to system events (resize, focus, close).

---

## Current limitations

| Feature | Status |
|---|---|
| Address bar UI | 🔜 |
| Bookmarks | 🔜 |
| Settings panel | 🔜 |
| Tab bar UI | 🔜 (tabs managed via keyboard only) |
| Context menu (right-click) | 🔜 |
| Find in page | 🔜 |
| Download manager | 🔜 |
| Devtools panel | 🔜 |

---

## Related documents

- [GPU Renderer](09-gpu-renderer.md) — the rendering backend
- [Resource Loading](10-resource-loading.md) — how pages are fetched
- [Performance](12-performance.md) — event loop optimisation
- [How Asteria Works](02-how-asteria-works.md) — the overall pipeline
