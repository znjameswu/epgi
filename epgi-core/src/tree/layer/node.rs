use crate::{
    foundation::{Arc, Canvas, False, Key, LayerProtocol, Protocol, True},
    tree::{
        AnyRenderObject, LayerPaint, Render, RenderObject, SelectCachedComposite,
        SelectLayerPaint,
    },
};

use super::{
    AnyLayerRenderObject, ArcAdoptedLayerRenderObject, ArcAnyLayerRenderObject,
    ArcChildLayerRenderObject, ChildLayerOrFragmentRef, ChildLayerRenderObject,
    LayerCompositionConfig, LayerMark, NoRecompositeToken,
};

pub struct LayerCache<C: Canvas, T> {
    pub(crate) paint_results: PaintResults<C>,
    /// This field should always be None if the layer does not enable cached composition
    /// There is no point in storing unadopt results if the layer is going to perform tree walk anyway
    /// However, we NEED to store adopt results for hit-test
    composite_results: Option<CompositeResults<C, T>>,
}

impl<C, T> LayerCache<C, T>
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
    pub layer: ArcChildLayerRenderObject<C>,
}

#[derive(derivative::Derivative)]
#[derivative(Clone(bound = ""))]
pub struct ComposableUnadoptedLayer<C: Canvas> {
    pub config: LayerCompositionConfig<C>,
    pub adopter_key: Option<Arc<dyn Key>>,
    pub layer: ArcAnyLayerRenderObject,
}

pub struct ComposableAdoptedLayer<C: Canvas> {
    pub config: LayerCompositionConfig<C>,
    pub layer: ArcAdoptedLayerRenderObject<C>,
}

impl<
        R,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > AnyLayerRenderObject
    for RenderObject<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Render<RenderObject = Self>
        + SelectLayerPaint<LAYER_PAINT>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    Self: AnyRenderObject
        + crate::sync::AnyLayerRenderObjectPaintExt
        + crate::sync::AnyLayerRenderObjectCompositeExt,
{
    fn mark(&self) -> &LayerMark {
        todo!()
    }

    fn as_any_arc_adopted_layer(self: Arc<Self>) -> Box<dyn std::any::Any> {
        todo!()
    }

    fn get_composited_cache_box(&self) -> Option<Box<dyn std::any::Any + Send + Sync>> {
        todo!()
    }
}

impl<
        R,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ChildLayerRenderObject<R::AdopterCanvas>
    for RenderObject<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Render<RenderObject = Self>
        + SelectLayerPaint<LAYER_PAINT>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    Self: crate::sync::ChildLayerRenderObjectCompositeExt<R::AdopterCanvas>,
{
    fn as_arc_any_layer_render_object(self: Arc<Self>) -> ArcAnyLayerRenderObject {
        todo!()
    }
}

// impl<R> ChildLayerRenderObject<<R::ParentProtocol as Protocol>::Canvas> for RenderObject<R>
// where
//     R: RenderNew<LayerPaint = True>,
//     R: LayerPaint,
//     R::ParentProtocol: LayerProtocol,
//     R::ChildProtocol: LayerProtocol,
// {
//     fn as_arc_any_layer_render_object(self: Arc<Self>) -> ArcAnyLayeredRenderObject {
//         todo!()
//     }
// }
