use std::{any::TypeId, collections::HashMap};

use epgi_2d::{Affine2d, Affine2dCanvas, BoxProtocol};
use epgi_core::{
    foundation::{SyncMpscReceiver, SyncMpscSender},
    tree::{ArcChildRenderObject, ChildHitTestEntry},
};

// use crate::gesture::PointerEventKind;

use crate::gesture::{PointerEventHandler, PointerEventVariantData, PointerInteractionVariantData};

use super::{PointerEvent, PointerInteractionId};

pub struct GestureArenaId();

struct PointerGestureManager {
    // We choose to use channel instead of arc mutex, because mutex could be unfair and thus indefinitely block our scheduler
    rx: SyncMpscReceiver<PointerEvent>,
    pointers_in_contact:
        HashMap<PointerInteractionId, Vec<Box<dyn ChildHitTestEntry<Affine2dCanvas>>>>,
    // arenas: HashMap<PointerInteractionId, Vec<GestureRecognizerHandle>>,
}

struct GestureArena {}

#[derive(Clone, Debug)]
struct GestureManagerHandle {
    tx: SyncMpscSender<PointerEvent>,
}

impl PointerGestureManager {
    fn flush(&mut self) {
        while let Ok(event) = self.rx.try_recv() {}
    }

    fn handle_pointer_event(
        &mut self,
        event: PointerEvent,
        root: ArcChildRenderObject<BoxProtocol>,
    ) {
        use PointerEventVariantData::*;
        use PointerInteractionVariantData::*;
        match event.variant {
            Add => todo!(),
            Remove => todo!(),

            Interaction {
                variant: Down(_) | PanZoomStart,
                ..
            }
            | Signal(_)
            | Hover(_) => {
                let hit_test_result =
                    root.hit_test(&event.common.physical_position, &Affine2d::IDENTITY);
                let entries = hit_test_result
                    .map(|hit_test_result| {
                        hit_test_result.find_interface::<dyn PointerEventHandler>(None)
                    })
                    .unwrap_or_default();

                if let Interaction { interaction_id, .. } = event.variant {
                    self.pointers_in_contact.insert(interaction_id, entries);
                }
            }

            Interaction {
                variant: Move(_) | PanZoomUpdate(_),
                interaction_id,
            } => {
                self.pointers_in_contact.get(&interaction_id);
            }

            Interaction {
                variant: Up(_) | Cancel | PanZoomEnd,
                interaction_id,
            } => {
                self.pointers_in_contact.remove(&interaction_id);
            }
        }
    }

    fn dispatch_event() {}

    fn handle_event(event: PointerEvent) {}
}


