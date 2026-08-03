use std::env;
use std::process;

use asteria::css_parser::Stylesheet;
use asteria::loader::ResourceLoader;
use asteria::parser::Parser;
use asteria::style::resolve_styles;
use asteria::tokenizer::Tokenizer;

fn main() {
    // ─── Read Input ──────────────────────────────────────────────
    //
    // Usage: cargo run -- <path-to-html-file>
    //
    // If no file is provided, use a built-in sample HTML string
    // so you can always run `cargo run` and see output immediately.

    let args: Vec<String> = env::args().collect();

    let mut loader = ResourceLoader::new();

    let resources = if args.len() > 1 && args[1] != "--window" {
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
        println!("No file provided (or only --window passed). Using built-in sample HTML.\n");
        println!("Usage: cargo run -- <path-to-html-file> [--window]\n");
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
    println!(
        "── HTML Tokens ({}) ─────────────────────────\n",
        tokens.len()
    );
    for (i, token) in tokens.iter().enumerate() {
        let slice = if (token.start as usize) < bytes.len() && (token.end as usize) <= bytes.len() {
            std::str::from_utf8(&bytes[token.start as usize..token.end as usize]).unwrap_or("???")
        } else {
            ""
        };

        let kind_str = format!("{:?}", token.kind);

        // Print token with its attributes if any
        if token.attributes.is_empty() {
            println!(
                "  [{:>3}] {:<20} {:>4}..{:<4}  {:?}",
                i, kind_str, token.start, token.end, slice
            );
        } else {
            println!(
                "  [{:>3}] {:<20} {:>4}..{:<4}  {:?}",
                i, kind_str, token.start, token.end, slice
            );
            for attr in &token.attributes {
                let name =
                    std::str::from_utf8(&bytes[attr.name_start as usize..attr.name_end as usize])
                        .unwrap_or("???");
                if attr.value_start == 0 && attr.value_end == 0 {
                    println!("        └─ attr: {}", name);
                } else {
                    let value = std::str::from_utf8(
                        &bytes[attr.value_start as usize..attr.value_end as usize],
                    )
                    .unwrap_or("???");
                    println!("        └─ attr: {}=\"{}\"", name, value);
                }
            }
        }
    }

    // ─── AOF Initialization ──────────────────────────────────────────
    asteria::devtools::config::AofConfig::full_inspection().apply();
    asteria::devtools::inspector::AofInspector::init(
        asteria::devtools::config::AofConfig::full_inspection(),
    );
    asteria::devtools::trace::record_event(asteria::devtools::trace::TraceEventKind::FrameBegin {
        frame_id: 1,
    });
    asteria::devtools::metrics::reset_frame_metrics();

    // ─── Phase 1: Parse into DOM ─────────────────────────────────

    asteria::devtools::trace::record_event(asteria::devtools::trace::TraceEventKind::ParseStart);
    let parser = Parser::new(&tokens, bytes);
    let dom = parser.parse();
    asteria::devtools::trace::record_event(asteria::devtools::trace::TraceEventKind::ParseEnd {
        node_count: dom.nodes.len(),
        duration_ms: 0.0,
    });

    println!(
        "\n── DOM Tree ({} nodes) ─────────────────────\n",
        dom.nodes.len()
    );
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
        println!(
            "\n── Combined CSS ({} bytes) ─────────────────\n",
            css_source.len()
        );
        println!("{}", css_source);

        // ─── Phase 2: Parse CSS ──────────────────────────────────

        let stylesheet = Stylesheet::parse(css_source.as_bytes());

        println!(
            "── CSS Rules ({}) ──────────────────────────\n",
            stylesheet.rules.len()
        );
        for (i, rule) in stylesheet.rules.iter().enumerate() {
            let selectors: Vec<String> = rule.selectors.iter().map(format_selector).collect();
            println!("  [{}] {} {{", i, selectors.join(", "));
            for decl in &rule.declarations {
                println!("        {}: {};", decl.property, decl.value);
            }
            println!("      }}");
        }

        // ─── Phase 3: Resolve Styles (with cascade + inheritance) ─

        asteria::devtools::trace::record_event(
            asteria::devtools::trace::TraceEventKind::StyleStart,
        );
        let styled = resolve_styles(&dom, &stylesheet, bytes);
        asteria::devtools::trace::record_event(
            asteria::devtools::trace::TraceEventKind::StyleEnd {
                styled_count: dom.nodes.len(),
                duration_ms: 0.0,
            },
        );

        println!("\n── Styled DOM Tree (typed ComputedStyle) ───\n");
        styled.print_tree(&dom, bytes);

        // ─── Phase 4: Layout Engine (Calculates 2D Geometry & Box Model) ─

        asteria::devtools::trace::record_event(
            asteria::devtools::trace::TraceEventKind::LayoutStart,
        );
        if let Some(layout_tree) =
            asteria::layout::layout_document(&styled, &dom, bytes, 800.0, 600.0)
        {
            asteria::devtools::trace::record_event(
                asteria::devtools::trace::TraceEventKind::LayoutEnd {
                    box_count: 0,
                    duration_ms: 0.0,
                },
            );
            println!("\n── Layout Tree (2D Bounding Boxes & Coordinates) ───\n");
            layout_tree.print_tree(&dom, bytes);

            // ─── Phase 5: Paint Engine (Generates Backend-Agnostic Display List) ─

            asteria::devtools::trace::record_event(
                asteria::devtools::trace::TraceEventKind::PaintStart,
            );
            let display_list = asteria::paint::build_display_list(&layout_tree, &dom, bytes);
            asteria::devtools::trace::record_event(
                asteria::devtools::trace::TraceEventKind::PaintEnd {
                    command_count: display_list.commands.len(),
                    duration_ms: 0.0,
                },
            );
            println!("\n── Display List (Visual Draw Commands) ───\n");
            asteria::paint::print_display_list(&display_list);

            // ─── Phase 6: Scene Graph & Segment Builder (GPU-First Architecture) ─

            asteria::devtools::trace::record_event(
                asteria::devtools::trace::TraceEventKind::SceneStart,
            );
            let scene = asteria::scene::build_scene_graph(&display_list, 256.0);
            asteria::devtools::trace::record_event(
                asteria::devtools::trace::TraceEventKind::SceneEnd {
                    node_count: scene.len(),
                    duration_ms: 0.0,
                },
            );
            println!("\n{}", scene);

            let mut segments = asteria::segment::SegmentBuilder::new(256.0);
            segments
                .build_segments(800.0, 600.0)
                .expect("Invalid viewport dimensions");
            println!("{}", segments);

            if args.contains(&"--window".to_string()) {
                println!("\n── Launching Hardware Renderer (wgpu) ─────────");
                asteria::renderer::window::window::run_window_loop(scene);
                return;
            }

            // ─── AOF Inspection ───────────────────────────────────────────────
            asteria::devtools::trace::record_event(
                asteria::devtools::trace::TraceEventKind::FrameEnd {
                    frame_id: 1,
                    duration_ms: 0.0,
                },
            );

            let snapshot = asteria::devtools::snapshot::EngineSnapshot::new()
                .with_dom(&dom)
                .with_style(&styled)
                .with_layout(&layout_tree)
                .with_scene(&scene)
                .with_segments(&segments);

            let mut energy = asteria::devtools::metrics::EnergyDiagnostics::new();
            energy.allocations = asteria::devtools::metrics::MEMORY_ALLOCATED
                .load(std::sync::atomic::Ordering::Relaxed);
            energy.gpu_uploads = asteria::devtools::metrics::GPU_VRAM_USED
                .load(std::sync::atomic::Ordering::Relaxed);
            energy.impact = energy.analyze_impact();

            asteria::devtools::inspector::AofInspector::inspect(snapshot, &energy, "trace.json");
        }
    } else {
        println!("\n── No stylesheets found ────────────────────");
        println!("  (Add a <style> block or <link rel=\"stylesheet\"> to see CSS in action)\n");
    }

    println!("\n═══════════════════════════════════════════════");
    println!(
        "  Done. {} tokens → {} DOM nodes",
        tokens.len(),
        dom.nodes.len()
    );
    if !css_source.is_empty() {
        println!(
            "  Full pipeline: Load → HTML → CSS → Style → Layout → Paint → Scene Graph → Segments"
        );
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
