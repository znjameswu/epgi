use crate::{foundation::Canvas, tree::NoRecompositeToken};

use super::{ComposableUnadoptedLayer, StructuredChildLayerOrFragment};

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
