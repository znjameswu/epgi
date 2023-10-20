use crate::{BoxOffset, Point2d, RRectRadius, Rect};

pub trait IntoKurbo {
    type Output;
    fn into_kurbo(self) -> Self::Output;
}

impl IntoKurbo for Point2d {
    type Output = peniko::kurbo::Point;

    fn into_kurbo(self) -> Self::Output {
        peniko::kurbo::Point {
            x: self.x as _,
            y: self.y as _,
        }
    }
}

impl IntoKurbo for Rect {
    type Output = peniko::kurbo::Rect;

    fn into_kurbo(self) -> Self::Output {
        peniko::kurbo::Rect {
            x0: self.l as _,
            y0: self.t as _,
            x1: self.r as _,
            y1: self.b as _,
        }
    }
}

pub fn into_kurbo_rrect(rect: Rect, radius: RRectRadius) -> peniko::kurbo::RoundedRect {
    peniko::kurbo::RoundedRect::new(
        rect.l as _,
        rect.t as _,
        rect.r as _,
        rect.b as _,
        peniko::kurbo::RoundedRectRadii {
            top_left: radius.tl_left as _,
            top_right: radius.tr_right as _,
            bottom_right: radius.bottom_right as _,
            bottom_left: radius.bottom_left as _,
        },
    )
}

pub const KURBO_RECT_ALL: peniko::kurbo::Rect = peniko::kurbo::Rect {
    x0: f64::MIN,
    y0: f64::MIN,
    x1: f64::MAX,
    y1: f64::MAX,
};
