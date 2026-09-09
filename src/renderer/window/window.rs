// ─── ASTERIA Window & Event Loop ─────────────────────────────────
//
// Phase 9C & Milestone 10: Interactive event loop built on winit.
//
// Architecture:
//   CursorMoved  → hit_test(x, y) → update NodeState::Hovered
//   MouseInput   → hit_test(x, y) → update NodeState::Active → link URL dispatch
//   MouseWheel   → update scroll_offset → rebuild scene batches → redraw
//
// All GPU rendering flows through the RenderGraph:
//   RenderGraph.prepare() → RenderGraph.render()
//   Pass order: RectPass[0] → ImagePass[1] → TextPass[2]
//
// Design principles:
//   - Events never block the render loop (Keshav's ThreadedScheduler handles async tasks)
//   - GPU pipelines are created ONCE at init; only data buffers are rebuilt per frame
//   - Tile invalidation (Pillar 4): skip GPU submission when 0 tiles are dirty
//   - Pass-specific methods accessed via RenderGraph.pass_downcast_mut::<T>()

use std::sync::Arc;
use winit::{
    event::{ElementState, Event, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowBuilder},
};

pub enum AsteriaUserEvent {
    SceneUpdated(SceneGraph),
}

use crate::css_parser::Stylesheet;
use crate::renderer::backend::wgpu_backend::WgpuBackend;
use crate::renderer::commands::batch_builder::BatchBuilder;
use crate::renderer::commands::command_builder::{CommandBuilder, RenderCommand};
use crate::renderer::graph::render_graph::RenderGraph;
use crate::renderer::passes::image_pass::ImagePass;
use crate::renderer::passes::rect_pass::RectPass;
use crate::renderer::passes::text_pass::TextPass;
use crate::scene::{NodeState, SceneGraph, SceneNodeId, build_scene_graph};
use crate::scheduler::ThreadedScheduler;
use crate::shell::{ShellEvent, TabManager};
use crate::shell_ui::{HEADER_HEIGHT, STATUS_BAR_HEIGHT, ShellHitTarget, ShellUiState};

pub struct AsteriaWindow {
    pub window: Arc<Window>,
    pub tab_manager: TabManager,
    pub scheduler: ThreadedScheduler,
    pub modifiers: ModifiersState,
}

impl AsteriaWindow {
    pub fn new(event_loop: &EventLoop<AsteriaUserEvent>, width: u32, height: u32) -> Self {
        Self::with_tab_manager(event_loop, width, height, TabManager::new())
    }

    pub fn with_tab_manager(
        event_loop: &EventLoop<AsteriaUserEvent>,
        width: u32,
        height: u32,
        tab_manager: TabManager,
    ) -> Self {
        let window = Arc::new(
            WindowBuilder::new()
                .with_title("Asteria Engine Browser Shell")
                .with_inner_size(winit::dpi::LogicalSize::new(width, height))
                .build(event_loop)
                .expect("Failed to create Window"),
        );

        Self {
            window,
            tab_manager,
            scheduler: ThreadedScheduler::new(4),
            modifiers: ModifiersState::default(),
        }
    }

