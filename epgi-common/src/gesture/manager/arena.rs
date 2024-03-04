use std::{any::TypeId, time::Instant};

use epgi_2d::Affine2dCanvas;
use epgi_core::{
    foundation::{Asc, PtrEq},
    tree::{AweakAnyRenderObject, ChildHitTestEntry},
};
use smallvec::SmallVec;

use crate::gesture::{
    AnyTransformedGestureRecognizer, GestureRecognizerTeamPolicy, RecognizerResponse,
    TransformedPointerEventHandler,
};

use super::{PointerInteractionEvent, PointerInteractionId, RecognitionResult};

pub(super) struct GestureArena {
    state: GestureArenaState,
    pending_sweep: bool,
}

impl GestureArena {
    pub(super) fn from_competing_teams(teams: Vec<GestureArenaTeam>) -> Self {
        Self {
            state: GestureArenaState::Competing { teams },
            pending_sweep: false,
        }
    }

    pub(super) fn is_resolved(&self) -> bool {
        self.state.is_resolved()
    }

    pub(super) fn is_closed(&self) -> bool {
        self.state.is_closed()
    }
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

pub(super) struct GestureArenaTeam {
    policy: GestureRecognizerTeamPolicy,
    entry: Asc<dyn ChildHitTestEntry<Affine2dCanvas>>,
    last_transformed: Box<dyn TransformedPointerEventHandler>,
    members: SmallVec<[GestureArenaTeamMemberHandle; 1]>,
}

impl GestureArenaTeam {
    pub(super) fn try_from_entry(
        entry: Asc<dyn ChildHitTestEntry<Affine2dCanvas>>,
        last_transformed: Box<dyn TransformedPointerEventHandler>,
    ) -> Option<Self> {
        let (policy, recognizer_type_ids) = last_transformed.all_gesture_recognizers()?;
        Some(Self {
            policy,
            entry,
            last_transformed,
            members: recognizer_type_ids
                .into_iter()
                .map(|recognizer_type_id| GestureArenaTeamMemberHandle {
                    recognizer_type_id,
                    last_result: RecognitionResult::Possible,
                })
                .collect(),
        })
    }
}

pub(super) struct GestureArenaTeamMemberHandle {
    pub(super) recognizer_type_id: TypeId,
    pub(super) last_result: RecognitionResult,
}

pub(super) struct GestureRecognizerHandle {
    entry: Asc<dyn ChildHitTestEntry<Affine2dCanvas>>,
    // TODO: Can we store Box<dyn AnyTransformedGestureRecognizerContainer> directly? This would require a query_interface_box which is hard to impl
    last_transformed: Box<dyn TransformedPointerEventHandler>,
    member_handle: GestureArenaTeamMemberHandle,
}

#[derive(Clone)]
pub(super) struct GestureRecognizerKey {
    render_object: AweakAnyRenderObject,
    recognizer_type_id: TypeId,
}

pub(super) struct AssociatedUpdates {
    pub(super) inner: Vec<(PointerInteractionId, GestureRecognizerKey)>,
}

impl AssociatedUpdates {
    pub(super) fn empty() -> Self {
        Self { inner: Vec::new() }
    }

    // pub(super) fn from(
    //     interaction_ids: impl IntoIterator<Item = PointerInteractionId>,
    //     recognizer_key: impl Fn() -> GestureRecognizerKey,
    // ) -> Self {
    //     Self {
    //         inner: interaction_ids
    //             .into_iter()
    //             .map(|interaction_id| (interaction_id, recognizer_key()))
    //             .collect(),
    //     }
    // }

    pub(super) fn extend(
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

    pub(super) fn extend_from_member(
        &mut self,
        interaction_ids: impl IntoIterator<Item = PointerInteractionId>,
        entry: &dyn ChildHitTestEntry<Affine2dCanvas>,
        recognizer_type_id: TypeId,
    ) {
        self.extend(interaction_ids, || GestureRecognizerKey {
            render_object: entry.render_object(),
            recognizer_type_id,
        })
    }

    // pub(super) fn append(&mut self, other: Self) {
    //     self.inner.extend(other.inner)
    // }
}

impl GestureArena {
    pub(super) fn handle_event(
        &mut self,
        event: &PointerInteractionEvent,
        associated_updates: &mut AssociatedUpdates,
    ) {
        debug_assert!(
            !self.pending_sweep,
            "Arena pending sweep should not receive more pointer events"
        );
        self.update_arena_state(
            event.interaction_id,
            |_| true,
            |last_transformed, entry| {
                *last_transformed = entry
                    .with_position(event.common.physical_position)
                    .query_interface_box::<dyn TransformedPointerEventHandler>()
                    .ok()
                    .expect("The entry should be a pointer event handler");
            },
            |_| true,
            |recognizer| recognizer.handle_event(event),
            associated_updates,
        );
    }

