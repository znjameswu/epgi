use std::{any::TypeId, time::Instant};

use epgi_2d::Affine2dCanvas;
use epgi_core::{
    foundation::PtrEq,
    tree::{AnyTransformedHitTestEntry, AweakAnyRenderObject, ChildHitTestEntry},
};
use smallvec::SmallVec;

use crate::gesture::AnyTransformedGestureRecognizerContainer;

use super::{
    AnyTransformedGestureRecognizerWrapper, PointerInteractionEvent, PointerInteractionId,
    RecognitionResult, RecognizerResponse,
};

pub struct GestureArena {
    state: GestureArenaState,
}

enum GestureArenaState {
    Competing { teams: Vec<GestureArenaTeam> },
    Resolved { winner: GestureRecognizerHandle },
    Closed,
}

impl GestureArenaState {
    fn is_resolved(&self) -> bool {
        matches!(&self, GestureArenaState::Resolved { .. })
    }

    fn is_closed(&self) -> bool {
        matches!(&self, GestureArenaState::Closed)
    }
}

struct GestureArenaTeam {
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
    member_handle: GestureArenaTeamMemberHandle,
}

#[derive(Clone)]
struct GestureRecognizerKey {
    render_object: AweakAnyRenderObject,
    recognizer_type_id: TypeId,
}

pub(super) struct AssociatedUpdates {
    pub(super) inner: Vec<(PointerInteractionId, GestureRecognizerKey)>,
}

impl AssociatedUpdates {
    fn empty() -> Self {
        Self { inner: Vec::new() }
    }

    fn from(
        interaction_ids: impl IntoIterator<Item = PointerInteractionId>,
        recognizer_key: impl Fn() -> GestureRecognizerKey,
    ) -> Self {
        Self {
            inner: interaction_ids
                .into_iter()
                .map(|interaction_id| (interaction_id, recognizer_key()))
                .collect(),
        }
    }

    fn extend(
        &mut self,
        interaction_ids: impl IntoIterator<Item = PointerInteractionId>,
        recognizer_key: impl Fn() -> GestureRecognizerKey,
    ) {
        self.inner.extend(
            interaction_ids
                .into_iter()
                .map(|interaction_id| (interaction_id, recognizer_key())),
        );
    }

    fn append(&mut self, mut other: Self) {
        self.inner.append(&mut other.inner)
    }
}

impl GestureArena {
    fn handle_event(&mut self, event: &PointerInteractionEvent) -> AssociatedUpdates {
        self.update_arena_state(
            event.interaction_id,
            |_| true,
            |last_transformed_container, entry| {
                *last_transformed_container = entry.with_position(event.common.physical_position)
            },
            |_| true,
            |recognizer| recognizer.handle_event(event),
        )
    }

    fn poll_revisit(
        &mut self,
        interaction_id: PointerInteractionId,
        current: Instant,
    ) -> AssociatedUpdates {
        self.update_arena_state(
            interaction_id,
            |_| true,
            |_, _| {},
            |member| matches!(member.last_result, RecognitionResult::Inconclusive { revisit } if revisit <= current),
            |recognizer| recognizer.query_recognition_state(interaction_id),
        )
    }

    fn poll_specific(
        &mut self,
        interaction_id: PointerInteractionId,
        key: &GestureRecognizerKey,
    ) -> AssociatedUpdates {
        self.update_arena_state(
            interaction_id,
            |entry| PtrEq(&entry.render_object()) == PtrEq(&key.render_object),
            |_, _| {},
            |member| member.recognizer_type_id == key.recognizer_type_id,
            |recognizer| recognizer.query_recognition_state(interaction_id),
        )
    }

