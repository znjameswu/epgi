use std::time::Instant;

use epgi_2d::BoxProtocol;
use epgi_common::{gesture::PointerGestureManager, PointerEvent};
use epgi_core::{
    foundation::SyncMpscReceiver,
    scheduler::{BuildStates, SchedulerExtension},
    tree::{ArcAnyLayerRenderObjectExt, ArcAnyRenderObjectExt, ArcChildRenderObject},
};

pub(crate) struct EpgiGlazierSchedulerExtension {
    pointer_gesture_manager: PointerGestureManager,
}

impl EpgiGlazierSchedulerExtension {
    pub(crate) fn new(rx: SyncMpscReceiver<PointerEvent>) -> Self {
        Self {
            pointer_gesture_manager: PointerGestureManager::new(rx),
        }
    }
}

impl SchedulerExtension for EpgiGlazierSchedulerExtension {
    fn on_frame_begin(&mut self, build_states: &BuildStates) {
        let root_render_object = build_states
            .root_render_object
            .clone()
            .downcast_arc_child::<BoxProtocol>()
            .expect("Root render object should use BoxProtocol");
        self.pointer_gesture_manager
            .flush_events(&root_render_object);
        self.pointer_gesture_manager
            .poll_revisit_all(Instant::now());
    }

    fn on_layout_complete(&mut self, build_states: &BuildStates) {}

    fn on_frame_complete(build_states: &BuildStates) {}

    fn on_extension_event(&mut self, event: Box<dyn std::any::Any + Send + Sync>) {
        // We can also flush events here to minimize latency caused by event flushes at the beginning of a frame.
    }
}
