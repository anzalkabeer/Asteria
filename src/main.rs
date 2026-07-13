use std::env;
use std::fs;
use std::process;

use asteria::tokenizer::Tokenizer;
use asteria::parser::Parser;

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
        // No file provided — use a built-in sample
        println!("No file provided. Using built-in sample HTML.\n");
        println!("Usage: cargo run -- <path-to-html-file>\n");
        String::from(r#"<!DOCTYPE html>
<html>
<head>
    <title>Asteria Sample</title>
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

    // ─── Tokenize ────────────────────────────────────────────────

    println!("═══════════════════════════════════════════════");
    println!("  ASTERIA HTML ENGINE — Phase 1 Inspector");
    println!("═══════════════════════════════════════════════\n");

    let mut tokenizer = Tokenizer::new(bytes);
    let tokens = tokenizer.tokenize();

    // Print all tokens
    println!("── Tokens ({}) ──────────────────────────────\n", tokens.len());
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

    // ─── Parse ───────────────────────────────────────────────────

    let parser = Parser::new(&tokens, bytes);
    let dom = parser.parse();

    println!("\n── DOM Tree ({} nodes) ─────────────────────\n", dom.nodes.len());
    dom.print_tree(bytes);

    println!("\n═══════════════════════════════════════════════");
    println!("  Done. {} tokens → {} DOM nodes", tokens.len(), dom.nodes.len());
    println!("═══════════════════════════════════════════════");
}
