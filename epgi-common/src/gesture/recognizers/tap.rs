use std::any::TypeId;

use epgi_2d::Point2d;
use epgi_core::scheduler::get_current_scheduler;

use crate::{
    ArcJobCallback, GestureRecognizer, PointerInteractionEvent, PointerInteractionId,
    RecognizerResponse,
};

pub struct TapGestureRecognizer {
    pub on_tap: ArcJobCallback,
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
        get_current_scheduler().create_sync_job(|job_builder| {
            (self.on_tap)(job_builder);
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
