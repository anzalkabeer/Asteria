# SPEC-12: Local File Scheme Jailing & Origin Security Policy

- **Related Issue:** [`../issues/AST-SEC-001.md`](../issues/AST-SEC-001.md), [`../issues/AST-SEC-002.md`](../issues/AST-SEC-002.md)
- **Target Subsystem:** Security & Resource Loader (`src/net/fetch.rs`)
- **Stage:** Stage 2 (P1)

---

## 1. Problem Statement
In `src/net/fetch.rs:145-180`, requesting `file://` URLs allows unconfined reads of arbitrary filesystem paths without directory jailing, allowing malicious documents to read sensitive local files.

---

## 2. Root Cause
The resource fetcher lacks security sandboxing and path canonicalization.

---

## 3. Recommended Solution
1. Implement canonicalized directory jailing in `src/net/fetch.rs`:
```rust
pub fn resolve_jailed_file_path(base_dir: &Path, requested_path: &Path) -> Result<PathBuf, SecurityError> {
    let canonical_base = base_dir.canonicalize().map_err(|_| SecurityError::AccessDenied)?;
    let resolved = base_dir.join(requested_path);
    let canonical_target = resolved.canonicalize().map_err(|_| SecurityError::AccessDenied)?;

    if !canonical_target.starts_with(&canonical_base) {
        return Err(SecurityError::DirectoryTraversalBlocked);
    }
    Ok(canonical_target)
}
```
2. Forbid web-loaded documents (`http://` or `https://` origins) from loading `file://` subresources.

---

## 4. Affected Source Locations
- `src/net/fetch.rs`

---

## 5. Verification & Acceptance Criteria
1. `test_file_scheme_jail_traversal_rejection`: Path `file:///jail/../../../etc/passwd` returns `SecurityError::DirectoryTraversalBlocked`.
2. Access within the designated document jail succeeds normally.
