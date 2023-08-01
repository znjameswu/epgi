use std::fmt::Debug;

use crate::common::LayerScope;

use super::Asc;

pub trait Protocol: std::fmt::Debug + Copy + Clone + Send + Sync + 'static {
    type Constraints: Constraints<Self::Size>;
    type Size: Debug + Clone + Send + Sync + 'static;
    type Offset: Debug + Clone + Send + Sync + 'static;
    type Intrinsics: Intrinsics;
    type SelfTransform: Debug + Clone + Send + Sync + 'static;
    type Canvas: Canvas;
    // fn transform_canvas(
    //     transform: &Self::SelfTransform,
    //     canvas_transform: &<Self::Canvas as Canvas>::Transform,
    // ) -> Self::SelfTransform;
    // fn point_in_area(
    //     size: Self::Size,
    //     transform: Self::CanvasTransformation,
    //     point_on_canvas: BoxOffset,
    // ) -> bool;
}

pub trait Constraints<Size>: Debug + PartialEq + Clone + Send + Sync + 'static {
    fn is_tight(&self) -> bool;
    fn constrains(&self, size: Size) -> Size;
}

pub trait Intrinsics: Debug + Send + Sync {
    fn eq_tag(&self, other: &Self) -> bool;
    fn eq_param(&self, other: &Self) -> bool;
}

struct TagEq<T: Intrinsics>(T);

impl<T> PartialEq<Self> for TagEq<T>
where
    T: Intrinsics,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_tag(&other.0)
    }
}

impl<T> Eq for TagEq<T>
where
    T: Intrinsics,
{
    fn assert_receiver_is_total_eq(&self) {}
}

struct ParamEq<T: Intrinsics>(T);

impl<T> PartialEq<Self> for ParamEq<T>
where
    T: Intrinsics,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_param(&other.0)
    }
}

pub trait Canvas: Sized + 'static {
    type Transform: Debug + Clone + Send + Sync;
    type PaintCommand: Send + Sync;

    type PaintContext<'a>: PaintContext<Canvas = Self>;
    type PaintScanner<'a>: PaintContext<Canvas = Self>;

    /// The Picture class in Flutter
    type Encoding: Send + Sync + 'static;

    fn composite(
        dst: &mut Self::Encoding,
        src: &Self::Encoding,
        transform: Option<&Self::Transform>,
    );
}

pub trait PaintContext {
    type Canvas: Canvas;
    fn add_command(&mut self, command: <Self::Canvas as Canvas>::PaintCommand);

    fn with_transform(
        &mut self,
        transform: <Self::Canvas as Canvas>::Transform,
        op: impl FnOnce(&mut Self),
    );

    // fn layer(&self) -> &Asc<Layer<Self::Canvas>>;
}
