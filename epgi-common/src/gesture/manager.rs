use std::collections::HashMap;

use epgi_2d::{Affine2d, Affine2dCanvas, BoxProtocol};
use epgi_core::{
    foundation::{SyncMpscReceiver, SyncMpscSender},
    tree::{ArcChildRenderObject, ChildHitTestEntry},
};

// use crate::gesture::PointerEventKind;

use crate::gesture::{PointerEventHandler, PointerEventInner};

use super::{PointerEvent, PointerId};

struct GestureManager {
    // We choose to use channel instead of arc mutex, because mutex could be unfair and thus indefinitely block our scheduler
    rx: SyncMpscReceiver<PointerEvent>,
    captured_pointers: HashMap<PointerId, Vec<Box<dyn ChildHitTestEntry<Affine2dCanvas>>>>,
}

struct GestureArena {
    
}

#[derive(Clone, Debug)]
struct GestureManagerHandle {
    tx: SyncMpscSender<PointerEvent>,
}

impl GestureManager {
    fn flush(&mut self) {
        while let Ok(event) = self.rx.try_recv() {}
    }

    fn handle_pointer_event(event: PointerEvent, root: ArcChildRenderObject<BoxProtocol>) {
        use PointerEventInner::*;
        match event.inner {
            Down { active } => todo!(),
            Signal { signal } => todo!(),
            PanZoomStart { synthesized } => todo!(),
            Hover { hover, synthesized } => {
                let hit_test_result =
                    root.hit_test(&event.base.physical_position, &Affine2d::IDENTITY);
                hit_test_result.map(|hit_test_result| {
                    hit_test_result.find_interface::<dyn PointerEventHandler>(None)
                }).unwrap_or_default();
                
            }

            Move {
                active,
                synthesized,
            } => todo!(),
            Up { hover } => todo!(),
            Cancel => todo!(),
            Add => todo!(),
            Remove => todo!(),
            PanZoomUpdate {
                update,
                synthesized,
            } => todo!(),
            PanZoomEnd { synthesized } => todo!(),
        }
    }

    fn dispatch_event() {}

    fn handle_event(event: PointerEvent) {}
}
