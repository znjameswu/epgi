use std::borrow::{Borrow, BorrowMut};

use crate::{
    foundation::{Container, HktContainer, Protocol, SurrogateProtocol},
    scheduler::get_current_scheduler,
    tree::{
        ArcChildRenderObject, Layout, LayoutByParent, LayoutResults, Render, RenderBase,
        RenderImpl, RenderObject,
    },
};

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
    R::Impl: ImplLayout<R>,
{
    fn visit_and_layout(&self) {
        let is_relayout_boundary = R::Impl::SIZED_BY_PARENT || !self.mark.parent_use_size();
        let needs_layout = self.mark.needs_layout();
        let descendant_has_layout = self.mark.descendant_has_layout();
        debug_assert!(
            is_relayout_boundary || needs_layout.is_err(),
            "A layout walk should not encounter a dirty non-boundary node. \
            Such node should be already laied-out by an ancester layout sometime earlier in this walk."
        );
        if !descendant_has_layout && needs_layout.is_err() {
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
                let memo = R::Impl::perform_layout_without_resize(
                    &mut inner_reborrow.render,
                    &layout_results.constraints,
                    &mut layout_results.size,
                    &inner_reborrow.children,
                );
                layout_results.memo = memo;
                self.mark.clear_self_needs_layout();
            }
            R::ChildContainer::clone_container(&inner.children)
        };
        if descendant_has_layout {
            children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                child.visit_and_layout()
            });
            self.mark.clear_descendant_has_layout();
        }
    }
}

pub trait ChildRenderObjectLayoutExt<PP: Protocol> {
    fn layout_use_size(&self, constraints: &PP::Constraints) -> PP::Size;

    fn layout(&self, constraints: &PP::Constraints);

    fn get_intrinsics(&self, intrinsics: &mut PP::Intrinsics);
}

impl<R, P> ChildRenderObjectLayoutExt<P> for RenderObject<R>
where
    R: Render,
    R::Impl: ImplLayout<R>,
    P: SurrogateProtocol<R::ParentProtocol>,
{
    fn layout_use_size(&self, constraints: &P::Constraints) -> P::Size {
        let converted_constraints = P::convert_constraints(constraints);
        let constraints: &<R::ParentProtocol as Protocol>::Constraints =
            converted_constraints.borrow();

        let needs_layout = self.mark.needs_layout();
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        let cache_fresh = match needs_layout {
            Ok(()) => inner_reborrow.cache.clear(),
            Err(no_relayout) => no_relayout.into(),
        };
        let cache = inner_reborrow.cache.layout_cache_ref(cache_fresh);
        if let Some(cache) = cache {
            if constraints == &cache.layout_results.constraints {
                let size = cache.layout_results.size.clone();
                // This return path does not need to clear needs_layout flag
                return P::recover_size(size);
            }
        }

        let (size, memo) = R::Impl::perform_full_layout(
            &mut inner_reborrow.render,
            &constraints,
            &inner_reborrow.children,
        );
        inner_reborrow.cache.insert_layout_results(
            LayoutResults::new(constraints.clone(), size.clone(), memo),
            cache_fresh,
        );

        self.mark.clear_self_needs_layout();
        self.mark.set_parent_use_size();
        P::recover_size(size)
    }

    fn layout(&self, constraints: &P::Constraints) {
        let converted_constraints = P::convert_constraints(constraints);
        let constraints: &<R::ParentProtocol as Protocol>::Constraints =
            converted_constraints.borrow();

        let needs_layout = self.mark.needs_layout();
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        let cache_fresh = match needs_layout {
            Ok(()) => inner_reborrow.cache.clear(),
            Err(no_relayout) => no_relayout.into(),
        };
        let cache = inner_reborrow.cache.layout_cache_ref(cache_fresh);
        if let Some(cache) = cache {
            if constraints == &cache.layout_results.constraints {
                // This return path does not need to clear needs_layout flag
                return;
            }
        }

        let (size, memo) = R::Impl::perform_full_layout(
            &mut inner_reborrow.render,
            &constraints,
            &inner_reborrow.children,
        );
        inner_reborrow.cache.insert_layout_results(
            LayoutResults::new(constraints.clone(), size, memo),
            cache_fresh,
        );
        self.mark.clear_self_needs_layout();
        self.mark.clear_parent_use_size();
    }

    fn get_intrinsics(&self, intrinsics: &mut P::Intrinsics) {
        let mut converted_intrinsics = match P::convert_intrinsics(intrinsics) {
            Ok(intrincsics) => intrincsics,
            Err(()) => return,
        };
        let intrinsics = converted_intrinsics.borrow_mut();

        let needs_layout = self.mark.needs_layout();
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        let cache_fresh = match needs_layout {
            Ok(()) => inner_reborrow.cache.clear(),
            Err(no_relayout) => no_relayout.into(),
        };

        inner_reborrow.cache.get_intrinsics_or_insert_with(
            intrinsics,
            |intrinsics| {
                inner_reborrow
                    .render
                    .compute_intrinsics(&inner_reborrow.children, intrinsics);
            },
            cache_fresh,
        );
    }
}

pub trait ImplLayout<R: RenderBase> {
    const SIZED_BY_PARENT: bool;
    fn perform_layout_without_resize(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        size: &mut <R::ParentProtocol as Protocol>::Size,
        children: &<R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
    ) -> R::LayoutMemo;
    fn perform_full_layout(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        children: &<R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
    ) -> (<R::ParentProtocol as Protocol>::Size, R::LayoutMemo);
}

impl<
        R: RenderBase,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ImplLayout<R> for RenderImpl<false, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Layout,
{
    const SIZED_BY_PARENT: bool = false;
    fn perform_layout_without_resize(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        size: &mut <R::ParentProtocol as Protocol>::Size,
        children: &<R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
    ) -> R::LayoutMemo {
        let (new_size, memo) = render.perform_layout(constraints, children);
        *size = new_size;
        memo
    }

    fn perform_full_layout(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        children: &<R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
    ) -> (<R::ParentProtocol as Protocol>::Size, R::LayoutMemo) {
        render.perform_layout(constraints, children)
    }
}

impl<
        R: RenderBase,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ImplLayout<R> for RenderImpl<true, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: LayoutByParent,
{
    const SIZED_BY_PARENT: bool = true;
    fn perform_layout_without_resize(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        size: &mut <R::ParentProtocol as Protocol>::Size,
        children: &<R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
    ) -> R::LayoutMemo {
        render.perform_layout(constraints, size, children)
    }

    fn perform_full_layout(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        children: &<R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
    ) -> (<R::ParentProtocol as Protocol>::Size, R::LayoutMemo) {
        let size = render.compute_size_by_parent(constraints);
        let memo = render.perform_layout(constraints, &size, children);
        (size, memo)
    }
}
