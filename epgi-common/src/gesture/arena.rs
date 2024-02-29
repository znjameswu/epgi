use std::any::TypeId;

use epgi_2d::{Affine2dCanvas, Point2d};
use epgi_core::{
    foundation::{CastInterfaceByRawPtr, InlinableDwsizeVec},
    tree::{AnyTransformedHitTestEntry, AweakAnyRenderObject, ChildHitTestEntry},
};
use smallvec::SmallVec;

use crate::gesture::AnyTransformedGestureRecognizerContainer;

use super::{
    AnyGestureRecognizerWrapper, AnyTransformedGestureRecognizerWrapper, PointerInteractionEvent,
    PointerInteractionId, RecognitionResult,
};

pub struct GestureArena {
    state: GestureArenaState,
}

enum GestureArenaState {
    Competing { teams: Vec<GestureRecognizerTeam> },
    Resolved { winner: GestureRecognizerHandle },
    Closed,
}

struct GestureRecognizerTeam {
    policy: GestureRecognizerTeamPolicy,
    self_has_won: bool,
    member_has_won: bool,
    entry: Box<dyn ChildHitTestEntry<Affine2dCanvas>>,
    last_transformed_container: Box<dyn AnyTransformedHitTestEntry>,
    members: SmallVec<[GestureArenaTeamMemberHandle; 1]>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum GestureRecognizerTeamPolicy {
    Competing,
    Cooperative,
    Hereditary,
}

struct GestureArenaTeamMemberHandle {
    recognizer_type_id: TypeId,
    last_result: RecognitionResult,
}

struct GestureRecognizerHandle {
    entry: Box<dyn ChildHitTestEntry<Affine2dCanvas>>,
    // TODO: Can we store Box<dyn AnyTransformedGestureRecognizerContainer> directly? This would require a query_interface_box which is hard to impl
    last_transformed_container: Box<dyn AnyTransformedHitTestEntry>,
    recognizer_type_id: TypeId,
    last_result: RecognitionResult,
}

impl GestureRecognizerHandle {
    fn get_recognizer_with_position(
        &mut self,
        hit_position: Point2d,
    ) -> Option<Box<dyn AnyTransformedGestureRecognizerWrapper>> {
        self.last_transformed_container = self.entry.with_position(hit_position);
        self.last_transformed_container
            .query_interface::<dyn AnyTransformedGestureRecognizerContainer>()
            .expect("The entry should be a gesture recognizer container")
            .get_recognizer(self.recognizer_type_id)
    }
}

#[derive(Clone)]
struct GestureRecognizerKey {
    render_object: AweakAnyRenderObject,
    recognizer_type_id: TypeId,
}

impl GestureArena {
    fn handle_event(
        &mut self,
        event: &PointerInteractionEvent,
    ) -> Vec<(PointerInteractionId, GestureRecognizerKey)> {
        use GestureArenaState::*;
        match &mut self.state {
            Competing { teams } => {
                let mut associated_updates = Vec::new();
                for team in teams.iter_mut() {
                    team.last_transformed_container =
                        team.entry.with_position(event.common.physical_position);
                    for member in team.members.iter_mut() {
                        let recognizer = team
                            .last_transformed_container
                            .query_interface::<dyn AnyTransformedGestureRecognizerContainer>()
                            .expect("The entry should be a gesture recognizer container")
                            .get_recognizer(member.recognizer_type_id);
                        let Some(mut recognizer) = recognizer else {
                            member.last_result = RecognitionResult::Impossible;
                            continue;
                        };
                        let response = recognizer.handle_event(event);
                        member.last_result = response.primary_result;
                        associated_updates.extend(response.associated_updates.into_iter().map(
                            |interaction_id| {
                                (
                                    interaction_id,
                                    GestureRecognizerKey {
                                        render_object: team.entry.render_object(),
                                        recognizer_type_id: member.recognizer_type_id,
                                    },
                                )
                            },
                        ));
                    }
                    team.members
                        .retain(|member| !member.last_result.is_impossible());
                }
                teams.retain(|team| !team.members.is_empty());
                todo!("Check if anyone requests resolution");
                todo!("Check if anyone wins by defualt");
                return associated_updates;
            }
            Resolved { winner } => {
                let recognizer =
                    winner.get_recognizer_with_position(event.common.physical_position);
                let Some(mut recognizer) = recognizer else {
                    self.state = Closed;
                    return Vec::new();
                };
                let response = recognizer.handle_event(event);
                winner.last_result = response.primary_result;
                let associated_updates = response
                    .associated_updates
                    .into_iter()
                    .map(|interaction_id| {
                        (
                            interaction_id,
                            GestureRecognizerKey {
                                render_object: winner.entry.render_object(),
                                recognizer_type_id: winner.recognizer_type_id,
                            },
                        )
                    })
                    .collect();
                if let RecognitionResult::Impossible = winner.last_result {
                    self.state = Closed;
                }
                return associated_updates;
            }
            Closed => {
                debug_assert!(
                    false,
                    "An arena should not be given events after it has closed. \
                    This indicates bugs in arena managers"
                );
                return Vec::new();
            }
        }
    }
}
