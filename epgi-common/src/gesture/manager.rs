mod arena;

use arena::*;

use std::{ops::Deref, time::Instant};

use epgi_2d::{Affine2d, Affine2dCanvas, BoxProtocol};
use epgi_core::{
    foundation::{Asc, AssertExt, MapEntryExtenision, SyncMpscReceiver, SyncMpscSender},
    tree::{ArcChildRenderObject, ChildHitTestEntry},
};
use hashbrown::{hash_map::Entry, HashMap};

// use crate::gesture::PointerEventKind;

use crate::gesture::{
    PointerEventVariantData, PointerInteractionEvent, PointerInteractionVariantData,
    RecognitionResult, TransformedPointerEventHandler,
};

use super::{PointerEvent, PointerInteractionId};

pub struct GestureArenaId();

pub struct PointerGestureManager {
    // We choose to use channel instead of arc mutex, because mutex could be unfair and thus indefinitely block our scheduler
    rx: SyncMpscReceiver<PointerEvent>,
    pointers_in_contact:
        HashMap<PointerInteractionId, Vec<Asc<dyn ChildHitTestEntry<Affine2dCanvas>>>>,
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
            associated_updates.append(arena.poll_revisit(interaction_id, current));
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
            entries: &Vec<impl Deref<Target = dyn ChildHitTestEntry<Affine2dCanvas>>>,
            event: &PointerEvent,
        ) {
            entries.iter().for_each(|entry| {
                entry
                    .with_position(event.common.physical_position)
                    .query_interface_ref::<dyn TransformedPointerEventHandler>()
                    .expect("The entry should be a pointer event handler")
                    .handle_pointer_event(&event)
            });
        }
        match &event.variant {
            Added => todo!(),
            Removed => todo!(),

            Signal(_) | Hover(_) => {
                let hit_test_result =
                    root.hit_test(&event.common.physical_position, &Affine2d::IDENTITY);
                let entries = hit_test_result
                    .map(|hit_test_result| {
                        hit_test_result.find_interface::<dyn TransformedPointerEventHandler>(None)
                    })
                    .unwrap_or_default();

                dispatch_pointer_event(&entries, &event);
            }

            Interaction {
                variant: Down(_) | PanZoomStart,
                interaction_id,
            } => {
                let hit_test_result =
                    root.hit_test(&event.common.physical_position, &Affine2d::IDENTITY);
                let entries: Vec<Asc<dyn ChildHitTestEntry<Affine2dCanvas>>> = hit_test_result
                    .map(|hit_test_result| {
                        hit_test_result
                            .find_interface::<dyn TransformedPointerEventHandler>(None)
                            .into_iter()
                            .map(Into::into)
                            .collect()
                    })
                    .unwrap_or_default();

                // Register gesture recognizer and dipatch pointer events.
                // We only register gesture recognizer at this time without giving them events. The events will be delivered to gesture recognizers later
                let teams = entries
                    .iter()
                    .filter_map(|entry| {
                        let handler = entry
                            .with_position(event.common.physical_position)
                            .query_interface_box::<dyn TransformedPointerEventHandler>()
                            .ok()
                            .expect("The entry should be a pointer event handler");
                        handler.handle_pointer_event(&event);
                        GestureArenaTeam::try_from_entry(entry.clone(), handler)
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

        if let Interaction {
            interaction_id,
            variant,
        } = event.variant
        {
            let associated_updates = self
                .arena_handle_event(&PointerInteractionEvent {
                    common: event.common,
                    interaction_id,
                    variant,
                })
                .expect("Arena should exist");
            self.process_associated_updates(associated_updates);
        }
    }

    fn process_associated_updates(&mut self, mut associated_updates: AssociatedUpdates) {
        loop {
            let mut next_associated_updates = AssociatedUpdates::empty();
            for (interaction_id, key) in associated_updates.inner {
                if let Some(associated_updates) = self.arena_poll_specific(interaction_id, &key) {
                    next_associated_updates.append(associated_updates);
                }
            }
            if next_associated_updates.inner.is_empty() {
                break;
            }
            associated_updates = next_associated_updates;
        }
    }
}

impl PointerGestureManager {
    fn arena_handle_event(&mut self, event: &PointerInteractionEvent) -> Option<AssociatedUpdates> {
        let mut entry = self.arenas.entry(event.interaction_id).occupied()?;
        let arena = entry.get_mut();
        let associated_updates = arena.handle_event(event);
        if arena.is_closed() {
            entry.remove();
        }
        return Some(associated_updates);
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
    ) -> Option<AssociatedUpdates> {
        let mut entry = self.arenas.entry(interaction_id).occupied()?;
        let arena = entry.get_mut();
        let associated_updates = arena.poll_specific(interaction_id, key);
        if arena.is_closed() {
            entry.remove();
        }
        return Some(associated_updates);
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
