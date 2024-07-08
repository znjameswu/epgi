use std::{collections::VecDeque, time::Instant};

const SLIDING_WINDOW_SIZE: usize = 60;
const SAMPLE_SIZE: usize = 10800;
pub(crate) struct FrameStats {
    pub frame_count: u64,
    pub build_time_sum: u64,
    pub layout_time_sum: u64,
    pub paint_time_sum: u64,
    pub raster_time_sum: u64,
    pub frame_time_sum: u64,
    // total_frame: u64,
    // total_time: u64,
    // window_ui_time_sum: u64,
    pub window_build_time_sum: u64,
    pub window_layout_time_sum: u64,
    pub window_paint_time_sum: u64,
    pub window_raster_time_sum: u64,
    pub window_samples: VecDeque<FrameStatSample>,

    pub build_time_low: u64,
    pub layout_time_low: u64,
    pub paint_time_low: u64,
    pub raster_time_low: u64,
    pub frame_time_ms_low: f32,

    samples: VecDeque<FrameStatSample>,
}

#[derive(Clone, Debug)]
pub(crate) struct FrameStatSample {
    pub(crate) timestamp: Instant,
    pub(crate) ui_metrics: epgi_core::scheduler::FrameMetrics,
    pub(crate) raster_time: u64,
}

impl FrameStats {
    pub(crate) fn new() -> Self {
        Self {
            frame_count: 0,
            build_time_sum: 0,
            layout_time_sum: 0,
            paint_time_sum: 0,
            raster_time_sum: 0,
            frame_time_sum: 0,
            // total_frame: 0,
            // total_time: 0,
            // window_ui_time_sum: 0,
            // window_raster_time_sum: 0,
            window_samples: VecDeque::with_capacity(SLIDING_WINDOW_SIZE),
            samples: VecDeque::with_capacity(SAMPLE_SIZE),
            window_build_time_sum: 0,
            window_layout_time_sum: 0,
            window_paint_time_sum: 0,
            window_raster_time_sum: 0,
            build_time_low: u64::MAX,
            layout_time_low: u64::MAX,
            paint_time_low: u64::MAX,
            raster_time_low: u64::MAX,
            frame_time_ms_low: f32::MAX,
        }
    }

    pub(crate) fn add_sample(&mut self, sample: FrameStatSample) {
        self.frame_count += 1;
        self.build_time_sum += sample.ui_metrics.build_time;
        self.layout_time_sum += sample.ui_metrics.layout_time;
        self.paint_time_sum += sample.ui_metrics.paint_time;
        self.raster_time_sum += sample.raster_time;
        if let Some(back) = self.samples.back() {
            self.frame_time_sum += (sample.timestamp - back.timestamp).as_micros() as u64;
        }

        let oldest = if self.window_samples.len() >= SLIDING_WINDOW_SIZE {
            self.window_samples.pop_front()
        } else {
            None
        };
        self.window_samples.push_back(sample.clone());

        // self.window_ui_time_sum += sample.ui_metrics.frame_time() as u64;
        // self.window_raster_time_sum += sample.raster_time;
        self.window_build_time_sum += sample.ui_metrics.build_time;
        self.window_layout_time_sum += sample.ui_metrics.layout_time;
        self.window_paint_time_sum += sample.ui_metrics.paint_time;
        self.window_raster_time_sum += sample.raster_time;
        if let Some(oldest) = oldest {
            self.window_build_time_sum -= oldest.ui_metrics.build_time;
            self.window_layout_time_sum -= oldest.ui_metrics.layout_time;
            self.window_paint_time_sum -= oldest.ui_metrics.paint_time;
            self.window_raster_time_sum -= oldest.raster_time;
        }
        if self.window_samples.len() == SLIDING_WINDOW_SIZE {
            self.build_time_low = self
                .build_time_low
                .min(self.window_build_time_sum / SLIDING_WINDOW_SIZE as u64);
            self.layout_time_low = self
                .layout_time_low
                .min(self.window_layout_time_sum / SLIDING_WINDOW_SIZE as u64);
            self.paint_time_low = self
                .paint_time_low
                .min(self.window_paint_time_sum / SLIDING_WINDOW_SIZE as u64);
            self.raster_time_low = self
                .raster_time_low
                .min(self.window_raster_time_sum / SLIDING_WINDOW_SIZE as u64);
            self.frame_time_ms_low = self
                .frame_time_ms_low
                .min(self.get_frame_time_ms_avg().unwrap());
        }

        if self.samples.len() >= SAMPLE_SIZE {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    pub(crate) fn get_frame_time_ms_avg(&self) -> Option<f32> {
        if self.window_samples.len() <= 1 {
            return None;
        }
        let (Some(back), Some(front)) = (self.window_samples.back(), self.window_samples.front())
        else {
            return None;
        };

        let frame_time_ms = back.timestamp.duration_since(front.timestamp).as_micros() as f32
            / (self.window_samples.len() - 1) as f32;
        Some(frame_time_ms / 1000.0)
    }

    // pub(crate) fn get_ui_time_ms_avg(&self) -> Option<f32> {
    //     if self.window_samples.len() == 0 {
    //         return None;
    //     }
    //     return Some(self.window_ui_time_sum as f32 / self.window_samples.len() as f32 / 1000.0);
    // }

    // pub(crate) fn get_raster_time_ms_avg(&self) -> Option<f32> {
    //     if self.window_samples.len() == 0 {
    //         return None;
    //     }
    //     return Some(
    //         self.window_raster_time_sum as f32 / self.window_samples.len() as f32 / 1000.0,
    //     );
    // }
}
