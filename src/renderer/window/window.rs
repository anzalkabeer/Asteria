use std::sync::Arc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::renderer::backend::wgpu_backend::WgpuBackend;
use crate::renderer::commands::batch_builder::BatchBuilder;
use crate::renderer::commands::command_builder::CommandBuilder;
use crate::renderer::graph::render_graph::RenderGraph;
use crate::renderer::passes::rect_pass::RectPass;
use crate::renderer::scheduler::batching::BatchPlanner;
use crate::scene::SceneGraph;

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

pub fn run_window_loop(scene: SceneGraph) {
    let event_loop = EventLoop::new().expect("Failed to create EventLoop");
    let asteria_window = AsteriaWindow::new(&event_loop, 800, 600);

    // Initialize WgpuBackend
    let mut backend = pollster::block_on(WgpuBackend::new(asteria_window.window.clone()));

    // Build commands and batches from SceneGraph
    let mut cmd_builder = CommandBuilder::new();
    cmd_builder.build_from_scene(&scene);

    let planned_batches = BatchPlanner::plan(&cmd_builder.commands);

    let mut batch_builder = BatchBuilder::new();
    // Assuming rect batch is the first one
    if let Some(rect_cmds) = planned_batches.get(0) {
        let owned_cmds: Vec<_> = rect_cmds
            .iter()
            .map(|&c| match c {
                crate::renderer::commands::command_builder::RenderCommand::SolidRect {
                    rect,
                    rgba,
                } => crate::renderer::commands::command_builder::RenderCommand::SolidRect {
                    rect: *rect,
                    rgba: *rgba,
                },
                crate::renderer::commands::command_builder::RenderCommand::Text {
                    text,
                    rect,
                    rgba,
                    font_size,
                } => crate::renderer::commands::command_builder::RenderCommand::Text {
                    text: text.clone(),
                    rect: *rect,
                    rgba: *rgba,
                    font_size: *font_size,
                },
            })
            .collect();
        batch_builder.build_batches(&owned_cmds);
    }

    // Initialize RenderGraph and Passes
    let mut render_graph = RenderGraph::new();
    let mut rect_pass = RectPass::new(&backend.device, backend.config.format);
    rect_pass.update_buffers(&backend.device, &batch_builder);
    render_graph.add_pass(Box::new(rect_pass));

    event_loop.set_control_flow(ControlFlow::Wait);

    let _ = event_loop.run(move |event, elwt| match event {
        Event::WindowEvent { event, window_id } if window_id == asteria_window.window.id() => {
            match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Resized(physical_size) => {
                    backend.resize(physical_size);
                    asteria_window.window.request_redraw();
                }
                WindowEvent::RedrawRequested => {
                    render_graph.prepare(&backend.device, &backend.queue);

                    let frame = backend
                        .surface
                        .get_current_texture()
                        .expect("Failed to acquire next swap chain texture");

                    let view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    let mut encoder = backend
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                    {
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });

                        render_graph.render(&mut rpass);
                    }

                    backend.queue.submit(Some(encoder.finish()));
                    frame.present();
                }
                _ => {}
            }
        }
        _ => {}
    });
}
