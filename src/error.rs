use winit::error::{EventLoopError, OsError};

// Root

#[derive(Debug)]
pub enum RootError {
    EventLoop(EventLoopError),
}

impl From<EventLoopError> for RootError {
    fn from(value: EventLoopError) -> Self {
        Self::EventLoop(value)
    }
}

// Engine

#[derive(Debug)]
pub enum EngineError {
    CreateSurfaceError(wgpu::CreateSurfaceError),
}

#[derive(Debug)]
pub enum DrawError {}