    pub(super) fn handle_event_and_try_sweep(
        &mut self,
        event: &PointerInteractionEvent,
        associated_updates: &mut AssociatedUpdates,
    ) {
        self.pending_sweep = true;
        // Event delivery is guarantee to touch every member, so we may check if we can sweep during the delivery iteration.
        let mut has_inconclusive = false;
        self.update_arena_state(
            event.interaction_id,
            |_| true,
            |last_transformed, entry| {
                *last_transformed = entry
                    .with_position(event.common.physical_position)
                    .query_interface_box::<dyn TransformedPointerEventHandler>()
                    .ok()
                    .expect("The entry should be a pointer event handler");
            },
            |_| true,
            |recognizer| {
                let response = recognizer.handle_event(event);
                if response.primary_result.is_inconclusive() {
                    has_inconclusive = true;
                }
                response
            },
            associated_updates,
        );
        if !has_inconclusive {
            self.sweep_immediately(event.interaction_id, associated_updates);
        }
    }

    pub(super) fn poll_revisit(
        &mut self,
        interaction_id: PointerInteractionId,
        current: Instant,
        associated_updates: &mut AssociatedUpdates,
    ) {
        self.update_arena_state(
            interaction_id,
            |_| true,
            |_, _| {},
            |member| {
                matches!(member.last_result,
                    RecognitionResult::Inconclusive { revisit } if revisit <= current
                )
            },
            |recognizer| recognizer.query_recognition_state(interaction_id),
            associated_updates,
        );
        if self.pending_sweep {
            // Well, this may have extra cost, since we visited **some** member in above iterations.
            // But resolving a pending sweep arena is never guaranteed to be the happy path
            self.sweep_if_conclusive(interaction_id, associated_updates)
        }
    }

    pub(super) fn poll_specific(
        &mut self,
        interaction_id: PointerInteractionId,
        key: &GestureRecognizerKey,
        associated_updates: &mut AssociatedUpdates,
    ) {
        self.update_arena_state(
            interaction_id,
            |entry| PtrEq(&entry.render_object()) == PtrEq(&key.render_object),
            |_, _| {},
            |member| member.recognizer_type_id == key.recognizer_type_id,
            |recognizer| recognizer.query_recognition_state(interaction_id),
            associated_updates,
        );
        if self.pending_sweep {
            self.sweep_if_conclusive(interaction_id, associated_updates);
        }
    }

    fn sweep_if_conclusive(
        &mut self,
        interaction_id: PointerInteractionId,
        associated_updates: &mut AssociatedUpdates,
    ) {
        use GestureArenaState::*;
        let has_inconclusive = match &self.state {
            Competing { teams } => teams.iter().any(|team| {
                team.members
                    .iter()
                    .any(|member| member.last_result.is_inconclusive())
            }),
            Resolved { winner } => winner.member_handle.last_result.is_inconclusive(),
            Closed => false,
        };
        if !has_inconclusive {
            self.sweep_immediately(interaction_id, associated_updates)
        }
    }

    fn sweep_immediately(
        &mut self,
        interaction_id: PointerInteractionId,
        associated_updates: &mut AssociatedUpdates,
    ) {
        let mut has_found_winner_by_swept = false;
        use GestureArenaState::*;
        match std::mem::replace(&mut self.state, Closed) {
            Competing { teams } => {
                teams.into_iter().for_each(|team| {
                    team.members.into_iter().for_each(|member| {
                        if !has_found_winner_by_swept {
                            if let Some(recognizer) = team
                                .last_transformed
                                .get_gesture_recognizer(member.recognizer_type_id)
                            {
                                has_found_winner_by_swept = true;
                                let response = recognizer.handle_arena_victory(interaction_id);
                                associated_updates.extend_from_member(
                                    response.associated_arenas,
                                    team.entry.as_ref(),
                                    member.recognizer_type_id,
                                );
                                if response.primary_result.is_impossible() {
                                    // If the response is impossible, the recognizer has done the cleanup itself
                                    // No need to evict it anymore
                                    return;
                                }
                                let response = recognizer.handle_arena_evict(interaction_id);
                                associated_updates.extend_from_member(
                                    response.associated_arenas,
                                    team.entry.as_ref(),
                                    member.recognizer_type_id,
                                );
                            }
                        } else {
                            evict_member(
                                team.last_transformed.as_ref(),
                                team.entry.as_ref(),
                                &member,
                                interaction_id,
                                associated_updates,
                            )
                        }
                    })
                });
            }
            Resolved { winner } => evict_recognizer(&winner, interaction_id, associated_updates),
            Closed => {}
        }
    }