    pub fn build_active_scene(
        &mut self,
        proxy: Option<winit::event_loop::EventLoopProxy<AsteriaUserEvent>>,
    ) -> SceneGraph {
        let size = self.window.inner_size();
        let viewport_w = size.width as f32;
        // Webpage viewport excludes shell chrome (Header + Status Bar)
        let viewport_h = (size.height as f32 - HEADER_HEIGHT - STATUS_BAR_HEIGHT).max(100.0);

        let sample_html_bytes = b"<!DOCTYPE html><html><head><style>body { background-color: #1e1e2e; color: #cdd6f4; } h1 { color: #89b4fa; font-size: 24px; } p { color: #a6adc8; font-size: 16px; } div { background-color: #313244; }</style></head><body><h1>Asteria Browser Engine</h1><p>Hardware-accelerated GPU renderer running with wgpu + winit.</p><div><p>Interactive Viewport: Scroll, Hover, Click supported!</p></div></body></html>";

        let active_tab = self.tab_manager.active_tab();
        let html_bytes = active_tab
            .page_resources
            .as_ref()
            .map(|r| r.html.bytes.as_slice())
            .unwrap_or(sample_html_bytes);

        if let Some(p) = proxy {
            let _ = self
                .scheduler
                .schedule(crate::scheduler::PipelineStage::ParseHtml {
                    url: active_tab.url.clone(),
                    bytes: html_bytes.to_vec(),
                    proxy: Some(p),
                    viewport_size: Some((viewport_w, viewport_h)),
                });
            // Return an empty scene immediately; the proxy will deliver the real scene
            return SceneGraph::new();
        }

        // Fallback: synchronous build
        let mut processor = crate::streaming_parser::StreamingHtmlProcessor::new();
        let _ = processor.receive_network_chunk(html_bytes, true);
        let dom = processor.finish();

        let mut css_source = Vec::new();
        if let Some(res) = &active_tab.page_resources {
            for sheet in &res.stylesheets {
                css_source.extend_from_slice(&sheet.bytes);
                css_source.push(b'\n');
            }
        }
        if css_source.is_empty() {
            // Also attempt to discover inline <style> tags from the parsed DOM
            let mut loader = crate::loader::ResourceLoader::new();
            let discovered = loader.load_html_string(
                std::str::from_utf8(html_bytes).unwrap_or(""),
                &active_tab.url,
            );
            for sheet in &discovered.stylesheets {
                css_source.extend_from_slice(&sheet.bytes);
                css_source.push(b'\n');
            }
        }
        if css_source.is_empty() {
            css_source.extend_from_slice(crate::scheduler::DEFAULT_FALLBACK_CSS);
        }

        let stylesheet = Stylesheet::parse(&css_source);
        let styled =
            crate::style::resolve_styles_with_viewport(&dom, &stylesheet, html_bytes, viewport_w);

        if let Some(layout) =
            crate::layout::layout_document(&styled, &dom, html_bytes, viewport_w, viewport_h)
        {
            let display_list = crate::paint::build_display_list(&layout, &dom, html_bytes);
            return build_scene_graph(&display_list, 256.0);
        }

        SceneGraph::new()
    }
}

// ─── Batch Builder Helper ─────────────────────────────────────────

fn build_rect_batch(scene: &SceneGraph, scroll_y: f32, vp_w: f32, vp_h: f32) -> BatchBuilder {
    let mut cmd_builder = CommandBuilder::new();
    cmd_builder.build_from_scene(scene);

    let top_clip = HEADER_HEIGHT;
    let bottom_clip = (vp_h - STATUS_BAR_HEIGHT).max(HEADER_HEIGHT);

    // Canvas background fill: Find the canvas/body background color from the scene (or fallback to #1e1e2e)
    let canvas_bg = scene
        .nodes
        .iter()
        .enumerate()
        .find_map(|(i, n)| {
            if matches!(n.kind, crate::scene::SceneNodeKind::SolidRect)
                && n.rect.x <= 0.0
                && n.rect.y <= 0.0
                && n.rect.width > 50.0
            {
                Some(scene.colors[i])
            } else {
                None
            }
        })
        .unwrap_or([0.118, 0.118, 0.180, 1.0]); // Catppuccin Mocha Base #1e1e2e

    let mut batch = BatchBuilder::new();

    // Flood the entire content viewport with the canvas background
    batch.add_quad_direct(
        0.0,
        top_clip,
        vp_w,
        (bottom_clip - top_clip).max(0.0),
        canvas_bg,
    );

    // Offset webpage content by HEADER_HEIGHT and scroll position, and clip to content bounds
    let scrolled: Vec<_> = cmd_builder
        .commands
        .into_iter()
        .filter_map(|c| match c {
            RenderCommand::SolidRect { rect, rgba } => {
                let mut r = rect;
                r[1] = r[1] + HEADER_HEIGHT - scroll_y;

                let r_top = r[1];
                let r_bottom = r[1] + r[3];
                if r_bottom <= top_clip || r_top >= bottom_clip {
                    return None;
                }
                let visible_top = r_top.max(top_clip);
                let visible_bottom = r_bottom.min(bottom_clip);
                r[1] = visible_top;
                r[3] = (visible_bottom - visible_top).max(0.0);
                Some(RenderCommand::SolidRect { rect: r, rgba })
            }
            // Text is rendered via TextPass (glyphon)
            RenderCommand::Text { .. } => None,
        })
        .collect();

    batch.append_batches(&scrolled, vp_w);
    batch
}

