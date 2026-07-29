use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::dom::{Dom, NodeId, NodeKind};

// ─── Resource Loader & Cache ─────────────────────────────────────
//
// The resource loader is Asteria's "fetch layer". It knows how to:
//
//   1. Load an HTML file from disk
//   2. Walk the parsed DOM to discover linked stylesheets
//      - `<style>` tags (inline CSS)
//      - `<link rel="stylesheet" href="...">` tags (external CSS files)
//   3. Resolve relative paths against the base document directory
//   4. Cache resources so the same file is never read twice
//   5. Package everything into a `PageResources` bundle for the pipeline
//
// This is a synchronous, local-files-only loader for now.
// When networking lands (reqwest), it will gain async HTTP fetching
// behind the same `Resource`/`PageResources` interface.
//
// The flow:
//   1. User calls `loader.load_file("index.html")`
//   2. Loader reads the HTML bytes, parses them into a DOM
//   3. Walks the DOM for `<style>` and `<link rel="stylesheet">`
//   4. Reads any linked `.css` files from disk (relative to base dir)
//   5. Returns `PageResources { html, stylesheets }`

// ─── Resource Types ──────────────────────────────────────────────

/// What kind of resource this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Html,
    Css,
}

/// A loaded resource — the raw bytes of a file plus metadata.
#[derive(Debug, Clone)]
pub struct Resource {
    /// The path or URL this resource was loaded from (canonical key)
    pub url: String,
    /// Whether this is HTML or CSS
    pub resource_type: ResourceType,
    /// The raw content bytes
    pub bytes: Vec<u8>,
}

/// Everything needed to render a page — the HTML document
/// and all its stylesheets (both inline and external).
#[derive(Debug)]
pub struct PageResources {
    /// The root HTML document
    pub html: Resource,
    /// All stylesheets, in document order:
    /// - Inline `<style>` blocks (url = "<inline:N>")
    /// - External `<link rel="stylesheet">` files (url = resolved path)
    pub stylesheets: Vec<Resource>,
}

// ─── Resource Cache ──────────────────────────────────────────────

/// Simple in-memory cache keyed by canonical path / URL string.
/// Prevents re-reading the same file twice in a session.
#[derive(Debug)]
pub struct ResourceCache {
    entries: HashMap<String, Resource>,
}

impl ResourceCache {
    pub fn new() -> Self {
        ResourceCache {
            entries: HashMap::new(),
        }
    }

    /// Check if a resource is already cached.
    pub fn contains(&self, url: &str) -> bool {
        self.entries.contains_key(url)
    }

    /// Retrieve a cached resource (cloned).
    pub fn get(&self, url: &str) -> Option<Resource> {
        self.entries.get(url).cloned()
    }

    /// Insert a resource into the cache.
    pub fn insert(&mut self, resource: Resource) {
        self.entries.insert(resource.url.clone(), resource);
    }

    /// How many resources are cached.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is the cache empty?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all cached entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

// ─── Resource Loader ─────────────────────────────────────────────

/// The main resource loader. Reads files from disk, discovers linked
/// stylesheets from the DOM, and caches everything for reuse.
pub struct ResourceLoader {
    /// In-memory cache to avoid re-reading files
    pub cache: ResourceCache,
}

impl ResourceLoader {
    /// Create a new ResourceLoader with an empty cache.
    pub fn new() -> Self {
        ResourceLoader {
            cache: ResourceCache::new(),
        }
    }

    /// Load an HTML file from disk and discover all its stylesheets.
    ///
    /// This is the main entry point for file-based loading:
    /// 1. Read the HTML file
    /// 2. Tokenize + parse into a DOM
    /// 3. Walk the DOM for `<style>` and `<link rel="stylesheet">`
    /// 4. Read any external CSS files
    /// 5. Return a `PageResources` bundle
    ///
    /// The DOM is parsed internally just for resource discovery —
    /// the caller will parse it again through the full pipeline.
    /// (This is intentional: the loader is infrastructure, not the engine.)
    pub fn load_file(&mut self, path: &str) -> Result<PageResources, LoadError> {
        let canonical = canonicalize_path(path)?;
        let base_dir = Path::new(&canonical).parent().map(|p| p.to_path_buf());

        // Load the HTML resource
        let html = self.read_resource(&canonical, ResourceType::Html)?;

        // Parse the HTML into a DOM for resource discovery
        let mut tokenizer = crate::tokenizer::Tokenizer::new(&html.bytes);
        let tokens = tokenizer.tokenize();
        let parser = crate::parser::Parser::new(&tokens, &html.bytes);
        let dom = parser.parse();

        // Discover stylesheets
        let stylesheets = self.discover_stylesheets(&dom, &html.bytes, base_dir.as_deref())?;

        Ok(PageResources { html, stylesheets })
    }

