use crate::foundation::{Canvas, Key};

use super::{
    ArcAnyLayerRenderObjectExt, ChildLayerOrFragmentRef, ChildLayerProducingIterator,
    ComposableChildLayer, ComposableUnadoptedLayer, LayerCompositionConfig, PaintResults,
};

pub struct NonCachingChildLayerProducingIterator<'a, PC, CC, F>
where
    PC: Canvas,
    CC: Canvas,
    F: Fn(&LayerCompositionConfig<PC>, &LayerCompositionConfig<CC>) -> LayerCompositionConfig<PC>,
{
    pub(crate) paint_results: &'a PaintResults<CC>,
    pub(crate) key: Option<&'a dyn Key>,
    pub(crate) unadopted_layers: Vec<ComposableUnadoptedLayer<PC>>,
    pub(crate) composition_config: &'a LayerCompositionConfig<PC>,
    pub(crate) transform_config: F,
}

impl<'a, PC, CC, F> ChildLayerProducingIterator<CC>
    for NonCachingChildLayerProducingIterator<'a, PC, CC, F>
where
    PC: Canvas,
    CC: Canvas,
    F: Fn(&LayerCompositionConfig<PC>, &LayerCompositionConfig<CC>) -> LayerCompositionConfig<PC>,
{
    fn for_each(
        &mut self,
        mut composite: impl FnMut(ChildLayerOrFragmentRef<'_, CC>) -> Vec<ComposableUnadoptedLayer<CC>>,
    ) {
        let mut subtree_unadopted_layers = Vec::new();
        for child in &self.paint_results.structured_children {
            let child_unadopted_layers = composite(child.into());
            subtree_unadopted_layers.extend(child_unadopted_layers);
        }
        subtree_unadopted_layers.extend(self.paint_results.detached_children.iter().cloned());
        // DFS traversal, working from end to front
        subtree_unadopted_layers.reverse();
        while let Some(child) = subtree_unadopted_layers.pop() {
            let adopter_key = &child.adopter_key;
            if adopter_key.is_none()
                || self.key.is_some_and(|key| {
                    adopter_key
                        .as_ref()
                        .is_some_and(|parent_key| <dyn Key>::eq(parent_key.as_ref(), key))
                })
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
            self.unadopted_layers.push(ComposableUnadoptedLayer {
                config: (self.transform_config)(&self.composition_config, &child.config),
                adopter_key: child.adopter_key,
                layer: child.layer,
            })
        }
    }
}

// /// Helper struct since the transform function is an anonymous type and thus
// /// cannot be named as monomorphized function pointer in associated function tables.
// /// This helper struct names the transform function by [OrphanLayer] type and
// /// keeps the anonymous type as a local variable in the [ChildLayerProducingIterator::for_each] method
// pub struct NonCachingOrphanChildLayerProducingIterator<'a, R>
// where
//     R: LayerRender,
//     R::ChildProtocol: LayerProtocol,
//     R::ParentProtocol: LayerProtocol,
// {
//     pub(crate) paint_results: &'a PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
//     pub(crate) key: Option<&'a dyn Key>,
//     pub(crate) unadopted_layers:
//         Vec<ComposableUnadoptedLayer<<R::ChildProtocol as Protocol>::Canvas>>,
//     pub(crate) composition_config:
//         &'a LayerCompositionConfig<<R::ChildProtocol as Protocol>::Canvas>,
// }

// impl<'a, R> ChildLayerProducingIterator<<R::ChildProtocol as Protocol>::Canvas>
//     for NonCachingOrphanChildLayerProducingIterator<'a, R>
// where
//     R: OrphanLayerRender,
//     R::ChildProtocol: LayerProtocol,
//     R::ParentProtocol: LayerProtocol,
// {
//     fn for_each(
//         &mut self,
//         composite: impl FnMut(
//             ChildLayerOrFragmentRef<'_, <R::ChildProtocol as Protocol>::Canvas>,
//         )
//             -> Vec<ComposableUnadoptedLayer<<R::ChildProtocol as Protocol>::Canvas>>,
//     ) {
//         let mut iter = NonCachingChildLayerProducingIterator {
//             paint_results: self.paint_results,
//             key: self.key,
//             unadopted_layers: Vec::new(),
//             composition_config: self.composition_config,
//             transform_config: R::transform_orphan_config,
//         };
//         iter.for_each(composite);
//         self.unadopted_layers = iter.unadopted_layers;
//     }
// }

pub struct CachingChildLayerProducingIterator<'a, CC: Canvas> {
    pub(crate) paint_results: &'a PaintResults<CC>,
    pub(crate) key: Option<&'a dyn Key>,
    pub(crate) unadopted_layers: Vec<ComposableUnadoptedLayer<CC>>,
}

impl<'a, CC> ChildLayerProducingIterator<CC> for CachingChildLayerProducingIterator<'a, CC>
where
    CC: Canvas,
{
    fn for_each(
        &mut self,
        mut composite: impl FnMut(ChildLayerOrFragmentRef<'_, CC>) -> Vec<ComposableUnadoptedLayer<CC>>,
    ) {
        let mut subtree_unadopted_layers = Vec::new();
        for child in &self.paint_results.structured_children {
            let child_unadopted_layers = composite(child.into());
            subtree_unadopted_layers.extend(child_unadopted_layers);
        }
        subtree_unadopted_layers.extend(self.paint_results.detached_children.iter().cloned());
        // DFS traversal, working from end to front
        subtree_unadopted_layers.reverse();
        while let Some(child) = subtree_unadopted_layers.pop() {
            let adopter_key = &child.adopter_key;
            if adopter_key.is_none()
                || self.key.is_some_and(|key| {
                    adopter_key
                        .as_ref()
                        .is_some_and(|parent_key| <dyn Key>::eq(parent_key.as_ref(), key))
                })
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
            self.unadopted_layers.push(ComposableUnadoptedLayer {
                config: child.config,
                adopter_key: child.adopter_key,
                layer: child.layer,
            })
        }
    }
}
