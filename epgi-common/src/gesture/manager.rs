use std::{collections::HashMap, sync::Mutex};

use epgi_2d::Affine2dCanvas;
use epgi_core::{
    foundation::{SyncMpscReceiver, SyncMpscSender},
    tree::HitTestConfig,
};

// use crate::gesture::PointerEventKind;

use crate::gesture::PointerEventInner;

use super::{PointerEvent, PointerId};

struct GestureManager {
    // We choose to use channel instead of arc mutex, because mutex could be unfair and thus indefinitely block our scheduler
    rx: SyncMpscReceiver<PointerEvent>,
    // captured_pointers: HashMap<PointerId, HitTestResults<Affine2dCanvas>>, TODO
}

#[derive(Clone, Debug)]
struct GestureManagerHandle {
    tx: SyncMpscSender<PointerEvent>,
}

impl GestureManager {
    fn flush(&mut self) {
        while let Ok(event) = self.rx.try_recv() {}
    }

    fn handle_pointer_event(event: PointerEvent) {
        use PointerEventInner::*;
        match event.inner {
            Down { active } => todo!(),
            Signal { signal } => todo!(),
            PanZoomStart { synthesized } => todo!(),
            Hover { hover, synthesized } => todo!(),

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
