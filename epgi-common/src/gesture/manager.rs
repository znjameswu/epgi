mod arena;

pub use arena::*;

use std::{collections::VecDeque, ops::Deref};

use epgi_2d::{Affine2d, Affine2dCanvas, BoxProtocol};
use epgi_core::{
    foundation::{Asc, AssertExt, SyncMpscReceiver, SyncMpscSender},
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
    pub fn flush(&mut self, root: &ArcChildRenderObject<BoxProtocol>) {
        while let Ok(event) = self.rx.try_recv() {
            self.handle_pointer_event(event, root.clone())
        }
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
                    .query_interface::<dyn TransformedPointerEventHandler>()
                    .expect("The entry should be a pointer event handler")
                    .handle_pointer_event(&event)
            });
        }
        match &event.variant {
            Add => todo!(),
            Remove => todo!(),

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
                        let transformed_entry = entry.with_position(event.common.physical_position);
                        let handler = transformed_entry
                            .query_interface::<dyn TransformedPointerEventHandler>()
                            .expect("The entry should be a pointer event handler");
                        handler.handle_pointer_event(&event);
                        let (policy, recognizer_type_ids) = handler.all_gesture_recognizers()?;
                        Some(GestureArenaTeam {
                            policy,
                            entry: entry.clone(),
                            last_transformed_entry: transformed_entry,
                            members: recognizer_type_ids
                                .into_iter()
                                .map(|recognizer_type_id| GestureArenaTeamMemberHandle {
                                    recognizer_type_id,
                                    last_result: RecognitionResult::Possible,
                                })
                                .collect(),
                        })
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
                        GestureArena {
                            state: GestureArenaState::Competing { teams },
                        },
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
                .arenas
                .get_mut(&interaction_id)
                .expect("Arena should exist")
                .handle_event(&PointerInteractionEvent {
                    common: event.common,
                    interaction_id,
                    variant,
                });
            self.process_associated_updates(associated_updates);
        }
    }

    fn process_associated_updates(&mut self, mut associated_updates: AssociatedUpdates) {
        let mut dequeue = VecDeque::new();
        loop {
            for (interaction_id, key) in associated_updates.inner {
                let Some(arena) = self.arenas.get_mut(&interaction_id) else {
                    continue;
                };
                dequeue.push_back(arena.poll_specific(interaction_id, &key));
            }
            let Some(next_associated_updates) = dequeue.pop_front() else {
                break;
            };
            associated_updates = next_associated_updates;
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
