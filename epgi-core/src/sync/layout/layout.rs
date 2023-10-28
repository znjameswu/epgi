use hashbrown::HashSet;

use crate::{
    foundation::{Parallel, Protocol, PtrEq},
    scheduler::get_current_scheduler,
    sync::TreeScheduler,
    tree::{
        AweakAnyRenderObject, DryLayoutFunctionTable, Render, RenderCache, RenderContextNode,
        RenderObject, RenderObjectInner,
    },
};

impl TreeScheduler {
    pub(crate) fn perform_layout(
        &mut self,
        boundaries_needing_relayout: HashSet<PtrEq<AweakAnyRenderObject>>,
    ) {
        // let mut boundaries_needing_relayout = boundaries_needing_relayout
        //     .into_iter()
        //     .filter_map(|x| {
        //         x.0.upgrade()
        //             .filter(|x| !x.element_context().is_unmounted())
        //     })
        //     .collect::<Vec<_>>();

        // boundaries_needing_relayout.sort_unstable_by_key(|object| object.element_context().depth);

        // for boundary in boundaries_needing_relayout {
        //     boundary.layout_without_resize()
        // }
    }
}

impl<R> RenderObjectInner<R>
where
    R: Render,
{
    fn is_relayout_boundary(&self) -> bool {
        R::DRY_LAYOUT_FUNCTION_TABLE.is_some()
            || self.cache.as_ref().is_some_and(|x| x.parent_use_size)
    }
}

impl<R> RenderObject<R>
where
    R: Render,
{
    fn is_relayout_boundary(&self) -> bool {
        R::DRY_LAYOUT_FUNCTION_TABLE.is_some()
            || self
                .inner
                .lock()
                .cache
                .as_ref()
                .is_some_and(|x| x.parent_use_size)
    }
    // fn layout_without_resize(&self) {
    //     let mut inner = self.inner.lock();
    //     inner.perform_layout_without_resize(&self.context)
    // }

    fn layout(&self, constraints: &<R::ParentProtocol as Protocol>::Constraints) {
        let mut inner = self.inner.lock();
        if let Some(cache) = &mut inner.cache {
            if cache.get_layout_for(&constraints).is_some() {
                cache.parent_use_size = false;
                return;
            }
        }
        inner.perform_wet_layout(constraints.clone(), false);
    }

    fn layout_use_size(
        &self,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
    ) -> <R::ParentProtocol as Protocol>::Size {
        let mut inner = self.inner.lock();

        if let Some(cache) = &mut inner.cache {
            if let Some(size) = cache.get_layout_for(&constraints) {
                let size = size.clone();
                cache.parent_use_size = false;
                return size;
            }
        }
        inner.perform_wet_layout(constraints.clone(), true).clone()
    }

    fn visit_and_layout(&self) {
        let is_relayout_boundary = self.context.is_relayout_boundary();
        let needs_layout = self.context.needs_layout();
        let subtree_has_layout = self.context.subtree_has_layout();
        debug_assert!(
            is_relayout_boundary || !needs_layout,
            "A layout walk should not encounter a dirty non-boundary node.
            Such node should be already laied-out by an ancester layout sometime earlier in this walk."
        );
        debug_assert!(
            subtree_has_layout || !needs_layout,
            "A dirty node should always mark its subtree as dirty"
        );
        if self.context.subtree_has_layout() {
            let children = {
                let mut inner = self.inner.lock();
                if is_relayout_boundary && needs_layout {
                    inner.layout_without_resize_inner(&self.context);
                    self.context.clear_self_needs_layout();
                }
                inner.render.children()
            };
            children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                child.visit_and_layout()
            });
            self.context.clear_subtree_has_layout();
        }
    }
}
impl<R> RenderObjectInner<R>
where
    R: Render,
{
    #[inline(always)]
    fn perform_wet_layout(
        &mut self,
        constraints: <R::ParentProtocol as Protocol>::Constraints,
        parent_use_size: bool,
    ) -> &<R::ParentProtocol as Protocol>::Size {
        let (size, memo) = if let Some(DryLayoutFunctionTable {
            compute_dry_layout,
            compute_layout_memo: perform_layout,
        }) = R::DRY_LAYOUT_FUNCTION_TABLE
        {
            let size = compute_dry_layout(&self.render, &constraints);
            let memo = perform_layout(&self.render, &constraints, &size);
            (size, memo)
        } else {
            self.render.perform_layout(&constraints)
        };
        return RenderCache::insert_into(&mut self.cache, constraints, parent_use_size, size, memo);
    }

    #[inline(always)]
    fn layout_without_resize_inner(&mut self, context: &RenderContextNode) {
        debug_assert!(self.is_relayout_boundary());
        let Some(cache) = self.cache.as_mut() else {
            panic!("Relayout should only be called on relayout boundaries which must retain their layout caches")
        };
        if cache.layout_results(context).is_some() {
            return;
        }
        let constraints = cache.constraints.clone();
        let parent_use_size = cache.parent_use_size;
        self.perform_wet_layout(constraints, parent_use_size);
        context.clear_self_needs_layout();
    }
}

pub(crate) mod layout_private {
    use crate::tree::RenderObject;

    use super::*;

    pub trait ChildRenderObjectLayoutExt<PP: Protocol> {
        fn layout_use_size(&self, constraints: &PP::Constraints) -> PP::Size;

        fn layout(&self, constraints: &PP::Constraints);

        /// Walk the tree and initiate layout on any dirty relayout boundaries.
        ///
        /// This method initiate two tree walks after encountering a dirty relayout boundary: first a layout phase, then a recursive visit phase.
        ///
        /// Layout tree walk will try to bypass as many subtrees as possible and cover the minimum tree regions as required by user-specified layout logic.
        ///
        /// Visit tree walk will walk into all dirty nodes inside the subtree.
        /// The second tree walk will very likely overlap with the first tree walk, which is an inherent inefficiency in this algorithm.
        fn visit_and_layout(&self);
    }

    impl<R> ChildRenderObjectLayoutExt<R::ParentProtocol> for RenderObject<R>
    where
        R: Render,
    {
        fn layout_use_size(
            &self,
            constraints: &<R::ParentProtocol as Protocol>::Constraints,
        ) -> <R::ParentProtocol as Protocol>::Size {
            self.layout_use_size(constraints)
        }

        fn layout(&self, constraints: &<R::ParentProtocol as Protocol>::Constraints) {
            self.layout(constraints)
        }

        fn visit_and_layout(&self) {
            self.visit_and_layout()
        }
    }

    pub trait AnyRenderObjectRelayoutExt {
        // fn layout_without_resize(&self);
    }

    impl<R> AnyRenderObjectRelayoutExt for RenderObject<R>
    where
        R: Render,
    {
        // fn layout_without_resize(&self) {
        //     self.layout_without_resize()
        // }
    }
}
