use macroquad::window::next_frame;
use std::time::{Duration, Instant};

/// Handles framerate with optional max fps lock. Call on_frame_start() in the beginning of the
/// frame and wait_for_next_frame().await in the end. If max_fps is set it will use
/// std::thread::sleep to ensure that max_fps is not exceeded. Depending on the OS and hardware
/// this might not be precise enough. So generally vsync should be prefered over using max_fps.
pub struct FPSControl {
    max_fps: Option<usize>,
    frame_start: Option<Instant>,
    min_frame_time: Option<Duration>,
}

impl Default for FPSControl {
    fn default() -> Self {
        Self::new()
    }
}

impl FPSControl {
    pub fn new() -> FPSControl {
        FPSControl {
            frame_start: None,
            max_fps: None,
            min_frame_time: None,
        }
    }

    pub fn with_max_fps(mut self, max_fps: usize) -> Self {
        self.max_fps = Some(max_fps);
        self.min_frame_time = Some(Duration::from_secs_f32(1. / max_fps as f32));

        self
    }

    pub fn on_frame_start(&mut self) {
        if self.max_fps.is_some() {
            self.frame_start = Some(Instant::now());
        }
    }

    pub async fn wait_for_next_frame(&self) {
        next_frame().await; // submit our render calls to our screen

        if self.max_fps.is_some() {
            let frame_start = self.frame_start.expect("this should be set on_frame_start");
            let min_frame_time = self.min_frame_time.expect("should be set in MaxFps mode");

            // wait for frametime to be at least minimum_frame_time which
            // results in a upper limit for the FPS
            let frame_finish = Instant::now();
            let frame_time = frame_finish.duration_since(frame_start);

            if frame_time < min_frame_time {
                let time_to_sleep = min_frame_time
                    .checked_sub(frame_time)
                    .expect("time subtraction failed");
                std::thread::sleep(time_to_sleep);
            }
        }
    }
}
