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
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowBuilder},
};

use crate::css_parser::Stylesheet;
use crate::parser::Parser;
use crate::renderer::backend::wgpu_backend::WgpuBackend;
use crate::renderer::commands::batch_builder::BatchBuilder;
use crate::renderer::commands::command_builder::{CommandBuilder, RenderCommand};
use crate::renderer::graph::render_graph::RenderGraph;
use crate::renderer::passes::image_pass::ImagePass;
use crate::renderer::passes::rect_pass::RectPass;
use crate::renderer::passes::text_pass::TextPass;
use crate::renderer::scheduler::batching::BatchPlanner;
use crate::scene::{NodeState, SceneGraph, SceneNodeId, build_scene_graph};
use crate::scheduler::ThreadedScheduler;
use crate::shell::{ShellEvent, TabManager};
use crate::tokenizer::Tokenizer;

// ─── Pass indices in the RenderGraph ──────────────────────────────
// These constants define the Z-order of GPU passes:
//   Backgrounds (0) → Images (1) → Text (2)
const RECT_PASS_INDEX: usize = 0;
const TEXT_PASS_INDEX: usize = 1;

pub struct AsteriaWindow {
    pub window: Arc<Window>,
    pub tab_manager: TabManager,
    pub scheduler: ThreadedScheduler,
    pub modifiers: ModifiersState,
}

impl AsteriaWindow {
    pub fn new(event_loop: &EventLoop<()>, width: u32, height: u32) -> Self {
        Self::with_tab_manager(event_loop, width, height, TabManager::new())
    }

