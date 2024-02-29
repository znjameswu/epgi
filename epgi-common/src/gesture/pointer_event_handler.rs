use std::{any::TypeId, sync::Arc, time::Instant};

use epgi_core::{
    foundation::{Inlinable64Vec, SyncMutex},
    tree::AweakAnyRenderObject,
};

use super::{
    GestureRecognizerTeamPolicy, PointerEvent, PointerInteractionEvent, PointerInteractionId,
};

pub trait PointerEventHandler {
    fn handle_pointer_event(&self, event: PointerEvent);
}

/// GestureHandler differs from PointerEventHandler in that gesture recognition is arena-based.
/// Once a winner is resolved, all other competitors lose and will no longer receives event.
///
/// All GestureRecognizer must have a PointerEventHandler impl and an interface table entry,
/// to optimize the hit-test down-selection interface design.
/// The PointerEventHandler impl can be left empty, however.
pub trait AnyTransformedGestureRecognizerContainer: PointerEventHandler {
    fn handle_pointer_interaction_start(
        &self,
    ) -> (
        GestureRecognizerTeamPolicy,
        Vec<Box<dyn AnyTransformedGestureRecognizerWrapper>>,
    );

    fn get_recognizer(
        &self,
        type_id: TypeId,
    ) -> Option<Box<dyn AnyTransformedGestureRecognizerWrapper>>;
}

pub enum RecognitionResult {
    Certain { confidence: f32 },
    Possible,
    Inconclusive { revisit: Instant },
    Impossible,
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
    pub associated_updates: Vec<PointerInteractionId>,
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

pub trait AnyTransformedGestureRecognizerWrapper {
    fn handle_event(&mut self, event: &PointerInteractionEvent) -> RecognizerResponse;

    /// Query the current recognition result of this recognizer without new events arriving.
    /// This typically happens because the recognizer reported inconclusive result previously.
    fn query_recognition_state(&self, interaction_id: PointerInteractionId) -> RecognizerResponse;

    /// Intepret pointer event into gestures. This happens because the recognizer has already won.
    fn handle_arena_victory(&mut self, interaction_id: PointerInteractionId) -> RecognizerResponse;

    /// Handle defeat and clean up. This happens because the arena has picked another winner.
    fn handle_arena_defeat(&mut self, interaction_id: PointerInteractionId) -> RecognizerResponse;

    fn recognizer_type_id(&self) -> TypeId;

    fn strip_position(self: Box<Self>) -> Box<dyn AnyGestureRecognizerWrapper>;
}

pub struct TransformedGestureRecognizerWrapper<R: GestureRecognizer> {
    recognizer: Arc<SyncMutex<R>>,
    hit_position: R::HitPosition,
}

impl<R> AnyTransformedGestureRecognizerWrapper for TransformedGestureRecognizerWrapper<R>
where
    R: GestureRecognizer,
{
    fn handle_event(&mut self, event: &PointerInteractionEvent) -> RecognizerResponse {
        self.recognizer
            .lock()
            .handle_event(&self.hit_position, event)
    }

    fn query_recognition_state(&self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        self.recognizer
            .lock()
            .query_recognition_state(interaction_id)
    }

    fn handle_arena_victory(&mut self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        self.recognizer.lock().handle_arena_victory(interaction_id)
    }

    fn handle_arena_defeat(&mut self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        self.recognizer.lock().handle_arena_defeat(interaction_id)
    }

    fn recognizer_type_id(&self) -> TypeId {
        TypeId::of::<R>()
    }

    fn strip_position(self: Box<Self>) -> Box<dyn AnyGestureRecognizerWrapper> {
        Box::new(GestureRecognizerWrapper {
            recognizer: self.recognizer,
        })
    }
}

pub struct GestureRecognizerWrapper<R: GestureRecognizer> {
    recognizer: Arc<SyncMutex<R>>,
}

pub trait AnyGestureRecognizerWrapper {
    /// Query the current recognition result of this recognizer without new events arriving.
    /// This typically happens because the recognizer reported inconclusive result previously.
    fn query_recognition_state(&self, interaction_id: PointerInteractionId) -> RecognizerResponse;

    /// Intepret pointer event into gestures. This happens because the recognizer has already won.
    fn handle_arena_victory(&mut self, interaction_id: PointerInteractionId) -> RecognizerResponse;

    /// Handle defeat and clean up. This happens because the arena has picked another winner.
    fn handle_arena_defeat(&mut self, interaction_id: PointerInteractionId) -> RecognizerResponse;

    fn recognizer_type_id(&self) -> TypeId;
}

impl<R> AnyGestureRecognizerWrapper for GestureRecognizerWrapper<R>
where
    R: GestureRecognizer,
{
    fn query_recognition_state(&self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        self.recognizer
            .lock()
            .query_recognition_state(interaction_id)
    }

    fn handle_arena_victory(&mut self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        self.recognizer.lock().handle_arena_victory(interaction_id)
    }

    fn handle_arena_defeat(&mut self, interaction_id: PointerInteractionId) -> RecognizerResponse {
        self.recognizer.lock().handle_arena_defeat(interaction_id)
    }

    fn recognizer_type_id(&self) -> TypeId {
        TypeId::of::<R>()
    }
}