// ─── Text Pass Population ─────────────────────────────────────────

fn populate_text_pass(
    text_pass: &mut TextPass,
    scene: &SceneGraph,
    scroll_y: f32,
    vp_w: f32,
    vp_h: f32,
) {
    text_pass.clear();
    let bounds = [
        0,
        HEADER_HEIGHT as i32,
        vp_w as i32,
        (vp_h - STATUS_BAR_HEIGHT).max(HEADER_HEIGHT) as i32,
    ];
    for (i, node) in scene.nodes.iter().enumerate() {
        if let crate::scene::SceneNodeKind::Text { font_size } = node.kind
            && let Some(text_run) = &scene.texts[i]
        {
            let color = scene.colors[i];
            // Offset webpage text by HEADER_HEIGHT and scroll position
            let y = node.rect.y + HEADER_HEIGHT - scroll_y;
            text_pass.add_text_clipped(
                &text_run.text,
                [node.rect.x, y],
                font_size,
                color,
                Some(bounds),
            );
        }
    }
}

// ─── Max Scroll Calculation ───────────────────────────────────────

fn max_scroll_for_scene(scene: &SceneGraph, viewport_height: f32) -> f32 {
    let max_y = scene
        .nodes
        .iter()
        .map(|n| n.rect.y + n.rect.height)
        .fold(0.0_f32, f32::max);
    (max_y - viewport_height + 20.0).max(0.0)
}

// ─── Main Window Loop ─────────────────────────────────────────────

