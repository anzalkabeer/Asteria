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
    event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::renderer::backend::wgpu_backend::WgpuBackend;
use crate::renderer::commands::batch_builder::BatchBuilder;
use crate::renderer::commands::command_builder::{CommandBuilder, RenderCommand};
use crate::renderer::graph::render_graph::RenderGraph;
use crate::renderer::passes::image_pass::ImagePass;
use crate::renderer::passes::rect_pass::RectPass;
use crate::renderer::passes::text_pass::TextPass;
use crate::renderer::scheduler::batching::BatchPlanner;
use crate::scene::{NodeState, SceneGraph, SceneNodeId};

// ─── Pass indices in the RenderGraph ──────────────────────────────
// These constants define the Z-order of GPU passes:
//   Backgrounds (0) → Images (1) → Text (2)
const RECT_PASS_INDEX: usize = 0;
const TEXT_PASS_INDEX: usize = 1;

pub struct AsteriaWindow {
    pub window: Arc<Window>,
}

impl AsteriaWindow {
    pub fn new(event_loop: &EventLoop<()>, width: u32, height: u32) -> Self {
        let window = WindowBuilder::new()
            .with_title("Asteria Engine")
            .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
            .build(event_loop)
            .expect("Failed to create winit window");

        Self {
            window: Arc::new(window),
        }
    }
}

// ─── Batch Builder Helper ─────────────────────────────────────────

