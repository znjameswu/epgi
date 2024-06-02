use crate::{
    foundation::{
        Arc, Container, ContainerOf, Inlinable64Vec, InlinableDwsizeVec, Protocol, Provide,
    },
    scheduler::{get_current_scheduler, JobId, LanePos},
    sync::LaneScheduler,
    tree::{
        ArcChildElementNode, BuildContext, Element, ElementNode, ElementReconcileItem,
        ElementWidgetPair, FullElement, HookContext, HookContextMode, HooksWithTearDowns,
        ImplElementNode, MainlineState,
    },
};

use super::{CommitResult, ImplCommitRenderObject};

pub trait ChildElementWidgetPairSyncBuildExt<P: Protocol> {
    fn rebuild_sync<'batch>(
        self,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> (ArcChildElementNode<P>, CommitResult<P>);

    fn rebuild_sync_box<'batch>(
        self: Box<Self>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> (ArcChildElementNode<P>, CommitResult<P>);
}

impl<E> ChildElementWidgetPairSyncBuildExt<E::ParentProtocol> for ElementWidgetPair<E>
where
    E: FullElement,
{
    fn rebuild_sync<'batch>(
        self,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> (
        ArcChildElementNode<E::ParentProtocol>,
        CommitResult<E::ParentProtocol>,
    ) {
        let subtree_results =
            self.element
                .reconcile_node_sync(Some(self.widget), job_ids, scope, lane_scheduler);
        (self.element, subtree_results)
    }

    fn rebuild_sync_box<'batch>(
        self: Box<Self>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> (
        ArcChildElementNode<E::ParentProtocol>,
        CommitResult<E::ParentProtocol>,
    ) {
        self.rebuild_sync(job_ids, scope, lane_scheduler)
    }
}

impl<E: FullElement> ElementNode<E> {
    pub(super) fn perform_rebuild_node_sync<'batch>(
        self: &Arc<Self>,
        widget: &E::ArcWidget,
        mut element: E,
        children: ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        mut hooks: HooksWithTearDowns,
        mut render_object: <<E as Element>::Impl as ImplElementNode<E>>::OptionArcRenderObject,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
        is_new_widget: bool,
    ) -> CommitResult<E::ParentProtocol> {
        let mut nodes_needing_unmount = Default::default();
        let hook_context = HookContext::new_sync(&mut hooks, HookContextMode::Rebuild);
        let mut ctx = BuildContext {
            lane_pos: LanePos::SYNC,
            element_context: &self.context,
            hook_context,
        };
        let results = E::perform_rebuild_element(
            &mut element,
            &widget,
            &mut ctx,
            provider_values,
            children,
            &mut nodes_needing_unmount,
        );

        let (state, change) = match results {
            Ok((items, shuffle)) => {
                assert!(
                    ctx.hook_context.has_finished(),
                    "A build function should always invoke every hook whenever it is called"
                );
                // Starting the unmounting as early as possible.
                // Unmount before updating render object can cause render object to hold reference to detached children,
                // Therfore, we need to ensure we do not read into render objects before the batch commit is done
                for node_needing_unmount in nodes_needing_unmount {
                    scope.spawn(|scope| node_needing_unmount.unmount(scope, lane_scheduler))
                }

                let results =
                    items.par_map_collect(&get_current_scheduler().sync_threadpool, |item| {
                        use ElementReconcileItem::*;
                        match item {
                            Keep(node) => node.visit_and_work_sync(job_ids, scope, lane_scheduler),
                            Update(pair) => pair.rebuild_sync_box(job_ids, scope, lane_scheduler),
                            Inflate(widget) => {
                                widget.inflate_sync(Some(self.context.clone()), lane_scheduler)
                            }
                        }
                    });
                let (mut children, changes) = results
                    .unzip_collect(|(child, commit_result)| (child, commit_result.render_object));

                let change = <E as Element>::Impl::rebuild_success_commit_render_object(
                    &element,
                    widget,
                    shuffle,
                    &mut children,
                    &mut render_object,
                    changes,
                    &self.context,
                    lane_scheduler,
                    scope,
                    is_new_widget,
                );
                (
                    MainlineState::Ready {
                        element,
                        hooks,
                        children,
                        render_object,
                    },
                    change,
                )
            }
            Err((children, err)) => {
                debug_assert!(
                    nodes_needing_unmount.is_empty(),
                    "An element that suspends itself should not request unmounting any child nodes"
                );

                (
                    MainlineState::RebuildSuspended {
                        suspended_hooks: hooks,
                        element,
                        children,
                        waker: err.waker,
                    },
                    <E as Element>::Impl::rebuild_suspend_commit_render_object(render_object),
                )
            }
        };
        self.commit_write_element(state);
        return CommitResult::new(change);
    }
}