pub fn run_window_loop(initial_scene: SceneGraph, tab_manager: TabManager) {
    let event_loop = EventLoopBuilder::<AsteriaUserEvent>::with_user_event()
        .build()
        .expect("Failed to create EventLoop");
    let mut asteria_window = AsteriaWindow::with_tab_manager(&event_loop, 800, 600, tab_manager);

    let mut backend = pollster::block_on(WgpuBackend::new(asteria_window.window.clone()));

    let proxy = event_loop.create_proxy();
    let mut scene = if initial_scene.nodes.is_empty() {
        asteria_window.build_active_scene(Some(proxy.clone()))
    } else {
        initial_scene
    };

    let mut cursor_pos: (f32, f32) = (0.0, 0.0);
    let mut current_scroll_y: f32 = 0.0;
    let mut target_scroll_y: f32 = 0.0;
    let mut hovered_node: Option<SceneNodeId> = None;
    let mut needs_redraw = true;
    let mut shell_ui = ShellUiState::new();

    // Sync omnibox with initial tab
    shell_ui.sync_with_tab_url(&asteria_window.tab_manager.active_tab().url);

    let mut render_graph = RenderGraph::new();
    let mut rect_pass = RectPass::new(&backend.device, backend.config.format);
    rect_pass.update_viewport(
        &backend.queue,
        backend.config.width as f32,
        backend.config.height as f32,
    );
    render_graph.add_pass(Box::new(rect_pass));
    render_graph.add_pass(Box::new(ImagePass::new(
        &backend.device,
        backend.config.format,
    )));
    render_graph.add_pass(Box::new(TextPass::new(
        &backend.device,
        &backend.queue,
        backend.config.format,
        backend.config.width,
        backend.config.height,
    )));

    event_loop.set_control_flow(ControlFlow::Wait);

    let _ = event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { event, window_id } if window_id == asteria_window.window.id() => {
                match event {
                    WindowEvent::CloseRequested => elwt.exit(),

                    WindowEvent::Resized(physical_size) => {
                        backend.resize(physical_size);

                        let vp_w = backend.config.width as f32;
                        let vp_h = backend.config.height as f32;

                        if let Some(rp) = render_graph.find_pass_mut::<RectPass>() {
                            rp.update_viewport(&backend.queue, vp_w, vp_h);
                        }

                        if let Some(tp) = render_graph.find_pass_mut::<TextPass>() {
                            tp.resize(backend.config.width, backend.config.height);
                        }

                        // Rebuild scene synchronously on resize to adapt to new layout width
                        scene = asteria_window.build_active_scene(None);

                        needs_redraw = true;
                        asteria_window.window.request_redraw();
                    }

                    WindowEvent::ModifiersChanged(modifiers) => {
                        asteria_window.modifiers = modifiers.state();
                    }

                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                logical_key,
                                state: ElementState::Pressed,
                                ref text,
                                ..
                            },
                        ..
                    } => {
                        let ctrl = asteria_window.modifiers.control_key();
                        let alt = asteria_window.modifiers.alt_key();

                        let mut handled = false;

                        // ── Global shortcuts (always active) ─────────
                        match (ctrl, alt, &logical_key) {
                            // Ctrl + T: New Tab
                            (true, false, Key::Character(c)) if c.eq_ignore_ascii_case("t") => {
                                let _ = asteria_window
                                    .tab_manager
                                    .handle_event(ShellEvent::NewTab("<sample>".to_string()));
                                shell_ui.sync_with_tab_url(
                                    &asteria_window.tab_manager.active_tab().url,
                                );
                                handled = true;
                            }
                            // Ctrl + W: Close Active Tab
                            (true, false, Key::Character(c)) if c.eq_ignore_ascii_case("w") => {
                                let idx = asteria_window.tab_manager.active_tab_index;
                                let _ = asteria_window
                                    .tab_manager
                                    .handle_event(ShellEvent::CloseTab(idx));
                                shell_ui.sync_with_tab_url(
                                    &asteria_window.tab_manager.active_tab().url,
                                );
                                handled = true;
                            }
                            // Ctrl + L: Focus Omnibox
                            (true, false, Key::Character(c)) if c.eq_ignore_ascii_case("l") => {
                                shell_ui.focus_omnibox();
                                needs_redraw = true;
                                asteria_window.window.request_redraw();
                                return;
                            }
                            // Alt + LeftArrow: Go Back
                            (false, true, Key::Named(NamedKey::ArrowLeft)) => {
                                let _ = asteria_window.tab_manager.handle_event(ShellEvent::GoBack);
                                shell_ui.sync_with_tab_url(
                                    &asteria_window.tab_manager.active_tab().url,
                                );
                                handled = true;
                            }
                            // Alt + RightArrow: Go Forward
                            (false, true, Key::Named(NamedKey::ArrowRight)) => {
                                let _ = asteria_window
                                    .tab_manager
                                    .handle_event(ShellEvent::GoForward);
                                shell_ui.sync_with_tab_url(
                                    &asteria_window.tab_manager.active_tab().url,
                                );
                                handled = true;
                            }
                            // Ctrl + R or F5: Reload
                            (true, false, Key::Character(c)) if c.eq_ignore_ascii_case("r") => {
                                let _ = asteria_window.tab_manager.handle_event(ShellEvent::Reload);
                                handled = true;
                            }
                            (false, false, Key::Named(NamedKey::F5)) => {
                                let _ = asteria_window.tab_manager.handle_event(ShellEvent::Reload);
                                handled = true;
                            }
                            _ => {}
                        }

                        // ── Omnibox text editing (when focused) ──────
                        if !handled && shell_ui.omnibox_focused {
                            match &logical_key {
                                Key::Named(NamedKey::Enter) => {
                                    let url = shell_ui.omnibox_text.clone();
                                    shell_ui.unfocus_omnibox();
                                    if let Err(e) = asteria_window.tab_manager.navigate(&url) {
                                        shell_ui.status_message = Some(format!("Error: {}", e));
                                    } else {
                                        shell_ui.sync_with_tab_url(
                                            &asteria_window.tab_manager.active_tab().url,
                                        );
                                    }
                                    handled = true;
                                }
                                Key::Named(NamedKey::Escape) => {
                                    shell_ui.omnibox_text =
                                        asteria_window.tab_manager.active_tab().url.clone();
                                    shell_ui.unfocus_omnibox();
                                    needs_redraw = true;
                                    asteria_window.window.request_redraw();
                                    return;
                                }
                                Key::Named(NamedKey::Backspace) => {
                                    shell_ui.backspace();
                                    needs_redraw = true;
                                    asteria_window.window.request_redraw();
                                    return;
                                }
                                Key::Named(NamedKey::Delete) => {
                                    shell_ui.delete_forward();
                                    needs_redraw = true;
                                    asteria_window.window.request_redraw();
                                    return;
                                }
                                Key::Named(NamedKey::ArrowLeft) => {
                                    shell_ui.cursor_left();
                                    needs_redraw = true;
                                    asteria_window.window.request_redraw();
                                    return;
                                }
                                Key::Named(NamedKey::ArrowRight) => {
                                    shell_ui.cursor_right();
                                    needs_redraw = true;
                                    asteria_window.window.request_redraw();
                                    return;
                                }
                                Key::Named(NamedKey::Home) => {
                                    shell_ui.cursor_home();
                                    needs_redraw = true;
                                    asteria_window.window.request_redraw();
                                    return;
                                }
                                Key::Named(NamedKey::End) => {
                                    shell_ui.cursor_end();
                                    needs_redraw = true;
                                    asteria_window.window.request_redraw();
                                    return;
                                }
                                _ => {
                                    // Handle character input
                                    if let Some(txt) = text {
                                        let s = txt.as_str();
                                        if !s.is_empty() && !ctrl && !alt {
                                            shell_ui.insert_str(s);
                                            needs_redraw = true;
                                            asteria_window.window.request_redraw();
                                            return;
                                        }
                                    }
                                }
                            }
                        }

                        if handled {
                            scene = asteria_window.build_active_scene(Some(proxy.clone()));
                            current_scroll_y = 0.0;
                            target_scroll_y = 0.0;
                            needs_redraw = true;
                            asteria_window.window.request_redraw();
                        }
                    }

                    WindowEvent::CursorMoved { position, .. } => {
                        cursor_pos = (position.x as f32, position.y as f32);
                        let window_h = asteria_window.window.inner_size().height as f32;
                        let tab_count = asteria_window.tab_manager.tabs.len();

                        // Hit test against shell UI first
                        let shell_target = shell_ui.hit_test(
                            cursor_pos.0,
                            cursor_pos.1,
                            asteria_window.window.inner_size().width as f32,
                            window_h,
                            tab_count,
                        );

                        let old_hover = shell_ui.hovered_target;
                        shell_ui.hovered_target = Some(shell_target);

                        if shell_target == ShellHitTarget::Webpage {
                            // Map cursor into webpage coordinate space
                            let page_y = cursor_pos.1 - HEADER_HEIGHT + current_scroll_y;
                            let new_hover = scene.hit_test(cursor_pos.0, page_y);

                            // Update link preview in status bar
                            shell_ui.hovered_link_url = new_hover
                                .and_then(|nid| scene.node_url(nid).map(|s| s.to_string()));

                            if new_hover != hovered_node {
                                if let Some(old_id) = hovered_node {
                                    scene.set_node_state(old_id, NodeState::Normal);
                                }
                                if let Some(new_id) = new_hover {
                                    scene.set_node_state(new_id, NodeState::Hovered);
                                }
                                hovered_node = new_hover;
                                needs_redraw = true;
                                asteria_window.window.request_redraw();
                            }
                        } else {
                            // Clear webpage hover when cursor is over chrome
                            shell_ui.hovered_link_url = None;
                            if let Some(old_id) = hovered_node.take() {
                                scene.set_node_state(old_id, NodeState::Normal);
                            }
                        }

                        // Redraw if shell hover target changed
                        if old_hover != shell_ui.hovered_target {
                            needs_redraw = true;
                            asteria_window.window.request_redraw();
                        }
                    }

                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    } => {
                        let window_w = asteria_window.window.inner_size().width as f32;
                        let window_h = asteria_window.window.inner_size().height as f32;
                        let tab_count = asteria_window.tab_manager.tabs.len();
                        let shell_target = shell_ui.hit_test(
                            cursor_pos.0,
                            cursor_pos.1,
                            window_w,
                            window_h,
                            tab_count,
                        );

                        let mut rebuild_scene = false;

                        match shell_target {
                            ShellHitTarget::Tab(idx) => {
                                asteria_window.tab_manager.switch_tab(idx);
                                shell_ui.sync_with_tab_url(
                                    &asteria_window.tab_manager.active_tab().url,
                                );
                                shell_ui.unfocus_omnibox();
                                rebuild_scene = true;
                            }
                            ShellHitTarget::TabClose(idx) => {
                                asteria_window.tab_manager.close_tab(idx);
                                shell_ui.sync_with_tab_url(
                                    &asteria_window.tab_manager.active_tab().url,
                                );
                                shell_ui.unfocus_omnibox();
                                rebuild_scene = true;
                            }
                            ShellHitTarget::NewTab => {
                                let _ = asteria_window
                                    .tab_manager
                                    .handle_event(ShellEvent::NewTab("<sample>".to_string()));
                                shell_ui.sync_with_tab_url(
                                    &asteria_window.tab_manager.active_tab().url,
                                );
                                shell_ui.unfocus_omnibox();
                                rebuild_scene = true;
                            }
                            ShellHitTarget::Back => {
                                let _ = asteria_window.tab_manager.handle_event(ShellEvent::GoBack);
                                shell_ui.sync_with_tab_url(
                                    &asteria_window.tab_manager.active_tab().url,
                                );
                                rebuild_scene = true;
                            }
                            ShellHitTarget::Forward => {
                                let _ = asteria_window
                                    .tab_manager
                                    .handle_event(ShellEvent::GoForward);
                                shell_ui.sync_with_tab_url(
                                    &asteria_window.tab_manager.active_tab().url,
                                );
                                rebuild_scene = true;
                            }
                            ShellHitTarget::Reload => {
                                let _ = asteria_window.tab_manager.handle_event(ShellEvent::Reload);
                                rebuild_scene = true;
                            }
                            ShellHitTarget::Home => {
                                let _ = asteria_window.tab_manager.navigate("<sample>");
                                shell_ui.sync_with_tab_url(
                                    &asteria_window.tab_manager.active_tab().url,
                                );
                                rebuild_scene = true;
                            }
                            ShellHitTarget::Omnibox => {
                                shell_ui.focus_omnibox();
                                needs_redraw = true;
                                asteria_window.window.request_redraw();
                            }
                            ShellHitTarget::OmniboxGo => {
                                let url = shell_ui.omnibox_text.clone();
                                shell_ui.unfocus_omnibox();
                                if let Err(e) = asteria_window.tab_manager.navigate(&url) {
                                    shell_ui.status_message = Some(format!("Error: {}", e));
                                } else {
                                    shell_ui.sync_with_tab_url(
                                        &asteria_window.tab_manager.active_tab().url,
                                    );
                                }
                                rebuild_scene = true;
                            }
                            ShellHitTarget::Webpage => {
                                shell_ui.unfocus_omnibox();
                                // Hit test webpage scene
                                let page_y = cursor_pos.1 - HEADER_HEIGHT + current_scroll_y;
                                if let Some(node_id) = scene.hit_test(cursor_pos.0, page_y) {
                                    scene.set_node_state(node_id, NodeState::Active);

                                    if let Some(url) = scene.node_url(node_id) {
                                        println!(
                                            "[ASTERIA NAV] Link Clicked → Target URL: {}",
                                            url
                                        );
                                        if let Err(e) = asteria_window.tab_manager.navigate(url) {
                                            shell_ui.status_message =
                                                Some(format!("Navigation failed: {}", e));
                                        } else {
                                            shell_ui.sync_with_tab_url(
                                                &asteria_window.tab_manager.active_tab().url,
                                            );
                                            rebuild_scene = true;
                                        }
                                    }
                                    needs_redraw = true;
                                    asteria_window.window.request_redraw();
                                }
                            }
                            ShellHitTarget::Settings | ShellHitTarget::StatusBar => {
                                // No-op for now
                            }
                        }

                        if rebuild_scene {
                            scene = asteria_window.build_active_scene(Some(proxy.clone()));
                            current_scroll_y = 0.0;
                            target_scroll_y = 0.0;
                            needs_redraw = true;
                            asteria_window.window.request_redraw();
                        }
                    }

                    WindowEvent::MouseWheel { delta, .. } => {
                        // Only scroll if cursor is in webpage area
                        if cursor_pos.1 > HEADER_HEIGHT
                            && cursor_pos.1 < (backend.config.height as f32 - STATUS_BAR_HEIGHT)
                        {
                            let scroll_dy = match delta {
                                MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                                MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                            };

                            let viewport_h =
                                (backend.config.height as f32 - HEADER_HEIGHT - STATUS_BAR_HEIGHT)
                                    .max(100.0);
                            let max_scroll = max_scroll_for_scene(&scene, viewport_h);
                            target_scroll_y = (target_scroll_y - scroll_dy).clamp(0.0, max_scroll);

                            needs_redraw = true;
                            asteria_window.window.request_redraw();
                        }
                    }

                    WindowEvent::RedrawRequested => {
                        let dirty_segs = scene.dirty_segments();
                        if dirty_segs.is_empty() && !needs_redraw {
                            return;
                        }
                        needs_redraw = false;

                        let vp_w = asteria_window.window.inner_size().width as f32;
                        let vp_h = asteria_window.window.inner_size().height as f32;

                        // Build webpage rect batch (canvas fill, offset by HEADER_HEIGHT, and clipped)
                        let mut new_batch = build_rect_batch(&scene, current_scroll_y, vp_w, vp_h);

                        // Overlay shell chrome rects on top
                        shell_ui.render_shell_rects(
                            &mut new_batch,
                            vp_w,
                            vp_h,
                            &asteria_window.tab_manager,
                        );

                        if let Some(rp) = render_graph.find_pass_mut::<RectPass>() {
                            rp.update_viewport(&backend.queue, vp_w, vp_h);
                            rp.update_buffers(&backend.device, &new_batch);
                        }

                        if let Some(tp) = render_graph.find_pass_mut::<TextPass>() {
                            // Populate webpage text (offset and clipped to content viewport)
                            populate_text_pass(tp, &scene, current_scroll_y, vp_w, vp_h);

                            // Overlay shell chrome text on top
                            let can_back = asteria_window
                                .tab_manager
                                .active_tab()
                                .history
                                .can_go_back();
                            let can_fwd = asteria_window
                                .tab_manager
                                .active_tab()
                                .history
                                .can_go_forward();
                            shell_ui.render_shell_text(
                                tp,
                                vp_w,
                                vp_h,
                                &asteria_window.tab_manager,
                                can_back,
                                can_fwd,
                            );
                        }

                        render_graph.prepare(&backend.device, &backend.queue);

                        let frame = match backend.surface.get_current_texture() {
                            Ok(frame) => frame,
                            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                let size = backend.size;
                                backend.resize(size);
                                needs_redraw = true;
                                asteria_window.window.request_redraw();
                                return;
                            }
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                eprintln!("[ASTERIA] GPU out of memory; exiting");
                                elwt.exit();
                                return;
                            }
                            Err(err) => {
                                eprintln!("[ASTERIA] surface error: {err:?}");
                                return;
                            }
                        };

                        let view = frame
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());

                        let mut encoder = backend.device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor {
                                label: Some("Asteria Frame Encoder"),
                            },
                        );

                        {
                            let mut rpass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("Asteria Render Pass"),
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view: &view,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            // Dark background matching shell chrome
                                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                                r: 0.067,
                                                g: 0.067,
                                                b: 0.106,
                                                a: 1.0,
                                            }),
                                            store: wgpu::StoreOp::Store,
                                        },
                                    })],
                                    depth_stencil_attachment: None,
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                });

                            render_graph.render(&mut rpass);
                        }

                        backend.queue.submit(std::iter::once(encoder.finish()));
                        frame.present();

                        scene.clear_dirty();
                    }

                    _ => {}
                }
            }
            Event::AboutToWait => {
                let diff = target_scroll_y - current_scroll_y;
                if diff.abs() > 0.5 {
                    current_scroll_y += diff * 0.15;
                    needs_redraw = true;
                    asteria_window.window.request_redraw();
                } else if diff.abs() > 0.0 {
                    current_scroll_y = target_scroll_y;
                    needs_redraw = true;
                    asteria_window.window.request_redraw();
                }
            }
            Event::UserEvent(AsteriaUserEvent::SceneUpdated(new_scene)) => {
                scene = new_scene;
                needs_redraw = true;
                asteria_window.window.request_redraw();
            }
            _ => {}
        }
    });
}
