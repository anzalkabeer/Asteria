use std::env;
use std::process;
use std::time::Instant;

use asteria::css_parser::Stylesheet;
use asteria::devtools::config::AofConfig;
use asteria::devtools::inspector::AofInspector;
use asteria::devtools::metrics::{GPU_VRAM_USED, MEMORY_ALLOCATED, reset_frame_metrics};
use asteria::devtools::snapshot::EngineSnapshot;
use asteria::devtools::trace::{TraceEventKind, record_event};
use asteria::parser::Parser;
use asteria::profiler::{EngineProfiler, EngineStage};
use asteria::scheduler::{PipelineStage, ThreadedScheduler};
use asteria::shell::{ShellEvent, TabManager};
use asteria::style::resolve_styles;
use asteria::tokenizer::Tokenizer;

fn main() {
    let args: Vec<String> = env::args().collect();

    // ─── Phase 0: Initialize Profiler, Devtools & Shell ────────────────
    let mut profiler = EngineProfiler::new();
    profiler.start_pipeline();

    AofConfig::full_inspection().apply();
    AofInspector::init(AofConfig::full_inspection());
    record_event(TraceEventKind::FrameBegin { frame_id: 1 });
    reset_frame_metrics();

    let mut tab_manager = TabManager::new();
    let mut threaded_scheduler = ThreadedScheduler::new(4);

    let target_url = if args.len() > 1 && args[1] != "--window" {
        &args[1]
    } else {
        println!("No file provided (or only --window passed). Using built-in sample HTML.\n");
        println!("Usage: cargo run -- <path-to-html-file> [--window] [--cli]\n");
        "<sample>"
    };

    if target_url != "<sample>"
        && let Err(e) = tab_manager.handle_event(ShellEvent::NavigateTo(target_url.to_string()))
    {
        eprintln!("Error loading {}: {}", target_url, e);
        process::exit(1);
    }

    let active_tab = tab_manager.active_tab();
    println!("═══════════════════════════════════════════════");
    println!("  ASTERIA ENGINE — Full Pipeline & Shell Inspector");
    println!("═══════════════════════════════════════════════\n");
    println!("Active Tab : {}", active_tab.title);
    println!("Active URL : {}\n", active_tab.url);

    // ─── Async Scheduler Pipeline Stage Dispatch ──────────────────────
    let sample_html_bytes = b"<!DOCTYPE html><html><head><style>body { background-color: #1e1e2e; color: #cdd6f4; } h1 { color: #89b4fa; font-size: 24px; } p { color: #a6adc8; font-size: 16px; } div { background-color: #313244; }</style></head><body><h1>Asteria Browser Engine</h1><p>Hardware-accelerated GPU renderer running with wgpu + winit.</p><div><p>Interactive Viewport: Scroll, Hover, Click supported!</p></div></body></html>";

    let bytes = active_tab
        .page_resources
        .as_ref()
        .map(|r| r.html.bytes.as_slice())
        .unwrap_or(sample_html_bytes);

    let async_parse_id = threaded_scheduler
        .schedule(PipelineStage::ParseHtml {
            url: active_tab.url.clone(),
            bytes: bytes.to_vec(),
        })
        .ok();

    // ─── Phase 1: Tokenize & Parse HTML ────────────────────────────────
    let parse_start = Instant::now();
    record_event(TraceEventKind::ParseStart);

    let mut tokenizer = Tokenizer::new(bytes);
    let tokens = tokenizer.tokenize();
    let parser = Parser::new(&tokens, bytes);
    let dom = parser.parse();

    let parse_duration = parse_start.elapsed();
    profiler.record_stage_duration(EngineStage::ParseHtml, parse_duration);
    record_event(TraceEventKind::ParseEnd {
        node_count: dom.nodes.len(),
        duration_ms: parse_duration.as_secs_f64() * 1000.0,
    });

    println!(
        "── HTML Tokens ({}) & DOM Tree ({} nodes) ─────\n",
        tokens.len(),
        dom.nodes.len()
    );
    dom.print_tree(bytes);

    // Verify async worker completed concurrently
    if let Some(expected_id) = async_parse_id
        && let Some(msg) = threaded_scheduler.poll_completed()
        && msg.task_id == expected_id
    {
        println!(
            "\n[Async Scheduler] Task #{} completed concurrently on background thread",
            msg.task_id
        );
    }

    // ─── Phase 2: Parse CSS Stylesheets ────────────────────────────────
    let css_start = Instant::now();
    let mut css_source = String::new();
    if let Some(ref resources) = active_tab.page_resources {
        for sheet in &resources.stylesheets {
            let text = std::str::from_utf8(&sheet.bytes).unwrap_or("");
            css_source.push_str(text);
            css_source.push('\n');
        }
    }

    if css_source.is_empty() {
        css_source = std::str::from_utf8(sample_css_bytes)
            .unwrap_or("")
            .to_string();
    }

    let stylesheet = Stylesheet::parse(css_source.as_bytes());
    let css_duration = css_start.elapsed();
    profiler.record_stage_duration(EngineStage::ParseCss, css_duration);

    if !stylesheet.rules.is_empty() {
        println!(
            "\n── CSS Rules ({}) ──────────────────────────\n",
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
    } else {
        println!("\n── No custom CSS rules (applying User-Agent defaults) ──\n");
    }

    // ─── Phase 3: Resolve Styles (Cascade & Inheritance) ───────────
    let style_start = Instant::now();
    record_event(TraceEventKind::StyleStart);

    let styled = resolve_styles(&dom, &stylesheet, bytes);

    let style_duration = style_start.elapsed();
    profiler.record_stage_duration(EngineStage::ResolveStyles, style_duration);
    record_event(TraceEventKind::StyleEnd {
        styled_count: dom.nodes.len(),
        duration_ms: style_duration.as_secs_f64() * 1000.0,
    });

    println!("\n── Styled DOM Tree (typed ComputedStyle) ───\n");
    styled.print_tree(&dom, bytes);

    // ─── Phase 4: 2D Layout Engine ─────────────────────────────────
    let layout_start = Instant::now();
    record_event(TraceEventKind::LayoutStart);

    let maybe_layout_tree = asteria::layout::layout_document(&styled, &dom, bytes, 800.0, 600.0);
    let layout_duration = layout_start.elapsed();
    profiler.record_stage_duration(EngineStage::Layout, layout_duration);

    let box_count = maybe_layout_tree
        .as_ref()
        .map(|tree| tree.box_count())
        .unwrap_or(0);
    record_event(TraceEventKind::LayoutEnd {
        box_count,
        duration_ms: layout_duration.as_secs_f64() * 1000.0,
    });

    if let Some(layout_tree) = maybe_layout_tree {
        println!("\n── Layout Tree (2D Bounding Boxes & Coordinates) ───\n");
        layout_tree.print_tree(&dom, bytes);

        // ─── Phase 5: Display List Paint Engine ────────────────────
        let paint_start = Instant::now();
        record_event(TraceEventKind::PaintStart);

        let display_list = asteria::paint::build_display_list(&layout_tree, &dom, bytes);

        let paint_duration = paint_start.elapsed();
        profiler.record_stage_duration(EngineStage::Paint, paint_duration);
        record_event(TraceEventKind::PaintEnd {
            command_count: display_list.commands.len(),
            duration_ms: paint_duration.as_secs_f64() * 1000.0,
        });

        println!("\n── Display List (Visual Draw Commands) ───\n");
        asteria::paint::print_display_list(&display_list);

        // ─── Phase 6: Scene Graph & Segments ───────────────────────
        let scene_start = Instant::now();
        record_event(TraceEventKind::SceneStart);

        let scene = asteria::scene::build_scene_graph(&display_list, 256.0);
        let scene_duration = scene_start.elapsed();
        profiler.record_stage_duration(EngineStage::Render, scene_duration);
        record_event(TraceEventKind::SceneEnd {
            node_count: scene.len(),
            duration_ms: scene_duration.as_secs_f64() * 1000.0,
        });

        let mut segments = asteria::segment::SegmentBuilder::new(256.0);
        let _ = segments.build_segments(800.0, 600.0);

<<<<<<< HEAD


=======
        // ─── Performance Profiler & AOF Inspection ─────────────────
>>>>>>> 39d3626 (Final push)
        profiler.set_counts(dom.nodes.len(), box_count, display_list.commands.len());
        let report = profiler.finish_pipeline();

        println!("\n{}", report.format_summary());

        record_event(TraceEventKind::FrameEnd {
            frame_id: 1,
            duration_ms: report.total_duration.as_secs_f64() * 1000.0,
        });

        let snapshot = EngineSnapshot::new()
            .with_dom(&dom)
            .with_style(&styled)
            .with_layout(&layout_tree)
            .with_scene(&scene)
            .with_segments(&segments);

        let mut energy = asteria::devtools::metrics::EnergyDiagnostics::new();
        energy.allocations = MEMORY_ALLOCATED.load(std::sync::atomic::Ordering::Relaxed);
        energy.gpu_uploads = GPU_VRAM_USED.load(std::sync::atomic::Ordering::Relaxed);
        energy.impact = energy.analyze_impact();

        AofInspector::inspect(snapshot, &energy, "trace.json");

        if !args.contains(&"--cli".to_string()) {
            println!("\n── Launching Hardware Renderer (wgpu) & OS Window Loop ─────────");
<<<<<<< HEAD
            asteria::renderer::window::window::run_window_loop(scene, tab_manager);
=======
            asteria::renderer::window::window::run_window_loop(tab_manager, scene);
>>>>>>> 39d3626 (Final push)
            return;
        }
    } else {
        profiler.set_counts(dom.nodes.len(), 0, 0);
        let report = profiler.finish_pipeline();
        println!("\n{}", report.format_summary());
    }

    println!("\n═══════════════════════════════════════════════");
    println!("  Pipeline Run Complete.");
    println!("═══════════════════════════════════════════════");
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
                    SimpleSelector::PseudoClass(name) => format!(":{}", name),
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
}
