use crate::foundation::{Canvas, Key};

use super::{ArcAnyLayerRenderObjectExt, PaintResults, RecordedChildLayer, RecordedOrphanLayer};

pub enum ChildLayerOrFragmentRef<'a, C: Canvas> {
    Fragment(&'a C::Encoding),
    Child(&'a RecordedChildLayer<C>),
    AdoptedChild(&'a RecordedChildLayer<C>),
}

pub struct ChildLayerProducingIterator<'a, CC: Canvas> {
    pub(crate) paint_results: &'a PaintResults<CC>,
    pub(crate) key: Option<&'a dyn Key>,
    pub(crate) orphan_layers: Vec<RecordedOrphanLayer<CC>>,
    pub(crate) adopted_layers: Vec<RecordedChildLayer<CC>>,
}

impl<'a, CC: Canvas> ChildLayerProducingIterator<'a, CC> {
    pub fn new(paint_results: &'a PaintResults<CC>, key: Option<&'a dyn Key>) -> Self {
        Self {
            paint_results,
            key,
            orphan_layers: Default::default(),
            adopted_layers: Default::default(),
        }
    }
}

impl<'a, CC: Canvas> ChildLayerProducingIterator<'a, CC> {
    pub fn for_each(
        &mut self,
        mut composite: impl FnMut(ChildLayerOrFragmentRef<'_, CC>) -> Vec<RecordedOrphanLayer<CC>>,
    ) {
        let mut collected_orphan_layers = Vec::new();
        for child in &self.paint_results.children {
            let child_orphan_layers = composite(child.into());
            collected_orphan_layers.extend(child_orphan_layers);
        }
        collected_orphan_layers.extend(self.paint_results.orphan_layers.iter().cloned());
        // DFS traversal, working from end to front
        collected_orphan_layers.reverse();
        // Also we pop from end to front
        while let Some(child) = collected_orphan_layers.pop() {
            let adopter_key = &child.adopter_key;
            if self
                .key
                .is_some_and(|key| <dyn Key>::eq_key(adopter_key.as_ref(), key))
            {
                if let Some(layer) = child.layer.clone().downcast_arc_child_layer::<CC>() {
                    let adopted_layer = RecordedChildLayer {
                        config: child.config,
                        layer,
                    };
                    let child_orphan_layers =
                        composite(ChildLayerOrFragmentRef::AdoptedChild(&adopted_layer));
                    collected_orphan_layers.extend(child_orphan_layers.into_iter().rev());
                    self.adopted_layers.push(adopted_layer);
                    continue;
                }
            }
            self.orphan_layers.push(child)
        }
    }
}
