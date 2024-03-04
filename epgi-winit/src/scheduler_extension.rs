use std::time::Instant;

use epgi_2d::BoxProtocol;
use epgi_common::{gesture::PointerGestureManager, PointerEvent};
use epgi_core::{
    foundation::SyncMpscReceiver, scheduler::SchedulerExtension, tree::ArcChildRenderObject,
};

pub(crate) struct EpgiGlazierSchedulerExtension {
    pointer_gesture_manager: PointerGestureManager,
    root: ArcChildRenderObject<BoxProtocol>,
}

impl EpgiGlazierSchedulerExtension {
    pub(crate) fn new(
        root: ArcChildRenderObject<BoxProtocol>,
        rx: SyncMpscReceiver<PointerEvent>,
    ) -> Self {
        Self {
            pointer_gesture_manager: PointerGestureManager::new(rx),
            root,
        }
    }
}

impl SchedulerExtension for EpgiGlazierSchedulerExtension {
    fn on_frame_begin(&mut self) {
        self.pointer_gesture_manager.flush_events(&self.root);
        self.pointer_gesture_manager
            .poll_revisit_all(Instant::now());
    }

    fn on_layout_complete(&mut self) {}

    fn on_frame_complete() {}

    fn on_extension_event(&mut self, event: Box<dyn std::any::Any + Send + Sync>) {
        // We can also flush events here to minimize latency caused by event flushes at the beginning of a frame.
    }
}
