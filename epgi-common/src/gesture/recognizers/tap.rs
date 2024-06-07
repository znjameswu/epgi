use std::any::TypeId;

use epgi_2d::Point2d;
use epgi_core::{foundation::SyncMutex, scheduler::get_current_scheduler};

use crate::{
    ArcJobCallback, GestureRecognizer, PointerInteractionEvent, PointerInteractionId,
    RecognizerResponse,
};

pub struct TapGestureRecognizer {
    inner: SyncMutex<TapGestureRecognizerInner>,
}

struct TapGestureRecognizerInner {
    pub on_tap: ArcJobCallback,
}

impl TapGestureRecognizer {
    pub fn new(on_tap: ArcJobCallback) -> Self {
        Self {
            inner: SyncMutex::new(TapGestureRecognizerInner { on_tap }),
        }
    }

    pub fn update(&self, on_tap: ArcJobCallback) {
        let mut inner = self.inner.lock();
        inner.on_tap = on_tap;
    }
}

impl GestureRecognizer for TapGestureRecognizer {
    fn handle_event(
        &self,
        position: &Point2d,
        event: &PointerInteractionEvent,
    ) -> RecognizerResponse {
        RecognizerResponse::possible()
    }

    fn query_recognition_state(&self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        RecognizerResponse::possible()
    }

    fn handle_arena_victory(&self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        let on_tap = self.inner.lock().on_tap.clone();
        get_current_scheduler().create_sync_job(|job_builder| {
            on_tap(job_builder);
        });
        RecognizerResponse::certain(1.0)
    }

    fn handle_arena_evict(&self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        RecognizerResponse::impossible()
    }

    fn on_detach(&self) {
        todo!()
    }

    fn recognizer_type_id(&self) -> std::any::TypeId {
        TypeId::of::<Self>()
    }
}
