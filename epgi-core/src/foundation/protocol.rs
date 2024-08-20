use std::{borrow::Borrow, fmt::Debug, ops::Mul};

use crate::tree::{
    ArcAnyLayerRenderObject, ArcChildLayerRenderObject, ArcChildRenderObject,
    LayerCompositionConfig, PaintResults,
};

use super::{Asc, Key};

pub trait Protocol: std::fmt::Debug + Copy + Clone + Send + Sync + 'static {
    type Constraints: PartialEq + Clone + Debug + Send + Sync;
    type Offset: Clone + Debug + Send + Sync + 'static;
    type Size: Clone + Debug + Send + Sync + 'static;
    // We cannot use reference to return intrinsic results, because we would still need to cache the result before returning.
    type Intrinsics: Intrinsics;
    type Canvas: Canvas;

    fn position_in_shape(
        position: &<Self::Canvas as Canvas>::HitPosition,
        offset: &Self::Offset,
        size: &Self::Size,
    ) -> bool;
}

pub trait LayerProtocol: Protocol {
    fn zero_offset() -> Self::Offset;

    fn offset_layer_transform(
        offset: &Self::Offset,
        transform: &<<Self as Protocol>::Canvas as Canvas>::Transform,
    ) -> <<Self as Protocol>::Canvas as Canvas>::Transform;

    fn offset_layer_composition_config(
        offset: &Self::Offset,
        config: &LayerCompositionConfig<Self::Canvas>,
    ) -> LayerCompositionConfig<Self::Canvas>;
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

pub trait Canvas: Clone + Sized + 'static {
    type Transform: Mul<Self::Transform, Output = Self::Transform>
        + Transform<Self>
        + TransformHitPosition<Self, Self>;
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
}

pub trait PaintContext {
    type Canvas: Canvas;

    fn add_command(&mut self, command: <Self::Canvas as Canvas>::PaintCommand<'_>);

    /// Call this method instead of [ChildRenderObject::paint]!
    /// [ChildRenderObject::paint]'s signature uses the actual concrete implementation of [PaintContext].
    /// This is different from this generic trait, which is designed to abstract over both the concrete implementation
    /// and the paint scanner for parallel painting.
    fn paint<P: Protocol<Canvas = Self::Canvas>>(
        &mut self,
        child: &ArcChildRenderObject<P>,
        offset: &P::Offset,
    );

    // fn paint_multiple<'a, P: Protocol<Canvas = Self::Canvas>>(
    //     &'a mut self,
    //     child_transform_pairs: impl Container<Item = (&'a ArcChildRenderObject<P>, &'a P::Transform)>,
    // );

    // Temporary signature
    fn add_layer<P: LayerProtocol<Canvas = Self::Canvas>>(
        &mut self,
        layer: ArcChildLayerRenderObject<Self::Canvas>,
        offset: &P::Offset,
    );

    // Temporary signature
    fn add_orphan_layer<P: LayerProtocol<Canvas = Self::Canvas>>(
        &mut self,
        layer: ArcAnyLayerRenderObject,
        adopter_key: Asc<dyn Key>,
        offset: &P::Offset,
    );

    fn with_transform(
        &mut self,
        transform: <Self::Canvas as Canvas>::Transform,
        op: impl FnOnce(&mut Self),
    );
}

pub trait Encoding<T>: Send + Sync + 'static {
    fn composite(&mut self, src: &Self, transform: Option<&T>);

    fn clear(&mut self);
}

pub trait TransformHitPosition<PC: Canvas, CC: Canvas>: Send + Sync {
    fn transform(&self, input: &PC::HitPosition) -> CC::HitPosition;
}

pub trait Transform<C: Canvas>:
    TransformHitPosition<C, C> + Debug + Clone + Send + Sync + 'static
{
    fn identity() -> Self;
    fn mul(&self, other: &Self) -> Self;
    fn inv(&self) -> Option<Self>;
}

/// A protocol that can be translated into P, so that widget with P as its `ParentProtocol`
/// can also behave like [ChildWidget] with this protocol
pub trait SurrogateProtocol<P: Protocol>: Protocol<Canvas = P::Canvas> {
    fn convert_constraints(value: &Self::Constraints) -> impl Borrow<<P as Protocol>::Constraints>;
    fn convert_offset(value: Self::Offset) -> P::Offset;
    fn recover_size(value: P::Size) -> Self::Size;
    fn convert_intrinsics(value: Self::Intrinsics) -> Result<P::Intrinsics, Self::Intrinsics>;
    fn recover_intrinsics(value: P::Intrinsics) -> Self::Intrinsics;
}

impl<P: Protocol> SurrogateProtocol<P> for P {
    fn convert_constraints(value: &Self::Constraints) -> impl Borrow<<P as Protocol>::Constraints> {
        value
    }
    fn convert_offset(value: Self::Offset) -> <P as Protocol>::Offset {
        value
    }
    fn recover_size(value: <P as Protocol>::Size) -> Self::Size {
        value
    }
    fn convert_intrinsics(
        value: Self::Intrinsics,
    ) -> Result<<P as Protocol>::Intrinsics, Self::Intrinsics> {
        Ok(value)
    }
    fn recover_intrinsics(value: <P as Protocol>::Intrinsics) -> Self::Intrinsics {
        value
    }
}
