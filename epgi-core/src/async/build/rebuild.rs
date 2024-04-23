use crate::{
    foundation::{Arc, Asc, InlinableDwsizeVec, Protocol, Provide},
    sync::CommitBarrier,
    tree::{
        ElementNode, ElementWidgetPair, FullElement, HooksWithEffects, WorkContext, WorkHandle,
    },
};

pub trait ChildElementWidgetPairAsyncBuildExt<P: Protocol> {
    fn rebuild_async(
        self,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    );

    fn rebuild_async_box(
        self: Box<Self>,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    );
}

impl<E> ChildElementWidgetPairAsyncBuildExt<E::ParentProtocol> for ElementWidgetPair<E>
where
    E: FullElement,
{
    fn rebuild_async(
        self,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    ) {
        let _ = self.element.reconcile_node_async(
            Some(self.widget),
            work_context,
            parent_handle,
            barrier,
        );
    }

    fn rebuild_async_box(
        self: Box<Self>,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    ) {
        self.rebuild_async(work_context, parent_handle, barrier)
    }
}

impl<E: FullElement> ElementNode<E> {
    pub(super) fn perform_rebuild_node_async(
        self: &Arc<Self>,
        widget: &E::ArcWidget,
        work_context: Asc<WorkContext>,
        mut hooks: HooksWithEffects,
        element: E,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        handle: &WorkHandle,
        barrier: CommitBarrier,
    ) {
        let lane_pos = work_context.lane_pos;

        let mut jobs = {
            self.context
                .mailbox
                .lock()
                .iter()
                .filter_map(|(job_id, update)| {
                    work_context
                        .job_ids()
                        .contains(job_id)
                        .then_some((*job_id, update.clone()))
                })
                .collect::<Vec<_>>()
        };
        jobs.sort_by_key(|(job_id, ..)| *job_id);

        let updates = jobs
            .into_iter()
            .flat_map(|(_, updates)| updates)
            .collect::<Vec<_>>();

        // let mut hooks = state.hooks;

        for update in updates {
            todo!()
        }

        // let mut hooks_iter = HookContext::new_rebuild(hooks);
        // let mut child_tasks = Default::default();
        // let mut nodes_needing_unmount = Default::default();
        // let reconciler = AsyncReconciler {
        //     host_handle: handle,
        //     work_context,
        //     child_tasks: &mut child_tasks,
        //     barrier,
        //     host_context: &self.context,
        //     hooks: &mut hooks_iter,
        //     nodes_needing_unmount: &mut nodes_needing_unmount,
        // };
        // let results = element.perform_rebuild_element(widget, provider_values, reconciler);
        // let new_stash = match results {
        //     Ok(element) => AsyncOutput::Completed {
        //         children: element.children(),
        //         results: BuildResults::from_pieces(hooks_iter, element, nodes_needing_unmount),
        //     },
        //     Err(err) => AsyncOutput::Suspended {
        //         suspend: Some(BuildSuspendResults::new(hooks_iter)),
        //         barrier: None,
        //     },
        // };

        // self.write_back_build_results::<false>(new_stash, lane_pos, handle, todo!());
        todo!("Child Tasks");
    }
}
