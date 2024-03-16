use std::{any::TypeId, time::Instant};

use epgi_2d::Point2d;
use epgi_core::foundation::{AsAny, Asc};

use super::{PointerEvent, PointerInteractionEvent, PointerInteractionId};

pub trait PointerEventHandler: Send + Sync {
    fn handle_pointer_event(&self, transformed_position: Point2d, event: &PointerEvent);

    fn all_gesture_recognizers(
        &self,
    ) -> Option<(GestureRecognizerTeamPolicy, Vec<Asc<dyn GestureRecognizer>>)> {
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
        match self {
            RecognitionResult::Certain { confidence } => Some(*confidence),
            _ => None,
        }
    }

    pub fn is_impossible(&self) -> bool {
        matches!(self, RecognitionResult::Impossible { .. })
    }

    pub fn is_inconclusive(&self) -> bool {
        matches!(self, RecognitionResult::Inconclusive { .. })
    }
}

pub struct RecognizerResponse {
    pub primary_result: RecognitionResult,
    pub associated_arenas: Vec<PointerInteractionId>,
}

impl RecognizerResponse {
    pub const fn possible() -> Self {
        Self {
            primary_result: RecognitionResult::Possible,
            associated_arenas: Vec::new(),
        }
    }

    pub const fn impossible() -> Self {
        Self {
            primary_result: RecognitionResult::Impossible,
            associated_arenas: Vec::new(),
        }
    }

    pub const fn certain(confidence: f32) -> Self {
        Self {
            primary_result: RecognitionResult::Certain { confidence },
            associated_arenas: Vec::new(),
        }
    }

    pub const fn inconclusive(revisit: Instant) -> Self {
        Self {
            primary_result: RecognitionResult::Inconclusive { revisit },
            associated_arenas: Vec::new(),
        }
    }
}

pub trait GestureRecognizer: AsAny + Send + Sync + 'static {
    /// If the primary response is impossible, then the implementation should also clean up
    /// as if handle_arena_evict has been called.
    fn handle_event(
        &self,
        transformed_position: &Point2d,
        event: &PointerInteractionEvent,
    ) -> RecognizerResponse;

    /// Query the current recognition result of this recognizer without new events arriving.
    /// This typically happens because the recognizer reported inconclusive result previously.
    fn query_recognition_state(&self, interaction_id: PointerInteractionId) -> RecognizerResponse;

    /// Intepret pointer event into gestures. This happens because the recognizer has already won.
    fn handle_arena_victory(&self, interaction_id: PointerInteractionId) -> RecognizerResponse;

    /// Handle defeat and clean up. This happens because the arena has picked another winner.
    ///
    /// The primary response will be ignored. The eviction is non-negotiable.
    ///
    /// This will also be called after handle_arena_victory when the arena is closed.
    fn handle_arena_evict(&self, interaction_id: PointerInteractionId) -> RecognizerResponse;

    fn recognizer_type_id(&self) -> TypeId;

    fn on_detach(&self);
}
