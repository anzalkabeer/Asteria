use std::sync::Arc;

use winit::event::{ElementState, Event, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoopBuilder};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::Window;

use crate::app::ShellWindow;
use crate::renderer::backend::wgpu_backend::WgpuBackend;
use crate::renderer::commands::batch_builder::BatchBuilder;
use crate::renderer::commands::command_builder::{CommandBuilder, RenderCommand};
use crate::renderer::graph::render_graph::RenderGraph;
use crate::renderer::passes::image_pass::ImagePass;
use crate::renderer::passes::rect_pass::RectPass;
use crate::renderer::passes::text_pass::TextPass;
use crate::renderer::window::chrome::{ChromeAction, ChromeShell};
use crate::scene::{SceneGraph, SceneNodeId};
use crate::scheduler::ThreadedScheduler;
use crate::shell::{ShellEvent, TabManager};
use crate::viewport::EngineViewport;

pub const RECT_PASS_INDEX: usize = 0;
pub const IMAGE_PASS_INDEX: usize = 1;
pub const TEXT_PASS_INDEX: usize = 2;

pub enum AsteriaUserEvent {
    SceneUpdated(SceneGraph),
}

pub struct AsteriaWindow {
    pub shell: ShellWindow,
    pub viewport: EngineViewport,
    pub scheduler: ThreadedScheduler,
    pub modifiers: ModifiersState,
    pub frame_counter: u64,
}

impl AsteriaWindow {
    pub fn window(&self) -> &Arc<Window> {
        &self.shell.window
    }

    pub fn tab_manager(&self) -> &TabManager {
        &self.shell.tab_manager
    }

    pub fn tab_manager_mut(&mut self) -> &mut TabManager {
        &mut self.shell.tab_manager
    }

    pub fn chrome(&self) -> &ChromeShell {
        &self.shell.chrome
    }

    pub fn chrome_mut(&mut self) -> &mut ChromeShell {
        &mut self.shell.chrome
    }

    pub fn new(event_loop: &winit::event_loop::EventLoop<AsteriaUserEvent>) -> Self {
        Self::with_tab_manager(event_loop, 960, 640, TabManager::new())
    }

    pub fn with_tab_manager(
        event_loop: &winit::event_loop::EventLoop<AsteriaUserEvent>,
        width: u32,
        height: u32,
        tab_manager: TabManager,
    ) -> Self {
        let shell = ShellWindow::new(
            event_loop,
            "ASTERIA // DIAGNOSTICS",
            width,
            height,
            tab_manager,
        );

        Self {
            shell,
            viewport: EngineViewport::new(),
            scheduler: ThreadedScheduler::new(4),
            modifiers: ModifiersState::default(),
            frame_counter: 0,
        }
    }
}

// ─── Batch Builder Helper ─────────────────────────────────────────

fn build_combined_rect_batch(
    chrome_scene: &SceneGraph,
    page_scene: &SceneGraph,
    scroll_y: f32,
    vp_w: f32,
) -> BatchBuilder {
    // 1. Page Content Rects (Scrolled)
    let mut scrolled_page = page_scene.clone();
    for node in &mut scrolled_page.nodes {
        node.rect.y -= scroll_y;
    }

    let mut page_cmd_builder = CommandBuilder::new();
    page_cmd_builder.build_from_scene(&scrolled_page);

    // 2. Chrome HUD Rects (Fixed)
    let mut chrome_cmd_builder = CommandBuilder::new();
    chrome_cmd_builder.build_from_scene(chrome_scene);

    let mut combined_rects: Vec<RenderCommand> = page_cmd_builder
        .commands
        .into_iter()
        .filter(|c| matches!(c, RenderCommand::SolidRect { .. }))
        .collect();

    let chrome_rects: Vec<RenderCommand> = chrome_cmd_builder
        .commands
        .into_iter()
        .filter(|c| matches!(c, RenderCommand::SolidRect { .. }))
        .collect();

    combined_rects.extend(chrome_rects);

    let mut batch = BatchBuilder::new();
    batch.append_batches(&combined_rects, vp_w);
    batch
}

// ─── Text Pass Population ─────────────────────────────────────────

