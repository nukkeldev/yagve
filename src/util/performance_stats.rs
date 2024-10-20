use std::time::{Duration, Instant};

const FPS_SMA_RESOLUTION: usize = 100;

#[derive(Debug)]
pub struct PerformanceStats {
    /// Time of the last frame
    last_frame: Option<Instant>,
    /// Durations in between frames
    frame_durations: [Duration; FPS_SMA_RESOLUTION],
    /// Number of recorded frames; always at `FPS_SMA_RESOLUTION` besides startup
    frames: u32,
    /// Total duration of last `FPS_SMA_RESOLUTION` frames
    frame_rate_accum: Duration,
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self {
            last_frame: None,
            frame_durations: [Default::default(); 100],
            frames: 1,
            frame_rate_accum: Default::default(),
        }
    }
}

impl PerformanceStats {
    pub fn add_frame(&mut self, time: Instant) {
        match self.last_frame {
            Some(last_frame) => {
                let duration = time - last_frame;
                self.last_frame = Some(time);

                // Save frame time, subtract oldest, and add newest to accum.
                self.frame_durations.rotate_right(1);
                self.frame_rate_accum -= self.frame_durations[0];
                self.frame_rate_accum += duration;
                self.frame_durations[0] = duration;

                self.frames = (self.frames + 1).min(FPS_SMA_RESOLUTION as u32);
            }
            None => self.last_frame = Some(time),
        }
    }

    pub fn get_frame_time(&self) -> Duration {
        self.frame_rate_accum / self.frames
    }
}