    /// This template method ensures state consistency on each state update and abstract away all the boilerplate code.
    fn update_arena_state(
        &mut self,
        interaction_id: PointerInteractionId,
        should_visit_team: impl Fn(&dyn ChildHitTestEntry<Affine2dCanvas>) -> bool,
        update_position: impl Fn(
            &mut Box<dyn AnyTransformedHitTestEntry>,
            &dyn ChildHitTestEntry<Affine2dCanvas>,
        ),
        should_visit_member: impl Fn(&GestureArenaTeamMemberHandle) -> bool,
        new_member_result: impl Fn(
            Box<dyn AnyTransformedGestureRecognizerWrapper>,
        ) -> RecognizerResponse,
    ) -> AssociatedUpdates {
        use GestureArenaState::*;
        match &mut self.state {
            Competing { teams } => {
                let mut associated_updates = AssociatedUpdates::empty();
                let mut has_requested_resolution = false;
                for team in teams.iter_mut() {
                    if !should_visit_team(team.entry.as_ref()) {
                        continue;
                    }
                    update_position(&mut team.last_transformed_container, team.entry.as_ref());
                    for member in team.members.iter_mut() {
                        if !should_visit_member(member) {
                            continue;
                        }
                        let recognizer = team
                            .last_transformed_container
                            .query_interface::<dyn AnyTransformedGestureRecognizerContainer>()
                            .expect("The entry should be a gesture recognizer container")
                            .get_recognizer(member.recognizer_type_id);
                        let Some(recognizer) = recognizer else {
                            member.last_result = RecognitionResult::Impossible;
                            continue;
                        };
                        let response = new_member_result(recognizer);
                        member.last_result = response.primary_result;
                        associated_updates.extend(response.associated_arenas, || {
                            GestureRecognizerKey {
                                render_object: team.entry.render_object(),
                                recognizer_type_id: member.recognizer_type_id,
                            }
                        });
                        if member.last_result.is_certain().is_some() {
                            has_requested_resolution = true;
                        }
                    }
                    team.members
                        .retain(|member| !member.last_result.is_impossible());
                }
                teams.retain(|team| !team.members.is_empty());

                if has_requested_resolution {
                    let winner = Self::resolve(std::mem::take(teams));
                    self.state = Resolved { winner };
                } else if teams.len() == 1 {
                    match Self::try_resolve_by_default(std::mem::take(teams)) {
                        Ok(winner) => self.state = Resolved { winner },
                        Err(_teams) => *teams = _teams,
                    }
                }
                if self.state.is_resolved() {
                    associated_updates.append(self.on_arena_resolved(interaction_id));
                }
                return associated_updates;
            }
            // Resolved branch is extracted into a separate method
            Resolved { .. } => self.update_arena_state_resolved(
                should_visit_team,
                update_position,
                should_visit_member,
                new_member_result,
            ),
            Closed => {
                debug_assert!(
                    false,
                    "An arena should not be accessible after it has closed. \
                    This indicates bugs in arena managers"
                );
                return AssociatedUpdates::empty();
            }
        }
    }

    fn on_arena_resolved(&mut self, interaction_id: PointerInteractionId) -> AssociatedUpdates {
        self.update_arena_state_resolved(
            |_| true,
            |_, _| {},
            |_| true,
            |recognizer| recognizer.handle_arena_victory(interaction_id),
        )
    }