fn populate_text_pass(
    text_pass: &mut TextPass,
    chrome_scene: &SceneGraph,
    page_scene: &SceneGraph,
    scroll_y: f32,
) {
    text_pass.clear();

    // 1. Page Content Text (Scrolled)
    for (i, node) in page_scene.nodes.iter().enumerate() {
        if let crate::scene::SceneNodeKind::Text { font_size } = node.kind
            && let Some(text_run) = &page_scene.texts[i]
        {
            let color = page_scene.colors[i];
            let y = node.rect.y - scroll_y;
            text_pass.add_text(&text_run.text, [node.rect.x, y], font_size, color);
        }
    }

    // 2. Chrome HUD Text (Fixed)
    for (i, node) in chrome_scene.nodes.iter().enumerate() {
        if let crate::scene::SceneNodeKind::Text { font_size } = node.kind
            && let Some(text_run) = &chrome_scene.texts[i]
        {
            let color = chrome_scene.colors[i];
            text_pass.add_text(&text_run.text, [node.rect.x, node.rect.y], font_size, color);
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

pub fn run_window_loop(_initial_scene: SceneGraph, tab_manager: TabManager) {
    let event_loop = EventLoopBuilder::<AsteriaUserEvent>::with_user_event()
        .build()
        .expect("Failed to create EventLoop");
    let mut asteria_window = AsteriaWindow::with_tab_manager(&event_loop, 960, 640, tab_manager);
    let mut backend = pollster::block_on(WgpuBackend::new(asteria_window.window().clone()));

    let mut cursor_pos: (f32, f32) = (0.0, 0.0);
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
            Event::WindowEvent { event, window_id } if window_id == asteria_window.window().id() => {
                match event {
                    WindowEvent::CloseRequested => elwt.exit(),

                    WindowEvent::Resized(physical_size) => {
                        if physical_size.width > 0 && physical_size.height > 0 {
                            backend.resize(physical_size);

                            if let Some(tp) =
                                render_graph.pass_downcast_mut::<TextPass>(TEXT_PASS_INDEX)
                            {
                                tp.resize(backend.config.width, backend.config.height);
                            }

                            needs_redraw = true;
                            asteria_window.window().request_redraw();
                        }
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

                        if asteria_window.chrome().is_address_bar_focused {
                            match logical_key {
                                Key::Named(NamedKey::Enter) => {
                                    let url = asteria_window.chrome().address_bar_buffer.clone();
                                    asteria_window.chrome_mut().is_address_bar_focused = false;
                                    if !url.is_empty() {
                                        let _ = asteria_window.tab_manager_mut().navigate(&url);
                                    }
                                    asteria_window.viewport.scroll_y = 0.0;
                                    asteria_window.viewport.target_scroll_y = 0.0;
                                    needs_redraw = true;
                                    asteria_window.window().request_redraw();
                                }
                                Key::Named(NamedKey::Escape) => {
                                    asteria_window.chrome_mut().is_address_bar_focused = false;
                                    needs_redraw = true;
                                    asteria_window.window().request_redraw();
                                }
                                Key::Named(NamedKey::Backspace) => {
                                    asteria_window.chrome_mut().address_bar_buffer.pop();
                                    needs_redraw = true;
                                    asteria_window.window().request_redraw();
                                }
                                Key::Character(c) => {
                                    asteria_window.chrome_mut().address_bar_buffer.push_str(c.as_str());
                                    needs_redraw = true;
                                    asteria_window.window().request_redraw();
                                }
                                _ => {}
                            }
                        } else {
                            let mut handled = false;
                            match (ctrl, alt, &logical_key) {
                                (true, false, Key::Character(c)) if c.eq_ignore_ascii_case("t") => {
                                    let _ = asteria_window
                                        .tab_manager_mut()
                                        .handle_event(ShellEvent::NewTab("<sample>".to_string()));
                                    handled = true;
                                }
                                (true, false, Key::Character(c)) if c.eq_ignore_ascii_case("w") => {
                                    let idx = asteria_window.tab_manager().active_tab_index;
                                    let _ = asteria_window
                                        .tab_manager_mut()
                                        .handle_event(ShellEvent::CloseTab(idx));
                                    handled = true;
                                }
                                (false, true, Key::Named(NamedKey::ArrowLeft)) => {
                                    let _ = asteria_window.tab_manager_mut().handle_event(ShellEvent::GoBack);
                                    handled = true;
                                }
                                (false, true, Key::Named(NamedKey::ArrowRight)) => {
                                    let _ = asteria_window
                                        .tab_manager_mut()
                                        .handle_event(ShellEvent::GoForward);
                                    handled = true;
                                }
                                (true, false, Key::Character(c)) if c.eq_ignore_ascii_case("r") => {
                                    let _ = asteria_window.tab_manager_mut().handle_event(ShellEvent::Reload);
                                    handled = true;
                                }
                                (false, false, Key::Named(NamedKey::F5)) => {
                                    let _ = asteria_window.tab_manager_mut().handle_event(ShellEvent::Reload);
                                    handled = true;
                                }
                                _ => {}
                            }

                            if handled {
                                asteria_window.viewport.scroll_y = 0.0;
                                asteria_window.viewport.target_scroll_y = 0.0;
                                needs_redraw = true;
                                asteria_window.window().request_redraw();
                            }
                        }
                    }

                    WindowEvent::CursorMoved { position, .. } => {
                        let scale_factor = asteria_window.window().scale_factor();
                        let logical_pos = position.to_logical::<f32>(scale_factor);
                        cursor_pos = (logical_pos.x, logical_pos.y);

                        let logical_size = asteria_window.window().inner_size().to_logical::<f32>(scale_factor);
                        let vp_w = logical_size.width;
                        let vp_h = logical_size.height;

                        let chrome_scene = asteria_window.chrome().build_chrome_scene(vp_w, vp_h, asteria_window.tab_manager());
                        let page_scene = asteria_window.viewport.build_page_scene(vp_w, vp_h, asteria_window.tab_manager());

                        let hit_y = if cursor_pos.1 <= 84.0 || cursor_pos.1 >= vp_h - 32.0 {
                            cursor_pos.1
                        } else {
                            cursor_pos.1 + asteria_window.viewport.scroll_y
                        };

                        let new_hover = if cursor_pos.1 <= 84.0 || cursor_pos.1 >= vp_h - 32.0 {
                            chrome_scene.hit_test(cursor_pos.0, hit_y)
                        } else {
                            page_scene.hit_test(cursor_pos.0, hit_y)
                        };

                        if new_hover != hovered_node {
                            hovered_node = new_hover;
                            needs_redraw = true;
                            asteria_window.window().request_redraw();
                        }
                    }

                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    } => {
                        let scale_factor = asteria_window.window().scale_factor();
                        let logical_size = asteria_window.window().inner_size().to_logical::<f32>(scale_factor);
                        let vp_w = logical_size.width;
                        let vp_h = logical_size.height;

                        let chrome_scene = asteria_window.chrome().build_chrome_scene(vp_w, vp_h, asteria_window.tab_manager());
                        let page_scene = asteria_window.viewport.build_page_scene(vp_w, vp_h, asteria_window.tab_manager());

                        if cursor_pos.1 <= 84.0 || cursor_pos.1 >= vp_h - 32.0 {
                            if let Some(node_id) = chrome_scene.hit_test(cursor_pos.0, cursor_pos.1) {
                                if let Some(url) = chrome_scene.node_url(node_id) {
                                    let (tab_mgr, chrome) = (&mut asteria_window.shell.tab_manager, &mut asteria_window.shell.chrome);
                                    match chrome.handle_action_click(url, tab_mgr) {
                                        Some(ChromeAction::RedrawRequired) => {
                                            needs_redraw = true;
                                            asteria_window.window().request_redraw();
                                        }
                                        Some(ChromeAction::MinimizeWindow) => {
                                            asteria_window.window().set_minimized(true);
                                        }
                                        Some(ChromeAction::MaximizeWindow) => {
                                            let is_max = asteria_window.window().is_maximized();
                                            asteria_window.window().set_maximized(!is_max);
                                        }
                                        Some(ChromeAction::CloseWindow) => {
                                            elwt.exit();
                                        }
                                        None => {}
                                    }
                                }
                            }
                        } else {
                            let hit_y = cursor_pos.1 + asteria_window.viewport.scroll_y;
                            if let Some(node_id) = page_scene.hit_test(cursor_pos.0, hit_y) {
                                if let Some(url) = page_scene.node_url(node_id) {
                                    if let Err(e) = asteria_window.tab_manager_mut().navigate(url) {
                                        eprintln!("Navigation failed: {}", e);
                                    } else {
                                        asteria_window.viewport.scroll_y = 0.0;
                                        asteria_window.viewport.target_scroll_y = 0.0;
                                        needs_redraw = true;
                                        asteria_window.window().request_redraw();
                                    }
                                }
                            }
                        }
                    }

                    WindowEvent::MouseWheel { delta, .. } => {
                        let scroll_dy = match delta {
                            MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                            MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                        };

                        let scale_factor = asteria_window.window().scale_factor();
                        let logical_size = asteria_window.window().inner_size().to_logical::<f32>(scale_factor);
                        let vp_w = logical_size.width;
                        let vp_h = logical_size.height;
                        let page_scene = asteria_window.viewport.build_page_scene(vp_w, vp_h, asteria_window.tab_manager());

                        let max_scroll = max_scroll_for_scene(&page_scene, vp_h - 84.0 - 32.0);
                        asteria_window.viewport.target_scroll_y = (asteria_window.viewport.target_scroll_y - scroll_dy).clamp(0.0, max_scroll);

                        needs_redraw = true;
                        asteria_window.window().request_redraw();
                    }

                    WindowEvent::RedrawRequested => {
                        if !needs_redraw {
                            return;
                        }
                        needs_redraw = false;

                        let scale_factor = asteria_window.window().scale_factor();
                        let logical_size = asteria_window.window().inner_size().to_logical::<f32>(scale_factor);
                        let vp_w = logical_size.width;
                        let vp_h = logical_size.height;

                        if vp_w <= 0.0 || vp_h <= 0.0 {
                            return;
                        }

                        let chrome_scene = asteria_window.chrome().build_chrome_scene(vp_w, vp_h, asteria_window.tab_manager());
                        let page_scene = asteria_window.viewport.build_page_scene(vp_w, vp_h, asteria_window.tab_manager());

                        let new_batch = build_combined_rect_batch(
                            &chrome_scene,
                            &page_scene,
                            asteria_window.viewport.scroll_y,
                            vp_w,
                        );

                        if let Some(rp) =
                            render_graph.pass_downcast_mut::<RectPass>(RECT_PASS_INDEX)
                        {
                            rp.update_buffers(&backend.device, &new_batch);
                        }

                        if let Some(tp) =
                            render_graph.pass_downcast_mut::<TextPass>(TEXT_PASS_INDEX)
                        {
                            populate_text_pass(tp, &chrome_scene, &page_scene, asteria_window.viewport.scroll_y);
                        }

                        render_graph.prepare(&backend.device, &backend.queue);

                        let frame = match backend.surface.get_current_texture() {
                            Ok(frame) => frame,
                            Err(wgpu::SurfaceError::Outdated) => {
                                backend.surface.configure(&backend.device, &backend.config);
                                asteria_window.window().request_redraw();
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
                            let bg_color = asteria_window.chrome().theme.background.to_rgba_f32();
                            let mut rpass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("Asteria Render Pass"),
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view: &view,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                                r: bg_color[0] as f64,
                                                g: bg_color[1] as f64,
                                                b: bg_color[2] as f64,
                                                a: bg_color[3] as f64,
                                            }),
                                            store: wgpu::StoreOp::Store,
                                        },
                                    })],
                                    depth_stencil_attachment: None,
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                });

                            // Hardware Scissor Rect Isolation: Render Full Canvas
                            rpass.set_scissor_rect(0, 0, backend.config.width, backend.config.height);
                            render_graph.render(&mut rpass);
                        }

                        backend.queue.submit(std::iter::once(encoder.finish()));
                        frame.present();
                    }

                    _ => {}
                }
            }
            Event::AboutToWait => {
                if asteria_window.viewport.update_scroll() {
                    needs_redraw = true;
                    asteria_window.window().request_redraw();
                }
            }
            Event::UserEvent(AsteriaUserEvent::SceneUpdated(_)) => {
                needs_redraw = true;
                asteria_window.window().request_redraw();
            }
            _ => {}
        }
    });
}
