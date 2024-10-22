use crate::{foundation::Canvas, tree::NoRecompositeToken};

use super::{ChildLayerOrFragment, RecordedChildLayer, RecordedOrphanLayer};

pub struct LayerCache<CC: Canvas, T> {
    pub(crate) paint_results: PaintResults<CC>,
    /// This field should always be None if the layer does not enable cached composition
    /// There is no point in storing unadopt results if the layer is going to perform tree walk anyway
    /// However, we NEED to store adopt results for hit-test
    composite_results: Option<CompositeResults<CC, T>>,
}

impl<CC, T> LayerCache<CC, T>
where
    CC: Canvas,
{
    pub(crate) fn new(
        paint_results: PaintResults<CC>,
        composite_results: Option<CompositeResults<CC, T>>,
    ) -> Self {
        Self {
            paint_results,
            composite_results,
        }
    }

    pub(crate) fn composite_results_ref(
        &self,
        _token: NoRecompositeToken,
    ) -> Option<&CompositeResults<CC, T>> {
        self.composite_results.as_ref()
    }

    pub(crate) fn insert_composite_results(
        &mut self,
        results: CompositeResults<CC, T>,
    ) -> &mut CompositeResults<CC, T> {
        self.composite_results.insert(results)
    }
}

pub struct PaintResults<C: Canvas> {
    pub children: Vec<ChildLayerOrFragment<C>>,
    pub orphan_layers: Vec<RecordedOrphanLayer<C>>,
}

pub struct CompositeResults<CC: Canvas, T> {
    pub(crate) adopted_layers: Vec<RecordedChildLayer<CC>>,
    pub(crate) cache: T,
}

#[derive(Clone)]
pub struct CompositionCache<CC: Canvas, CM> {
    // The reason we store untransformed unadopted layers (using child canvas) instead of transfomed ones (using parent canvas)
    // is that transform needs a transform_config, which is acquired when composite, and can be different with each composite attempt.
    pub(crate) orphan_layers: Vec<RecordedOrphanLayer<CC>>,
    pub(crate) memo: CM,
}
