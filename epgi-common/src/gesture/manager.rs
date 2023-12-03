use std::{collections::HashMap, sync::Mutex};

use epgi_core::{
    foundation::{SyncMpscReceiver, SyncMpscSender},
    tree::HitTestResults,
};

// use crate::gesture::PointerEventKind;

use crate::gesture::{PointerDownEvent, PointerPanZoomStartEvent};

use super::PointerEvent;

struct GestureManager {
    // We choose to use channel instead of arc mutex, because mutex could be unfair and thus indefinitely block our scheduler
    rx: SyncMpscReceiver<PointerEvent>,
    hit_test_cache: HashMap<usize, HitTestResults>,
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
        use PointerEvent::*;
        match event {
            Down(PointerDownEvent { common, active }) => {
                // let mut hit_test_result = HitTestResults::new();
            },
            PanZoomStart(PointerPanZoomStartEvent {
                common,
                synthesized,
            }) => todo!(),

            Signal(_) => todo!(),
            Hover(_) => todo!(),

            Up(_) => todo!(),
            Cancel(_) => todo!(),
            PanZoomEnd(_) => todo!(),

            Move(_) => todo!(),
            PanZoomUpdate(_) => todo!(),

            Add(_) => todo!(),
            Remove(_) => todo!(),
        }
    }

    fn dispatch_event() {}

    fn handle_event(event: PointerEvent) {}
}