    /// Load from an in-memory HTML string (for testing or built-in samples).
    ///
    /// `base_url` is used as the cache key only, not for resolving relative
    /// resource paths, since in-memory documents have no base directory.
    /// For in-memory strings, use something like `"<sample>"`.
    pub fn load_html_string(&mut self, html: &str, base_url: &str) -> PageResources {
        let html_resource = Resource {
            url: base_url.to_string(),
            resource_type: ResourceType::Html,
            bytes: html.as_bytes().to_vec(),
        };

        self.cache.insert(html_resource.clone());

        // Parse for resource discovery
        let mut tokenizer = crate::tokenizer::Tokenizer::new(&html_resource.bytes);
        let tokens = tokenizer.tokenize();
        let parser = crate::parser::Parser::new(&tokens, &html_resource.bytes);
        let dom = parser.parse();

        // Discover stylesheets (no base_dir for in-memory strings)
        let stylesheets = self
            .discover_stylesheets(&dom, &html_resource.bytes, None)
            .unwrap_or_default();

        PageResources {
            html: html_resource,
            stylesheets,
        }
    }

    /// Read a file from disk (or cache) and return it as a Resource.
    fn read_resource(
        &mut self,
        canonical_path: &str,
        resource_type: ResourceType,
    ) -> Result<Resource, LoadError> {
        // Check cache first
        if let Some(cached) = self.cache.get(canonical_path) {
            return Ok(cached);
        }

        // Read from disk
        let bytes = fs::read(canonical_path).map_err(|err| LoadError::IoError {
            path: canonical_path.to_string(),
            message: err.to_string(),
        })?;

        let resource = Resource {
            url: canonical_path.to_string(),
            resource_type,
            bytes,
        };

        self.cache.insert(resource.clone());
        Ok(resource)
    }

    /// Walk a parsed DOM to discover all stylesheets (inline + external).
    /// Returns them in document order using an explicit worklist stack.
    fn discover_stylesheets(
        &mut self,
        dom: &Dom,
        source: &[u8],
        base_dir: Option<&Path>,
    ) -> Result<Vec<Resource>, LoadError> {
        let mut stylesheets = Vec::new();
        let mut inline_counter = 0u32;

        // Worklist stack initialized with root node
        let mut worklist = vec![dom.root()];

        while let Some(node_id) = worklist.pop() {
            let handled = self.process_stylesheet_node(
                dom,
                node_id,
                source,
                base_dir,
                &mut stylesheets,
                &mut inline_counter,
            )?;

            // If this node wasn't a handled stylesheet node (<style> or <link rel="stylesheet">),
            // enqueue its children in reverse order so popping yields document order (left-to-right).
            if !handled {
                let node = dom.get(node_id);
                for &child_id in node.children.iter().rev() {
                    worklist.push(child_id);
                }
            }
        }

        Ok(stylesheets)
    }

