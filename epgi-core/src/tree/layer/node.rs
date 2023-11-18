use crate::{
    foundation::{Arc, Canvas, Key, LayerProtocol, Protocol},
    tree::{Render, RenderObject},
};

use super::{
    AnyLayerNode, ArcAdoptedLayerNode, ArcAnyLayerNode, ArcChildLayerNode, ChildLayerNode,
    ChildLayerOrFragmentRef, Layer, LayerCompositionConfig, NoRecompositeToken,
};

pub struct PaintCache<C: Canvas, T> {
    pub(crate) paint_results: PaintResults<C>,
    /// This field should always be None if the layer does not enable cached composition
    /// There is no point in storing adopt results if the layer is going to perform tree walk anyway
    composite_results: Option<CompositeResults<C, T>>,
}

impl<C, T> PaintCache<C, T>
where
    C: Canvas,
{
    pub(crate) fn new(
        paint_results: PaintResults<C>,
        composite_results: Option<CompositeResults<C, T>>,
    ) -> Self {
        Self {
            paint_results,
            composite_results,
        }
    }

    pub(crate) fn composite_results_ref(
        &self,
        token: NoRecompositeToken,
    ) -> Option<&CompositeResults<C, T>> {
        self.composite_results.as_ref()
    }

    pub(crate) fn insert_composite_results(
        &mut self,
        results: CompositeResults<C, T>,
    ) -> &mut CompositeResults<C, T> {
        self.composite_results.insert(results)
    }
}

pub struct PaintResults<C: Canvas> {
    pub structured_children: Vec<StructuredChildLayerOrFragment<C>>,
    pub detached_children: Vec<ComposableUnadoptedLayer<C>>,
}

pub struct CompositeResults<C: Canvas, T> {
    pub(crate) unadopted_layers: Vec<ComposableUnadoptedLayer<C>>,
    pub(crate) cached_composition: T,
}

pub enum StructuredChildLayerOrFragment<C: Canvas> {
    Fragment(C::Encoding),
    StructuredChild(ComposableChildLayer<C>),
}

impl<'a, C> Into<ChildLayerOrFragmentRef<'a, C>> for &'a StructuredChildLayerOrFragment<C>
where
    C: Canvas,
{
    fn into(self) -> ChildLayerOrFragmentRef<'a, C> {
        match self {
            StructuredChildLayerOrFragment::Fragment(x) => ChildLayerOrFragmentRef::Fragment(x),
            StructuredChildLayerOrFragment::StructuredChild(x) => {
                ChildLayerOrFragmentRef::StructuredChild(x)
            }
        }
    }
}

pub struct ComposableChildLayer<C: Canvas> {
    pub config: LayerCompositionConfig<C>,
    pub layer: ArcChildLayerNode<C>,
}

#[derive(derivative::Derivative)]
#[derivative(Clone(bound = ""))]
pub struct ComposableUnadoptedLayer<C: Canvas> {
    pub config: LayerCompositionConfig<C>,
    pub adopter_key: Option<Arc<dyn Key>>,
    pub layer: ArcAnyLayerNode,
}

pub struct ComposableAdoptedLayer<C: Canvas> {
    pub config: LayerCompositionConfig<C>,
    pub layer: ArcAdoptedLayerNode<C>,
}

impl<R, L> AnyLayerNode for RenderObject<R>
where
    R: Render<LayerOrUnit = L>,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
    L: Layer<
        ParentCanvas = <R::ParentProtocol as Protocol>::Canvas,
        ChildCanvas = <R::ChildProtocol as Protocol>::Canvas,
    >,
{
    fn mark(&self) -> &super::LayerMark {
        todo!()
    }

    fn as_any_arc_adopted_layer(self: Arc<Self>) -> Box<dyn std::any::Any> {
        todo!()
    }

    fn get_composited_cache_box(&self) -> Option<Box<dyn std::any::Any + Send + Sync>> {
        todo!()
    }
}

impl<R, L> ChildLayerNode<L::ParentCanvas> for RenderObject<R>
where
    R: Render<LayerOrUnit = L>,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
    L: Layer<
        ParentCanvas = <R::ParentProtocol as Protocol>::Canvas,
        ChildCanvas = <R::ChildProtocol as Protocol>::Canvas,
    >,
{
    fn as_arc_any_layer_node(self: Arc<Self>) -> ArcAnyLayerNode {
        todo!()
    }
}
