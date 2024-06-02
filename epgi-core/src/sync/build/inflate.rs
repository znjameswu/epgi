use crate::{
    foundation::{
        Arc, AsIterator, Container, InlinableDwsizeVec, Protocol, Provide, SyncMutex,
        EMPTY_CONSUMED_TYPES,
    },
    scheduler::{get_current_scheduler, LanePos},
    sync::{LaneScheduler, RenderObjectCommitResult},
    tree::{
        ArcChildElementNode, ArcElementContextNode, AsyncWorkQueue, BuildContext, Element,
        ElementBase, ElementContextNode, ElementNode, ElementSnapshot, ElementSnapshotInner,
        FullElement, HookContext, HookContextMode, HooksWithTearDowns, Mainline, MainlineState,
        Widget,
    },
};

use super::{provider::read_and_update_subscriptions_sync, CommitResult, ImplCommitRenderObject};

pub trait ChildWidgetSyncInflateExt<PP: Protocol> {
    fn inflate_sync(
        self: Arc<Self>,
        parent_context: Option<ArcElementContextNode>,
        lane_scheduler: &LaneScheduler,
    ) -> (ArcChildElementNode<PP>, CommitResult<PP>);
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
        CommitResult<<<T as Widget>::Element as ElementBase>::ParentProtocol>,
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
    ) -> (Arc<ElementNode<E>>, CommitResult<E::ParentProtocol>) {
        let node = Arc::new_cyclic(|weak| ElementNode {
            context: Arc::new(ElementContextNode::new_for::<E>(
                weak.clone() as _,
                parent_context,
                widget,
            )),
            snapshot: SyncMutex::new(ElementSnapshot::new(
                widget.clone(),
                ElementSnapshotInner::Mainline(Mainline {
                    state: None,
                    async_queue: AsyncWorkQueue::new_empty(),
                }),
            )),
        });

        let consumed_values = read_and_update_subscriptions_sync(
            E::get_consumed_types(widget),
            EMPTY_CONSUMED_TYPES,
            &node.context,
            lane_scheduler,
        );

        let commit_result = Self::perform_inflate_node_sync::<true>(
            &node,
            widget,
            None,
            consumed_values,
            lane_scheduler,
        );
        return (node, commit_result);
    }

    pub(super) fn perform_inflate_node_sync<const FIRST_INFLATE: bool>(
        self: &Arc<Self>,
        widget: &E::ArcWidget,
        suspended_hooks: Option<HooksWithTearDowns>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        lane_scheduler: &LaneScheduler,
    ) -> CommitResult<E::ParentProtocol> {
        let hook_mode = if suspended_hooks.is_none() {
            HookContextMode::Inflate
        } else {
            HookContextMode::PollInflate
        };
        let mut hooks = suspended_hooks.unwrap_or_default();
        let mut ctx = BuildContext {
            lane_pos: LanePos::SYNC,
            element_context: &self.context,
            hook_context: HookContext::new_sync(&mut hooks, hook_mode),
        };
        let result = E::perform_inflate_element(&widget, &mut ctx, provider_values);

        let (state, change) = match result {
            Ok((element, child_widgets)) => {
                if !ctx.hook_context.has_finished() {
                    panic!("A build function should always invoke every hook whenever it is called")
                }

                let results = child_widgets.par_map_collect(
                    &get_current_scheduler().sync_threadpool,
                    |child_widget| {
                        child_widget.inflate_sync(Some(self.context.clone()), lane_scheduler)
                    },
                );
                let (mut children, render_object_changes) = results
                    .unzip_collect(|(child, commit_result)| (child, commit_result.render_object));

                debug_assert!(
                    !render_object_changes
                        .as_iter()
                        .any(RenderObjectCommitResult::is_keep_render_object),
                    "Fatal logic bug in epgi-core reconcile logic. Please file issue report."
                );

                let (render_object, render_object_commit_result) =
                    <E as Element>::Impl::inflate_success_commit_render_object(
                        &element,
                        widget,
                        &mut children,
                        render_object_changes,
                        &self.context,
                        lane_scheduler,
                    );

                (
                    MainlineState::Ready {
                        element,
                        hooks,
                        children,
                        render_object,
                    },
                    render_object_commit_result,
                )
            }
            Err(err) => (
                MainlineState::InflateSuspended {
                    suspended_hooks: hooks,
                    waker: err.waker,
                },
                RenderObjectCommitResult::Suspend,
            ),
        };
        if FIRST_INFLATE {
            self.commit_write_element_first_inflate(state);
        } else {
            self.commit_write_element(state)
        }
        return CommitResult::new(change);
    }
}
