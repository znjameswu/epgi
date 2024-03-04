use epgi_2d::Point2d;
use epgi_common::PointerButtons;

pub(crate) trait ToEpgiExt {
    type Output;
    fn to_epgi(&self) -> Self::Output;
}

impl ToEpgiExt for winit::dpi::PhysicalPosition<f64> {
    type Output = Point2d;
    fn to_epgi(&self) -> Point2d {
        Point2d {
            x: self.x as _,
            y: self.y as _,
        }
    }
}

impl ToEpgiExt for winit::event::MouseButton {
    type Output = PointerButtons;

    fn to_epgi(&self) -> Self::Output {
        use winit::event::MouseButton::*;
        match self {
            Left => PointerButtons::PRIMARY_MOUSE_BUTTON,
            Right => PointerButtons::SECONDARY_MOUSE_BUTTON,
            Middle => PointerButtons::MIDDLE_MOUSE_BUTTON,
            Back => PointerButtons::BACK_MOUSE_BUTTON,
            Forward => PointerButtons::FORWARD_MOUSE_BUTTON,
            Other(index) => PointerButtons::from_bits_retain(1 << index + 5),
        }
    }
}
