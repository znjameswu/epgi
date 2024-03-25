use crate::{
    foundation::{ConstBool, Container, HktContainer, Protocol},
    scheduler::get_current_scheduler,
    sync::BuildScheduler,
    tree::{
        ArcChildRenderObject, DryLayout, DryLayoutFunctionTable, HasLayoutMemo, Layout,
        LayoutCache, LayoutResults, Render, RenderMark, RenderNew, RenderObject,
        RenderObjectInnerOld, RenderObjectOld, SelectCachedComposite, SelectLayerPaint, TreeNode,
    },
};

impl BuildScheduler {
    pub(crate) fn perform_layout(&mut self) {
        self.root_render_object.visit_and_layout();
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

impl<
        R,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > AnyRenderObjectLayoutExt
    for RenderObject<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: RenderNew<RenderObject = Self>
        + SelectLayerPaint<LAYER_PAINT>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: SelectLayoutImpl<DRY_LAYOUT>,
{
    fn visit_and_layout(&self) {
        let is_relayout_boundary = DRY_LAYOUT || !self.mark.parent_use_size();
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
            let inner_reborrow = &mut *inner;
            if is_relayout_boundary && needs_layout.is_ok() {
                let layout_results = inner_reborrow.cache.last_layout_results_mut().expect(
                    "Relayout should only be called on relayout boundaries \
                    that has been laid out at least once",
                );
                // We not only keeps the orignial constraints, we also keep painting offset.
                let memo = R::perform_layout_without_resize(
                    &mut inner_reborrow.render,
                    &layout_results.constraints,
                    &mut layout_results.size,
                    &inner_reborrow.children,
                );
                layout_results.memo = memo;
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

impl<R> AnyRenderObjectLayoutExt for RenderObjectOld<R>
where
    R: Render,
{
    fn visit_and_layout(&self) {
        let is_relayout_boundary = self.mark.parent_not_use_size::<R>();
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

pub trait ChildRenderObjectLayoutExt<PP: Protocol> {
    fn layout_use_size(&self, constraints: &PP::Constraints) -> PP::Size;

    fn layout(&self, constraints: &PP::Constraints);
}

impl<
        R,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ChildRenderObjectLayoutExt<R::ParentProtocol>
    for RenderObject<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: RenderNew<RenderObject = Self>
        + SelectLayerPaint<LAYER_PAINT>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: SelectLayoutImpl<DRY_LAYOUT>,
{
    fn layout_use_size(
        &self,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
    ) -> <R::ParentProtocol as Protocol>::Size {
        let needs_layout = self.mark.needs_layout();
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        if let Err(token) = needs_layout {
            if let Some(cache) = inner_reborrow.cache.layout_cache_ref(token) {
                if constraints == &cache.layout_results.constraints {
                    let size = cache.layout_results.size.clone();
                    return size;
                }
            }
        }
        let (size, memo) = inner_reborrow
            .render
            .perform_wet_layout(&constraints, &inner_reborrow.children);
        inner_reborrow.cache.insert_layout_cache(LayoutCache::new(
            LayoutResults::new(constraints.clone(), size.clone(), memo),
            None,
            None,
        ));

        self.mark.clear_self_needs_layout();
        self.mark.set_parent_use_size();
        size
    }

    fn layout(&self, constraints: &<R::ParentProtocol as Protocol>::Constraints) {
        let needs_layout = self.mark.needs_layout();
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        if let Err(token) = needs_layout {
            if let Some(cache) = inner_reborrow.cache.layout_cache_ref(token) {
                if constraints == &cache.layout_results.constraints {
                    return;
                }
            }
        }
        let (size, memo) = inner_reborrow
            .render
            .perform_wet_layout(&constraints, &inner_reborrow.children);
        inner_reborrow.cache.insert_layout_cache(LayoutCache::new(
            LayoutResults::new(constraints.clone(), size, memo),
            None,
            None,
        ));
        self.mark.clear_self_needs_layout();
        self.mark.clear_parent_use_size();
    }
}

impl<R> ChildRenderObjectLayoutExt<R::ParentProtocol> for RenderObjectOld<R>
where
    R: Render,
{
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
        self.mark.set_parent_not_use_size::<R>();
    }

    fn layout_use_size(
        &self,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
    ) -> <R::ParentProtocol as Protocol>::Size {
        self.mark.clear_parent_not_use_size::<R>();
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
}

impl<R> RenderObjectInnerOld<R>
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
            compute_layout_memo,
        }) = R::DRY_LAYOUT_FUNCTION_TABLE
        {
            let size = compute_dry_layout(&self.render, &constraints);
            let memo = compute_layout_memo(&self.render, &constraints, &size, &self.children);
            (size, memo)
        } else {
            self.render.perform_layout(&constraints, &self.children)
        };
        let cache = self.cache.insert_layout_cache(LayoutCache::new(
            LayoutResults::new(constraints, size, memo),
            None,
            None,
        ));

        mark.clear_self_needs_layout();
        return &cache.layout_results.size;
    }

    #[inline(always)]
    fn really_layout_without_resize_inner(&mut self, mark: &RenderMark) {
        let Some(DryLayoutFunctionTable {
            compute_layout_memo,
            ..
        }) = R::DRY_LAYOUT_FUNCTION_TABLE
        else {
            panic!("THis operation cannot be performed on non-relayout-boundary objects")
        };
        let layout_results = self.cache.last_layout_results_mut().expect(
            "Relayout should only be called on relayout boundaries \
            that has been laid out at least once",
        );
        let memo = compute_layout_memo(
            &self.render,
            &layout_results.constraints,
            &layout_results.size,
            &self.children,
        );
        layout_results.memo = memo;
        mark.clear_self_needs_layout();
    }
}

pub trait SelectLayoutImpl<const DRY_LAYOUT: bool>: TreeNode + HasLayoutMemo {
    fn perform_layout_without_resize(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &mut <Self::ParentProtocol as Protocol>::Size,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> Self::LayoutMemo;
    fn perform_wet_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo);
}

impl<T> SelectLayoutImpl<false> for T
where
    T: Layout,
{
    fn perform_layout_without_resize(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &mut <Self::ParentProtocol as Protocol>::Size,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> Self::LayoutMemo {
        let (new_size, memo) = self.perform_layout(constraints, children);
        *size = new_size;
        memo
    }

    fn perform_wet_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo) {
        self.perform_layout(constraints, children)
    }
}

impl<T> SelectLayoutImpl<true> for T
where
    T: DryLayout,
{
    fn perform_layout_without_resize(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &mut <Self::ParentProtocol as Protocol>::Size,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> Self::LayoutMemo {
        self.compute_layout_memo(constraints, size, children)
    }

    fn perform_wet_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo) {
        let size = self.compute_dry_layout(constraints);
        let memo = self.compute_layout_memo(constraints, &size, children);
        (size, memo)
    }
}
