use crate::foundation::{HktContainer, Protocol, SyncMutex};

use super::{
    layer_render_function_table_of, ArcChildRenderObject, ArcElementContextNode, ArcLayerNodeOf,
    LayerOrUnit, LayerRenderFunctionTable, NoRelayoutToken, Render, RenderMark,
};

pub struct RenderObject<R: Render> {
    pub(crate) element_context: ArcElementContextNode,
    pub(crate) mark: RenderMark,
    pub(crate) layer_mark: <R::LayerOrUnit as LayerOrUnit<R>>::LayerMark,
    pub(crate) layer_node: ArcLayerNodeOf<R>,
    pub(crate) inner: SyncMutex<RenderObjectInner<R>>,
}

impl<R> RenderObject<R>
where
    R: Render,
{
    pub fn new(
        render: R,
        children: <R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
        element_context: ArcElementContextNode,
    ) -> Self {
        // debug_assert!(
        //     element_context.has_render,
        //     "A render object node must have a render context node in its element context node"
        // );
        let layer = match layer_render_function_table_of::<R>() {
            LayerRenderFunctionTable::LayerNode {
                create_arc_layer_node,
                ..
            } => create_arc_layer_node(&render),
            LayerRenderFunctionTable::None { create } => create(),
        };
        Self {
            element_context,
            mark: RenderMark::new(),
            layer_mark: <R::LayerOrUnit as LayerOrUnit<R>>::create_layer_mark(),
            layer_node: layer,
            inner: SyncMutex::new(RenderObjectInner {
                layout_results: None,
                render,
                children,
            }),
        }
    }
}

pub(crate) struct RenderObjectInner<R: Render> {
    // parent: Option<AweakParentRenderObject<R::SelfProtocol>>,
    // boundaries: Option<RenderObjectBoundaries>,
    layout_results: Option<
        LayoutResults<
            R::ParentProtocol,
            R::LayoutMemo,
            <R::LayerOrUnit as LayerOrUnit<R>>::LayerCache,
        >,
    >,
    pub(crate) render: R,
    pub(crate) children:
        <R::ChildContainer as HktContainer>::Container<ArcChildRenderObject<R::ChildProtocol>>,
}

pub(crate) struct LayoutResults<P: Protocol, M, LC> {
    pub(crate) constraints: P::Constraints,
    pub(crate) parent_use_size: bool,
    pub(crate) size: P::Size,
    pub(crate) memo: M,
    pub(crate) paint_results: Option<LC>,
}

impl<R> RenderObjectInner<R>
where
    R: Render,
{
    // The ZST token guards against accidentally accessing staled layout results
    #[inline(always)]
    pub(crate) fn layout_results_ref(
        &self,
        _token: &NoRelayoutToken,
    ) -> Option<
        &LayoutResults<
            R::ParentProtocol,
            R::LayoutMemo,
            <R::LayerOrUnit as LayerOrUnit<R>>::LayerCache,
        >,
    > {
        self.layout_results.as_ref()
    }

    // The ZST token guards against accidentally accessing staled layout results
    #[inline(always)]
    pub(crate) fn layout_results_mut(
        &mut self,
        _token: &NoRelayoutToken,
    ) -> Option<
        &mut LayoutResults<
            R::ParentProtocol,
            R::LayoutMemo,
            <R::LayerOrUnit as LayerOrUnit<R>>::LayerCache,
        >,
    > {
        self.layout_results.as_mut()
    }

    pub(crate) fn insert_layout_results(
        &mut self,
        layout_results: LayoutResults<
            R::ParentProtocol,
            R::LayoutMemo,
            <R::LayerOrUnit as LayerOrUnit<R>>::LayerCache,
        >,
    ) -> &mut LayoutResults<
        R::ParentProtocol,
        R::LayoutMemo,
        <R::LayerOrUnit as LayerOrUnit<R>>::LayerCache,
    > {
        self.layout_results.insert(layout_results)
    }

    #[inline(always)]
    pub(crate) fn last_layout_config_ref(
        &self,
    ) -> Option<(&<R::ParentProtocol as Protocol>::Constraints, &bool)> {
        self.layout_results
            .as_ref()
            .map(|layout_results| (&layout_results.constraints, &layout_results.parent_use_size))
    }

    #[inline(always)]
    pub(crate) fn last_layout_config_mut(
        &mut self,
    ) -> Option<(&mut <R::ParentProtocol as Protocol>::Constraints, &mut bool)> {
        self.layout_results.as_mut().map(|layout_results| {
            (
                &mut layout_results.constraints,
                &mut layout_results.parent_use_size,
            )
        })
    }
}

impl<P, M, LC> LayoutResults<P, M, LC>
where
    P: Protocol,
{
    pub(crate) fn new(
        constraints: P::Constraints,
        parent_use_size: bool,
        size: P::Size,
        memo: M,
        paint_results: Option<LC>,
    ) -> Self {
        Self {
            constraints,
            parent_use_size,
            size,
            memo,
            paint_results,
        }
    }
}

// pub(crate) struct RenderCache<P: Protocol, M, LC = ()> {
//     pub(crate) constraints: P::Constraints,
//     pub(crate) parent_use_size: bool,
//     layout_results: Option<LayoutResults<P, M, LC>>,
// }

// impl<P, M, LC> RenderCache<P, M, LC>
// where
//     P: Protocol,
// {
//     pub(crate) fn new(
//         constraints: P::Constraints,
//         parent_use_size: bool,
//         layout_results: Option<LayoutResults<P, M, LC>>,
//     ) -> Self {
//         Self {
//             constraints,
//             parent_use_size,
//             layout_results,
//         }
//     }
//     pub(crate) fn layout_results(&self, mark: &RenderMark) -> Option<&LayoutResults<P, M, LC>> {
//         if mark.needs_layout() {
//             return None;
//         }
//         self.layout_results.as_ref()
//     }
// }

// impl<P, M> RenderCache<P, M>
// where
//     P: Protocol,
// {
//     #[inline]
//     pub fn get_layout_for(&mut self, constraints: &P::Constraints) -> Option<&P::Size> {
//         let Some(layout_results) = &mut self.layout_results else {
//             return None;
//         };
//         if &self.constraints == constraints {
//             return Some(&layout_results.size);
//         }
//         return None;
//     }

//     /// An almost-zero-overhead way to write into cache while holding reference to [Size]
//     pub fn insert_into(
//         dst: &mut Option<Self>,
//         constraints: P::Constraints,
//         parent_use_size: bool,
//         size: P::Size,
//         memo: M,
//     ) -> &P::Size {
//         &dst.insert(RenderCache {
//             constraints,
//             parent_use_size,
//             layout_results: None,
//         })
//         .layout_results
//         .insert(LayoutResults {
//             size,
//             memo,
//             paint_results: None,
//         })
//         .size
//     }

//     /// Return: whether a layout is needed.
//     pub(crate) fn set_root_constraints(
//         dst: &mut Option<Self>,
//         constraints: P::Constraints,
//     ) -> bool {
//         match dst {
//             Some(inner) => {
//                 debug_assert!(
//                     inner.parent_use_size == false,
//                     "Root render object should not have parent_use_size"
//                 );
//                 if inner.constraints.eq(&constraints) {
//                     return false;
//                 }
//                 inner.constraints = constraints;
//                 inner.layout_results = None;
//                 return true;
//             }
//             None => {
//                 *dst = Some(RenderCache {
//                     constraints: constraints,
//                     parent_use_size: false,
//                     layout_results: None,
//                 });
//                 return true;
//             }
//         }
//     }
// }
