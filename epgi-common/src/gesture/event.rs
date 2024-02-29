use std::time::Instant;

use epgi_2d::{BoxOffset, Point2d};

#[derive(Clone, Debug)]
pub struct PointerEvent {
    pub common: PointerEventCommonData,
    pub variant: PointerEventVariantData,
}

// This is the events that can be truly routed by Flutter's PointerRouter
pub struct PointerInteractionEvent {
    pub common: PointerEventCommonData,
    pub interaction_id: PointerInteractionId,
    pub variant: PointerInteractionVariantData,
}

#[derive(Clone, Debug)]
pub enum PointerEventVariantData {
    // Could be synthesized
    Interaction {
        interaction_id: PointerInteractionId,
        variant: PointerInteractionVariantData,
    },
    Hover(PointerHoverData),
    Signal(PointerSignalData),
    Add,
    Remove,
}

#[derive(Clone, Debug)]
pub enum PointerInteractionVariantData {
    Down(PointerContactData),
    // Could be synthesized
    Move(PointerContactData),
    Up(PointerHoverData),
    Cancel,
    // Could be synthesized
    PanZoomStart,
    // Could be synthesized
    PanZoomUpdate(PointerPanZoomUpdateData),
    // Could be synthesized
    PanZoomEnd,
}

pub enum PointerEvent2 {
    Add(PointerAddEvent),
    Remove(PointerRemoveEvent),
    Hover(PointerHoverEvent),
    Down(PointerDownEvent),
    Move(PointerMoveEvent),
    Up(PointerUpEvent),
    Cancel(PointerCancelEvent),
    PanZoomStart(PointerPanZoomStartEvent),
    PanZoomUpdate(PointerPanZoomUpdateEvent),
    PanZoomEnd(PointerPanZoomEndEvent),
    Signal(PointerSignalEvent),
}

impl PointerEvent2 {
    fn common(&self) -> &PointerEventCommonData {
        use PointerEvent2::*;
        match self {
            Add(PointerAddEvent { common, .. })
            | Remove(PointerRemoveEvent { common, .. })
            | Hover(PointerHoverEvent { common, .. })
            | Down(PointerDownEvent { common, .. })
            | Move(PointerMoveEvent { common, .. })
            | Up(PointerUpEvent { common, .. })
            | Cancel(PointerCancelEvent { common, .. })
            | PanZoomStart(PointerPanZoomStartEvent { common, .. })
            | PanZoomUpdate(PointerPanZoomUpdateEvent { common, .. })
            | PanZoomEnd(PointerPanZoomEndEvent { common, .. })
            | Signal(PointerSignalEvent { common, .. }) => common,
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct PointerAddEvent {
    pub common: PointerEventCommonData,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct PointerRemoveEvent {
    pub common: PointerEventCommonData,
}

/// The pointer has moved with respect to the device while the pointer is not in contact with the device.
///
/// Hover event has no registered buttons, even for stylus, since we followed https://github.com/flutter/flutter/issues/30454
#[repr(C)]
#[derive(Clone, Debug)]
pub struct PointerHoverEvent {
    pub common: PointerEventCommonData,
    pub hover: PointerHoverData,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct PointerDownEvent {
    pub common: PointerEventCommonData,
    pub interaction_id: PointerInteractionId,
    pub contact: PointerContactData,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct PointerMoveEvent {
    pub common: PointerEventCommonData,
    pub interaction_id: PointerInteractionId,
    pub contact: PointerContactData,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct PointerUpEvent {
    pub common: PointerEventCommonData,
    pub interaction_id: PointerInteractionId,
    pub hover: PointerHoverData,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct PointerCancelEvent {
    pub common: PointerEventCommonData,
    pub interaction_id: PointerInteractionId,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct PointerPanZoomStartEvent {
    pub common: PointerEventCommonData,
    pub interaction_id: PointerInteractionId,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct PointerPanZoomUpdateEvent {
    pub common: PointerEventCommonData,
    pub interaction_id: PointerInteractionId,

    pub pan: BoxOffset,
    // pub pan_delta: BoxOffset,
    pub scale: f32,
    pub rotation: f32,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct PointerPanZoomEndEvent {
    pub common: PointerEventCommonData,
    pub interaction_id: PointerInteractionId,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct PointerSignalEvent {
    pub common: PointerEventCommonData,
    pub signal: PointerSignalData,
}

#[derive(Clone, Debug)]
pub struct PointerEventCommonData {
    pub time_stamp: Instant,
    pub physical_position: Point2d,
    pub pointer_kind: PointerDeviceKind,
    pub synthesized: bool,
}
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct PointerInteractionId(pub(crate) u64);

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum PointerDeviceKind {
    Touch,
    Mouse,
    Stylus,
    InvertedStylus,
    Trackpad,
}

#[derive(Clone, Debug)]
pub struct PointerContactData {
    pub buttons: PointerButtons,
    // Pointer down could be caused by a stylus secondary button which is still hovering
    pub hover: PointerHoverData,
    pub profile: PointerContactProfile,
}

#[derive(Clone, Debug)]
pub struct PointerContactProfile {
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
pub struct PointerPanZoomUpdateData {
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
