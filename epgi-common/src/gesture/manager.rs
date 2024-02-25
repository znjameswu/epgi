use std::{any::TypeId, collections::HashMap};

use epgi_2d::{Affine2d, Affine2dCanvas, BoxProtocol};
use epgi_core::{
    foundation::{SyncMpscReceiver, SyncMpscSender},
    tree::{ArcChildRenderObject, ChildHitTestEntry},
};

// use crate::gesture::PointerEventKind;

use crate::gesture::{PointerEventHandler, PointerEventInner};

use super::{PointerEvent, PointerId};

pub struct GestureArenaId();

struct PointerGestureManager {
    // We choose to use channel instead of arc mutex, because mutex could be unfair and thus indefinitely block our scheduler
    rx: SyncMpscReceiver<PointerEvent>,
    captured_pointers: HashMap<PointerId, Vec<Box<dyn ChildHitTestEntry<Affine2dCanvas>>>>,
    arenas: HashMap<PointerId, Vec<GestureRecognizerHandle>>,
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
        use PointerEventInner::*;
        match event.inner {
            Down { .. } | Signal { .. } | Hover { .. } | PanZoomStart { .. } => {
                let hit_test_result =
                    root.hit_test(&event.base.physical_position, &Affine2d::IDENTITY);
                let entries = hit_test_result
                    .map(|hit_test_result| {
                        hit_test_result.find_interface::<dyn PointerEventHandler>(None)
                    })
                    .unwrap_or_default();

                if let Down { .. } | PanZoomStart { .. } = event.inner {
                    self.captured_pointers
                        .insert(event.base.pointer_id, entries);
                }
            }

            Move { .. } | PanZoomUpdate { .. } => {
                self.captured_pointers.get(&event.base.pointer_id);
            }
            Up { .. } | Cancel | PanZoomEnd { .. } => {
                self.captured_pointers.remove(&event.base.pointer_id);
            }
            Add => todo!(),
            Remove => todo!(),
        }
    }

    fn dispatch_event() {}

    fn handle_event(event: PointerEvent) {}
}

struct GestureRecognizerHandle {
    entry: Box<dyn ChildHitTestEntry<Affine2dCanvas>>,
    type_id: TypeId,
}
