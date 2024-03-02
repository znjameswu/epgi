use std::{any::TypeId, sync::Arc, time::Instant};

use epgi_core::foundation::SyncMutex;

use super::{PointerEvent, PointerInteractionEvent, PointerInteractionId};

pub trait TransformedPointerEventHandler: Send {
    fn handle_pointer_event(&self, event: &PointerEvent);

    fn all_gesture_recognizers(&self) -> Option<(GestureRecognizerTeamPolicy, Vec<TypeId>)> {
        None
    }

    #[allow(unused_variables)]
    fn get_gesture_recognizer(
        &self,
        type_id: TypeId,
    ) -> Option<Box<dyn AnyTransformedGestureRecognizer>> {
        None
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum GestureRecognizerTeamPolicy {
    Competing,
    Cooperative,
    Hereditary,
}

pub enum RecognitionResult {
    Impossible,
    Inconclusive { revisit: Instant },
    Possible,
    Certain { confidence: f32 },
}

impl RecognitionResult {
    pub fn is_certain(&self) -> Option<f32> {
        use RecognitionResult::*;
        match self {
            Certain { confidence } => Some(*confidence),
            _ => None,
        }
    }

    pub fn is_impossible(&self) -> bool {
        use RecognitionResult::*;
        match self {
            Impossible => true,
            _ => false,
        }
    }
}

pub struct RecognizerResponse {
    pub primary_result: RecognitionResult,
    pub associated_arenas: Vec<PointerInteractionId>,
}

pub trait GestureRecognizer: 'static {
    type HitPosition;

    fn handle_event(
        &mut self,
        position: &Self::HitPosition,
        event: &PointerInteractionEvent,
    ) -> RecognizerResponse;

    /// Query the current recognition result of this recognizer without new events arriving.
    /// This typically happens because the recognizer reported inconclusive result previously.
    fn query_recognition_state(&self, interaction_id: PointerInteractionId) -> RecognizerResponse;

    /// Intepret pointer event into gestures. This happens because the recognizer has already won.
    fn handle_arena_victory(&mut self, interaction_id: PointerInteractionId) -> RecognizerResponse;

    /// Handle defeat and clean up. This happens because the arena has picked another winner.
    fn handle_arena_defeat(&mut self, interaction_id: PointerInteractionId) -> RecognizerResponse;
}

pub trait AnyTransformedGestureRecognizer {
    fn handle_event(&self, event: &PointerInteractionEvent) -> RecognizerResponse;

    /// Query the current recognition result of this recognizer without new events arriving.
    /// This typically happens because the recognizer reported inconclusive result previously.
    fn query_recognition_state(&self, interaction_id: PointerInteractionId) -> RecognizerResponse;

    /// Intepret pointer event into gestures. This happens because the recognizer has already won.
    fn handle_arena_victory(&self, interaction_id: PointerInteractionId) -> RecognizerResponse;

    /// Handle defeat and clean up. This happens because the arena has picked another winner.
    fn handle_arena_defeat(&self, interaction_id: PointerInteractionId) -> RecognizerResponse;

    fn recognizer_type_id(&self) -> TypeId;
}

pub struct TransformedGestureRecognizer<R: GestureRecognizer> {
    recognizer: Arc<SyncMutex<R>>,
    hit_position: R::HitPosition,
}

impl<R> AnyTransformedGestureRecognizer for TransformedGestureRecognizer<R>
where
    R: GestureRecognizer,
{
    fn handle_event(&self, event: &PointerInteractionEvent) -> RecognizerResponse {
        self.recognizer
            .lock()
            .handle_event(&self.hit_position, event)
    }

    fn query_recognition_state(&self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        self.recognizer
            .lock()
            .query_recognition_state(interaction_id)
    }

    fn handle_arena_victory(&self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        self.recognizer.lock().handle_arena_victory(interaction_id)
    }

    fn handle_arena_defeat(&self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        self.recognizer.lock().handle_arena_defeat(interaction_id)
    }

    fn recognizer_type_id(&self) -> TypeId {
        TypeId::of::<R>()
    }
}
