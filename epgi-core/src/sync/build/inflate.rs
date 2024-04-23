use crate::{
    foundation::{
        Arc, AsIterator, Container, InlinableDwsizeVec, Protocol, Provide, SyncMutex,
        EMPTY_CONSUMED_TYPES,
    },
    scheduler::get_current_scheduler,
    sync::{LaneScheduler, SubtreeRenderObjectChange, SyncBuildContext, SyncHookContext},
    tree::{
        ArcChildElementNode, ArcElementContextNode, AsyncWorkQueue, Element, ElementBase,
        ElementContextNode, ElementNode, ElementSnapshot, ElementSnapshotInner, FullElement,
        Mainline, MainlineState, Widget,
    },
};

use super::{provider::read_and_update_subscriptions_sync, ImplReconcileCommit};

pub trait ChildWidgetSyncInflateExt<PP: Protocol> {
    fn inflate_sync(
        self: Arc<Self>,
        parent_context: Option<ArcElementContextNode>,
        lane_scheduler: &LaneScheduler,
    ) -> (ArcChildElementNode<PP>, SubtreeRenderObjectChange<PP>);
}

impl<T> ChildWidgetSyncInflateExt<<<T as Widget>::Element as ElementBase>::ParentProtocol> for T
where
    T: Widget,
{
    fn inflate_sync(
        self: Arc<Self>,
        parent_context: Option<ArcElementContextNode>,
        lane_scheduler: &LaneScheduler,
    ) -> (
        ArcChildElementNode<<<T as Widget>::Element as ElementBase>::ParentProtocol>,
        SubtreeRenderObjectChange<<<T as Widget>::Element as ElementBase>::ParentProtocol>,
    ) {
        let (node, results) = ElementNode::<T::Element>::inflate_node_sync(
            &self.into_arc_widget(),
            parent_context,
            lane_scheduler,
        );
        (node as _, results)
    }
}

impl<E: FullElement> ElementNode<E> {
    pub(super) fn inflate_node_sync(
        widget: &E::ArcWidget,
        parent_context: Option<ArcElementContextNode>,
        lane_scheduler: &LaneScheduler,
    ) -> (
        Arc<ElementNode<E>>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        let node = Arc::new_cyclic(|weak| ElementNode {
            context: Arc::new(ElementContextNode::new_for::<E>(
                weak.clone() as _,
                parent_context,
                widget,
            )),
            snapshot: SyncMutex::new(ElementSnapshot {
                widget: widget.clone(),
                inner: ElementSnapshotInner::Mainline(Mainline {
                    state: None,
                    async_queue: AsyncWorkQueue::new_empty(),
                }),
            }),
        });

        let consumed_values = read_and_update_subscriptions_sync(
            E::get_consumed_types(widget),
            EMPTY_CONSUMED_TYPES,
            &node.context,
            lane_scheduler,
        );

        let subtree_results = Self::perform_inflate_node_sync::<true>(
            &node,
            widget,
            SyncHookContext::new_inflate(),
            consumed_values,
            lane_scheduler,
        );
        return (node, subtree_results);
    }

    pub(super) fn perform_inflate_node_sync<const FIRST_INFLATE: bool>(
        self: &Arc<Self>,
        widget: &E::ArcWidget,
        mut hook_context: SyncHookContext,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        lane_scheduler: &LaneScheduler,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        let result = E::perform_inflate_element(
            &widget,
            SyncBuildContext {
                hooks: &mut hook_context,
                element_context: &self.context,
            }
            .into(),
            provider_values,
        );

        let (state, change) = match result {
            Ok((element, child_widgets)) => {
                let results = child_widgets.par_map_collect(
                    &get_current_scheduler().sync_threadpool,
                    |child_widget| {
                        child_widget.inflate_sync(Some(self.context.clone()), lane_scheduler)
                    },
                );
                let (mut children, changes) = results.unzip_collect(|x| x);

                debug_assert!(
                    !changes
                        .as_iter()
                        .any(SubtreeRenderObjectChange::is_keep_render_object),
                    "Fatal logic bug in epgi-core reconcile logic. Please file issue report."
                );

                let (render_object, change) = <E as Element>::Impl::inflate_success_commit(
                    &element,
                    widget,
                    &mut children,
                    changes,
                    &self.context,
                    lane_scheduler,
                );

                (
                    MainlineState::Ready {
                        element,
                        hooks: hook_context.hooks,
                        children,
                        render_object,
                    },
                    change,
                )
            }
            Err(err) => (
                MainlineState::InflateSuspended {
                    suspended_hooks: hook_context.hooks,
                    waker: err.waker,
                },
                SubtreeRenderObjectChange::Suspend,
            ),
        };
        if FIRST_INFLATE {
            self.commit_write_element_first_inflate(state);
        } else {
            self.commit_write_element(state)
        }
        return change;
    }
}
