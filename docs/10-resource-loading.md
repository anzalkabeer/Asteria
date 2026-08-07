# Resource Loading

> **Purpose:** Explain how Asteria discovers, fetches, and manages the resources needed to render a page.
>
> **Audience:** Contributors and developers interested in networking and I/O.
>
> **Estimated reading time:** 10 minutes
>
> **Prerequisites:** [How Asteria Works](02-how-asteria-works.md)

---

## What is resource loading?

A web page is rarely just one file. An HTML document typically references external stylesheets, images, fonts, and scripts. The resource loader's job is to discover these references, fetch the actual bytes, and make them available to the rest of the pipeline.

---

## The resource loader

**Source file:** `src/loader.rs`

The `ResourceLoader` is Asteria's fetch layer. Given a URL or file path, it:

1. Loads the HTML document (from disk or via HTTP/HTTPS)
2. Parses it into a temporary DOM to discover linked resources
3. Finds `<style>` tags (inline CSS) and `<link rel="stylesheet">` tags (external CSS)
4. Resolves relative paths against the document's base directory or URL
5. Loads each discovered stylesheet
6. Packages everything into a `PageResources` bundle

### PageResources

```rust
pub struct PageResources {
    pub html: Resource,              // The root HTML document
    pub stylesheets: Vec<Resource>,  // All CSS (inline + external), in document order
}

pub struct Resource {
    pub url: String,           // Path or URL this was loaded from
    pub resource_type: ResourceType,  // Html or Css
    pub bytes: Vec<u8>,        // Raw content bytes
}
```

This bundle is what the engine pipeline consumes. The HTML bytes go to the tokenizer/parser. The stylesheet bytes go to the CSS tokenizer/parser.

### Resource discovery

The loader walks the temporary DOM looking for:

| Element | Attribute | Discovery |
|---|---|---|
| `<style>` | *(text content)* | Inline CSS — extract the text between `<style>` and `</style>` |
| `<link>` | `rel="stylesheet" href="..."` | External CSS — resolve the `href` and load the file |

Inline stylesheets get synthetic URLs like `<inline:0>`, `<inline:1>` to distinguish them in the cache.

---

## Resource cache

**Source file:** `src/loader.rs` (ResourceCache), `src/cache.rs` (LruCache)

The `ResourceCache` ensures the same file is never loaded twice. It maps canonical paths/URLs to their loaded `Resource`:

```
Cache lookup:
  "styles/main.css" → Cache hit → return cached bytes
  "styles/theme.css" → Cache miss → load from disk → cache → return
```

For longer-lived caching, Asteria also has an `LruCache` (least-recently-used eviction cache) that bounds memory usage by evicting the oldest entries when the cache reaches capacity.

---

## Networking stack

**Source directory:** `src/net/`

For HTTP and HTTPS URLs, Asteria has a custom networking stack — no third-party HTTP crate.

### DNS resolver

**Source file:** `src/net/dns.rs`

Translates domain names to IP addresses with an in-memory TTL cache:

```
Lookup "example.com"
  → Cache check → miss
  → System DNS query → 93.184.216.34 (TTL: 300s)
  → Cache store
  → Return IP

Lookup "example.com" (again, within 300s)
  → Cache check → hit
  → Return 93.184.216.34 (no network round-trip)
```

### TCP connections

**Source file:** `src/net/tcp.rs`

The `TcpConnection` wraps `std::net::TcpStream` with a connection pool (`ConnectionPool`) that reuses connections to the same host:port. The `Stream` abstraction unifies plain TCP and TLS-wrapped connections behind a common interface.

### TLS

**Source file:** `src/net/tls.rs`

For HTTPS connections, `TlsConnector` wraps the TCP stream in a TLS session using [rustls](https://github.com/rustls/rustls) with Mozilla's root certificates from [webpki-roots](https://crates.io/crates/webpki-roots).

### HTTP client

**Source file:** `src/net/http.rs`

`HttpClient` implements HTTP/1.1 GET requests:

1. Parse the URL into scheme, host, port, and path
2. Resolve the host via DNS
3. Open a TCP connection (reusing from pool if available)
4. Upgrade to TLS if HTTPS
5. Send the HTTP request headers
6. Read the response headers and body
7. Follow redirects (3xx status codes) up to a limit

The `Url` struct handles URL parsing with support for `http://` and `https://` schemes, explicit ports, and path resolution.

### Streaming resource bus

**Source file:** `src/net/bus.rs`

`StreamingResourceBus` is an MPSC (multi-producer, single-consumer) channel for delivering resource data asynchronously. Network fetches can push chunks to the bus as they arrive, and the engine can consume them progressively:

```
Network thread          │         Engine thread
                        │
HttpClient.get()        │
  → chunk 1 ──────────►│──────► StreamingHtmlProcessor
  → chunk 2 ──────────►│──────► (incremental DOM growth)
  → chunk 3 (EOF) ────►│──────► Parser finalised
```

---

## Image handling

**Source file:** `src/image.rs`

Asteria detects image formats by inspecting magic bytes at the start of the file:

| Format | Magic bytes |
|---|---|
| PNG | `89 50 4E 47 0D 0A 1A 0A` |
| JPEG | `FF D8 FF` |
| BMP | `42 4D` ("BM") |
| GIF | `47 49 46` ("GIF") |
| WebP | `52 49 46 46 ... 57 45 42 50` ("RIFF...WEBP") |
| TIFF | `49 49 2A 00` or `4D 4D 00 2A` |
| SVG | Contains `<svg` in first 1024 bytes |

The `ImageDecoder` manages decoded image data and provides an LRU cache for decoded results, preventing repeated decoding of the same image.

---

## Loading flow

Here's the complete flow from URL to renderable resources:

```
User requests "https://example.com/page.html"
  │
  ▼
ResourceLoader.load_url("https://example.com/page.html")
  │
  ├── DNS: resolve "example.com" → IP address
  ├── TCP: connect to IP:443
  ├── TLS: handshake (rustls)
  ├── HTTP: GET /page.html → 200 OK + HTML bytes
  │
  ├── Quick-parse HTML to discover resources
  │     ├── Found: <style>body { ... }</style>
  │     └── Found: <link rel="stylesheet" href="style.css">
  │
  ├── Load "style.css" (HTTP GET → CSS bytes)
  │
  └── Return PageResources {
        html: Resource { bytes: [...] },
        stylesheets: [
          Resource { url: "<inline:0>", bytes: [...] },
          Resource { url: "style.css", bytes: [...] },
        ]
      }
```

---

## Current limitations

| Feature | Status |
|---|---|
| HTTP/2 | 🔜 |
| WebSockets | 🔜 |
| Font loading and rendering | ✅ (via glyphon, not custom-loaded fonts yet) |
| `@import` CSS loading | 🔜 |
| Image loading from network | ✅ (format detection, decode pipeline) |
| Parallel resource fetching | 🔜 (currently sequential) |
| Service workers | 🔜 |
| Cache headers (Cache-Control, ETag) | 🔜 |

---

## Related documents

- [How Asteria Works](02-how-asteria-works.md) — where resource loading fits in the pipeline
- [HTML Engine](04-html-engine.md) — streaming parser integration
- [Browser Shell](11-browser-shell.md) — navigation triggers resource loading
- [Performance](12-performance.md) — caching strategy
- [Design Decisions](18-design-decisions.md) — why a custom networking stack
