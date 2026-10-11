# ISSUE: AST-SEC-002 — Missing Content Security Policy & Origin Isolation

- **Classification:** SECURITY CONCERN
- **Severity:** Medium
- **Priority:** P2
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Web Security & HTML Parser
- **Specification Link:** [`../specifications/README.md`](../specifications/README.md)

---

## 1. Problem Statement
Asteria has no concept of Content Security Policy (CSP), Same-Origin Policy (SOP), or Cross-Origin Resource Sharing (CORS). External stylesheets, images, and network fetches from arbitrary origins are parsed and loaded without checking security headers.

---

## 2. Evidence & Source Location
- **Location:** [`src/html.rs:40-110`](../../../src/html.rs#L40-L110) & [`src/net/fetch.rs:80-105`](../../../src/net/fetch.rs#L80-L105)
Zero verification of response headers (`Content-Security-Policy`, `Access-Control-Allow-Origin`).

---

## 3. Root Cause
Asteria has not yet implemented modern web security origin tracking.

---

## 4. Desired Behaviour
Parse `Content-Security-Policy` HTTP headers and `<meta http-equiv="Content-Security-Policy">` tags, enforcing script, style, and image origin allowlists.
