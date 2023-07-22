use crate::{
    foundation::{Arc, Asc, HktContainer, Parallel, Protocol, SmallSet},
    scheduler::{get_current_scheduler, JobId},
    sync::{CommitBarrier, SubtreeCommitResult, TreeScheduler},
};

use super::{
    try_convert_if_same_type, ArcChildElementNode, ArcChildWidget, ArcElementContextNode,
    ArcWidget, BuildContext, Element, ElementNode, Widget, Work, WorkContext, WorkHandle,
};

pub trait Reconciler2 {
    fn build_context_mut(&mut self) -> &mut BuildContext;

    fn into_reconcile<CP: Protocol, I: Parallel<Item = ReconcileItem<CP>>>(
        self,
        items: I,
    ) -> <I::HktContainer as HktContainer>::Container<ArcChildElementNode<CP>>;
}

/// Reconciler has to be a concrete type, not a trait. Due to the virtual function context that it is going to be used upon.
// #[derive(Clone)]
pub struct Reconciler<'a, 'batch> {
    // executor: Asc<E>,
    kind: ReconcilerKind<'a, 'batch>,
    host_context: &'a ArcElementContextNode,
}

pub enum ReconcilerKind<'a, 'batch> {
    Sync {
        job_ids: &'a SmallSet<JobId>,
        scope: &'a rayon::Scope<'batch>,
        tree_scheduler: &'batch TreeScheduler,
        subtree_results: &'a mut SubtreeCommitResult,
    },
    Async {
        host_handle: &'a WorkHandle,
        work_context: Asc<WorkContext>,
        child_tasks: &'a mut Vec<Box<dyn FnOnce() + Send + Sync + 'static>>,
        barrier: CommitBarrier,
    },
}

pub enum ReconcileItem<CP: Protocol> {
    Rebuild(Box<dyn ChildElementWidgetPair<CP>>),
    Inflate(ArcChildWidget<CP>),
}

impl<CP> ReconcileItem<CP>
where
    CP: Protocol,
{
    fn into_async_item(
        self,
        work_context: Asc<WorkContext>,
        host_element_context: ArcElementContextNode,
        barrier: CommitBarrier,
    ) -> AsyncReconcileItem<CP> {
        match self {
            ReconcileItem::Rebuild(_) => todo!(),
            ReconcileItem::Inflate(widget) => {
                let handle = WorkHandle::new();
                todo!()
            }
        }
    }
    fn into_reconcile_with(self, reconciler: Reconciler) {}
}

struct AsyncReconcileItem<CP: Protocol> {
    inner: AsyncReconcileItemInner<CP>,
    work_context: Asc<WorkContext>,
    parent_handle: WorkHandle,
    barrier: CommitBarrier,
}

enum AsyncReconcileItemInner<CP: Protocol> {
    Rebuild(Box<dyn ChildElementWidgetPair<CP>>),
    Inflate(ArcChildElementNode<CP>),
}

impl<CP> AsyncReconcileItem<CP>
where
    CP: Protocol,
{
    fn element(&self) -> ArcChildElementNode<CP> {
        match &self.inner {
            AsyncReconcileItemInner::Rebuild(pair) => pair.element(),
            AsyncReconcileItemInner::Inflate(element) => element.clone(),
        }
    }
    fn perform_reconcile(self) {
        match self.inner {
            AsyncReconcileItemInner::Rebuild(pair) => {
                pair.rebuild_async_box(self.work_context, self.parent_handle, self.barrier)
            }
            AsyncReconcileItemInner::Inflate(element) => todo!(),
        }
    }
}

// fn inflate_widget<W:Widget>(widget: &Asc<W>, build_context: BuildContext)

impl<'a, 'batch> Reconciler<'a, 'batch> {
    pub fn new_sync(
        job_ids: &'a SmallSet<JobId>,
        element_context: &'a ArcElementContextNode,
        scope: &'a rayon::Scope<'batch>,
        subtree_results: &'a mut SubtreeCommitResult,
    ) -> Self {
        todo!()
    }

    pub fn new_async(
        element_context: &'a ArcElementContextNode,
        work_context: Asc<WorkContext>,
        host_handle: &'a WorkHandle,
        child_tasks: &'a mut Vec<Box<dyn FnOnce() + Send + Sync + 'static>>,
        barrier: CommitBarrier,
    ) -> Self {
        Self {
            kind: ReconcilerKind::Async {
                child_tasks,
                barrier,
                host_handle,
                work_context,
            },
            host_context: element_context,
        }
    }

    pub fn into_reconcile<CP: Protocol, I: Parallel<Item = ReconcileItem<CP>>>(
        self,
        items: I,
    ) -> <I::HktContainer as HktContainer>::Container<ArcChildElementNode<CP>> {
        // items.par_map_collect(&get_current_scheduler().threadpool, todo!());
        match self.kind {
            ReconcilerKind::Sync {
                job_ids,
                scope,
                tree_scheduler,
                subtree_results,
            } => items
                .par_map_collect(&get_current_scheduler().threadpool, |item| match item {
                    ReconcileItem::Rebuild(pair) => {
                        pair.rebuild_sync_box(job_ids, scope, tree_scheduler)
                    }
                    ReconcileItem::Inflate(widget) => {
                        widget.inflate_sync(self.host_context, job_ids, scope, tree_scheduler)
                    }
                })
                .map(|(node, subtree_result)| {
                    *subtree_results = subtree_results.merge(subtree_result);
                    node
                }),
            ReconcilerKind::Async {
                child_tasks,
                barrier,
                host_handle,
                work_context,
            } => {
                let async_items = items.map(|item: ReconcileItem<CP>| {
                    item.into_async_item(
                        work_context.clone(),
                        self.host_context.clone(),
                        barrier.clone(),
                    )
                });
                let results = async_items.map_ref(|item| item.element().clone());
                child_tasks.extend(async_items.map(|item: AsyncReconcileItem<CP>| {
                    Box::new(move || item.perform_reconcile())
                        as Box<dyn FnOnce() + Send + Sync + 'static>
                }));
                results
            }
        }
    }
}

impl<E> ElementNode<E>
where
    E: Element,
{
    pub(crate) fn can_rebuild_with(
        self: Arc<Self>,
        widget: ArcChildWidget<E::SelfProtocol>,
    ) -> Option<ElementWidgetPair<E>> {
        let old_widget = self.widget();
        if let Ok(widget) = try_convert_if_same_type(&old_widget, widget) {
            if widget.key() == old_widget.key() {
                return Some(ElementWidgetPair {
                    widget,
                    element: self,
                });
            }
        }
        return None;
    }
}

pub(crate) struct ElementWidgetPair<E: Element> {
    pub(crate) widget: E::ArcWidget,
    pub(crate) element: Arc<ElementNode<E>>,
}

impl<E> Clone for ElementWidgetPair<E>
where
    E: Element,
{
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            element: self.element.clone(),
        }
    }
}

pub trait ChildElementWidgetPair<P: Protocol>:
    crate::sync::reconciler_private::ChildElementWidgetPairSyncBuildExt<P>
    + crate::r#async::reconciler_private::ChildElementWidgetPairAsyncBuildExt<P>
    + Send
    + Sync
    + 'static
{
    fn element(&self) -> ArcChildElementNode<P>;
}

impl<E> ChildElementWidgetPair<E::SelfProtocol> for ElementWidgetPair<E>
where
    E: Element,
{
    fn element(&self) -> ArcChildElementNode<E::SelfProtocol> {
        self.element.clone() as _
    }
}

mod reconciler_private {}
