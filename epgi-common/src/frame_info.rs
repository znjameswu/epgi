use std::time::{Instant, SystemTime};

#[derive(PartialEq, Debug, Clone)]
pub struct FrameInfo {
    pub instant: Instant,
    pub system_time: SystemTime,
    pub frame_count: u64,
}

impl FrameInfo {
    pub fn now(frame_count: u64) -> Self {
        Self {
            instant: Instant::now(),
            system_time: SystemTime::now(),
            frame_count,
        }
    }
}
