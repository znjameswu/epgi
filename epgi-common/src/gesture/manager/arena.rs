use std::{any::TypeId, sync::Arc, time::Instant};

use epgi_2d::Affine2d;
use epgi_core::foundation::{Asc, Aweak, PtrEq, TransformHitPosition};
use smallvec::SmallVec;

use crate::{
    gesture::{GestureRecognizerTeamPolicy, PointerEventHandler, RecognizerResponse},
    GestureRecognizer,
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
    transform: Affine2d,
    handler: Aweak<dyn PointerEventHandler>,
    members: SmallVec<[GestureArenaTeamMemberHandle; 1]>,
}

impl GestureArenaTeam {
    pub(super) fn try_from_entry(
        transform: &Affine2d,
        handler: &Arc<dyn PointerEventHandler>,
    ) -> Option<Self> {
        let (policy, recognizers) = handler.all_gesture_recognizers()?;
        Some(Self {
            policy,
            transform: transform.clone(),
            handler: Arc::downgrade(handler),
            members: recognizers
                .into_iter()
                .map(GestureArenaTeamMemberHandle::new)
                .collect(),
        })
    }
}

pub(super) struct GestureRecognizerHandle {
    transform: Affine2d,
    handler: Aweak<dyn PointerEventHandler>,
    member_handle: GestureArenaTeamMemberHandle,
}

#[derive(Clone)]
pub(super) struct GestureRecognizerKey {
    handler: Aweak<dyn PointerEventHandler>,
    recognizer_type_id: TypeId,
}

pub(super) struct GestureArenaTeamMemberHandle {
    recognizer_type_id: TypeId,
    recognizer: Asc<dyn GestureRecognizer>,
    last_result: RecognitionResult,
}

impl GestureArenaTeamMemberHandle {
    fn new(recognizer: Asc<dyn GestureRecognizer>) -> Self {
        Self {
            recognizer_type_id: recognizer.recognizer_type_id(),
            recognizer,
            last_result: RecognitionResult::Possible,
        }
    }

    fn recognizer_type_id(&self) -> &TypeId {
        &self.recognizer_type_id
    }

    fn last_result(&self) -> &RecognitionResult {
        &self.last_result
    }

    #[inline(always)]
    fn with_recognizer(
        &mut self,
        handler: &Aweak<dyn PointerEventHandler>,
        associated_updates: &mut AssociatedUpdates,
        op: impl FnOnce(&dyn GestureRecognizer) -> RecognizerResponse,
    ) {
        let response = op(self.recognizer.as_ref());
        self.last_result = response.primary_result;
        associated_updates.extend(response.associated_arenas, || GestureRecognizerKey {
            handler: handler.clone(),
            recognizer_type_id: self.recognizer_type_id,
        });
    }
}

fn evict_recognizer(
    handle: GestureRecognizerHandle,
    interaction_id: PointerInteractionId,
    associated_updates: &mut AssociatedUpdates,
) {
    evict_member(
        &handle.handler,
        handle.member_handle,
        interaction_id,
        associated_updates,
    )
}

