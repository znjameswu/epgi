mod component_element;
mod render_element;
mod suspense_element;

use crate::{
    foundation::{Arc, ContainerOf},
    sync::{LaneScheduler, RenderObjectCommitResult},
    tree::{
        ArcChildElementNode, ArcElementContextNode, ChildRenderObjectsUpdateCallback, Element,
        ElementBase, ElementNode, FullElement, HooksWithCleanups, ImplElementNode, MainlineState,
    },
};

impl<E: FullElement> ElementNode<E> {
    pub(super) fn commit_write_element(
        self: &Arc<Self>,
        state: MainlineState<E, HooksWithCleanups>,
    ) {
        // Collecting async work is necessary, even if we are inflating!
        // Since it could be an InflateSuspended node and an async batch spawned a secondary root on this node.
        let async_work_needing_start = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_reborrow = &mut *snapshot;
            let mainline = snapshot_reborrow
                .inner
                .mainline_mut()
                .expect("An unmounted element node should not be reachable by a rebuild!");
            debug_assert!(
                mainline.async_queue.current_ref().is_none(),
                "An async work should not be executing alongside a sync work"
            );
            mainline.state = Some(state);
            self.setup_execute_backqueue(
                mainline,
                &snapshot_reborrow.widget,
                &snapshot_reborrow.element_lock_held,
            )
        };

        if let Some(async_work_needing_start) = async_work_needing_start {
            let node = self.clone();
            node.execute_reconcile_node_async_detached(async_work_needing_start);
        }
    }

    pub(super) fn commit_write_element_first_inflate(
        self: &Arc<Self>,
        state: MainlineState<E, HooksWithCleanups>,
    ) {
        let mut snapshot = self.snapshot.lock();
        let snapshot_reborrow = &mut *snapshot;
        let mainline = snapshot_reborrow
            .inner
            .mainline_mut()
            .expect("An unmounted element node should not be reachable by a rebuild!");
        debug_assert!(
            mainline.async_queue.is_empty(),
            "The first-time inflate should not see have any other async work"
        );
        mainline.state = Some(state);
    }
}

pub trait ImplCommitRenderObject<E: Element<Impl = Self>>: ImplElementNode<E> {
    // Reason for this signature: we need to ensure the happy path (rebuild success with no suspense)
    // do not require a lock since there is nothing to write.
    // And there is a lot of cloned resources from visit_inspect that has no further use
    // (since visit do not occupy node, it has no choice but to clone resources out)
    fn visit_commit_render_object<'batch>(
        element_node: &ElementNode<E>,
        render_object: Self::OptionArcRenderObject,
        render_object_changes: ContainerOf<
            <E as ElementBase>::ChildContainer,
            RenderObjectCommitResult<<E as ElementBase>::ChildProtocol>,
        >,
        lane_scheduler: &'batch LaneScheduler,
        scope: &rayon::Scope<'batch>,
        self_rebuild_suspended: bool,
    ) -> RenderObjectCommitResult<<E as ElementBase>::ParentProtocol>;

    // Reason for this signature: there is going to be a write_back regardless
    // You have to return the resources since they are moved out when we occupy the node, and later they need to move back
    fn rebuild_success_commit_render_object<'batch>(
        element: &mut E,
        widget: &E::ArcWidget,
        shuffle: Option<ChildRenderObjectsUpdateCallback<E::ChildContainer, E::ChildProtocol>>,
        children: &mut ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        render_object: Option<Self::OptionArcRenderObject>,
        render_object_changes: ContainerOf<
            E::ChildContainer,
            RenderObjectCommitResult<E::ChildProtocol>,
        >,
        element_context: &ArcElementContextNode,
        lane_scheduler: &'batch LaneScheduler,
        scope: &rayon::Scope<'batch>,
        is_new_widget: bool,
    ) -> (
        Self::OptionArcRenderObject,
        RenderObjectCommitResult<E::ParentProtocol>,
    );

    // Detach render object if any
    fn rebuild_suspend_commit_render_object(
        render_object: Option<Self::OptionArcRenderObject>,
    ) -> RenderObjectCommitResult<E::ParentProtocol>;

    fn inflate_success_commit_render_object(
        element: &mut E,
        widget: &E::ArcWidget,
        children: &mut ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        render_object_changes: ContainerOf<
            E::ChildContainer,
            RenderObjectCommitResult<E::ChildProtocol>,
        >,
        element_context: &ArcElementContextNode,
        lane_scheduler: &LaneScheduler,
    ) -> (
        Self::OptionArcRenderObject,
        RenderObjectCommitResult<E::ParentProtocol>,
    );

    // Detach render object if any
    fn detach_render_object(render_object: &Self::OptionArcRenderObject);

    /// In async batches, whether to wait or to commit immediately if the child suspends during inflating.
    ///
    /// This mainly determines how [`Suspense`] works together with `startTransition` or generally any async batch.
    /// And it's highly recommended to only set this for a [`Suspense`]-like component because this is a highly improvised parameter tailored for [`Suspense`].
    ///
    /// In React's design, `startTransition` can decide to not wait for a content to resolve, if the suspend happens during the contenxt's initial inflating **and** the [`Suspense`] catching it does not have any remaining exisiting content (thus safe to show a fallback). See: https://react.dev/reference/react/Suspense#preventing-already-revealed-content-from-hiding
    ///
    /// If set to true, then in an async batch, any descendant suspended work won't generate a `CommitBarrier` and thus won't prevent the batch from being committed.
    const ALLOW_ASYNC_COMMIT_INFLATE_SUSPENDED_CHILD: bool = false;
}
