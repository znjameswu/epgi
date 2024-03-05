use epgi_2d::Point2d;
use epgi_core::foundation::Asc;

use crate::{
    ArcCallback, GestureRecognizer, PointerInteractionEvent, PointerInteractionId,
    RecognitionResult, RecognizerResponse,
};

pub struct TapGestureRecognizer {
    pub device_pixel_ratio: f32,
    pub on_tap: ArcCallback,
}

impl GestureRecognizer for TapGestureRecognizer {
    type HitPosition = Point2d;

    fn handle_event(
        &mut self,
        position: &Self::HitPosition,
        event: &PointerInteractionEvent,
    ) -> RecognizerResponse {
        RecognizerResponse::possible()
    }

    fn query_recognition_state(&self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        RecognizerResponse::possible()
    }

    fn handle_arena_victory(&mut self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        (self.on_tap)();
        RecognizerResponse::certain(1.0)
    }

    fn handle_arena_evict(&mut self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        RecognizerResponse::impossible()
    }

    fn on_detach(&mut self) {
        todo!()
    }
}