    pub fn with_tab_manager(
        event_loop: &EventLoop<()>,
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

    pub fn build_active_scene(&mut self) -> SceneGraph {
        let size = self.window.inner_size();
        let viewport_w = size.width as f32;
        let viewport_h = size.height as f32;

        let sample_html_bytes = b"<!DOCTYPE html><html><head><style>body { background-color: #1e1e2e; color: #cdd6f4; } h1 { color: #89b4fa; font-size: 24px; } p { color: #a6adc8; font-size: 16px; } div { background-color: #313244; }</style></head><body><h1>Asteria Browser Engine</h1><p>Hardware-accelerated GPU renderer running with wgpu + winit.</p><div><p>Interactive Viewport: Scroll, Hover, Click supported!</p></div></body></html>";

        let active_tab = self.tab_manager.active_tab();
        let html_bytes = active_tab
            .page_resources
            .as_ref()
            .map(|r| r.html.bytes.as_slice())
            .unwrap_or(sample_html_bytes);

        let mut tokenizer = Tokenizer::new(html_bytes);
        let tokens = tokenizer.tokenize();
        let dom = Parser::new(&tokens, html_bytes).parse();

        let css_bytes = active_tab
            .page_resources
            .as_ref()
            .and_then(|r| r.stylesheets.first())
            .map(|c| c.bytes.as_slice())
            .unwrap_or(b"");

        let stylesheet = Stylesheet::parse(css_bytes);
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

fn build_rect_batch(scene: &SceneGraph, scroll_y: f32, vp_w: f32) -> BatchBuilder {
    let mut cmd_builder = CommandBuilder::new();
    cmd_builder.build_from_scene(scene);

    let scrolled: Vec<_> = cmd_builder
        .commands
        .iter()
        .map(|c| match c {
            RenderCommand::SolidRect { rect, rgba } => {
                let mut r = *rect;
                r[1] -= scroll_y;
                RenderCommand::SolidRect {
                    rect: r,
                    rgba: *rgba,
                }
            }
            RenderCommand::Text {
                text,
                rect,
                rgba,
                font_size,
            } => {
                let mut r = *rect;
                r[1] -= scroll_y;
                RenderCommand::Text {
                    text: text.clone(),
                    rect: r,
                    rgba: *rgba,
                    font_size: *font_size,
                }
            }
        })
        .collect();

    let planned = BatchPlanner::plan(&scrolled);
    let mut batch = BatchBuilder::new();
    if let Some(rect_cmds) = planned.first() {
        let owned: Vec<RenderCommand> = rect_cmds.iter().map(|&c| c.clone()).collect();
        batch.append_batches(&owned, vp_w);
    }
    batch
}

// ─── Text Pass Population ─────────────────────────────────────────

fn populate_text_pass(text_pass: &mut TextPass, scene: &SceneGraph, scroll_y: f32) {
    text_pass.clear();
    for (i, node) in scene.nodes.iter().enumerate() {
        if let crate::scene::SceneNodeKind::Text { font_size } = node.kind
            && let Some(text_run) = &scene.texts[i]
        {
            let color = scene.colors[i];
            let y = node.rect.y - scroll_y;
            text_pass.add_text(&text_run.text, [node.rect.x, y], font_size, color);
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
    let event_loop = EventLoop::new().expect("Failed to create EventLoop");
    let mut asteria_window = AsteriaWindow::with_tab_manager(&event_loop, 800, 600, tab_manager);

    let mut backend = pollster::block_on(WgpuBackend::new(asteria_window.window.clone()));

    let mut scene = if initial_scene.nodes.is_empty() {
        asteria_window.build_active_scene()
    } else {
        initial_scene
    };

    let mut cursor_pos: (f32, f32) = (0.0, 0.0);
    let mut current_scroll_y: f32 = 0.0;
    let mut target_scroll_y: f32 = 0.0;
    let mut hovered_node: Option<SceneNodeId> = None;
    let mut needs_redraw = true;

    let mut render_graph = RenderGraph::new();
    render_graph.add_pass(Box::new(RectPass::new(
        &backend.device,
        backend.config.format,
    )));
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

                        if let Some(tp) =
                            render_graph.pass_downcast_mut::<TextPass>(TEXT_PASS_INDEX)
                        {
                            tp.resize(backend.config.width, backend.config.height);
                        }

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
                                ..
                            },
                        ..
                    } => {
                        let ctrl = asteria_window.modifiers.control_key();
                        let alt = asteria_window.modifiers.alt_key();

                        let mut handled = false;
                        match (ctrl, alt, &logical_key) {
                            // Ctrl + T: New Tab
                            (true, false, Key::Character(c)) if c.eq_ignore_ascii_case("t") => {
                                let _ = asteria_window
                                    .tab_manager
                                    .handle_event(ShellEvent::NewTab("<sample>".to_string()));
                                handled = true;
                            }
                            // Ctrl + W: Close Active Tab
                            (true, false, Key::Character(c)) if c.eq_ignore_ascii_case("w") => {
                                let idx = asteria_window.tab_manager.active_tab_index;
                                let _ = asteria_window
                                    .tab_manager
                                    .handle_event(ShellEvent::CloseTab(idx));
                                handled = true;
                            }
                            // Alt + LeftArrow: Go Back
                            (false, true, Key::Named(NamedKey::ArrowLeft)) => {
                                let _ = asteria_window.tab_manager.handle_event(ShellEvent::GoBack);
                                handled = true;
                            }
                            // Alt + RightArrow: Go Forward
                            (false, true, Key::Named(NamedKey::ArrowRight)) => {
                                let _ = asteria_window
                                    .tab_manager
                                    .handle_event(ShellEvent::GoForward);
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

                        if handled {
                            scene = asteria_window.build_active_scene();
                            current_scroll_y = 0.0;
                            target_scroll_y = 0.0;
                            needs_redraw = true;
                            asteria_window.window.request_redraw();
                        }
                    }

                    WindowEvent::CursorMoved { position, .. } => {
                        cursor_pos = (position.x as f32, position.y as f32);

                        let hit_y = cursor_pos.1 + current_scroll_y;
                        let new_hover = scene.hit_test(cursor_pos.0, hit_y);

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
                    }

                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    } => {
                        let hit_y = cursor_pos.1 + current_scroll_y;
                        if let Some(node_id) = scene.hit_test(cursor_pos.0, hit_y) {
                            scene.set_node_state(node_id, NodeState::Active);

                            if let Some(url) = scene.node_url(node_id) {
                                println!("[ASTERIA NAV] Link Clicked → Target URL: {}", url);
                                if let Err(e) = asteria_window.tab_manager.navigate(url) {
                                    eprintln!("Navigation failed: {}", e);
                                } else {
                                    scene = asteria_window.build_active_scene();
                                    current_scroll_y = 0.0;
                                    target_scroll_y = 0.0;
                                }
                            }
                            needs_redraw = true;
                            asteria_window.window.request_redraw();
                        }
                    }

                    WindowEvent::MouseWheel { delta, .. } => {
                        let scroll_dy = match delta {
                            MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                            MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                        };

                        let max_scroll = max_scroll_for_scene(&scene, backend.config.height as f32);
                        target_scroll_y = (target_scroll_y - scroll_dy).clamp(0.0, max_scroll);

                        needs_redraw = true;
                        asteria_window.window.request_redraw();
                    }

                    WindowEvent::RedrawRequested => {
                        let dirty_segs = scene.dirty_segments();
                        if dirty_segs.is_empty() && !needs_redraw {
                            return;
                        }
                        needs_redraw = false;

                        let vp_w = asteria_window.window.inner_size().width as f32;
                        let new_batch = build_rect_batch(&scene, current_scroll_y, vp_w);
                        if let Some(rp) =
                            render_graph.pass_downcast_mut::<RectPass>(RECT_PASS_INDEX)
                        {
                            rp.update_buffers(&backend.device, &new_batch);
                        }

                        if let Some(tp) =
                            render_graph.pass_downcast_mut::<TextPass>(TEXT_PASS_INDEX)
                        {
                            populate_text_pass(tp, &scene, current_scroll_y);
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
                                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                                r: 1.0,
                                                g: 1.0,
                                                b: 1.0,
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
            _ => {}
        }
    });
}
