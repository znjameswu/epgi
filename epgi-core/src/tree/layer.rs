mod fragment;
mod iterator;
mod mark;
mod node;

pub use fragment::*;
pub use iterator::*;
pub use mark::*;
pub use node::*;

use std::{any::Any, ops::Mul};

use crate::foundation::{Arc, Aweak, Canvas, Transform};

use super::AnyRenderObject;

// pub type ArcChildLayer<C> = Arc<dyn ChildLayer<ParentCanvas = C>>;
// pub type ArcParentLayer<C> = Arc<dyn ParentLayer<ChildCanvas = C>>;
// pub type AweakParentLayer<C> = Aweak<dyn ParentLayer<ChildCanvas = C>>;
// pub type ArcAnyLayer = Arc<dyn AnyLayer>;

pub type ArcChildLayerRenderObject<C> = Arc<dyn ChildLayerRenderObject<C>>;
pub type AweakChildLayerRenderObject<C> = Aweak<dyn ChildLayerRenderObject<C>>;
pub type AweakLayeredRenderObject<PC, CC> = Arc<dyn LayerRenderObject<PC, CC>>;
pub type ArcLayeredRenderObject<PC, CC> = Arc<dyn LayerRenderObject<PC, CC>>;
pub type ArcAnyLayerRenderObject = Arc<dyn AnyLayerRenderObject>;
pub type AweakAnyLayerRenderObject = Aweak<dyn AnyLayerRenderObject>;

pub trait ChildLayerProducingIterator<CC: Canvas> {
    fn for_each(
        &mut self,
        composite: impl FnMut(ChildLayerOrFragmentRef<'_, CC>) -> Vec<ComposableUnadoptedLayer<CC>>,
    );
}

pub enum ChildLayerOrFragmentRef<'a, C: Canvas> {
    Fragment(&'a C::Encoding),
    StructuredChild(&'a ComposableChildLayer<C>),
    AdoptedChild(&'a ComposableChildLayer<C>),
}

pub trait ChildLayerRenderObject<PC: Canvas>:
    crate::sync::ChildLayerRenderObjectCompositeExt<PC> + Send + Sync
{
    fn as_arc_any_layer_render_object(self: Arc<Self>) -> ArcAnyLayerRenderObject;
}

pub trait LayerRenderObject<PC: Canvas, CC: Canvas>: Send + Sync {}

pub trait AnyLayerRenderObject:
    AnyRenderObject
    + crate::sync::AnyLayerRenderObjectPaintExt
    + crate::sync::AnyLayerRenderObjectCompositeExt
    + Send
    + Sync
{
    fn mark(&self) -> &LayerMark;

    fn as_any_arc_child_layer(self: Arc<Self>) -> Box<dyn Any>;

    fn get_composited_cache_box(&self) -> Option<Box<dyn Any + Send + Sync>>;
}

trait ArcAnyLayerRenderObjectExt {
    fn downcast_arc_adopted_layer<C: Canvas>(self) -> Option<ArcChildLayerRenderObject<C>>;
    // fn downcast_arc_parent_layer<C: Canvas>(self)
    //     -> Result<ArcParentLayerNode<C>, ArcAnyLayerNode>;
}

impl ArcAnyLayerRenderObjectExt for ArcAnyLayerRenderObject {
    fn downcast_arc_adopted_layer<C: Canvas>(self) -> Option<ArcChildLayerRenderObject<C>> {
        self.as_any_arc_child_layer()
            .downcast::<Arc<dyn ChildLayerRenderObject<C>>>()
            .ok()
            .map(|x| *x)
    }
    // fn downcast_arc_parent_layer<C: Canvas>(
    //     self,
    // ) -> Result<ArcParentLayerNode<C>, ArcAnyLayerNode> {
    //     todo!()
    // }
}

#[derive(derivative::Derivative)]
#[derivative(Clone(bound = ""))]
// #[non_exhaustive]
pub struct LayerCompositionConfig<C: Canvas> {
    pub transform: C::Transform,
}

impl<C> LayerCompositionConfig<C>
where
    C: Canvas,
{
    pub fn new() -> Self {
        Self {
            transform: <C::Transform as Transform<_>>::identity(),
        }
    }

    pub fn transform(&self) -> Option<&C::Transform> {
        Some(&self.transform)
    }
}

impl<'a, C> Mul<&'a LayerCompositionConfig<C>> for &'a LayerCompositionConfig<C>
where
    C: Canvas,
{
    type Output = &'a LayerCompositionConfig<C>;

    fn mul(self, rhs: &'a LayerCompositionConfig<C>) -> Self::Output {
        todo!()
    }
}

impl<C> Mul<LayerCompositionConfig<C>> for LayerCompositionConfig<C>
where
    C: Canvas,
{
    type Output = LayerCompositionConfig<C>;

    fn mul(self, rhs: LayerCompositionConfig<C>) -> Self::Output {
        todo!()
    }
}
