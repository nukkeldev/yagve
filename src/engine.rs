use std::sync::Arc;
use std::time::{Duration, Instant};

use log::*;
use pollster::FutureExt as _;
use winit::event::ElementState;
use winit::event_loop::ControlFlow;

use crate::graphics::GraphicsContext;
use crate::settings::GraphicsSettings;
use crate::util::error::DrawError;
use crate::util::performance_stats::PerformanceStats;

#[derive(Debug)]
pub struct Engine<'a> {
    window_attributes: winit::window::WindowAttributes,
    window: Option<Arc<winit::window::Window>>,
    has_focus: bool,

    graphics_context: Option<GraphicsContext<'a>>,
    graphics_settings: GraphicsSettings,

    next_frame_time: Instant,

    performance_stats: PerformanceStats,
}

impl<'a> Engine<'a> {
    // CONFIGURATION

    pub fn new(window_attributes: winit::window::WindowAttributes) -> Self {
        async {
            Self {
                window_attributes,
                window: None,
                has_focus: false,
                graphics_context: None,
                graphics_settings: Default::default(),
                next_frame_time: Instant::now(),
                performance_stats: Default::default(),
            }
        }
        .block_on()
    }

    pub fn with_graphics_settings(mut self, graphics_settings: GraphicsSettings) -> Self {
        self.graphics_settings = graphics_settings;
        self
    }

    // DRAWING

    fn can_draw(&self) -> bool {
        Instant::now() >= self.next_frame_time
    }

    pub fn draw(&mut self) -> Result<(), DrawError> {
        self.graphics_context.as_mut().unwrap().draw();
        self.performance_stats.add_frame(Instant::now());

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
            self.graphics_context = Some(
                GraphicsContext::new(
                    &self.graphics_settings,
                    self.window.as_ref().unwrap().clone(),
                )
                .block_on(),
            )
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::{KeyEvent, WindowEvent};
        use winit::keyboard::{KeyCode, PhysicalKey};

        match event {
            WindowEvent::Focused(is_focused) => {
                self.has_focus = is_focused;
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::CloseRequested => self.exit(event_loop),
            WindowEvent::RedrawRequested => 'block: {
                if !(self.has_focus || self.graphics_settings.render_without_focus) {
                    break 'block;
                }

                if self.graphics_settings.frametime_or_vsync.is_none()
                    || self.next_frame_time <= Instant::now()
                {
                    if let Err(error) = self.draw() {
                        error!("Draw Error: {error:?}");
                    }

                    if let Some(frametime) = self.graphics_settings.frametime_or_vsync {
                        self.next_frame_time = Instant::now() + frametime;
                    }
                }

                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::Resized(_) => {
                if let Some(gc) = &mut self.graphics_context {
                    gc.reconfigure_surface(self.window.as_ref().unwrap(), &self.graphics_settings);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: key,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                if let PhysicalKey::Code(kc) = key {
                    match kc {
                        KeyCode::KeyF => debug!(
                            "Framerate: {:.3} fps",
                            1.0 / self.performance_stats.get_frame_time().as_secs_f64()
                        ),
                        _ => {}
                    }
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                // Keyboard Modifiers
            }
            _ => {}
        }
    }
}
