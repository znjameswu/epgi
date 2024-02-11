use std::{fmt::Debug, ops::Mul};

use crate::tree::{ArcChildRenderObject, ComposableChildLayer, PaintResults};

pub trait Protocol: std::fmt::Debug + Copy + Clone + Send + Sync + 'static {
    type Constraints: PartialEq + Clone + Debug + Send + Sync;
    type Size: Clone + Debug + Send + Sync + 'static;
    type Offset: Clone + Debug + Send + Sync + 'static;
    // We cannot use reference to return intrinsic results, because we would still need to cache the result before returning.
    type Intrinsics: Intrinsics;
    type Transform: Clone + Debug + Send + Sync + 'static;
    type Canvas: Canvas;
    fn transform_canvas(
        transform: &Self::Transform,
        transform_canvas: &<Self::Canvas as Canvas>::Transform,
    ) -> Self::Transform;
    // fn point_in_area(
    //     size: Self::Size,
    //     transform: Self::CanvasTransformation,
    //     point_on_canvas: BoxOffset,
    // ) -> bool;
}

pub trait Intrinsics: Debug + Send + Sync {
    fn eq_tag(&self, other: &Self) -> bool;
    fn eq_param(&self, other: &Self) -> bool;
}

impl Intrinsics for () {
    fn eq_tag(&self, _other: &Self) -> bool {
        true
    }

    fn eq_param(&self, _other: &Self) -> bool {
        true
    }
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
    type Transform: Mul<Self::Transform, Output = Self::Transform>
        + Transform<Self, Self>
        + Debug
        + Clone
        + Send
        + Sync
        + 'static;
    type PaintCommand<'a>: Send + Sync;

    type PaintContext<'a>: PaintContext<Canvas = Self>;
    type PaintScanner<'a>: PaintContext<Canvas = Self>;

    /// The Picture class in Flutter
    type Encoding: Send + Sync + 'static;

    type HitPosition: Clone + Send + Sync;

    // fn paint_layer(
    //     layer: ArcParentLayer<Self>,
    //     scan: impl FnOnce(&mut Self::PaintScanner<'_>),
    //     paint: impl FnOnce(&mut Self::PaintContext<'_>),
    // );

    fn paint_render_objects<P: LayerProtocol<Canvas = Self>>(
        render_objects: impl IntoIterator<Item = ArcChildRenderObject<P>>,
    ) -> PaintResults<Self>;

    // fn paint_render_objects<P: Protocol<Canvas = Self>>(
    //     render_objects: impl Parallel<Item = ArcChildRenderObject<P>>,
    // ) -> PaintResults<Self>;

    // The following methods are here to avoid creating and impl-ing (outside this crate) new traits for vello encodings.
    // Although we can wrap vello encoding in a new type, I think it is too inconvenient.
    fn composite_encoding(
        dst: &mut Self::Encoding,
        src: &Self::Encoding,
        transform: Option<&Self::Transform>,
    );

    fn clear(this: &mut Self::Encoding);

    fn new_encoding() -> Self::Encoding;

    fn transform_hit_position(
        transform: &Self::Transform,
        hit_position: &Self::HitPosition,
    ) -> Self::HitPosition;

    fn identity_transform() -> Self::Transform;

    fn mul_transform_ref(a: &Self::Transform, b: &Self::Transform) -> Self::Transform;
}

pub trait PaintContext {
    type Canvas: Canvas;

    fn add_command(&mut self, command: <Self::Canvas as Canvas>::PaintCommand<'_>);

    // /// Get access to the parent layer to create a new [Layer].
    // ///
    // /// Do not call this method if you do not intend to push a new layer,
    // /// even if this method seems to allow arbitrary operation.
    // // The method was forced to designed as such to avoid mutable borrow conflicts from two closures.
    // fn paint_layered_child(
    //     &mut self,
    //     op: impl FnOnce(&<Self::Canvas as Canvas>::Transform) -> ArcChildLayer<Self::Canvas>,
    // );

    fn paint<P: Protocol<Canvas = Self::Canvas>>(
        &mut self,
        child: &ArcChildRenderObject<P>,
        transform: &P::Transform,
    );

    // fn paint_multiple<'a, P: Protocol<Canvas = Self::Canvas>>(
    //     &'a mut self,
    //     child_transform_pairs: impl Container<Item = (&'a ArcChildRenderObject<P>, &'a P::Transform)>,
    // );

    fn add_layer(&mut self, op: impl FnOnce() -> ComposableChildLayer<Self::Canvas>);
}

pub trait Encoding<T>: Send + Sync + 'static {
    fn composite(&mut self, src: &Self, transform: Option<&T>);

    fn clear(&mut self);
}

pub trait LayerProtocol:
    Protocol<Transform = <<Self as Protocol>::Canvas as Canvas>::Transform>
{
}

impl<P> LayerProtocol for P where
    P: Protocol<Transform = <<P as Protocol>::Canvas as Canvas>::Transform>
{
}

pub trait Transform<PC: Canvas, CC: Canvas> {
    fn transform(&self, input: &PC::HitPosition) -> CC::HitPosition;
}
