use crate::{
    foundation::{Canvas, Protocol},
    tree::{ChildLayerProducingIterator, LayerCompositionConfig, Render},
};

pub trait HasCompositeImpl<R: Render, AdopterCanvas: Canvas> {
    fn composite_to(
        render: &R,
        encoding: &mut AdopterCanvas::Encoding,
        child_iterator: &mut impl ChildLayerProducingIterator<<R::ChildProtocol as Protocol>::Canvas>,
        composition_config: &LayerCompositionConfig<AdopterCanvas>,
    );

    fn transform_config(
        self_config: &LayerCompositionConfig<AdopterCanvas>,
        child_config: &LayerCompositionConfig<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<AdopterCanvas>;
}

pub trait HasCachedCompositeImpl<R: Render, AdopterCanvas: Canvas> {
    type CompositionCache: Send + Sync + Clone + 'static;

    fn composite_into_cache(
        render: &R,
        child_iterator: &mut impl ChildLayerProducingIterator<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> Self::CompositionCache;

    fn composite_from_cache_to(
        render: &R,
        encoding: &mut AdopterCanvas::Encoding,
        cache: &Self::CompositionCache,
        composition_config: &LayerCompositionConfig<AdopterCanvas>,
    );

    fn transform_config(
        self_config: &LayerCompositionConfig<AdopterCanvas>,
        child_config: &LayerCompositionConfig<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<AdopterCanvas>;
}
