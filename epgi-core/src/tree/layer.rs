mod fragment;
mod iterator;
mod mark;

pub use fragment::*;
pub use iterator::*;
pub use mark::*;

use std::{any::Any, ops::Mul};

use crate::foundation::{Arc, Aweak, Canvas, Identity, Key, SyncMutex};

// pub type ArcChildLayer<C> = Arc<dyn ChildLayer<ParentCanvas = C>>;
// pub type ArcParentLayer<C> = Arc<dyn ParentLayer<ChildCanvas = C>>;
// pub type AweakParentLayer<C> = Aweak<dyn ParentLayer<ChildCanvas = C>>;
// pub type ArcAnyLayer = Arc<dyn AnyLayer>;

pub type ArcChildLayerNode<C> = Arc<dyn ChildLayerNode<C>>;
// pub type ArcParentLayerNode<C> = Arc<dyn ParentLayerNode<C>>;
pub type ArcAdoptedLayerNode<C> = Arc<dyn AdoptedLayerNode<C>>;
pub type ArcAnyLayerNode = Arc<dyn AnyLayerNode>;
pub type AweakAnyLayerNode = Aweak<dyn AnyLayerNode>;
// #[allow(type_alias_bounds)]
// pub type ArcLayerNodeOf<R: Render> = Arc<
//     dyn Layer<
//         ParentCanvas = <R::ParentProtocol as Protocol>::Canvas,
//         ChildCanvas = <R::ChildProtocol as Protocol>::Canvas,
//     >,
// >;

pub trait Layer: Send + Sync + Sized + 'static {
    type ParentCanvas: Canvas;
    type ChildCanvas: Canvas;

    fn composite_to(
        &self,
        encoding: &mut <Self::ParentCanvas as Canvas>::Encoding,
        child_iterator: &mut impl ChildLayerProducingIterator<Self::ChildCanvas>,
        composition_config: &LayerCompositionConfig<Self::ParentCanvas>,
    );

    fn transform_config(
        self_config: &LayerCompositionConfig<Self::ParentCanvas>,
        child_config: &LayerCompositionConfig<Self::ChildCanvas>,
    ) -> LayerCompositionConfig<Self::ParentCanvas>;

    fn repaint(
        &self,
        old_results: Option<&PaintResults<Self::ChildCanvas>>,
    ) -> PaintResults<Self::ChildCanvas>;

    fn key(&self) -> Option<&Arc<dyn Key>>;

    // const PERFORM_ORPHAN_COMPOSITION: Option<PerformOrphanComposition<Self>> = None;

    /// Should default to the unit type `()` or [Never].
    type CachedComposition: Clone + Send + Sync;
    const CACHED_COMPOSITION_FUNCTION_TABLE: Option<CachedCompositionFunctionTable<Self>> = None;
}

// pub struct PerformOrphanComposition<L>
// where
//     L: Layer,
// {
//     composite_orphan_to: fn(
//         encoding: &mut <L::ChildCanvas as Canvas>::Encoding,
//         child_iterator: &mut NonCachingOrphanChildLayerProducingIterator<'_, L>,
//         composition_config: &LayerCompositionConfig<L::ChildCanvas>,
//     ),

//     transform_orphan_config: fn(
//         self_config: &LayerCompositionConfig<L::ChildCanvas>,
//         child_config: &LayerCompositionConfig<L::ChildCanvas>,
//     ) -> LayerCompositionConfig<L::ChildCanvas>,

//     adopter_key: fn(&L) -> Option<&Arc<dyn Key>>,
// }

pub trait OrphanLayer: Layer {
    fn composite_orphan_to(
        encoding: &mut <Self::ChildCanvas as Canvas>::Encoding,
        child_iterator: &mut impl ChildLayerProducingIterator<Self::ChildCanvas>,
        composition_config: &LayerCompositionConfig<Self::ChildCanvas>,
    );

    fn transform_orphan_config(
        self_config: &LayerCompositionConfig<Self::ChildCanvas>,
        child_config: &LayerCompositionConfig<Self::ChildCanvas>,
    ) -> LayerCompositionConfig<Self::ChildCanvas>;

    fn adopter_key(&self) -> Option<&Arc<dyn Key>>;
}

pub struct CachedCompositionFunctionTable<L: Layer> {
    pub composite_to_cache: fn(
        &L,
        child_iterator: &mut CachingChildLayerProducingIterator<'_, L::ChildCanvas>,
    ) -> L::CachedComposition,

    pub composite_from_cache_to: fn(
        &L,
        encoding: &mut <L::ParentCanvas as Canvas>::Encoding,
        cache: &L::CachedComposition,
        composition_config: &LayerCompositionConfig<L::ParentCanvas>,
    ),
}

