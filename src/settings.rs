use std::time::Duration;

#[derive(Debug)]
pub struct GraphicsSettings {
    pub frametime_or_vsync: Option<Duration>,
    pub render_without_focus: bool,
}

impl GraphicsSettings {
    /// Enables vsync
    pub fn with_vsync(mut self) -> Self {
        self.frametime_or_vsync = None;
        self
    }

    /// Sets the engine to try and run at a constant frametime + disables vsync
    pub fn with_framerate(mut self, framerate: f64) -> Self {
        self.frametime_or_vsync = Some(Duration::from_secs_f64(1.0 / framerate));
        self
    }

    pub fn with_render_without_focus(mut self, render_without_focus: bool) -> Self {
        self.render_without_focus = render_without_focus;
        self
    }
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            frametime_or_vsync: None,
            render_without_focus: false,
        }
    }
}