    /// This is the resolved branch of [Self::update_arena_state]
    #[inline]
    fn update_arena_state_resolved(
        &mut self,
        should_visit_team: impl Fn(&dyn ChildHitTestEntry<Affine2dCanvas>) -> bool,
        update_position: impl Fn(
            &mut Box<dyn AnyTransformedHitTestEntry>,
            &dyn ChildHitTestEntry<Affine2dCanvas>,
        ),
        should_visit_member: impl Fn(&GestureArenaTeamMemberHandle) -> bool,
        new_member_result: impl Fn(
            Box<dyn AnyTransformedGestureRecognizerWrapper>,
        ) -> RecognizerResponse,
    ) -> AssociatedUpdates {
        use GestureArenaState::*;
        let Resolved { winner } = &mut self.state else {
            panic!("Cannot invoke this method if you are not sure this arena is resolved")
        };
        if !should_visit_team(winner.entry.as_ref()) || !should_visit_member(&winner.member_handle)
        {
            return AssociatedUpdates::empty();
        }
        update_position(
            &mut winner.last_transformed_container,
            winner.entry.as_ref(),
        );
        let recognizer = winner
            .last_transformed_container
            .query_interface::<dyn AnyTransformedGestureRecognizerContainer>()
            .expect("The entry should be a gesture recognizer container")
            .get_recognizer(winner.member_handle.recognizer_type_id);
        let Some(recognizer) = recognizer else {
            self.state = Closed;
            return AssociatedUpdates::empty();
        };
        let response = new_member_result(recognizer);
        winner.member_handle.last_result = response.primary_result;
        let associated_updates =
            AssociatedUpdates::from(response.associated_arenas, || GestureRecognizerKey {
                render_object: winner.entry.render_object(),
                recognizer_type_id: winner.member_handle.recognizer_type_id,
            });
        if let RecognitionResult::Impossible = winner.member_handle.last_result {
            self.state = Closed;
        }
        return associated_updates;
    }

    fn resolve(teams: Vec<GestureArenaTeam>) -> GestureRecognizerHandle {
        let mut highest_confidence: f32 = f32::MIN;
        let mut winner: Option<GestureRecognizerHandle> = None;
        for team in teams.into_iter() {
            use GestureRecognizerTeamPolicy::*;
            match team.policy {
                Competing | Cooperative => {
                    let mut winner_member: Option<GestureArenaTeamMemberHandle> = None;
                    for member in team.members.into_iter() {
                        if let RecognitionResult::Certain { confidence } = member.last_result {
                            if confidence > highest_confidence {
                                highest_confidence = confidence;
                                winner_member = Some(member);
                            }
                        }
                    }
                    if let Some(winner_member) = winner_member {
                        winner = Some(GestureRecognizerHandle {
                            entry: team.entry,
                            last_transformed_container: team.last_transformed_container,
                            member_handle: winner_member,
                        });
                    }
                }
                Hereditary => {
                    let mut has_requested_resolution = false;
                    for member in team.members.iter() {
                        if let RecognitionResult::Certain { confidence } = member.last_result {
                            if confidence > highest_confidence {
                                highest_confidence = confidence;
                                has_requested_resolution = true;
                            }
                        }
                    }
                    if has_requested_resolution {
                        let winner_member =
                            team.members.into_iter().next().expect("Impossible to fail");
                        winner = Some(GestureRecognizerHandle {
                            entry: team.entry,
                            last_transformed_container: team.last_transformed_container,
                            member_handle: winner_member,
                        });
                    }
                }
            }
        }
        let winner = winner.expect(
            "Resolve should only be perform on arena \
                which has at least one member requested for resolution",
        );
        return winner;
    }

    fn try_resolve_by_default(
        mut teams: Vec<GestureArenaTeam>,
    ) -> Result<GestureRecognizerHandle, Vec<GestureArenaTeam>> {
        let [team] = teams.as_mut_slice() else {
            return Err(teams);
        };
        if team.policy == GestureRecognizerTeamPolicy::Competing && team.members.len() != 1 {
            return Err(teams);
        }
        // Else, the team is either cooperative, hereditary, or competing but with only a single member.
        // In all cases, the first member will win by default.
        let [team]: [GestureArenaTeam; 1] = teams.try_into().ok().expect("Impossible to fail");
        let winner_member = team
            .members
            .into_iter()
            .next()
            .expect("Empty gesture teams should have already been deleted");
        let winner = GestureRecognizerHandle {
            entry: team.entry,
            last_transformed_container: team.last_transformed_container,
            member_handle: winner_member,
        };
        return Ok(winner);
    }
}