    // fn evict_all(
    //     &mut self,
    //     interaction_id: PointerInteractionId,
    //     associated_updates: &mut AssociatedUpdates,
    // ) {
    //     use GestureArenaState::*;
    //     match std::mem::replace(&mut self.state, Closed) {
    //         Competing { teams } => teams.into_iter().for_each(|team| {
    //             team.members.into_iter().for_each(|member| {
    //                 evict_member(
    //                     team.last_transformed.as_ref(),
    //                     team.entry.as_ref(),
    //                     &member,
    //                     interaction_id,
    //                     associated_updates,
    //                 )
    //             })
    //         }),
    //         Resolved { winner } => evict_recognizer(&winner, interaction_id, associated_updates),
    //         Closed => {}
    //     }
    // }

    /// This template method ensures state consistency on each state update and abstract away all the boilerplate code.
    fn update_arena_state(
        &mut self,
        interaction_id: PointerInteractionId,
        mut should_visit_team: impl FnMut(&dyn ChildHitTestEntry<Affine2dCanvas>) -> bool,
        mut update_position: impl FnMut(
            &mut Box<dyn TransformedPointerEventHandler>,
            &dyn ChildHitTestEntry<Affine2dCanvas>,
        ),
        mut should_visit_member: impl FnMut(&GestureArenaTeamMemberHandle) -> bool,
        mut new_member_result: impl FnMut(
            Box<dyn AnyTransformedGestureRecognizer>,
        ) -> RecognizerResponse,
        associated_updates: &mut AssociatedUpdates,
    ) {
        use GestureArenaState::*;
        match &mut self.state {
            Competing { teams } => {
                let mut has_requested_resolution = false;
                for team in teams.iter_mut() {
                    if !should_visit_team(team.entry.as_ref()) {
                        continue;
                    }
                    update_position(&mut team.last_transformed, team.entry.as_ref());
                    for member in team.members.iter_mut() {
                        if !should_visit_member(member) {
                            continue;
                        }
                        let recognizer = team
                            .last_transformed
                            .get_gesture_recognizer(member.recognizer_type_id);
                        let Some(recognizer) = recognizer else {
                            member.last_result = RecognitionResult::Impossible;
                            continue;
                        };
                        let response = new_member_result(recognizer);
                        member.last_result = response.primary_result;
                        associated_updates.extend_from_member(
                            response.associated_arenas,
                            team.entry.as_ref(),
                            member.recognizer_type_id,
                        );
                        if member.last_result.is_certain().is_some() {
                            has_requested_resolution = true;
                        }
                    }
                    team.members
                        .retain(|member| !member.last_result.is_impossible());
                }
                teams.retain(|team| !team.members.is_empty());

                if has_requested_resolution {
                    let winner = Self::resolve_must_succeed(
                        std::mem::take(teams),
                        interaction_id,
                        associated_updates,
                    );
                    self.state = Resolved { winner };
                } else if teams.len() == 1 {
                    match Self::try_resolve_by_default(
                        std::mem::take(teams),
                        interaction_id,
                        associated_updates,
                    ) {
                        Ok(winner) => self.state = Resolved { winner },
                        Err(_teams) => *teams = _teams,
                    }
                }
                if self.is_resolved() {
                    self.on_arena_resolved(interaction_id, associated_updates);
                }
            }
            // Resolved branch is extracted into a separate method
            Resolved { .. } => self.update_arena_state_resolved(
                should_visit_team,
                update_position,
                should_visit_member,
                new_member_result,
                associated_updates,
            ),
            Closed => {
                debug_assert!(
                    false,
                    "An arena should not be accessible after it has closed. \
                    This indicates bugs in arena managers"
                );
            }
        }
    }

    fn on_arena_resolved(
        &mut self,
        interaction_id: PointerInteractionId,
        associated_updates: &mut AssociatedUpdates,
    ) {
        self.update_arena_state_resolved(
            |_| true,
            |_, _| {},
            |_| true,
            |recognizer| recognizer.handle_arena_victory(interaction_id),
            associated_updates,
        )
    }