    /// Process a single DOM node for stylesheet content (<style> or <link rel="stylesheet">).
    /// Returns Ok(true) if the node was a handled stylesheet (so traversal should stop for its children),
    /// or Ok(false) if it was not.
    fn process_stylesheet_node(
        &mut self,
        dom: &Dom,
        node_id: NodeId,
        source: &[u8],
        base_dir: Option<&Path>,
        stylesheets: &mut Vec<Resource>,
        inline_counter: &mut u32,
    ) -> Result<bool, LoadError> {
        let node = dom.get(node_id);

        if let NodeKind::Element { tag_start, tag_end } = &node.kind {
            let tag_name =
                std::str::from_utf8(&source[*tag_start as usize..*tag_end as usize]).unwrap_or("");

            // ─── <style> → inline CSS ────────────────────────
            if tag_name.eq_ignore_ascii_case("style") {
                let mut css_text = String::new();
                for &child_id in &node.children {
                    let child = dom.get(child_id);
                    if let NodeKind::Text { start, end } = &child.kind {
                        let text = std::str::from_utf8(&source[*start as usize..*end as usize])
                            .unwrap_or("");
                        css_text.push_str(text);
                    }
                }

                if !css_text.is_empty() {
                    let url = format!("<inline:{}>", inline_counter);
                    *inline_counter += 1;
                    stylesheets.push(Resource {
                        url,
                        resource_type: ResourceType::Css,
                        bytes: css_text.into_bytes(),
                    });
                }

                return Ok(true); // Handled stylesheet node
            }

            // ─── <link rel="stylesheet" href="..."> → external CSS ──
            if tag_name.eq_ignore_ascii_case("link") {
                let mut is_stylesheet = false;
                let mut href: Option<String> = None;

                for &(ns, ne, vs, ve) in &node.attributes {
                    let attr_name =
                        std::str::from_utf8(&source[ns as usize..ne as usize]).unwrap_or("");

                    if attr_name.eq_ignore_ascii_case("rel") && vs != 0 && ve != 0 {
                        let attr_value =
                            std::str::from_utf8(&source[vs as usize..ve as usize]).unwrap_or("");
                        if attr_value.eq_ignore_ascii_case("stylesheet") {
                            is_stylesheet = true;
                        }
                    }

                    if attr_name.eq_ignore_ascii_case("href") && vs != 0 && ve != 0 {
                        let attr_value =
                            std::str::from_utf8(&source[vs as usize..ve as usize]).unwrap_or("");
                        if !attr_value.is_empty() {
                            href = Some(attr_value.to_string());
                        }
                    }
                }

                if is_stylesheet {
                    if let Some(href_value) = href {
                        let resolved = resolve_path(&href_value, base_dir);
                        match self.read_resource(&resolved, ResourceType::Css) {
                            Ok(resource) => stylesheets.push(resource),
                            Err(err) => {
                                eprintln!(
                                    "Warning: Could not load stylesheet '{}': {}",
                                    href_value, err
                                );
                            }
                        }
                    }
                    return Ok(true); // Handled stylesheet node
                }
            }
        }

        Ok(false)
    }
}

// ─── Path Resolution ─────────────────────────────────────────────

/// Resolve a potentially relative path against a base directory.
///
/// Examples:
///   resolve_path("style.css", Some("/home/user/site/"))
///     → "/home/user/site/style.css"
///   resolve_path("css/main.css", Some("/home/user/site/"))
///     → "/home/user/site/css/main.css"
///   resolve_path("/absolute/path.css", Some("/home/user/site/"))
///     → "/absolute/path.css"
///   resolve_path("style.css", None)
///     → "style.css"
fn resolve_path(href: &str, base_dir: Option<&Path>) -> String {
    let href_path = Path::new(href);

    // If it's already absolute, use it directly
    if href_path.is_absolute() {
        return normalize_path_string(href);
    }

    // If we have a base directory, join relative to it
    if let Some(base) = base_dir {
        let joined = base.join(href);
        return normalize_path_string(&joined.to_string_lossy());
    }

    // No base dir — return as-is
    href.to_string()
}

/// Try to canonicalize a path, returning a normalized canonical path string on success
/// or `LoadError::IoError` on canonicalization failure.
fn canonicalize_path(path: &str) -> Result<String, LoadError> {
    match fs::canonicalize(path) {
        Ok(canonical) => Ok(normalize_path_string(&canonical.to_string_lossy())),
        Err(err) => Err(LoadError::IoError {
            path: path.to_string(),
            message: err.to_string(),
        }),
    }
}

/// Normalize a path string. On Windows, converts backslashes to forward slashes
/// and strips the extended-length path prefix (`\\?\` or `//?/`).
/// On non-Windows platforms, preserves the path unchanged.
fn normalize_path_string(path: &str) -> String {
    if cfg!(windows) {
        let normalized = path.replace('\\', "/");
        if normalized.starts_with("//?/") {
            normalized[4..].to_string()
        } else {
            normalized
        }
    } else {
        path.to_string()
    }
}

// ─── Errors ──────────────────────────────────────────────────────

/// Errors that can occur during resource loading.
#[derive(Debug)]
pub enum LoadError {
    /// Failed to read a file from disk
    IoError { path: String, message: String },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::IoError { path, message } => {
                write!(f, "Failed to read '{}': {}", path, message)
            }
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ResourceCache ────────────────────────────────────────────

