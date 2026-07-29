use std::env;
use std::process;

use asteria::tokenizer::Tokenizer;
use asteria::parser::Parser;
use asteria::css_parser::Stylesheet;
use asteria::style::resolve_styles;
use asteria::loader::ResourceLoader;

fn main() {
    // ─── Read Input ──────────────────────────────────────────────
    //
    // Usage: cargo run -- <path-to-html-file>
    //
    // If no file is provided, use a built-in sample HTML string
    // so you can always run `cargo run` and see output immediately.

    let args: Vec<String> = env::args().collect();

    let mut loader = ResourceLoader::new();

    let resources = if args.len() > 1 {
        // Load HTML from file path — the loader handles discovery of
        // <style> blocks and <link rel="stylesheet"> references
        let path = &args[1];
        match loader.load_file(path) {
            Ok(resources) => resources,
            Err(err) => {
                eprintln!("Error: {}", err);
                process::exit(1);
            }
        }
    } else {
        // No file provided — use a built-in sample with embedded CSS
        println!("No file provided. Using built-in sample HTML.\n");
        println!("Usage: cargo run -- <path-to-html-file>\n");
        loader.load_html_string(
            r#"<!DOCTYPE html>
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
</html>"#,
            "<sample>",
        )
    };

    let bytes = &resources.html.bytes;

    // ─── Phase 1: Tokenize HTML ──────────────────────────────────

    println!("═══════════════════════════════════════════════");
    println!("  ASTERIA HTML ENGINE — Full Pipeline Inspector");
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

    // ─── Phase 2: Merge all CSS from discovered stylesheets ──────

    println!("\n── Resources Discovered ─────────────────────\n");
    println!("  HTML: {}", resources.html.url);
    println!("  Stylesheets: {}", resources.stylesheets.len());
    for (i, sheet) in resources.stylesheets.iter().enumerate() {
        println!("    [{}] {} ({} bytes)", i, sheet.url, sheet.bytes.len());
    }
    println!("  Cache entries: {}", loader.cache.len());

    // Concatenate all stylesheet content into one CSS source
    let mut css_source = String::new();
    for sheet in &resources.stylesheets {
        let text = std::str::from_utf8(&sheet.bytes).unwrap_or("");
        css_source.push_str(text);
        css_source.push('\n');
    }

    if !css_source.is_empty() {
        println!("\n── Combined CSS ({} bytes) ─────────────────\n", css_source.len());
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

        // ─── Phase 3: Resolve Styles (with cascade + inheritance) ─

        let styled = resolve_styles(&dom, &stylesheet, bytes);

        println!("\n── Styled DOM Tree (typed ComputedStyle) ───\n");
        styled.print_tree(&dom, bytes);

        // ─── Phase 4: Layout Engine (Calculates 2D Geometry & Box Model) ─

        if let Some(layout_tree) = asteria::layout::layout_document(&styled, &dom, bytes, 800.0, 600.0) {
            println!("\n── Layout Tree (2D Bounding Boxes & Coordinates) ───\n");
            layout_tree.print_tree(&dom, bytes);
        }
    } else {
        println!("\n── No stylesheets found ────────────────────");
        println!("  (Add a <style> block or <link rel=\"stylesheet\"> to see CSS in action)\n");
    }

    println!("\n═══════════════════════════════════════════════");
    println!("  Done. {} tokens → {} DOM nodes", tokens.len(), dom.nodes.len());
    if !css_source.is_empty() {
        println!("  Full pipeline: Load → HTML Tokenize/Parse → CSS Parse → Cascade/Style → Layout Engine");
    }
    println!("═══════════════════════════════════════════════");
}

// ─── Helpers ─────────────────────────────────────────────────────

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
