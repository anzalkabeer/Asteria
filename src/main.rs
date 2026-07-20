use std::env;
use std::fs;
use std::process;

use asteria::tokenizer::Tokenizer;
use asteria::parser::Parser;
use asteria::dom::{Dom, NodeId, NodeKind};
use asteria::css_parser::Stylesheet;
use asteria::style::resolve_styles;

fn main() {
    // ─── Read Input ──────────────────────────────────────────────
    //
    // Usage: cargo run -- <path-to-html-file>
    //
    // If no file is provided, use a built-in sample HTML string
    // so you can always run `cargo run` and see output immediately.

    let args: Vec<String> = env::args().collect();

    let html = if args.len() > 1 {
        // Read HTML from file path provided as argument
        let path = &args[1];
        match fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Error reading file '{}': {}", path, err);
                process::exit(1);
            }
        }
    } else {
        // No file provided — use a built-in sample with embedded CSS
        println!("No file provided. Using built-in sample HTML.\n");
        println!("Usage: cargo run -- <path-to-html-file>\n");
        String::from(r#"<!DOCTYPE html>
<html>
<head>
    <title>Asteria Sample</title>
    <style>
        h1 { color: red; font-size: 24px; }
        .main { background-color: #f0f0f0; }
        p { color: #333; margin: 10px; }
        strong { font-weight: bold; }
    </style>
</head>
<body>
    <h1 class="main">Hello, Asteria!</h1>
    <p>A <strong>simple</strong> test.</p>
    <!-- A comment -->
    <br/>
</body>
</html>"#)
    };

    let bytes = html.as_bytes();

    // ─── Phase 1: Tokenize HTML ──────────────────────────────────

    println!("═══════════════════════════════════════════════");
    println!("  ASTERIA HTML ENGINE — Phase 1+2 Inspector");
    println!("═══════════════════════════════════════════════\n");

    let mut tokenizer = Tokenizer::new(bytes);
    let tokens = tokenizer.tokenize();

    // Print all tokens
    println!("── HTML Tokens ({}) ─────────────────────────\n", tokens.len());
    for (i, token) in tokens.iter().enumerate() {
        let slice = if (token.start as usize) < bytes.len() && (token.end as usize) <= bytes.len() {
            std::str::from_utf8(&bytes[token.start as usize..token.end as usize]).unwrap_or("???")
        } else {
            ""
        };

        let kind_str = format!("{:?}", token.kind);

        // Print token with its attributes if any
        if token.attributes.is_empty() {
            println!("  [{:>3}] {:<20} {:>4}..{:<4}  {:?}", i, kind_str, token.start, token.end, slice);
        } else {
            println!("  [{:>3}] {:<20} {:>4}..{:<4}  {:?}", i, kind_str, token.start, token.end, slice);
            for attr in &token.attributes {
                let name = std::str::from_utf8(&bytes[attr.name_start as usize..attr.name_end as usize]).unwrap_or("???");
                if attr.value_start == 0 && attr.value_end == 0 {
                    println!("        └─ attr: {}", name);
                } else {
                    let value = std::str::from_utf8(&bytes[attr.value_start as usize..attr.value_end as usize]).unwrap_or("???");
                    println!("        └─ attr: {}=\"{}\"", name, value);
                }
            }
        }
    }

    // ─── Phase 1: Parse into DOM ─────────────────────────────────

    let parser = Parser::new(&tokens, bytes);
    let dom = parser.parse();

    println!("\n── DOM Tree ({} nodes) ─────────────────────\n", dom.nodes.len());
    dom.print_tree(bytes);

    // ─── Phase 2: Extract CSS from <style> elements ──────────────

    let css_source = extract_style_content(&dom, bytes);

    if !css_source.is_empty() {
        println!("\n── Extracted CSS ({} bytes) ────────────────\n", css_source.len());
        println!("{}", css_source);

        // ─── Phase 2: Parse CSS ──────────────────────────────────

        let stylesheet = Stylesheet::parse(css_source.as_bytes());

        println!("── CSS Rules ({}) ──────────────────────────\n", stylesheet.rules.len());
        for (i, rule) in stylesheet.rules.iter().enumerate() {
            let selectors: Vec<String> = rule
                .selectors
                .iter()
                .map(|sel| format_selector(sel))
                .collect();
            println!("  [{}] {} {{", i, selectors.join(", "));
            for decl in &rule.declarations {
                println!("        {}: {};", decl.property, decl.value);
            }
            println!("      }}");
        }

        // ─── Phase 2: Resolve Styles ─────────────────────────────

        let styled = resolve_styles(&dom, &stylesheet, bytes);

        println!("\n── Styled DOM Tree ─────────────────────────\n");
        styled.print_tree(&dom, bytes);
    } else {
        println!("\n── No <style> element found ─────────────────");
        println!("  (Add a <style> block to see CSS styling in action)\n");
    }

    println!("\n═══════════════════════════════════════════════");
    println!("  Done. {} tokens → {} DOM nodes", tokens.len(), dom.nodes.len());
    if !css_source.is_empty() {
        println!("  CSS pipeline: tokenize → parse → resolve → styled tree");
    }
    println!("═══════════════════════════════════════════════");
}

// ─── Helpers ─────────────────────────────────────────────────────

/// Walk the DOM to find <style> elements and extract their text content.
/// Returns the concatenated CSS text from all <style> blocks.
fn extract_style_content(dom: &Dom, source: &[u8]) -> String {
    let mut css = String::new();
    collect_style_text(dom, dom.root(), source, &mut css);
    css
}

/// Recursively search for <style> elements and collect their text children.
fn collect_style_text(dom: &Dom, node_id: NodeId, source: &[u8], css: &mut String) {
    let node = dom.get(node_id);

    if let NodeKind::Element { tag_start, tag_end } = &node.kind {
        let tag_name =
            std::str::from_utf8(&source[*tag_start as usize..*tag_end as usize])
                .unwrap_or("");

        if tag_name.eq_ignore_ascii_case("style") {
            // Collect all text children of this <style> element
            for &child_id in &node.children {
                let child = dom.get(child_id);
                if let NodeKind::Text { start, end } = &child.kind {
                    let text = std::str::from_utf8(&source[*start as usize..*end as usize])
                        .unwrap_or("");
                    css.push_str(text);
                }
            }
            return; // Don't recurse into <style> children further
        }
    }

    // Recurse into other elements
    for &child_id in &node.children {
        collect_style_text(dom, child_id, source, css);
    }
}

/// Format a Selector for display.
fn format_selector(selector: &asteria::css_parser::Selector) -> String {
    use asteria::css_parser::SimpleSelector;

    selector
        .parts
        .iter()
        .map(|compound| {
            compound
                .iter()
                .map(|simple| match simple {
                    SimpleSelector::Tag(name) => name.clone(),
                    SimpleSelector::Class(name) => format!(".{}", name),
                    SimpleSelector::Id(name) => format!("#{}", name),
                    SimpleSelector::Universal => "*".to_string(),
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
}