    /// This is the resolved branch of [Self::update_arena_state]
    #[inline]
    fn update_arena_state_resolved(
        &mut self,
        mut should_visit_team: impl FnMut(&dyn ChildHitTestEntry<Affine2dCanvas>) -> bool,
        mut update_position: impl FnMut(
            &mut Box<dyn TransformedPointerEventHandler>,
            &dyn ChildHitTestEntry<Affine2dCanvas>,
        ),
        mut should_visit_member: impl FnMut(&GestureArenaTeamMemberHandle) -> bool,
        mut new_member_result: impl FnMut(
            Box<dyn AnyTransformedGestureRecognizer>,
        ) -> RecognizerResponse,
        associated_updates: &mut AssociatedUpdates,
    ) {
        use GestureArenaState::*;
        let Resolved { winner } = &mut self.state else {
            panic!("Cannot invoke this method if you are not sure this arena is resolved")
        };
        if !should_visit_team(winner.entry.as_ref()) || !should_visit_member(&winner.member_handle)
        {
            return;
        }
        update_position(&mut winner.last_transformed, winner.entry.as_ref());
        let recognizer = winner
            .last_transformed
            .get_gesture_recognizer(winner.member_handle.recognizer_type_id);
        let Some(recognizer) = recognizer else {
            self.state = Closed;
            return;
        };
        let response = new_member_result(recognizer);
        winner.member_handle.last_result = response.primary_result;
        associated_updates.extend_from_member(
            response.associated_arenas,
            winner.entry.as_ref(),
            winner.member_handle.recognizer_type_id,
        );
        if let RecognitionResult::Impossible = winner.member_handle.last_result {
            self.state = Closed;
        }
    }

    fn resolve_must_succeed(
        teams: Vec<GestureArenaTeam>,
        interaction_id: PointerInteractionId,
        associated_updates: &mut AssociatedUpdates,
    ) -> GestureRecognizerHandle {
        let mut highest_confidence: f32 = f32::MIN;
        let mut winner: Option<GestureRecognizerHandle> = None;
        for team in teams.into_iter() {
            use GestureRecognizerTeamPolicy::*;
            let better_candidate = match team.policy {
                Competing | Cooperative => {
                    let mut candidate: Option<GestureArenaTeamMemberHandle> = None;
                    for member in team.members.into_iter() {
                        if let RecognitionResult::Certain { confidence } = member.last_result {
                            if confidence > highest_confidence {
                                highest_confidence = confidence;
                                if let Some(old_winner_member) = candidate.replace(member) {
                                    evict_member(
                                        team.last_transformed.as_ref(),
                                        team.entry.as_ref(),
                                        &old_winner_member,
                                        interaction_id,
                                        associated_updates,
                                    );
                                }
                                continue;
                            }
                        }
                        evict_member(
                            team.last_transformed.as_ref(),
                            team.entry.as_ref(),
                            &member,
                            interaction_id,
                            associated_updates,
                        );
                    }
                    candidate
                }
                Hereditary => {
                    let mut has_better_candidate = false;
                    for member in team.members.iter() {
                        if let RecognitionResult::Certain { confidence } = member.last_result {
                            if confidence > highest_confidence {
                                highest_confidence = confidence;
                                has_better_candidate = true;
                            }
                        }
                    }
                    let mut members_iter = team.members.into_iter();
                    let candidate = has_better_candidate
                        .then(|| members_iter.next().expect("Impossible to fail"));
                    for member in members_iter {
                        evict_member(
                            team.last_transformed.as_ref(),
                            team.entry.as_ref(),
                            &member,
                            interaction_id,
                            associated_updates,
                        )
                    }
                    candidate
                }
            };
            if let Some(better_candidate) = better_candidate {
                let old_candidate = winner.replace(GestureRecognizerHandle {
                    entry: team.entry,
                    last_transformed: team.last_transformed,
                    member_handle: better_candidate,
                });
                if let Some(old_candidate) = old_candidate {
                    evict_recognizer(&old_candidate, interaction_id, associated_updates);
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
        interaction_id: PointerInteractionId,
        associated_updates: &mut AssociatedUpdates,
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
        let mut members_iter = team.members.into_iter();
        let winner_member = members_iter
            .next()
            .expect("Empty gesture teams should have already been deleted");
        for member in members_iter {
            evict_member(
                team.last_transformed.as_ref(),
                team.entry.as_ref(),
                &member,
                interaction_id,
                associated_updates,
            )
        }
        let winner = GestureRecognizerHandle {
            entry: team.entry,
            last_transformed: team.last_transformed,
            member_handle: winner_member,
        };
        return Ok(winner);
    }
}

fn evict_recognizer(
    handle: &GestureRecognizerHandle,
    interaction_id: PointerInteractionId,
    associated_updates: &mut AssociatedUpdates,
) {
    evict_member(
        handle.last_transformed.as_ref(),
        handle.entry.as_ref(),
        &handle.member_handle,
        interaction_id,
        associated_updates,
    )
}

fn evict_member(
    last_transformed: &dyn TransformedPointerEventHandler,
    entry: &dyn ChildHitTestEntry<Affine2dCanvas>,
    member: &GestureArenaTeamMemberHandle,
    interaction_id: PointerInteractionId,
    associated_updates: &mut AssociatedUpdates,
) {
    last_transformed
        .get_gesture_recognizer(member.recognizer_type_id)
        .map(|recognizer| {
            associated_updates.extend_from_member(
                recognizer
                    .handle_arena_evict(interaction_id)
                    .associated_arenas,
                entry,
                member.recognizer_type_id,
            )
        });
}
