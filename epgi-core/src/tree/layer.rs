mod fragment;
mod iterator;
mod mark;
mod node;

pub use fragment::*;
pub use iterator::*;
pub use mark::*;
pub use node::*;

use std::{any::Any, ops::Mul};

use crate::foundation::{Arc, Aweak, Canvas, Identity, Key, LayerProtocol, Protocol};

use super::{AnyRenderObject, Render};

// pub type ArcChildLayer<C> = Arc<dyn ChildLayer<ParentCanvas = C>>;
// pub type ArcParentLayer<C> = Arc<dyn ParentLayer<ChildCanvas = C>>;
// pub type AweakParentLayer<C> = Aweak<dyn ParentLayer<ChildCanvas = C>>;
// pub type ArcAnyLayer = Arc<dyn AnyLayer>;

pub type ArcChildLayerRenderObject<C> = Arc<dyn ChildLayerRenderObject<C>>;
// pub type ArcParentLayerNode<C> = Arc<dyn ParentLayerNode<C>>;
pub type ArcAdoptedLayerRenderObject<C> = Arc<dyn AdoptedLayerRenderObject<C>>;
pub type ArcAnyLayerRenderObject = Arc<dyn AnyLayerRenderObject>;
pub type AweakAnyLayerRenderObject = Aweak<dyn AnyLayerRenderObject>;

pub trait LayerRender: Render<LayerOrUnit = Self> + Send + Sync + Sized + 'static
where
    <Self as Render>::ParentProtocol: LayerProtocol,
    <Self as Render>::ChildProtocol: LayerProtocol,
{
    fn composite_to(
        encoding: &mut <<Self::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        child_iterator: &mut impl ChildLayerProducingIterator<<Self::ChildProtocol as Protocol>::Canvas>,
        composition_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
    );

    fn transform_config(
        self_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
        child_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>;

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

pub trait OrphanLayerRender: LayerRender
where
    <Self as Render>::ParentProtocol: LayerProtocol,
    <Self as Render>::ChildProtocol: LayerProtocol,
{
    fn composite_orphan_to(
        encoding: &mut <<Self::ChildProtocol as Protocol>::Canvas as Canvas>::Encoding,
        child_iterator: &mut impl ChildLayerProducingIterator<<Self::ChildProtocol as Protocol>::Canvas>,
        composition_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    );

    fn transform_orphan_config(
        self_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
        child_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>;

    fn adopter_key(&self) -> Option<&Arc<dyn Key>>;
}

pub struct CachedCompositionFunctionTable<R: LayerRender>
where
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    pub composite_into_cache: fn(
        child_iterator: &mut CachingChildLayerProducingIterator<
            '_,
            <R::ChildProtocol as Protocol>::Canvas,
        >,
    ) -> R::CachedComposition,

    pub composite_from_cache_to: fn(
        encoding: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        cache: &R::CachedComposition,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
    ),
}

pub trait CachedLayer: LayerRender
where
    <Self as Render>::ParentProtocol: LayerProtocol,
    <Self as Render>::ChildProtocol: LayerProtocol,
{
    const PERFORM_CACHED_COMPOSITION: Option<CachedCompositionFunctionTable<Self>> =
        Some(CachedCompositionFunctionTable {
            composite_into_cache: |child_iterator| {
                <Self as CachedLayer>::composite_into_cache(child_iterator)
            },
            composite_from_cache_to: Self::composite_from_cache_to,
        });
    fn composite_into_cache(
        child_iterator: &mut impl ChildLayerProducingIterator<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> Self::CachedComposition;

    fn composite_from_cache_to(
        encoding: &mut <<Self::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        cache: &Self::CachedComposition,
        composition_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
    );
}

pub trait ChildLayerProducingIterator<CC: Canvas> {
    fn for_each(
        &mut self,
        composite: impl FnMut(ChildLayerOrFragmentRef<'_, CC>) -> Vec<ComposableUnadoptedLayer<CC>>,
    );
}

pub enum ChildLayerOrFragmentRef<'a, C: Canvas> {
    Fragment(&'a C::Encoding),
    StructuredChild(&'a ComposableChildLayer<C>),
    AdoptedChild(&'a ComposableAdoptedLayer<C>),
}

pub trait ChildLayerRenderObject<PC: Canvas>:
    crate::sync::composite_private::ChildLayerCompositeExt<PC> + Send + Sync
{
    fn as_arc_any_layer_render_object(self: Arc<Self>) -> ArcAnyLayerRenderObject;
}

// pub trait ParentLayerNode<CC: Canvas>: Send + Sync {}

pub trait AdoptedLayerRenderObject<PC: Canvas>: Send + Sync {
    fn composite_to(
        &self,
        encoding: &mut PC::Encoding,
        composition_config: &LayerCompositionConfig<PC>,
    ) -> Vec<ComposableUnadoptedLayer<PC>>;
}

// impl<L> AdoptedLayerNode<L::ChildCanvas> for LayerNode<L>
// where
//     L: OrphanLayer,
// {
//     fn composite_to(
//         &self,
//         encoding: &mut <L::ChildCanvas as Canvas>::Encoding,
//         composition_config: &LayerCompositionConfig<L::ChildCanvas>,
//     ) -> Vec<ComposableUnadoptedLayer<L::ChildCanvas>> {
//         // let inner = self.inner.lock();
//         // let cache = inner
//         //     .cache
//         //     .as_ref()
//         //     .expect("Layer should only be composited after they are painted");
//         // let mut iter = NonCachingOrphanChildLayerProducingIterator::<'_, L> {
//         //     painting_results: &cache.paint_results,
//         //     key: inner.layer.key().map(Arc::as_ref),
//         //     unadopted_layers: Vec::new(),
//         //     composition_config,
//         // };
//         // <L as OrphanLayer>::composite_orphan_to(encoding, &mut iter, composition_config);
//         // return iter.unadopted_layers;
//         todo!()
//     }
// }

pub trait AnyLayerRenderObject:
    AnyRenderObject
    + crate::sync::paint_private::AnyLayerPaintExt
    + crate::sync::composite_private::AnyLayerCompositeExt
    + Send
    + Sync
{
    fn mark(&self) -> &LayerMark;

    fn as_any_arc_adopted_layer(self: Arc<Self>) -> Box<dyn Any>;

    fn get_composited_cache_box(&self) -> Option<Box<dyn Any + Send + Sync>>;
}

trait ArcAnyLayerRenderObjectExt {
    fn downcast_arc_adopted_layer<C: Canvas>(self) -> Option<ArcAdoptedLayerRenderObject<C>>;
    // fn downcast_arc_parent_layer<C: Canvas>(self)
    //     -> Result<ArcParentLayerNode<C>, ArcAnyLayerNode>;
}

impl ArcAnyLayerRenderObjectExt for ArcAnyLayerRenderObject {
    fn downcast_arc_adopted_layer<C: Canvas>(self) -> Option<ArcAdoptedLayerRenderObject<C>> {
        self.as_any_arc_adopted_layer()
            .downcast::<Arc<dyn AdoptedLayerRenderObject<C>>>()
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
