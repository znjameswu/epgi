use hashbrown::HashSet;

use crate::{
    common::{
        AweakAnyRenderObject, Element, LayoutExecutor, PerformLayout, Render, RenderObject,
        RenderObjectInner,
    },
    foundation::{Arc, Protocol, PtrEq},
    scheduler::get_current_scheduler,
    sync::TreeScheduler,
};

impl TreeScheduler {
    pub(crate) fn perform_layout(
        &mut self,
        boundaries_needing_relayout: HashSet<PtrEq<AweakAnyRenderObject>>,
    ) {
        let mut boundaries_needing_relayout = boundaries_needing_relayout
            .into_iter()
            .filter_map(|x| {
                x.0.upgrade()
                    .filter(|x| !x.element_context().is_unmounted())
            })
            .collect::<Vec<_>>();

        boundaries_needing_relayout.sort_unstable_by_key(|object| object.element_context().depth);

        for boundary in boundaries_needing_relayout {
            boundary.layout_without_resize()
        }
    }
}

impl<R> RenderObjectInner<R>
where
    R: Render,
{
    fn is_relayout_boundary(&self) -> bool {
        R::PERFORM_LAYOUT.sized_by_parent() || self.cache.parent_use_size() == Some(true)
    }
}

impl<R> RenderObject<R>
where
    R: Render,
{
    fn is_relayout_boundary(&self) -> bool {
        R::PERFORM_LAYOUT.sized_by_parent()
            || self.inner.lock().cache.parent_use_size() == Some(false)
    }
    fn layout_without_resize(&self) {
        let mut inner = self.inner.lock();
        debug_assert!(inner.is_relayout_boundary());
        let Some(cache) = inner.cache.inner.as_mut() else {
            panic!("Relayout should only be called on relayout boundaries which must retain their layout caches")
        };
        if cache.layout.is_some() {
            return;
        }
        let constraints = cache.constraints.clone();
        let parent_use_size = cache.parent_use_size;
        get_current_scheduler()
            .threadpool
            .0
            .in_place_scope(|scope| {
                inner.perform_wet_layout(constraints, parent_use_size, LayoutExecutor { scope });
            })
    }

    fn layout_detached<'a, 'layout>(
        self: Arc<Self>,
        constraints: <<R::Element as Element>::SelfProtocol as Protocol>::Constraints,
        executor: LayoutExecutor<'a, 'layout>,
    ) {
        executor.scope.spawn(move |scope| {
            let mut inner = self.inner.lock();
            if inner.cache.get_layout_for(&constraints, false).is_some() {
                return;
            }
            inner.perform_wet_layout(constraints, false, LayoutExecutor { scope });
        });
    }

    fn layout<'a, 'layout>(
        &'a self,
        constraints: <<R::Element as Element>::SelfProtocol as Protocol>::Constraints,
        executor: LayoutExecutor<'a, 'layout>,
    ) {
        let mut inner = self.inner.lock();
        if inner.cache.get_layout_for(&constraints, false).is_some() {
            return;
        }
        inner.perform_wet_layout(constraints, false, executor);
    }

    fn layout_use_size<'a, 'layout>(
        &'a self,
        constraints: <<R::Element as Element>::SelfProtocol as Protocol>::Constraints,
        executor: LayoutExecutor<'a, 'layout>,
    ) -> <<R::Element as Element>::SelfProtocol as Protocol>::Size {
        let mut inner = self.inner.lock();

        if let Some(size) = inner.cache.get_layout_for(&constraints, true) {
            return size.clone();
        }
        inner
            .perform_wet_layout(constraints, true, executor)
            .clone()
    }
}
impl<R> RenderObjectInner<R>
where
    R: Render,
{
    #[inline(always)]
    fn perform_wet_layout<'a, 'layout>(
        &'a mut self,
        constraints: <<R::Element as Element>::SelfProtocol as Protocol>::Constraints,
        parent_use_size: bool,
        executor: LayoutExecutor<'a, 'layout>,
    ) -> &<<R::Element as Element>::SelfProtocol as Protocol>::Size {
        use PerformLayout::*;
        let (size, memo) = match R::PERFORM_LAYOUT {
            WetLayout { perform_layout } => perform_layout(&self.render, &constraints, executor),
            DryLayout {
                compute_dry_layout,
                perform_layout,
            } => {
                let size = compute_dry_layout(&self.render, &constraints);
                let memo = perform_layout(&self.render, &constraints, &size, executor);
                (size, memo)
            }
        };

        return &self
            .cache
            .insert_layout_results(constraints, parent_use_size, size, memo);
    }
}

pub(crate) mod layout_private {
    use crate::common::RenderObject;

    use super::*;

    pub trait ChildRenderObjectLayoutExt<SP: Protocol> {
        fn layout_use_size<'a, 'layout>(
            &'a self,
            constraints: SP::Constraints,
            executor: LayoutExecutor<'a, 'layout>,
        ) -> SP::Size;

        fn layout<'a, 'layout>(
            &'a self,
            constraints: SP::Constraints,
            executor: LayoutExecutor<'a, 'layout>,
        );

        fn layout_detached<'a, 'layout>(
            self: Arc<Self>,
            constraints: SP::Constraints,
            executor: LayoutExecutor<'a, 'layout>,
        );
    }

    impl<R> ChildRenderObjectLayoutExt<<R::Element as Element>::SelfProtocol> for RenderObject<R>
    where
        R: Render,
    {
        fn layout_use_size<'a, 'layout>(
            &'a self,
            constraints: <<R::Element as Element>::SelfProtocol as Protocol>::Constraints,
            executor: LayoutExecutor<'a, 'layout>,
        ) -> <<R::Element as Element>::SelfProtocol as Protocol>::Size {
            self.layout_use_size(constraints, executor)
        }

        fn layout<'a, 'layout>(
            &'a self,
            constraints: <<R::Element as Element>::SelfProtocol as Protocol>::Constraints,
            executor: LayoutExecutor<'a, 'layout>,
        ) {
            self.layout(constraints, executor)
        }

        fn layout_detached<'a, 'layout>(
            self: Arc<Self>,
            constraints: <<R::Element as Element>::SelfProtocol as Protocol>::Constraints,
            executor: LayoutExecutor<'a, 'layout>,
        ) {
            self.layout_detached(constraints, executor)
        }
    }

    pub trait AnyRenderObjectRelayoutExt {
        fn layout_without_resize(&self);
    }

    impl<R> AnyRenderObjectRelayoutExt for RenderObject<R>
    where
        R: Render,
    {
        fn layout_without_resize(&self) {
            self.layout_without_resize()
        }
    }
}
