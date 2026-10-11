# ISSUE: AST-SEC-001 — Unrestricted Local File URI Reading

- **Classification:** SECURITY CONCERN
- **Severity:** High
- **Priority:** P1
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Networking & Resource Loader
- **Specification Link:** [`../specifications/SPEC-12.md`](../specifications/SPEC-12.md)

---

## 1. Problem Statement
In `src/net/fetch.rs:145-180`, requesting a `file://` scheme URL causes Asteria to read arbitrary local files from the filesystem using standard `std::fs::read` without path canonicalization, directory jailing, or user permissions. A malicious webpage could embed `<img src="file:///etc/passwd">` or reference local sensitive files.

---

## 2. Evidence & Source Location
- **Location:** [`src/net/fetch.rs:145-180`](../../../src/net/fetch.rs#L145-L180)
```rust
"file" => {
    let path = url.path;
    let data = std::fs::read(path)?; // Unrestricted local disk read
    Ok(Response::ok(data))
}
```

---

## 3. Root Cause
The resource fetcher was implemented for local developer testing without security confinement boundaries.

---

## 4. Desired Behaviour
Implement directory jailing: reject relative directory traversal (`..`), enforce a strict base root for local files, and block web-loaded documents (`http://` origins) from requesting `file://` subresources.