    #[test]
    fn test_cache_empty() {
        let cache = ResourceCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert!(!cache.contains("anything"));
        assert!(cache.get("anything").is_none());
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = ResourceCache::new();
        let resource = Resource {
            url: "test.css".to_string(),
            resource_type: ResourceType::Css,
            bytes: b"body { color: red; }".to_vec(),
        };

        cache.insert(resource.clone());

        assert!(cache.contains("test.css"));
        assert_eq!(cache.len(), 1);

        let retrieved = cache.get("test.css").unwrap();
        assert_eq!(retrieved.url, "test.css");
        assert_eq!(retrieved.resource_type, ResourceType::Css);
        assert_eq!(retrieved.bytes, b"body { color: red; }");
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = ResourceCache::new();
        cache.insert(Resource {
            url: "a.css".to_string(),
            resource_type: ResourceType::Css,
            bytes: vec![],
        });

        assert!(cache.get("b.css").is_none());
        assert!(!cache.contains("b.css"));
    }

    #[test]
    fn test_cache_overwrite() {
        let mut cache = ResourceCache::new();
        cache.insert(Resource {
            url: "x.css".to_string(),
            resource_type: ResourceType::Css,
            bytes: b"old".to_vec(),
        });
        cache.insert(Resource {
            url: "x.css".to_string(),
            resource_type: ResourceType::Css,
            bytes: b"new".to_vec(),
        });

        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get("x.css").unwrap().bytes, b"new");
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = ResourceCache::new();
        cache.insert(Resource {
            url: "a.css".to_string(),
            resource_type: ResourceType::Css,
            bytes: vec![],
        });
        cache.insert(Resource {
            url: "b.css".to_string(),
            resource_type: ResourceType::Css,
            bytes: vec![],
        });

        assert_eq!(cache.len(), 2);
        cache.clear();
        assert!(cache.is_empty());
    }

    // ── Path Resolution ──────────────────────────────────────────

    #[test]
    fn test_resolve_relative_path() {
        let base = Path::new("/home/user/site");
        let result = resolve_path("style.css", Some(base));
        assert_eq!(result, "/home/user/site/style.css");
    }

    #[test]
    fn test_resolve_nested_relative_path() {
        let base = Path::new("/home/user/site");
        let result = resolve_path("css/main.css", Some(base));
        assert_eq!(result, "/home/user/site/css/main.css");
    }

    #[test]
    fn test_resolve_no_base_dir() {
        let result = resolve_path("style.css", None);
        assert_eq!(result, "style.css");
    }

    // ── load_html_string ─────────────────────────────────────────

    #[test]
    fn test_load_html_string_no_css() {
        let mut loader = ResourceLoader::new();
        let resources = loader.load_html_string("<p>Hello</p>", "<test>");

        assert_eq!(resources.html.url, "<test>");
        assert_eq!(resources.html.resource_type, ResourceType::Html);
        assert!(resources.stylesheets.is_empty());
    }

    #[test]
    fn test_load_html_string_with_inline_style() {
        let mut loader = ResourceLoader::new();
        let html = r#"<html><head><style>h1 { color: red; }</style></head><body><h1>Hello</h1></body></html>"#;
        let resources = loader.load_html_string(html, "<test>");

        assert_eq!(resources.stylesheets.len(), 1);
        assert_eq!(resources.stylesheets[0].url, "<inline:0>");
        assert_eq!(resources.stylesheets[0].resource_type, ResourceType::Css);

        let css_text = std::str::from_utf8(&resources.stylesheets[0].bytes).unwrap();
        assert!(css_text.contains("color: red"));
    }

    #[test]
    fn test_load_html_string_multiple_style_blocks() {
        let mut loader = ResourceLoader::new();
        let html = r#"<html>
            <head>
                <style>h1 { color: red; }</style>
                <style>p { margin: 10px; }</style>
            </head>
            <body><h1>Hello</h1><p>World</p></body>
        </html>"#;
        let resources = loader.load_html_string(html, "<test>");

        assert_eq!(resources.stylesheets.len(), 2);
        assert_eq!(resources.stylesheets[0].url, "<inline:0>");
        assert_eq!(resources.stylesheets[1].url, "<inline:1>");
    }