pub trait CachedLayer: Layer {
    const PERFORM_CACHED_COMPOSITION: Option<CachedCompositionFunctionTable<Self>> =
        Some(CachedCompositionFunctionTable {
            composite_to_cache: |layer, child_iterator| {
                <Self as CachedLayer>::composite_to_cache(layer, child_iterator)
            },
            composite_from_cache_to: Self::composite_from_cache_to,
        });
    fn composite_to_cache(
        &self,
        child_iterator: &mut impl ChildLayerProducingIterator<Self::ChildCanvas>,
    ) -> Self::CachedComposition;

    fn composite_from_cache_to(
        &self,
        encoding: &mut <Self::ParentCanvas as Canvas>::Encoding,
        cache: &Self::CachedComposition,
        composition_config: &LayerCompositionConfig<Self::ParentCanvas>,
    );
}

pub trait ChildLayerProducingIterator<CC: Canvas> {
    fn for_each(
        &mut self,
        composite: impl FnMut(ChildLayerOrFragmentRef<'_, CC>) -> Vec<ComposableUnadoptedLayer<CC>>,
    );
}

pub struct LayerNode<L: Layer> {
    pub(crate) mark: LayerMark,
    pub(crate) inner: SyncMutex<LayerNodeInner<L>>,
}

pub struct LayerNodeInner<L: Layer> {
    pub(crate) layer: L,
    pub(crate) cache: Option<LayerCache<L::ChildCanvas, L::CachedComposition>>,
}

pub struct LayerCache<C: Canvas, T> {
    pub(crate) paint_results: PaintResults<C>,
    /// This field should always be None if the layer does not enable cached composition
    /// There is no point in storing adopt results if the layer is going to perform tree walk anyway
    pub(crate) composition_cache: Option<CompositionResults<C, T>>,
}

pub struct CompositionResults<C: Canvas, T> {
    pub(crate) unadopted_layers: Vec<ComposableUnadoptedLayer<C>>,
    pub(crate) cached_composition: T,
}

impl<L> LayerNode<L>
where
    L: Layer,
{
    pub(crate) fn new(layer: L) -> Self {
        Self {
            mark: LayerMark::new(),
            inner: SyncMutex::new(LayerNodeInner { layer, cache: None }),
        }
    }
}

pub trait ChildLayerNode<PC: Canvas>: Send + Sync {
    fn composite_to(
        &self,
        encoding: &mut PC::Encoding,
        composition_config: &LayerCompositionConfig<PC>,
    ) -> Vec<ComposableUnadoptedLayer<PC>>;

    fn as_arc_any_layer_node(self: Arc<Self>) -> ArcAnyLayerNode;
}

impl<L> ChildLayerNode<L::ParentCanvas> for LayerNode<L>
where
    L: Layer,
{
    fn composite_to(
        &self,
        encoding: &mut <L::ParentCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<L::ParentCanvas>,
    ) -> Vec<ComposableUnadoptedLayer<L::ParentCanvas>> {
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        let cache = inner_reborrow
            .cache
            .as_mut()
            .expect("Layer should only be composited after they are painted");
        if let Some(CachedCompositionFunctionTable {
            composite_to_cache,
            composite_from_cache_to,
        }) = L::CACHED_COMPOSITION_FUNCTION_TABLE
        {
            let composition_cache = 'composition_cache: {
                if !self.mark.needs_composite() {
                    if let Some(composition_cache) = cache.composition_cache.as_ref() {
                        composite_from_cache_to(
                            &inner_reborrow.layer,
                            encoding,
                            &composition_cache.cached_composition,
                            composition_config,
                        );
                        break 'composition_cache composition_cache;
                    }
                }
                let mut iter = CachingChildLayerProducingIterator {
                    painting_results: &cache.paint_results,
                    key: inner_reborrow.layer.key().map(Arc::as_ref),
                    unadopted_layers: Vec::new(),
                };
                let results = composite_to_cache(&inner_reborrow.layer, &mut iter);
                composite_from_cache_to(
                    &inner_reborrow.layer,
                    encoding,
                    &results,
                    composition_config,
                );
                cache.composition_cache.insert(CompositionResults {
                    unadopted_layers: iter.unadopted_layers,
                    cached_composition: results,
                })
            };
            return composition_cache
                .unadopted_layers
                .iter()
                .map(|unadopted_layer| ComposableUnadoptedLayer {
                    config: L::transform_config(composition_config, &unadopted_layer.config),
                    adopter_key: unadopted_layer.adopter_key.clone(),
                    layer: unadopted_layer.layer.clone(),
                })
                .collect();
        } else {
            let mut iter = NonCachingChildLayerProducingIterator {
                painting_results: &cache.paint_results,
                key: inner_reborrow.layer.key().map(Arc::as_ref),
                unadopted_layers: Vec::new(),
                composition_config,
                transform_config: L::transform_config,
            };
            <L as Layer>::composite_to(
                &inner_reborrow.layer,
                encoding,
                &mut iter,
                composition_config,
            );
            return iter.unadopted_layers;
        }
    }

