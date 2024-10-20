use log::*;

use winit::{error::EventLoopError, window::WindowAttributes};
use yagve::{engine::Engine, settings::GraphicsSettings};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), EventLoopError> {
    pretty_env_logger::init_timed();
    info!("YAGVE v{VERSION}");

    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut engine = Engine::new(WindowAttributes::default().with_title("YAGVX"))
        .with_graphics_settings(GraphicsSettings::default().with_framerate(60.0));

    event_loop.run_app(&mut engine)?;

    Ok(())
}
