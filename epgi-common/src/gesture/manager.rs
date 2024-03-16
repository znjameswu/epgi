mod arena;

use arena::*;

use std::{any::TypeId, time::Instant};

use epgi_2d::{Affine2d, BoxProtocol};
use epgi_core::{
    foundation::{Arc, AssertExt, SyncMpscReceiver, SyncMpscSender, TransformHitPosition},
    tree::{ArcChildRenderObject, HitTestResults},
};
use hashbrown::{hash_map::Entry, HashMap};

// use crate::gesture::PointerEventKind;

use crate::gesture::{
    PointerEventHandler, PointerEventVariantData, PointerInteractionEvent,
    PointerInteractionVariantData, RecognitionResult,
};

use super::{PointerEvent, PointerInteractionId};

pub struct GestureArenaId();

pub struct PointerGestureManager {
    // We choose to use channel instead of arc mutex, because mutex could be unfair and thus indefinitely block our scheduler
    rx: SyncMpscReceiver<PointerEvent>,
    pointers_in_contact:
        HashMap<PointerInteractionId, Vec<(Affine2d, Arc<dyn PointerEventHandler>)>>,
    arenas: HashMap<PointerInteractionId, GestureArena>,
}

impl PointerGestureManager {
    pub fn new(rx: SyncMpscReceiver<PointerEvent>) -> Self {
        Self {
            rx,
            pointers_in_contact: Default::default(),
            arenas: Default::default(),
        }
    }

    pub fn flush_events(&mut self, root: &ArcChildRenderObject<BoxProtocol>) {
        while let Ok(event) = self.rx.try_recv() {
            self.handle_pointer_event(event, root.clone())
        }
    }

    pub fn poll_revisit_all(&mut self, current: Instant) {
        let mut associated_updates = AssociatedUpdates::empty();
        for (&interaction_id, arena) in self.arenas.iter_mut() {
            arena.poll_revisit(interaction_id, current, &mut associated_updates);
        }
        self.process_associated_updates(associated_updates);
    }

    fn handle_pointer_event(
        &mut self,
        event: PointerEvent,
        root: ArcChildRenderObject<BoxProtocol>,
    ) {
        use PointerEventVariantData::*;
        use PointerInteractionVariantData::*;

        fn dispatch_pointer_event(
            entries: &Vec<(Affine2d, Arc<dyn PointerEventHandler>)>,
            event: &PointerEvent,
        ) {
            entries.iter().for_each(|(transform, handler)| {
                handler.handle_pointer_event(
                    transform.transform(&event.common.physical_position),
                    &event,
                )
            });
        }
        match &event.variant {
            Added => { // TODO
            }
            Removed => { // TODO
            }

            Signal(_)
            | Hover(_)
            | Interaction {
                variant: Down(_) | PanZoomStart,
                ..
            } => {
                let mut results = HitTestResults::new(
                    event.common.physical_position,
                    TypeId::of::<dyn PointerEventHandler>(),
                );
                root.hit_test(&mut results);
                let entries = results
                    .targets
                    .into_iter()
                    .map(|(transform, render_object)| {
                        let receiver = render_object
                            .query_interface_arc::<dyn PointerEventHandler>()
                            .ok()
                            .expect(
                                "Hit test should only return render objects \
                                        with the requested interface",
                            );
                        (transform, receiver)
                    })
                    .collect::<Vec<_>>();

                dispatch_pointer_event(&entries, &event);

                if let Interaction {
                    variant: Down(_) | PanZoomStart,
                    interaction_id,
                } = &event.variant
                {
                    // Register gesture recognizer and dipatch pointer events.
                    // We only register gesture recognizer at this time without giving them events. The events will be delivered to gesture recognizers later
                    let teams = entries
                        .iter()
                        .filter_map(|(transform, handler)| {
                            // This is only for pointer event handler, not for gesture recognizers.
                            GestureArenaTeam::try_from_entry(transform, handler)
                        })
                        .collect();

                    self.pointers_in_contact
                        .insert(*interaction_id, entries)
                        .debug_assert_with(
                            Option::is_none,
                            "Interaction ID should be unique for every pointer down or pointer pan zoom start event",
                        );
                    self.arenas
                        .insert(
                            *interaction_id,
                            GestureArena::from_competing_teams(teams),
                        )
                        .debug_assert_with(
                            Option::is_none,
                            "Interaction ID should be unique for every pointer down or pointer pan zoom start event",
                        );
                }
            }

            Interaction {
                variant,
                interaction_id,
            } => {
                let Entry::Occupied(entry) = self.pointers_in_contact.entry(*interaction_id) else {
                    panic!("Pointer interaction should be registered")
                };
                dispatch_pointer_event(entry.get(), &event);
                if let Up(_) | Cancel | PanZoomEnd = variant {
                    entry.remove();
                }
            }
        }

        // Dispatch for gestures
        if let Interaction {
            interaction_id,
            variant,
        } = event.variant
        {
            let mut associated_updates = AssociatedUpdates::empty();
            if let Up(_) | Cancel | PanZoomEnd = variant {
                self.arena_handle_event_and_try_sweep(
                    &PointerInteractionEvent {
                        common: event.common,
                        interaction_id,
                        variant,
                    },
                    &mut associated_updates,
                );
            } else {
                let is_down = matches!(variant, Down(_));
                let found_arena = self.arena_handle_event(
                    &PointerInteractionEvent {
                        common: event.common,
                        interaction_id,
                        variant,
                    },
                    &mut associated_updates,
                );
                if is_down {
                    debug_assert!(
                        found_arena,
                        // Because the create arena impl has no sweep and default logic.
                        // We are obliged to try resolve by default at least once, and this is done in handle_event method.
                        "Down event should create an arena, even if there is no member inside.",
                    )
                }
            }
            self.process_associated_updates(associated_updates);
        }
    }

