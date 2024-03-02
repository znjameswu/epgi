use epgi_common::gesture::PointerGestureManager;
use epgi_core::{foundation::unbounded_channel_sync, scheduler::SchedulerExtension};

pub(crate) struct EpgiGlazierSchedulerExtension {
    pointer_gesture_manager: PointerGestureManager,
}

impl EpgiGlazierSchedulerExtension {
    pub(crate) fn new() -> Self {
        let (tx, rx) = unbounded_channel_sync();
        Self {
            pointer_gesture_manager: PointerGestureManager::new(rx),
        }
    }
}

impl SchedulerExtension for EpgiGlazierSchedulerExtension {
    fn on_frame_begin(&mut self) {}

    fn on_layout_complete(&mut self) {}

    fn on_frame_complete() {}

    fn on_extension_event(&mut self, event: Box<dyn std::any::Any + Send + Sync>) {}
}
