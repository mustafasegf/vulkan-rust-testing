#![allow(dead_code, unused)]
#![allow(clippy::eq_op)]

use std::{convert::TryFrom, sync::Arc};

use bytemuck::{Pod, Zeroable};
use egui::{
    epaint::Shadow, style::Margin, vec2, Align, Align2, Color32, Frame, Rounding, Slider, Window,
};
use egui_winit_vulkano::{Gui, GuiConfig};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        CommandBufferInheritanceInfo, CommandBufferUsage, RenderPassBeginInfo, SubpassContents,
    },
    device::{Device, Queue},
    format::Format,
    image::{ImageAccess, SampleCount},
    memory::allocator::StandardMemoryAllocator,
    pipeline::{
        graphics::{
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    sync::GpuFuture,
};
use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    renderer::SwapchainImageView,
    window::{VulkanoWindows, WindowDescriptor},
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

pub fn main() {
    // Winit event loop
    let event_loop = EventLoop::new();
    // Vulkano context
    let context = VulkanoContext::new(VulkanoConfig::default());
    // Vulkano windows (create one)
    let mut windows = VulkanoWindows::default();
    windows.create_window(&event_loop, &context, &WindowDescriptor::default(), |ci| {
        ci.image_format = Some(vulkano::format::Format::B8G8R8A8_SRGB)
    });
    // Create out gui pipeline
    //
    let queue = context.graphics_queue().clone();
    let image_format = windows
        .get_primary_renderer_mut()
        .unwrap()
        .swapchain_format();

    let memory_allocator = context.memory_allocator();

    #[repr(C)]
    #[derive(Default, Debug, Copy, Clone, Zeroable, Pod)]
    struct Vertex {
        position: [f32; 2],
        color: [f32; 4],
    }

    vulkano::impl_vertex!(Vertex, position, color);

    let vertecies = [
        Vertex {
            position: [-0.5, -0.25],
            color: [1.0, 0.0, 0.0, 1.0],
        },
        Vertex {
            position: [0.0, 0.5],
            color: [0.0, 1.0, 0.0, 1.0],
        },
        Vertex {
            position: [0.25, -0.1],
            color: [0.0, 0.0, 1.0, 1.0],
        },
    ];

    let vertex_buffer = {
        CpuAccessibleBuffer::from_iter(
            memory_allocator,
            BufferUsage {
                vertex_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            vertecies,
        )
        .expect("failed to create buffer")
    };

    mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: "
            #version 450
            layout(location = 0) in vec2 position;
            layout(location = 1) in vec4 color;

            layout(location = 0) out vec4 v_color;
            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
                v_color = color;
            }"
        }
    }

    mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            src: "
            #version 450
            layout(location = 0) in vec4 v_color;

            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = v_color;
            }"
        }
    }
    let vs = vs::load(queue.device().clone()).expect("failed to create shader module");
    let fs = fs::load(queue.device().clone()).expect("failed to create shader module");

    let render_pass = vulkano::ordered_passes_renderpass!(
        queue.device().clone(),
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: image_format,
                samples: SampleCount::Sample1,
            }
        },
        passes: [
            { color: [color], depth_stencil: {}, input: [] }, // Draw what you want on this pass
            { color: [color], depth_stencil: {}, input: [] } // Gui render pass
        ]
    )
    .expect("can't create render pass");

    let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

    let pipeline = GraphicsPipeline::start()
        .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .input_assembly_state(InputAssemblyState::new())
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .render_pass(subpass.clone())
        .multisample_state(MultisampleState {
            rasterization_samples: SampleCount::Sample1,
            ..Default::default()
        })
        .build(queue.device().clone())
        .unwrap();

    // Create an allocator for command-buffer data
    let command_buffer_allocator =
        StandardCommandBufferAllocator::new(queue.device().clone(), Default::default());

    // Create gui subpass
    let mut gui = Gui::new_with_subpass(
        &event_loop,
        windows.get_primary_renderer_mut().unwrap().surface(),
        windows.get_primary_renderer_mut().unwrap().graphics_queue(),
        Subpass::from(render_pass.clone(), 1).unwrap(),
        GuiConfig {
            preferred_format: Some(vulkano::format::Format::B8G8R8A8_SRGB),
            ..Default::default()
        },
    );

    let mut open_gui = true;
    let mut view = 0;

    // Create gui state (pass anything your state requires)
    event_loop.run(move |event, _, control_flow| {
        let renderer = windows.get_primary_renderer_mut().unwrap();
        match event {
            Event::WindowEvent { event, window_id } if window_id == renderer.window().id() => {
                // Update Egui integration so the UI works!
                let _pass_events_to_game = !gui.update(&event);
                match event {
                    WindowEvent::Resized(_) => {
                        renderer.resize();
                    }
                    WindowEvent::ScaleFactorChanged { .. } => {
                        renderer.resize();
                    }
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => (),
                }
            }
            Event::RedrawRequested(window_id) if window_id == window_id => {
                gui.immediate_ui(|gui| {
                    let ctx = gui.context();
                    Window::new("Transparent Window")
                        .open(&mut open_gui)
                        .default_width(300.0)
                        .show(&ctx, |ui| {
                            ui.heading("egui");
                            ui.add(Slider::new(&mut view, -200..=200).text("age"));
                        });
                });
                let before_future = renderer.acquire().unwrap();

                let image = renderer.swapchain_image_view();
                let mut builder = AutoCommandBufferBuilder::primary(
                    &command_buffer_allocator,
                    queue.queue_family_index(),
                    CommandBufferUsage::OneTimeSubmit,
                )
                .unwrap();

                let dimensions = image.image().dimensions().width_height();
                let framebuffer = Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![image],
                        ..Default::default()
                    },
                )
                .unwrap();

                // Begin render pipeline commands
                builder
                    .begin_render_pass(
                        RenderPassBeginInfo {
                            clear_values: vec![Some([0.0, 0.0, 0.0, 1.0].into())],
                            ..RenderPassBeginInfo::framebuffer(framebuffer)
                        },
                        SubpassContents::SecondaryCommandBuffers,
                    )
                    .unwrap();

                // Render first draw pass
                let mut secondary_builder = AutoCommandBufferBuilder::secondary(
                    &command_buffer_allocator,
                    queue.queue_family_index(),
                    CommandBufferUsage::MultipleSubmit,
                    CommandBufferInheritanceInfo {
                        render_pass: Some(subpass.clone().into()),
                        ..Default::default()
                    },
                )
                .unwrap();
                secondary_builder
                    .bind_pipeline_graphics(pipeline.clone())
                    .set_viewport(
                        0,
                        vec![Viewport {
                            origin: [0.0, 0.0],
                            dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                            depth_range: 0.0..1.0,
                        }],
                    )
                    .bind_vertex_buffers(0, vertex_buffer.clone())
                    .draw(vertex_buffer.len() as u32, 1, 0, 0)
                    .unwrap();
                let cb = secondary_builder.build().unwrap();
                builder.execute_commands(cb).unwrap();

                // Move on to next subpass for gui
                builder
                    .next_subpass(SubpassContents::SecondaryCommandBuffers)
                    .unwrap();
                // Draw gui on subpass
                let cb = gui.draw_on_subpass_image(dimensions);
                builder.execute_commands(cb).unwrap();

                // Last end render pass
                builder.end_render_pass().unwrap();
                let command_buffer = builder.build().unwrap();
                let after_future = before_future
                    .then_execute(queue.clone(), command_buffer)
                    .unwrap();

                let after_future = after_future.boxed();
                // Present swapchain
                renderer.present(after_future, true);
            }
            Event::MainEventsCleared => {
                renderer.window().request_redraw();
            }
            _ => (),
        }
    });
}
