use epgi_2d::Point2d;

use crate::{
    ArcCallback, GestureRecognizer, PointerInteractionEvent, PointerInteractionId,
    RecognizerResponse,
};

pub struct TapGestureRecognizer {
    pub on_tap: ArcCallback,
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
        (self.on_tap)();
        RecognizerResponse::certain(1.0)
    }

    fn handle_arena_evict(&self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        RecognizerResponse::impossible()
    }

    fn on_detach(&self) {
        todo!()
    }

    fn recognizer_type_id(&self) -> std::any::TypeId {
        todo!()
    }
}
