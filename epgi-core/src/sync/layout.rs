use crate::{
    foundation::{Container, HktContainer, Protocol},
    scheduler::get_current_scheduler,
    sync::BuildScheduler,
    tree::{
        ArcChildRenderObject, DryLayout, HasLayoutMemo, ImplRenderBySuper, Layout, LayoutCache,
        LayoutResults, Render, RenderImpl, RenderObject, SelectCachedComposite, SelectLayerPaint,
        TreeNode,
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
    R: Render<RenderObject = Self>
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
    R: Render<RenderObject = Self>
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

pub trait ImplLayout<R: Render> {
    fn perform_layout_without_resize(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        size: &mut <R::ParentProtocol as Protocol>::Size,
        children: &<R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
    ) -> R::LayoutMemo;
    fn perform_wet_layout(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        children: &<R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
    ) -> (<R::ParentProtocol as Protocol>::Size, R::LayoutMemo);
}

impl<
        R: Render,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ImplLayout<R> for RenderImpl<R, false, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Layout,
{
    fn perform_layout_without_resize(
        render: &mut R,
        constraints: &<<R>::ParentProtocol as Protocol>::Constraints,
        size: &mut <<R>::ParentProtocol as Protocol>::Size,
        children: &<<R>::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<<R>::ChildProtocol>,
        >,
    ) -> R::LayoutMemo {
        let (new_size, memo) = render.perform_layout(constraints, children);
        *size = new_size;
        memo
    }

    fn perform_wet_layout(
        render: &mut R,
        constraints: &<<R>::ParentProtocol as Protocol>::Constraints,
        children: &<<R>::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<<R>::ChildProtocol>,
        >,
    ) -> (<<R>::ParentProtocol as Protocol>::Size, <R>::LayoutMemo) {
        render.perform_layout(constraints, children)
    }
}

impl<
        R: Render,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ImplLayout<R> for RenderImpl<R, true, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: DryLayout,
{
    fn perform_layout_without_resize(
        render: &mut R,
        constraints: &<<R>::ParentProtocol as Protocol>::Constraints,
        size: &mut <<R>::ParentProtocol as Protocol>::Size,
        children: &<<R>::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<<R>::ChildProtocol>,
        >,
    ) -> R::LayoutMemo {
        render.compute_layout_memo(constraints, size, children)
    }

    fn perform_wet_layout(
        render: &mut R,
        constraints: &<<R>::ParentProtocol as Protocol>::Constraints,
        children: &<<R>::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<<R>::ChildProtocol>,
        >,
    ) -> (<<R>::ParentProtocol as Protocol>::Size, <R>::LayoutMemo) {
        let size = render.compute_dry_layout(constraints);
        let memo = render.compute_layout_memo(constraints, &size, children);
        (size, memo)
    }
}

impl<T> ImplLayout<T::Render> for T
where
    T: ImplRenderBySuper,
{
    fn perform_layout_without_resize(
        render: &mut T::Render,
        constraints: &<<T::Render as TreeNode>::ParentProtocol as Protocol>::Constraints,
        size: &mut <<T::Render as TreeNode>::ParentProtocol as Protocol>::Size,
        children: &<<T::Render as TreeNode>::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<<T::Render as TreeNode>::ChildProtocol>,
        >,
    ) -> <T::Render as HasLayoutMemo>::LayoutMemo {
        T::Super::perform_layout_without_resize(render, constraints, size, children)
    }

    fn perform_wet_layout(
        render: &mut T::Render,
        constraints: &<<T::Render as TreeNode>::ParentProtocol as Protocol>::Constraints,
        children: &<<T::Render as TreeNode>::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<<T::Render as TreeNode>::ChildProtocol>,
        >,
    ) -> (
        <<T::Render as TreeNode>::ParentProtocol as Protocol>::Size,
        <T::Render as HasLayoutMemo>::LayoutMemo,
    ) {
        T::Super::perform_wet_layout(render, constraints, children)
    }
}