    #[test]
    fn test_load_html_string_caches_html() {
        let mut loader = ResourceLoader::new();
        let _resources = loader.load_html_string("<p>Hello</p>", "<test>");

        assert!(loader.cache.contains("<test>"));
    }

    // ── Inline stylesheet discovery ──────────────────────────────

    #[test]
    fn test_discover_inline_style_empty() {
        let html = b"<html><body><p>Hello</p></body></html>";
        let mut tokenizer = crate::tokenizer::Tokenizer::new(html);
        let tokens = tokenizer.tokenize();
        let parser = crate::parser::Parser::new(&tokens, html);
        let dom = parser.parse();

        let mut loader = ResourceLoader::new();
        let stylesheets = loader.discover_stylesheets(&dom, html, None).unwrap();
        assert!(stylesheets.is_empty());
    }

    #[test]
    fn test_discover_inline_style() {
        let html = b"<html><head><style>body { margin: 0; }</style></head><body></body></html>";
        let mut tokenizer = crate::tokenizer::Tokenizer::new(html);
        let tokens = tokenizer.tokenize();
        let parser = crate::parser::Parser::new(&tokens, html);
        let dom = parser.parse();

        let mut loader = ResourceLoader::new();
        let stylesheets = loader.discover_stylesheets(&dom, html, None).unwrap();

        assert_eq!(stylesheets.len(), 1);
        let css = std::str::from_utf8(&stylesheets[0].bytes).unwrap();
        assert!(css.contains("margin: 0"));
    }

    // ── Link stylesheet discovery ────────────────────────────────

    #[test]
    fn test_discover_link_stylesheet_no_file() {
        // <link rel="stylesheet" href="missing.css"> should warn but not fail
        let html =
            br#"<html><head><link rel="stylesheet" href="missing.css"></head><body></body></html>"#;
        let mut tokenizer = crate::tokenizer::Tokenizer::new(html);
        let tokens = tokenizer.tokenize();
        let parser = crate::parser::Parser::new(&tokens, html);
        let dom = parser.parse();

        let mut loader = ResourceLoader::new();
        let stylesheets = loader.discover_stylesheets(&dom, html, None).unwrap();

        // Missing file → warning printed, not added to stylesheets
        assert!(stylesheets.is_empty());
    }

    #[test]
    fn test_discover_link_not_stylesheet() {
        // <link rel="icon" href="favicon.ico"> should be ignored
        let html =
            br#"<html><head><link rel="icon" href="favicon.ico"></head><body></body></html>"#;
        let mut tokenizer = crate::tokenizer::Tokenizer::new(html);
        let tokens = tokenizer.tokenize();
        let parser = crate::parser::Parser::new(&tokens, html);
        let dom = parser.parse();

        let mut loader = ResourceLoader::new();
        let stylesheets = loader.discover_stylesheets(&dom, html, None).unwrap();
        assert!(stylesheets.is_empty());
    }

    // ── Normalize path ───────────────────────────────────────────

    #[test]
    #[cfg(windows)]
    fn test_normalize_backslashes() {
        assert_eq!(
            normalize_path_string("C:\\Users\\test\\file.css"),
            "C:/Users/test/file.css"
        );
    }

    #[test]
    #[cfg(windows)]
    fn test_normalize_strips_prefix() {
        assert_eq!(
            normalize_path_string("//?/C:/Users/test/file.css"),
            "C:/Users/test/file.css"
        );
    }

    // ── LoadError Display ────────────────────────────────────────

    #[test]
    fn test_load_error_display() {
        let err = LoadError::IoError {
            path: "test.html".to_string(),
            message: "file not found".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("test.html"));
        assert!(msg.contains("file not found"));
    }

    // ── load_file with non-existent file ─────────────────────────

    #[test]
    fn test_load_file_not_found() {
        let mut loader = ResourceLoader::new();
        let result = loader.load_file("this_file_does_not_exist_12345.html");
        assert!(result.is_err());
    }
}