    fn as_arc_any_layer_node(self: Arc<Self>) -> ArcAnyLayerNode {
        todo!()
    }
}

// pub trait ParentLayerNode<CC: Canvas>: Send + Sync {}

pub trait AdoptedLayerNode<PC: Canvas>: Send + Sync {
    fn composite_to(
        &self,
        encoding: &mut PC::Encoding,
        composition_config: &LayerCompositionConfig<PC>,
    ) -> Vec<ComposableUnadoptedLayer<PC>>;
}

impl<L> AdoptedLayerNode<L::ChildCanvas> for LayerNode<L>
where
    L: OrphanLayer,
{
    fn composite_to(
        &self,
        encoding: &mut <L::ChildCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<L::ChildCanvas>,
    ) -> Vec<ComposableUnadoptedLayer<L::ChildCanvas>> {
        let inner = self.inner.lock();
        let cache = inner
            .cache
            .as_ref()
            .expect("Layer should only be composited after they are painted");
        let mut iter = NonCachingOrphanChildLayerProducingIterator::<'_, L> {
            painting_results: &cache.paint_results,
            key: inner.layer.key().map(Arc::as_ref),
            unadopted_layers: Vec::new(),
            composition_config,
        };
        <L as OrphanLayer>::composite_orphan_to(encoding, &mut iter, composition_config);
        return iter.unadopted_layers;
    }
}

pub trait AnyLayerNode: crate::sync::paint_private::AnyLayerPaintExt + Send + Sync {
    fn mark(&self) -> &LayerMark;

    fn as_any_arc_adopted_layer(self: Arc<Self>) -> Box<dyn Any>;

    fn get_composited_cache_box(&self) -> Option<Box<dyn Any + Send + Sync>>;
}

impl<L> AnyLayerNode for LayerNode<L>
where
    L: Layer,
{
    fn mark(&self) -> &LayerMark {
        &self.mark
    }

    fn as_any_arc_adopted_layer(self: Arc<Self>) -> Box<dyn Any> {
        todo!()
    }

    fn get_composited_cache_box(&self) -> Option<Box<dyn Any + Send + Sync>> {
        if let Some(CachedCompositionFunctionTable { .. }) = L::CACHED_COMPOSITION_FUNCTION_TABLE {
            self.inner
                .lock()
                .cache
                .as_ref()
                .and_then(|cache| cache.composition_cache.as_ref())
                .map(|cache| Box::new(cache.cached_composition.clone()) as _)
        } else {
            None
        }
    }
}

trait ArcAnyLayerNodeExt {
    fn downcast_arc_adopted_layer<C: Canvas>(self) -> Option<ArcAdoptedLayerNode<C>>;
    // fn downcast_arc_parent_layer<C: Canvas>(self)
    //     -> Result<ArcParentLayerNode<C>, ArcAnyLayerNode>;
}

impl ArcAnyLayerNodeExt for ArcAnyLayerNode {
    fn downcast_arc_adopted_layer<C: Canvas>(self) -> Option<ArcAdoptedLayerNode<C>> {
        self.as_any_arc_adopted_layer()
            .downcast::<Arc<dyn AdoptedLayerNode<C>>>()
            .ok()
            .map(|x| *x)
    }
    // fn downcast_arc_parent_layer<C: Canvas>(
    //     self,
    // ) -> Result<ArcParentLayerNode<C>, ArcAnyLayerNode> {
    //     todo!()
    // }
}

pub struct PaintResults<C: Canvas> {
    pub structured_children: Vec<StructuredChildLayerOrFragment<C>>,
    pub detached_children: Vec<ComposableUnadoptedLayer<C>>,
}

impl<C> Default for PaintResults<C>
where
    C: Canvas,
{
    fn default() -> Self {
        Self {
            structured_children: Default::default(),
            detached_children: Default::default(),
        }
    }
}

pub enum StructuredChildLayerOrFragment<C: Canvas> {
    Fragment(C::Encoding),
    StructuredChild(ComposableChildLayer<C>),
}

pub enum ChildLayerOrFragmentRef<'a, C: Canvas> {
    Fragment(&'a C::Encoding),
    StructuredChild(&'a ComposableChildLayer<C>),
    AdoptedChild(&'a ComposableAdoptedLayer<C>),
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

#[derive(derivative::Derivative)]
#[derivative(Clone(bound = ""))]
#[non_exhaustive]
pub struct LayerCompositionConfig<C: Canvas> {
    pub transform: C::Transform,
}

impl<C> LayerCompositionConfig<C>
where
    C: Canvas,
{
    pub fn new() -> Self {
        Self {
            transform: Identity::IDENTITY,
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
