use std::{borrow::Cow, fs::read_to_string, sync::Arc};

use crate::settings::GraphicsSettings;

pub const SHADERS: &[&str] = &["shader"];

#[derive(Debug)]
pub struct GraphicsContext<'window> {
    adapter: wgpu::Adapter,
    surface: wgpu::Surface<'window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    shaders: Vec<wgpu::RenderPipeline>,
}

impl<'a> GraphicsContext<'a> {
    /// Creates a new graphics context for the `window`, panics on error.
    pub async fn new(settings: &GraphicsSettings, window: Arc<winit::window::Window>) -> Self {
        let (width, height) = {
            let size = window.inner_size();
            (size.width.max(1), size.height.max(1))
        };

        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface), // Request an adapter compatible with our surface
            })
            .await
            .expect("No compatible adapters found.");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_alignment(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await
            .expect("Failed to create device.");

        Self::configure_surface(&surface, &adapter, &device, window.as_ref(), settings);

        let mut ctx = Self {
            adapter,
            surface,
            device,
            queue,
            shaders: vec![],
        };

        for shader in SHADERS {
            ctx.load_shader(&format!("shaders/{shader}.wgsl"));
        }

        ctx
    }

    fn configure_surface(
        surface: &wgpu::Surface,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        window: &winit::window::Window,
        settings: &GraphicsSettings,
    ) {
        let (width, height) = {
            let size = window.inner_size();
            (size.width.max(1), size.height.max(1))
        };

        let mut config = surface.get_default_config(&adapter, width, height).unwrap();
        // Set the initial graphics settings.
        config.present_mode = if settings.frametime_or_vsync.is_some() {
            wgpu::PresentMode::AutoNoVsync
        } else {
            wgpu::PresentMode::AutoVsync
        };

        surface.configure(&device, &config);
    }

    pub fn reconfigure_surface(
        &mut self,
        window: &winit::window::Window,
        settings: &GraphicsSettings,
    ) {
        Self::configure_surface(&self.surface, &self.adapter, &self.device, window, settings);
    }

    pub fn load_shader(&mut self, shader: &str) {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                    &read_to_string(shader).expect(&format!("Failed to read shader: {shader}")),
                )),
            });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let swapchain_capabilities = self.surface.get_capabilities(&self.adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let render_pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(swapchain_format.into())],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        self.shaders.push(render_pipeline);
    }

    pub fn draw(&mut self) {
        for shader in &self.shaders {
            let frame = self
                .surface
                .get_current_texture()
                .expect("Failed to acquire to next swapchain texture.");
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                rp.set_pipeline(shader);
                rp.draw(0..3, 0..1);
            }

            self.queue.submit(Some(encoder.finish()));
            frame.present();
        }
    }
}
