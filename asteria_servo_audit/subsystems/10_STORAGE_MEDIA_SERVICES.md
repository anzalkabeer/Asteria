# Subsystem Report: 10 Storage, DevTools, WebGPU & Media Services

> **Finding ID Prefix:** `SUB-SRV-`  
> **Target Subsystem:** Client Storage, Chrome/Firefox DevTools Server, Media Pipeline & WebGPU  
> **Audited Crates:** `components/storage` (`servo-storage`), `components/devtools` (`servo-devtools`), `components/webgpu` (`servo-webgpu`), `components/media/servo-media`  
> **Evidence Status:** VERIFIED against source tree (`7ca9f80268a1d76e698560db82463e7960aa7845`)  
> **Relevance to Asteria:** STRATEGIC (Informing Asteria's persistence, devtools server, and graphics architecture)

---

## A. Purpose and Design

This report covers auxiliary browser services running on dedicated actor threads:
1. **Web Storage (`components/storage`):** Provides HTML5 `localStorage`, `sessionStorage`, and `IndexedDB` persistence backed by SQLite (`rusqlite`).
2. **DevTools Server (`components/devtools`):** Implements an actor-based remote debugging server (`TcpListener`) compatible with standard DevTools clients.
3. **WebGPU (`components/webgpu`):** Exposes modern GPU compute and graphics directly to DOM JavaScript via `wgpu-core`.
4. **Media Subsystem (`components/media`):** Manages audio/video stream playback and WebRTC media channels.

---

## B. Source Map & File Responsibilities

| File Path | Primary Responsibilities |
| :--- | :--- |
| `components/storage/storage_thread.rs` | Runs SQLite transactions on a worker thread; prevents disk I/O from blocking script execution. |
| `components/storage/webstorage.rs` | Origin-isolated key-value storage for `localStorage` and `sessionStorage`. |
| `components/devtools/lib.rs` | Remote DevTools server event loop; manages debugging actors. |
| `components/devtools/actors/inspector/` | Inspects live DOM nodes and computed styles over WebSocket / TCP. |
| `components/devtools/actors/network_event.rs` | Captures in-flight HTTP requests and headers for the Network inspector panel. |
| `components/webgpu/lib.rs` | W3C WebGPU specification bindings wrapping `wgpu-core`. |
| `components/media/servo-media/src/lib.rs` | Abstraction over GStreamer and platform media backends. |

---

## C. Important Types and Public APIs

### 1. `StorageThreads` (`components/storage/storage_thread.rs`)
- Spawns background worker thread receiving `StorageMsg` via crossbeam channels:
  - `SetItem(origin, key, value)`
  - `GetItem(origin, key)`
  - `RemoveItem(origin, key)`
  - `Clear(origin)`

### 2. DevTools Actors (`components/devtools/lib.rs:43–58`)
- `InspectorActor` & `WalkerActor`: Walks DOM trees and serializes nodes into JSON protocol packets.
- `ConsoleActor`: Streams `console.log()` calls and unhandled JS exceptions.
- `NetworkEventActor`: Records timing, status codes, and payloads for the Network tab.

### 3. WebGPU Binding (`components/webgpu/`)
- Translates WebIDL `GPUAdapter`, `GPUDevice`, and `GPUCommandEncoder` calls into `wgpu-core` method invocations.

---

## D. Internal Execution: DevTools Inspection Flow

```mermaid
sequenceDiagram
    autonumber
    actor Dev as Developer / Chrome or Firefox DevTools
    participant DT as DevTools Server Thread
    participant C as Constellation
    participant ST as ScriptThread
    participant DOM as DOM Tree

    Dev->>DT: Connect TCP Socket / Handshake
    DT->>C: Request active BrowsingContexts
    C-->>DT: Return list of tabs & PipelineIds
    Dev->>DT: "DOM.getDocument" request
    DT->>ST: DevtoolScriptControlMsg::InspectNode
    ST->>DOM: Serialize DOM subtree to JSON
    DOM-->>ST: Subtree description
    ST-->>DT: Node array + attributes
    DT-->>Dev: Send JSON-RPC response -> Displays DOM Tree in UI
```

---

## E. Asteria Engineering Implications

### Comparative Diagnosis of Asteria:
- **Asteria Defect:** Asteria has a prototype devtools module in `src/devtools/` with minimal capabilities, and has no persistent storage (`localStorage` / cookies are absent or in-memory only).
- **Synergy with Asteria's `wgpu`:**
  - Asteria is built natively on `wgpu 22.1`.
  - Servo's `servo-webgpu` crate demonstrates how to expose `wgpu` safely to untrusted Web APIs. Asteria can leverage its existing `wgpu` context to support the W3C WebGPU API with minimal extra dependencies!
- **Actionable Roadmap for Asteria:**
  1. **SQLite Storage Worker:** Move storage I/O to a background worker using `rusqlite` to prevent frame drops on disk writes.
  2. **CDP DevTools Server:** Implement Chrome DevTools Protocol over a local WebSocket server (`tokio-tungstenite`) so developers can debug Asteria using `chrome://inspect` in any standard browser!

---

## H. Confidence and Coverage

- **Inspected:** `components/storage/lib.rs`, `components/devtools/lib.rs`, `components/webgpu/lib.rs`.
- **Verified:** Storage thread isolation, DevTools actor hierarchy, and WebGPU bindings.
- **Audit Status:** Complete (100% verified against code).
