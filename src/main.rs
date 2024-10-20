use std::{borrow::Cow, fs::read_to_string, sync::Arc};

use log::*;
use pollster::FutureExt as _;

use wgpu::include_wgsl;
use winit::window::WindowAttributes;
use yagvx::error::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const SHADERS: &[&str] = &["shader"];

#[derive(Debug)]
struct GraphicsSettings {
    framerate_or_vsync: Option<u32>,
}

impl GraphicsSettings {
    /// Enables vsync
    pub fn with_vsync(mut self) -> Self {
        self.framerate_or_vsync = None;
        self
    }

    /// Sets the engine to try and run at a constant FPS + disables vsync
    pub fn with_framerate(mut self, framerate: u32) -> Self {
        self.framerate_or_vsync = Some(framerate);
        self
    }
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            framerate_or_vsync: None,
        }
    }
}

#[derive(Debug)]
struct GraphicsContext<'window> {
    adapter: wgpu::Adapter,
    surface: wgpu::Surface<'window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    shaders: Vec<wgpu::RenderPipeline>,
}

impl<'a> GraphicsContext<'a> {
    /// Creates a new graphics context for the `window`, panics on error.
    pub async fn new(window: Arc<winit::window::Window>) -> Self {
        let (width, height) = {
            let size = window.inner_size();
            (size.width.max(1), size.height.max(1))
        };

        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(window).unwrap();
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

        let config = surface.get_default_config(&adapter, width, height).unwrap();
        surface.configure(&device, &config);

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
}

#[derive(Debug)]
struct Engine<'a> {
    window_attributes: winit::window::WindowAttributes,
    window: Option<Arc<winit::window::Window>>,

    graphics_context: Option<GraphicsContext<'a>>,
    graphics_settings: GraphicsSettings,
}

impl<'a> Engine<'a> {
    // CONFIGURATION

    pub async fn new(window_attributes: winit::window::WindowAttributes) -> Self {
        Self {
            window_attributes,
            window: None,
            graphics_context: None,
            graphics_settings: Default::default(),
        }
    }

    pub fn with_graphics_settings(mut self, graphics_settings: GraphicsSettings) -> Self {
        self.graphics_settings = graphics_settings;
        self
    }

    // DRAWING

    pub fn draw(&mut self) -> Result<(), DrawError> {
        if let Some(ref gc) = self.graphics_context {
            for shader in &gc.shaders {
                let frame = gc
                    .surface
                    .get_current_texture()
                    .expect("Failed to acquire to next swapchain texture.");
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = gc
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

                gc.queue.submit(Some(encoder.finish()));
                frame.present();
            }
        }

        Ok(())
    }

    // EXITING

    fn exit(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        event_loop.exit();
    }
}

impl<'a> winit::application::ApplicationHandler for Engine<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // Create a new window if needed.
        if self.window.is_none() {
            self.window = Some(Arc::new(
                event_loop
                    .create_window(self.window_attributes.clone())
                    .unwrap(), // We have serious issues.
            ));
            self.graphics_context =
                Some(GraphicsContext::new(self.window.as_ref().unwrap().clone()).block_on())
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent as Event;

        match event {
            Event::CloseRequested => self.exit(event_loop),
            Event::RedrawRequested => {
                if let Err(error) = self.draw() {
                    error!("Draw Error: {error:?}");
                }

                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), RootError> {
    pretty_env_logger::init_timed();
    info!("YAGVX v{VERSION}");

    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut engine = Engine::new(WindowAttributes::default().with_title("YAGVX"))
        .block_on()
        .with_graphics_settings(GraphicsSettings::default().with_framerate(60));

    event_loop.run_app(&mut engine)?;

    Ok(())
}