fn evict_member(
    handler: &Aweak<dyn PointerEventHandler>,
    mut member: GestureArenaTeamMemberHandle,
    interaction_id: PointerInteractionId,
    associated_updates: &mut AssociatedUpdates,
) {
    member.with_recognizer(handler, associated_updates, |recognizer| {
        recognizer.handle_arena_evict(interaction_id)
    });
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
        handler: &Aweak<dyn PointerEventHandler>,
        recognizer_type_id: TypeId,
    ) {
        self.extend(interaction_ids, || GestureRecognizerKey {
            handler: handler.clone(),
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
            |_| true,
            |transform, recognizer| {
                recognizer.handle_event(&transform.transform(&event.common.position), event)
            },
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
            |_| true,
            |transform, recognizer| {
                let response =
                    recognizer.handle_event(&transform.transform(&event.common.position), event);
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
            |member| {
                matches!(member.last_result,
                    RecognitionResult::Inconclusive { revisit } if revisit <= current
                )
            },
            |_, recognizer| recognizer.query_recognition_state(interaction_id),
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
            |handler| PtrEq(handler) == PtrEq(&key.handler),
            |member| member.recognizer.recognizer_type_id() == key.recognizer_type_id,
            |_, recognizer| recognizer.query_recognition_state(interaction_id),
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
                    team.members.into_iter().for_each(|mut member| {
                        if !has_found_winner_by_swept {
                            has_found_winner_by_swept = true;
                            member.with_recognizer(
                                &team.handler,
                                associated_updates,
                                |recognizer| recognizer.handle_arena_victory(interaction_id),
                            );
                            if member.last_result.is_impossible() {
                                // If the response is impossible, the recognizer has done the cleanup itself
                                // No need to evict it anymore
                                return;
                            }
                            member.with_recognizer(
                                &team.handler,
                                associated_updates,
                                |recognizer| recognizer.handle_arena_evict(interaction_id),
                            );
                        } else {
                            evict_member(&team.handler, member, interaction_id, associated_updates)
                        }
                    })
                });
            }
            Resolved { winner } => evict_recognizer(winner, interaction_id, associated_updates),
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
        mut should_visit_team: impl FnMut(&Aweak<dyn PointerEventHandler>) -> bool,
        mut should_visit_member: impl FnMut(&GestureArenaTeamMemberHandle) -> bool,
        mut new_member_result: impl FnMut(&Affine2d, &dyn GestureRecognizer) -> RecognizerResponse,
        associated_updates: &mut AssociatedUpdates,
    ) {
        use GestureArenaState::*;
        match &mut self.state {
            Competing { teams } => {
                let mut has_requested_resolution = false;
                for team in teams.iter_mut() {
                    if !should_visit_team(&team.handler) {
                        continue;
                    }
                    for member in team.members.iter_mut() {
                        if !should_visit_member(member) {
                            continue;
                        }
                        member.with_recognizer(&team.handler, associated_updates, |recognizer| {
                            new_member_result(&team.transform, recognizer)
                        });
                        if member.last_result().is_certain().is_some() {
                            has_requested_resolution = true;
                        }
                    }
                    team.members
                        .retain(|member| !member.last_result().is_impossible());
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
                } else if teams.is_empty() {
                    self.state = Closed
                }
                if self.is_resolved() {
                    self.on_arena_resolved(interaction_id, associated_updates);
                }
            }
            // Resolved branch is extracted into a separate method
            Resolved { .. } => self.update_arena_state_resolved(
                should_visit_team,
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
            |_| true,
            |_, recognizer| recognizer.handle_arena_victory(interaction_id),
            associated_updates,
        )
    }

    /// This is the resolved branch of [Self::update_arena_state]
    #[inline]
    fn update_arena_state_resolved(
        &mut self,
        mut should_visit_team: impl FnMut(&Aweak<dyn PointerEventHandler>) -> bool,
        mut should_visit_member: impl FnMut(&GestureArenaTeamMemberHandle) -> bool,
        mut new_member_result: impl FnMut(&Affine2d, &dyn GestureRecognizer) -> RecognizerResponse,
        associated_updates: &mut AssociatedUpdates,
    ) {
        use GestureArenaState::*;
        let Resolved { winner } = &mut self.state else {
            panic!("Cannot invoke this method if you are not sure this arena is resolved")
        };
        if !should_visit_team(&winner.handler) || !should_visit_member(&winner.member_handle) {
            return;
        }
        winner
            .member_handle
            .with_recognizer(&winner.handler, associated_updates, |recognizer| {
                new_member_result(&winner.transform, recognizer)
            });
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
                                        &team.handler,
                                        old_winner_member,
                                        interaction_id,
                                        associated_updates,
                                    );
                                }
                                continue;
                            }
                        }
                        evict_member(&team.handler, member, interaction_id, associated_updates);
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
                        evict_member(&team.handler, member, interaction_id, associated_updates)
                    }
                    candidate
                }
            };
            if let Some(better_candidate) = better_candidate {
                let old_candidate = winner.replace(GestureRecognizerHandle {
                    transform: team.transform,
                    handler: team.handler,
                    member_handle: better_candidate,
                });
                if let Some(old_candidate) = old_candidate {
                    evict_recognizer(old_candidate, interaction_id, associated_updates);
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
            evict_member(&team.handler, member, interaction_id, associated_updates)
        }
        let winner = GestureRecognizerHandle {
            transform: team.transform,
            handler: team.handler,
            member_handle: winner_member,
        };
        return Ok(winner);
    }
}
