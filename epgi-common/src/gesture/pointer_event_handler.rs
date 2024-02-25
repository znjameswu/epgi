use std::{any::TypeId, sync::Arc, time::Instant};

use epgi_core::foundation::SyncMutex;

use super::{GestureRecognizerTeamPolicy, PointerEvent, PointerId};

pub trait PointerEventHandler {
    fn handle_pointer_event(&self, event: PointerEvent);
}

/// GestureHandler differs from PointerEventHandler in that gesture recognition is arena-based.
/// Once a winner is resolved, all other competitors lose and will no longer receives event.
///
/// All GestureRecognizer must have a PointerEventHandler impl and an interface table entry,
/// to optimize the hit-test down-selection interface design.
/// The PointerEventHandler impl can be left empty, however.
pub trait GestureRecognizerContainer: PointerEventHandler {
    fn handle_pointer_down(
        &self,
    ) -> (
        GestureRecognizerTeamPolicy,
        Vec<Box<dyn AnyTransformedGestureRecognizer>>,
    );

    fn get_recognizer(&self, type_id: TypeId) -> Option<Box<dyn AnyTransformedGestureRecognizer>>;
}

pub enum RecognitionResult {
    Certain { confidence: f32 },
    Possible,
    Inconclusive { revisit: Instant },
    Impossible,
}

pub trait GestureRecognizer: 'static {
    type HitPosition;

    /// Query the current recognition result of this recognizer without new events arriving.
    /// This typically happens because the recognizer reported inconclusive result previously.
    fn current_recognition_result(&self, pointer_id: PointerId) -> RecognitionResult;

    /// Compete for pointer event intepretation in areana. This happens because a new pointer event has arrived while the arena is active.
    ///
    /// The recognizer is expected to update its recognition result.
    /// If declaring a defeat, then the implementation must have already performed clean-up
    /// as if [GestureRecognizer::handle_arena_defeat] has been called.
    fn handle_arena_compete(&mut self, position: &Self::HitPosition, event: &PointerEvent);

    /// Intepret pointer event into gestures. This happens because the recognizer has already won.
    fn handle_arena_victory(&mut self, position: &Self::HitPosition, event: &PointerEvent);

    /// The recognizer has been given ultimatum that this arena is going to close no matter its reply.
    /// This happnes because a second pointer down event has been fired after the first pointer up event has been fired.
    ///
    /// The arena will be forcefully swept even if this recognizer still reports inconclusive.
    /// If this recognizer reports inconclusive, then it will be treated as confidence 0.
    fn handle_arena_close(&mut self, trigger: &PointerEvent);

    /// Handle defeat and clean up. This happens because the arena has picked another winner.
    fn handle_arena_defeat(&mut self, pointer_id: PointerId);

    /// Report all other pointer ids that this recognizer is intepreting as a single multi-touch gesture.
    ///
    /// In order to recognize a multi-touch gesture, the gesture recognizer should be in multiple gesture arenas at the same time.
    /// And victory/defeat in any arena will propagate to other arena.
    ///
    /// The results are expected to be symmetric and transitive. You may include 
    fn associated_pointers(&self, pointer_id: PointerId) -> Option<Vec<PointerId>>;
}

pub trait AnyTransformedGestureRecognizer {
    fn current_recognition_result(&self, pointer_id: PointerId) -> RecognitionResult;

    fn handle_arena_compete(&self, event: &PointerEvent) -> RecognitionResult;

    fn handle_arena_victory(&self, event: &PointerEvent);

    fn handle_arena_close(&self, trigger: &PointerEvent) -> RecognitionResult;

    fn handle_arena_defeat(&self, pointer_id: PointerId);

    fn associated_pointers(&self, pointer_id: PointerId) -> Option<Vec<PointerId>>;

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
    fn current_recognition_result(&self, pointer_id: PointerId) -> RecognitionResult {
        self.recognizer
            .lock()
            .current_recognition_result(pointer_id)
    }

    fn handle_arena_compete(&self, event: &PointerEvent) -> RecognitionResult {
        let mut recognizer = self.recognizer.lock();
        recognizer.handle_arena_compete(&self.hit_position, event);
        recognizer.current_recognition_result(event.base.pointer_id.clone())
    }

    fn handle_arena_victory(&self, event: &PointerEvent) {
        self.recognizer
            .lock()
            .handle_arena_victory(&self.hit_position, event)
    }

    fn handle_arena_defeat(&self, pointer_id: PointerId) {
        self.recognizer.lock().handle_arena_defeat(pointer_id)
    }

    fn handle_arena_close(&self, trigger: &PointerEvent) -> RecognitionResult {
        let mut recognizer = self.recognizer.lock();
        recognizer.handle_arena_close(trigger);
        recognizer.current_recognition_result(trigger.base.pointer_id)
    }
    fn associated_pointers(&self, pointer_id: PointerId) -> Option<Vec<PointerId>> {
        self.recognizer.lock().associated_pointers(pointer_id)
    }

    fn recognizer_type_id(&self) -> TypeId {
        TypeId::of::<R>()
    }
}
