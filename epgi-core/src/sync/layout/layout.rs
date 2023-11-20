use crate::{
    foundation::{Container, Protocol},
    scheduler::get_current_scheduler,
    sync::TreeScheduler,
    tree::{
        DryLayoutFunctionTable, LayoutCache, LayoutResults, Render, RenderMark, RenderObject,
        RenderObjectInner,
    },
};

impl TreeScheduler {
    pub(crate) fn perform_layout(&mut self) {
        self.root_render_object.visit_and_layout();
    }
}

impl<R> RenderObjectInner<R>
where
    R: Render,
{
    // fn is_relayout_boundary(&self) -> bool {
    //     R::DRY_LAYOUT_FUNCTION_TABLE.is_some()
    //         || self
    //             .cache
    //             .last_layout_config_ref()
    //             .is_some_and(|(_, parent_use_size)| !*parent_use_size)
    // }
}

impl<R> RenderObject<R>
where
    R: Render,
{
    // fn is_relayout_boundary(&self) -> bool {
    //     R::DRY_LAYOUT_FUNCTION_TABLE.is_some()
    //         || self
    //             .inner
    //             .lock()
    //             .cache
    //             .last_layout_config_mut()
    //             .is_some_and(|(_, parent_use_size)| !*parent_use_size)
    // }
    // fn layout_without_resize(&self) {
    //     let mut inner = self.inner.lock();
    //     inner.perform_layout_without_resize(&self.context)
    // }

    fn layout(&self, constraints: &<R::ParentProtocol as Protocol>::Constraints) {
        let needs_layout = self.mark.needs_layout();
        let mut inner = self.inner.lock();
        if let Err(token) = needs_layout {
            if let Some(cache) = inner.cache.layout_cache_ref(token) {
                if constraints == &cache.layout_results.constraints {
                    return;
                }
            }
        }
        inner.perform_wet_layout(constraints.clone(), &self.mark);
        self.mark.set_is_relayout_boundary::<R>();
    }

    fn layout_use_size(
        &self,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
    ) -> <R::ParentProtocol as Protocol>::Size {
        self.mark.clear_is_relayout_boundary::<R>();
        let needs_layout = self.mark.needs_layout();
        let mut inner = self.inner.lock();
        if let Err(token) = needs_layout {
            if let Some(cache) = inner.cache.layout_cache_ref(token) {
                if constraints == &cache.layout_results.constraints {
                    let size = cache.layout_results.size.clone();
                    return size;
                }
            }
        }
        inner
            .perform_wet_layout(constraints.clone(), &self.mark)
            .clone()
    }

    fn visit_and_layout(&self) {
        let is_relayout_boundary = self.mark.is_relayout_boundary::<R>();
        let needs_layout = self.mark.needs_layout();
        let subtree_has_layout = self.mark.subtree_has_layout();
        debug_assert!(
            is_relayout_boundary || needs_layout.is_err(),
            "A layout walk should not encounter a dirty non-boundary node. \
            Such node should be already laied-out by an ancester layout sometime earlier in this walk."
        );
        debug_assert!(
            subtree_has_layout || needs_layout.is_err(),
            "A dirty node should always mark its subtree as dirty"
        );
        if !subtree_has_layout {
            return;
        }
        let children = {
            let mut inner = self.inner.lock();
            if is_relayout_boundary && needs_layout.is_ok() {
                inner.really_layout_without_resize_inner(&self.mark);
                self.mark.clear_self_needs_layout();
            }
            inner.children.map_ref_collect(Clone::clone)
        };
        children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
            child.visit_and_layout()
        });
        self.mark.clear_subtree_has_layout();
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
        mark: &RenderMark,
    ) -> &<R::ParentProtocol as Protocol>::Size {
        let (size, memo) = if let Some(DryLayoutFunctionTable {
            compute_dry_layout,
            compute_layout_memo: perform_layout,
        }) = R::DRY_LAYOUT_FUNCTION_TABLE
        {
            let size = compute_dry_layout(&self.render, &constraints);
            let memo = perform_layout(&self.render, &constraints, &size, &self.children);
            (size, memo)
        } else {
            self.render.perform_layout(&constraints, &self.children)
        };
        let cache = self.cache.insert_layout_cache(LayoutCache::new(
            LayoutResults::new(constraints, size, memo),
            None,
        ));

        mark.clear_self_needs_layout();
        return &cache.layout_results.size;
    }

    #[inline(always)]
    fn really_layout_without_resize_inner(&mut self, mark: &RenderMark) {
        let constraints = self.cache.last_layout_config_mut().expect(
            "Relayout should only be called on relayout boundaries \
            that has been laid out at least once",
        );
        let constraints = constraints.clone();
        self.perform_wet_layout(constraints, mark);
    }
}

pub(crate) mod layout_private {
    use crate::tree::RenderObject;

    use super::*;

    pub trait ChildRenderObjectLayoutExt<PP: Protocol> {
        fn layout_use_size(&self, constraints: &PP::Constraints) -> PP::Size;

        fn layout(&self, constraints: &PP::Constraints);
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
    }

    pub trait AnyRenderObjectLayoutExt {
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

    impl<R> AnyRenderObjectLayoutExt for RenderObject<R>
    where
        R: Render,
    {
        fn visit_and_layout(&self) {
            self.visit_and_layout()
        }
    }
}
