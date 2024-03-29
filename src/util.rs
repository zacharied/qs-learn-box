use std::time::{Duration, Instant};

use super::consts::system::*;

pub struct FpsGraph {
    history: [f64; FPS_GRAPH_SAMPLE_COUNT],
    i: usize,
}

impl FpsGraph {
    pub fn new() -> Self {
        FpsGraph {
            history: [0.; FPS_GRAPH_SAMPLE_COUNT],
            i: 0,
        }
    }

    pub fn log_fps(&mut self, fps: f64) {
        self.history[self.i] = fps;
        self.i = (self.i + 1) % FPS_GRAPH_SAMPLE_COUNT;
    }

    pub fn recent_average_fps(&self) -> Option<f64> {
        let mut sum = 0.;
        for f in self.history.iter() {
            if !f.is_normal() {
                return None;
            }
            sum += *f;
        }
        Some(sum / self.history.len() as f64)
    }
}

pub struct Countdown {
    start: Instant,
    duration: Duration
}

impl Countdown {
    pub fn new(duration: Duration) -> Self {
        Self {
            start: Instant::now(),
            duration
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn is_done(&self) -> bool {
        self.start.elapsed() > self.duration
    }
}