    fn process_associated_updates(&mut self, mut associated_updates: AssociatedUpdates) {
        loop {
            let mut next_associated_updates = AssociatedUpdates::empty();
            for (interaction_id, key) in associated_updates.inner {
                self.arena_poll_specific(interaction_id, &key, &mut next_associated_updates);
            }
            if next_associated_updates.inner.is_empty() {
                break;
            }
            associated_updates = next_associated_updates;
        }
    }
}

impl PointerGestureManager {
    fn arena_handle_event(
        &mut self,
        event: &PointerInteractionEvent,
        associated_updates: &mut AssociatedUpdates,
    ) -> bool {
        let Entry::Occupied(mut entry) = self.arenas.entry(event.interaction_id) else {
            return false;
        };
        let arena = entry.get_mut();
        arena.handle_event(event, associated_updates);
        if arena.is_closed() {
            entry.remove();
        }
        return true;
    }

    fn arena_handle_event_and_try_sweep(
        &mut self,
        event: &PointerInteractionEvent,
        associated_updates: &mut AssociatedUpdates,
    ) -> bool {
        let Entry::Occupied(mut entry) = self.arenas.entry(event.interaction_id) else {
            return false;
        };
        let arena = entry.get_mut();
        arena.handle_event_and_try_sweep(event, associated_updates);
        if arena.is_closed() {
            entry.remove();
        }
        return true;
    }

    // fn arena_poll_revisit(
    //     &mut self,
    //     interaction_id: PointerInteractionId,
    //     current: Instant,
    // ) -> Option<AssociatedUpdates> {
    //     let mut entry = self.arenas.entry(interaction_id).occupied()?;
    //     let arena = entry.get_mut();
    //     let associated_updates = arena.poll_revisit(interaction_id, current);
    //     if arena.is_closed() {
    //         entry.remove();
    //     }
    //     return Some(associated_updates);
    // }

    fn arena_poll_specific(
        &mut self,
        interaction_id: PointerInteractionId,
        key: &GestureRecognizerKey,
        associated_updates: &mut AssociatedUpdates,
    ) {
        let Entry::Occupied(mut entry) = self.arenas.entry(interaction_id) else {
            return;
        };
        let arena = entry.get_mut();
        arena.poll_specific(interaction_id, key, associated_updates);
        if arena.is_closed() {
            entry.remove();
        }
    }
}

#[derive(Clone, Debug)]
pub struct GestureManagerHandle {
    tx: SyncMpscSender<PointerEvent>,
}

impl GestureManagerHandle {
    pub fn send_pointer_event(&self, event: PointerEvent) {
        self.tx
            .send(event)
            .expect("Gesture manager should be up and running to receive pointer events")
    }
}
