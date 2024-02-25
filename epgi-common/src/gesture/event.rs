use std::time::Instant;

use epgi_2d::{BoxOffset, Point2d};

#[derive(Clone, Debug)]
pub struct PointerEvent {
    pub base: PointerBaseData,
    pub inner: PointerEventInner,
}

#[derive(Clone, Debug)]
pub enum PointerEventInner {
    Hover {
        hover: PointerHoverData,
        synthesized: bool,
    },
    Down {
        active: PointerActiveData,
    },
    Move {
        active: PointerActiveData,
        synthesized: bool,
    },
    Up {
        hover: PointerHoverData,
    },
    Cancel,
    Add,
    Remove,
    PanZoomStart {
        synthesized: bool,
    },
    PanZoomUpdate {
        update: PointerPanZoonUpdateData,
        synthesized: bool,
    },
    PanZoomEnd {
        synthesized: bool,
    },
    Signal {
        signal: PointerSignalData,
    },
}

// pub enum PointerEvent {
//     Hover(PointerHoverEvent),
//     Down(PointerDownEvent),
//     Move(PointerMoveEvent),
//     Up(PointerUpEvent),
//     Cancel(PointerCancelEvent),
//     Add(PointerAddEvent),
//     Remove(PointerRemoveEvent),
//     PanZoomStart(PointerPanZoomStartEvent),
//     PanZoomUpdate(PointerPanZoomUpdateEvent),
//     PanZoomEnd(PointerPanZoomEndEvent),
//     Signal(PointerSignalEvent),
//     // Scroll(PointerScrollEvent),
//     // ScrollInertialCancel(PointerScrollInertialCancelEvent),
//     // Scale(PointerScaleEvent),
// }

// /// The pointer has moved with respect to the device while the pointer is not in contact with the device.
// ///
// /// Hover event has no registered buttons, even for stylus, since we followed https://github.com/flutter/flutter/issues/30454
// pub struct PointerHoverEvent {
//     pub base: PointerBaseData,
//     pub hover: PointerHoverData,
//     pub synthesized: bool,
// }

// pub struct PointerDownEvent {
//     pub base: PointerBaseData,
//     pub active: PointerActiveData,
// }

// pub struct PointerMoveEvent {
//     pub base: PointerBaseData,
//     pub active: PointerActiveData,
//     pub synthesized: bool,
// }

// pub struct PointerUpEvent {
//     pub base: PointerBaseData,
//     pub hover: PointerHoverData,
// }

// pub struct PointerCancelEvent {
//     pub base: PointerBaseData,
// }

// pub struct PointerAddEvent {
//     pub base: PointerBaseData,
// }

// pub struct PointerRemoveEvent {
//     pub base: PointerBaseData,
// }

// pub struct PointerPanZoomStartEvent {
//     pub base: PointerBaseData,
//     pub synthesized: bool,
// }

// pub struct PointerPanZoomUpdateEvent {
//     pub base: PointerBaseData,
//     pub synthesized: bool,
//     pub pan: BoxOffset,
//     // pub pan_delta: BoxOffset,
//     pub scale: f32,
//     pub rotation: f32,
// }

// pub struct PointerPanZoomEndEvent {
//     pub base: PointerBaseData,
//     pub synthesized: bool,
// }

// pub struct PointerSignalEvent {
//     pub base: PointerBaseData,
//     pub signal: PointerSignalData,
// }

#[derive(Clone, Debug)]
pub struct PointerBaseData {
    pub time_stamp: Instant,
    pub physical_position: Point2d,
    pub pointer_kind: PointerDeviceKind,
    pub pointer_id: PointerId,
}
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct PointerId(pub(crate) u64);

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum PointerDeviceKind {
    Touch,
    Mouse,
    Stylus,
    InvertedStylus,
    Trackpad,
}

#[derive(Clone, Debug)]
pub struct PointerActiveData {
    pub buttons: PointerButtons,
    // Pointer down could be caused by a stylus secondary button which is still hovering
    pub hover: PointerHoverData,
    pub contact: PointerContactData,
}

#[derive(Clone, Debug)]
pub struct PointerContactData {
    pub pressure: f32,
    pub pressure_min: f32,
    pub pressure_max: f32,
    pub size: f32,
    pub radius_major: f32,
    pub radius_minor: f32,
    pub radius_min: f32,
    pub radius_max: f32,
    pub orientation: f32,
    pub tilt: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct PointerHoverData {
    pub distance: f32,
    pub distance_max: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct PointerPanZoonUpdateData {
    pub pan: BoxOffset,
    // pub pan_delta: BoxOffset,
    pub scale: f32,
    pub rotation: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum PointerSignalData {
    Scroll { physical_delta: BoxOffset },
    ScrollInertialCancel,
    Scale { scale: f32 },
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct PointerButtons: u32 {
        const PRIMARY_BUTTON = 0x1;
        const SECONDARY_BUTTON = 0x2;
        const TERTIARY_BUTTON = 0x4;
        const BACK_MOUSE_BUTTON = 0x8;
        const FORWARD_MOUSE_BUTTON = 0x10;
        const PRIMARY_MOUSE_BUTTON = 0x1;
        const SECONDARY_MOUSE_BUTTON = 0x2;
        const MIDDLE_MOUSE_BUTTON = 0x4;
        const STYLUS_CONTACT = 0x1;
        const PRIMARY_STYLUS_BUTTON = 0x2;
        const SECONDARY_STYLUS_BUTTON = 0x4;
        const TOUCH_CONTACT = 0x1;
    }
}