fn build_rect_batch(scene: &SceneGraph, scroll_y: f32) -> BatchBuilder {
    // Build commands from scene, applying scroll offset on Y axis
    let mut cmd_builder = CommandBuilder::new();
    cmd_builder.build_from_scene(scene);

    // Apply scroll offset to all commands
    let scrolled: Vec<_> = cmd_builder
        .commands
        .iter()
        .map(|c| match c {
            RenderCommand::SolidRect { rect, rgba } => {
                let mut r = *rect;
                r[1] -= scroll_y; // Apply scroll offset on Y
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
        batch.build_dirty_batches(&owned, scene);
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

/// Calculate the maximum scroll offset based on the tallest node
/// in the scene, minus the viewport height. Prevents scrolling
/// past content into empty white space.
fn max_scroll_for_scene(scene: &SceneGraph, viewport_height: f32) -> f32 {
    let max_y = scene
        .nodes
        .iter()
        .map(|n| n.rect.y + n.rect.height)
        .fold(0.0_f32, f32::max);
    (max_y - viewport_height).max(0.0)
}

// ─── Main Window Loop ─────────────────────────────────────────────

pub fn run_window_loop(mut scene: SceneGraph, mut tab_manager: crate::shell::TabManager) {
    let event_loop = EventLoop::new().expect("Failed to create EventLoop");
    let asteria_window = AsteriaWindow::new(&event_loop, 800, 600);

    // Initialize WgpuBackend (Device, Queue, Surface)
    let mut backend = pollster::block_on(WgpuBackend::new(asteria_window.window.clone()));

    // ─── Phase 9C & Milestone 10: Interactive State ──────────────
    let mut cursor_pos: (f32, f32) = (0.0, 0.0);
    let mut current_scroll_y: f32 = 0.0;
    let mut target_scroll_y: f32 = 0.0;
    let mut hovered_node: Option<SceneNodeId> = None;
    let mut needs_redraw = true;

    // ─── RenderGraph — GPU passes created ONCE, data rebuilt per frame ───
    // Pass order: RectPass[0] → ImagePass[1] → TextPass[2]
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
                    // ─── Window Close ────────────────────────────
                    WindowEvent::CloseRequested => elwt.exit(),

                    // ─── Window Resize ───────────────────────────
                    // Rebuild TextPass with new viewport dimensions
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

                    // ─── Mouse Cursor Movement (Hover / Hit Test) ─
                    // Milestone 10: Update NodeState::Hovered & invalidate node.
                    WindowEvent::CursorMoved { position, .. } => {
                        cursor_pos = (position.x as f32, position.y as f32);

                        // Adjust for scroll offset when hit-testing
                        let hit_y = cursor_pos.1 + current_scroll_y;
                        let new_hover = scene.hit_test(cursor_pos.0, hit_y);

                        if new_hover != hovered_node {
                            // Restore previously hovered node to Normal state
                            if let Some(old_id) = hovered_node {
                                scene.set_node_state(old_id, NodeState::Normal);
                            }
                            // Set newly hovered node to Hovered state
                            if let Some(new_id) = new_hover {
                                scene.set_node_state(new_id, NodeState::Hovered);
                            }
                            hovered_node = new_hover;
                            needs_redraw = true;
                            asteria_window.window.request_redraw();
                        }
                    }

                    // ─── Mouse Click (Select / Activate & Link Navigation) ─
                    // Milestone 10: Update NodeState::Active & trigger link dispatch.
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    } => {
                        let hit_y = cursor_pos.1 + current_scroll_y;
                        if let Some(node_id) = scene.hit_test(cursor_pos.0, hit_y) {
                            scene.set_node_state(node_id, NodeState::Active);

                            // Check for HTML <a> link URL
                            if let Some(url) = scene.node_url(node_id) {
                                println!("[ASTERIA NAV] Link Clicked → Target URL: {}", url);
                                // Milestone 10 Component 3: Trigger TabManager navigation
                                if let Err(e) = tab_manager.navigate(url) {
                                    eprintln!("Navigation failed: {}", e);
                                }
                            } else {
                                let kind_label = match scene.node_kind(node_id) {
                                    Some(crate::scene::SceneNodeKind::SolidRect) => "SolidRect",
                                    Some(crate::scene::SceneNodeKind::Text { .. }) => "Text",
                                    Some(crate::scene::SceneNodeKind::Image) => "Image",
                                    Some(crate::scene::SceneNodeKind::Border { .. }) => "Border",
                                    Some(crate::scene::SceneNodeKind::Container) => "Container",
                                    None => "Unknown",
                                };
                                println!(
                                    "[ASTERIA] Click @ ({:.0},{:.0}) → Node {:?} [{}]",
                                    cursor_pos.0, cursor_pos.1, node_id, kind_label,
                                );
                            }

                            needs_redraw = true;
                            asteria_window.window.request_redraw();
                        }
                    }

                    // ─── Mouse Scroll (Viewport Scroll) ──────────
                    // Phase 9C: Accumulate scroll delta, clamp to [0, max_scroll].
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

                    // ─── Frame Render ─────────────────────────────
                    // Milestone 10: Skip GPU submission if 0 tiles are dirty!
                    WindowEvent::RedrawRequested => {
                        let dirty_segs = scene.dirty_segments();
                        if dirty_segs.is_empty() && !needs_redraw {
                            return; // 0% GPU / VRAM usage when scene is clean!
                        }
                        needs_redraw = false;

                        // ── Update pass data via RenderGraph downcast ──
                        // RectPass: rebuild vertex/index buffers with scroll offset
                        let new_batch = build_rect_batch(&scene, current_scroll_y);
                        if let Some(rp) =
                            render_graph.pass_downcast_mut::<RectPass>(RECT_PASS_INDEX)
                        {
                            rp.update_buffers(&backend.device, &new_batch);
                        }

                        // TextPass: repopulate glyph buffers with scroll offset
                        if let Some(tp) =
                            render_graph.pass_downcast_mut::<TextPass>(TEXT_PASS_INDEX)
                        {
                            populate_text_pass(tp, &scene, current_scroll_y);
                        }

                        // ── RenderGraph.prepare() → shape glyphs, upload buffers ──
                        render_graph.prepare(&backend.device, &backend.queue);

                        // ── GPU Frame Submission ──────────────────────
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

                            // ── RenderGraph.render() → execute all passes in Z-order ──
                            render_graph.render(&mut rpass);
                        }

                        backend.queue.submit(std::iter::once(encoder.finish()));
                        frame.present();

                        // Clear dirty flags after successful render
                        scene.clear_dirty();
                    }

                    _ => {}
                }
            }
            Event::AboutToWait => {
                // Smooth scroll interpolation
                let diff = target_scroll_y - current_scroll_y;
                if diff.abs() > 0.5 {
                    current_scroll_y += diff * 0.15; // smooth factor
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
