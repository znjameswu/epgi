use crate::foundation::{Canvas, Key};

use super::{
    ArcAnyLayerRenderObjectExt, ComposableChildLayer, ComposableUnadoptedLayer, PaintResults,
};

pub enum ChildLayerOrFragmentRef<'a, C: Canvas> {
    Fragment(&'a C::Encoding),
    StructuredChild(&'a ComposableChildLayer<C>),
    AdoptedChild(&'a ComposableChildLayer<C>),
}

pub struct ChildLayerProducingIterator<'a, CC: Canvas> {
    pub(crate) paint_results: &'a PaintResults<CC>,
    pub(crate) key: Option<&'a dyn Key>,
    pub(crate) unadopted_layers: Vec<ComposableUnadoptedLayer<CC>>,
    pub(crate) adopted_layers: Vec<ComposableChildLayer<CC>>,
}

impl<'a, CC: Canvas> ChildLayerProducingIterator<'a, CC> {
    pub fn new(paint_results: &'a PaintResults<CC>, key: Option<&'a dyn Key>) -> Self {
        Self {
            paint_results,
            key,
            unadopted_layers: Default::default(),
            adopted_layers: Default::default(),
        }
    }
}

impl<'a, CC: Canvas> ChildLayerProducingIterator<'a, CC> {
    pub fn for_each(
        &mut self,
        mut composite: impl FnMut(ChildLayerOrFragmentRef<'_, CC>) -> Vec<ComposableUnadoptedLayer<CC>>,
    ) {
        let mut subtree_unadopted_layers = Vec::new();
        for child in &self.paint_results.children {
            let child_unadopted_layers = composite(child.into());
            subtree_unadopted_layers.extend(child_unadopted_layers);
        }
        subtree_unadopted_layers.extend(self.paint_results.orphan_layers.iter().cloned());
        // DFS traversal, working from end to front
        subtree_unadopted_layers.reverse();
        while let Some(child) = subtree_unadopted_layers.pop() {
            let adopter_key = &child.adopter_key;
            if self
                .key
                .is_some_and(|key| <dyn Key>::eq(adopter_key.as_ref(), key))
            {
                if let Some(layer) = child.layer.clone().downcast_arc_adopted_layer::<CC>() {
                    let adopted_child_layer = ComposableChildLayer {
                        config: child.config,
                        layer,
                    };
                    let child_unadopted_layers =
                        composite(ChildLayerOrFragmentRef::AdoptedChild(&adopted_child_layer));
                    subtree_unadopted_layers.extend(child_unadopted_layers.into_iter().rev());
                    continue;
                }
            }
            self.unadopted_layers.push(child)
        }
    }
}
