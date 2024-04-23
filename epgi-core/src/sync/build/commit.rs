mod component_element;
mod render_element;
mod suspense_element;

use crate::{
    foundation::{Arc, ContainerOf},
    sync::{LaneScheduler, SubtreeRenderObjectChange},
    tree::{
        ArcChildElementNode, ArcElementContextNode, ChildRenderObjectsUpdateCallback, Element,
        ElementBase, ElementNode, FullElement, HooksWithTearDowns, ImplElementNode, MainlineState,
    },
};

impl<E: FullElement> ElementNode<E> {
    pub(super) fn commit_write_element(
        self: &Arc<Self>,
        state: MainlineState<E, HooksWithTearDowns>,
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
                mainline.async_queue.current().is_none(),
                "An async work should not be executing alongside a sync work"
            );
            mainline.state = Some(state);
            self.prepare_execute_backqueue(mainline, &snapshot_reborrow.widget)
        };

        if let Some(async_work_needing_start) = async_work_needing_start {
            let node = self.clone();
            node.execute_rebuild_node_async_detached(async_work_needing_start);
        }
    }

    pub(super) fn commit_write_element_first_inflate(
        self: &Arc<Self>,
        state: MainlineState<E, HooksWithTearDowns>,
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

pub trait ImplReconcileCommit<E: Element<Impl = Self>>: ImplElementNode<E> {
    // Reason for this signature: we need to ensure the happy path (rebuild success with no suspense)
    // do not require a lock since there is nothing to write.
    // And there is a lot of cloned resources from visit_inspect that has no further use
    // (since visit do not occupy node, it has no choice but to clone resources out)
    fn visit_commit(
        element_node: &ElementNode<E>,
        render_object: Self::OptionArcRenderObject,
        render_object_changes: ContainerOf<
            <E as ElementBase>::ChildContainer,
            SubtreeRenderObjectChange<<E as ElementBase>::ChildProtocol>,
        >,
        lane_scheduler: &LaneScheduler,
        scope: &rayon::Scope<'_>,
        self_rebuild_suspended: bool,
    ) -> SubtreeRenderObjectChange<<E as ElementBase>::ParentProtocol>;

    // Reason for this signature: there is going to be a write_back regardless
    // You have to return the resources since they are moved out when we occupy the node, and later they need to move back
    fn rebuild_success_commit(
        element: &E,
        widget: &E::ArcWidget,
        shuffle: Option<ChildRenderObjectsUpdateCallback<E::ChildContainer, E::ChildProtocol>>,
        children: &mut ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        render_object: &mut Self::OptionArcRenderObject,
        render_object_changes: ContainerOf<
            E::ChildContainer,
            SubtreeRenderObjectChange<E::ChildProtocol>,
        >,
        element_context: &ArcElementContextNode,
        lane_scheduler: &LaneScheduler,
        scope: &rayon::Scope<'_>,
        is_new_widget: bool,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol>;

    fn rebuild_suspend_commit(
        render_object: Self::OptionArcRenderObject,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol>;

    fn inflate_success_commit(
        element: &E,
        widget: &E::ArcWidget,
        children: &mut ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        render_object_changes: ContainerOf<
            E::ChildContainer,
            SubtreeRenderObjectChange<E::ChildProtocol>,
        >,
        element_context: &ArcElementContextNode,
        lane_scheduler: &LaneScheduler,
    ) -> (
        Self::OptionArcRenderObject,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    );
}
