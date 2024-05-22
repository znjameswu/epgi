/// Basic concepts:
///
/// 1. Purge: Simple wipe the existence and stop any execution of this lane, disregarding any lane mark consistencies.
///     1. Destroys lane mark consistency under the subtree.
///     2. Only suitable for batch retirement or subtree unmount, when the subtree is certainly not going to be revisited by the given batch.
/// 2. Remove: Purge the subtree of this lane, then requeueing any other lane that is unblocked by the purging.
/// Surprisingly, remove is remarkably similar to the reorder operation
/// 3. Cancel: Remove this lane in the **descendants**. For the subtree root, we first purge this lane and then put it into backqueue.
/// We do not try to requeue anything in the subtree root.
use crate::{
    foundation::{Arc, Container, ContainerOf},
    r#async::AsyncReconcile,
    scheduler::{get_current_scheduler, LanePos},
    sync::LaneScheduler,
    tree::{
        ArcChildElementNode, AsyncDequeueResult, AsyncInflating, AsyncOutput,
        AsyncQueueCurrentEntry, AsyncStash, AsyncWorkQueue, AweakElementContextNode,
        ConsumerWorkSpawnToken, Element, ElementBase, ElementNode, ElementSnapshot,
        ElementSnapshotInner, FullElement, ImplProvide, Mainline, SubscriptionDiff,
    },
};

pub(in super::super) struct CancelAsync<I> {
    pub(super) lane_pos: LanePos,
    pub(super) spawned_consumers: Option<Vec<(AweakElementContextNode, ConsumerWorkSpawnToken)>>,
    pub(super) subscription_diff: SubscriptionDiff,
    pub(super) new_children: Option<I>,
}

pub(in super::super) struct RemoveAsync<E: ElementBase> {
    purge: Result<
        CancelAsync<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
        Option<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
    >,
    start: Option<AsyncReconcile<E>>,
}

impl<E: FullElement> ElementNode<E> {
    // // This functions serves as an example function on how to invoke the kit
    // pub(super) fn cancel_async_work(
    //     self: &Arc<Self>,
    //     lane_pos: LanePos,
    //     lane_scheduler: &BuildScheduler,
    // ) {
    //     let remove_result = {
    //         let mut snapshot = self.snapshot.lock();
    //         let snapshot_reborrow = &mut *snapshot;
    //         let mainline = snapshot
    //             .inner
    //             .mainline_mut()
    //             .expect("Cancel can only be called on mainline nodes");
    //         Self::prepare_cancel_async_work(mainline, lane_pos, lane_scheduler)
    //     };

    //     match remove_result {
    //         Ok(remove) => self.perform_cancel_async_work(remove),
    //         Err(Some(children)) => children
    //             .par_for_each(&get_current_scheduler().threadpool, |child| {
    //                 child.remove_async_work_in_subtree(lane_pos)
    //             }),
    //         Err(None) => {}
    //     }
    // }

    pub(in super::super) fn prepare_cancel_async_work(
        mainline: &mut Mainline<E>,
        lane_pos: LanePos,
        lane_scheduler: &LaneScheduler,
    ) -> Result<
        CancelAsync<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
        Option<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
    > {
        let Mainline { state, async_queue } = mainline;
        use AsyncDequeueResult::*;
        use AsyncOutput::*;
        match async_queue.try_remove(lane_pos) {
            FoundCurrent(AsyncQueueCurrentEntry {
                stash:
                    AsyncStash {
                        handle,
                        subscription_diff,
                        spawned_consumers,
                        output,
                    },
                widget,
                work_context,
            }) => {
                handle.abort();
                async_queue.push_backqueue(
                    widget,
                    work_context,
                    lane_scheduler
                        .get_commit_barrier_for(lane_pos)
                        .expect("CommitBarrier should exist"),
                );
                Ok(CancelAsync {
                    lane_pos,
                    spawned_consumers: spawned_consumers,
                    subscription_diff,
                    new_children: match output {
                        Completed(results) => Some(results.children),
                        _ => None,
                    },
                })
            }
            FoundBackqueue(_) => Err(state
                .as_ref()
                .expect("A mainline tree walk should not encounter another sync work.")
                .children_cloned()),
            NotFound => Err(None),
        }
    }

    pub(in super::super) fn perform_cancel_async_work(
        self: &Arc<Self>,
        cancel: CancelAsync<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
    ) {
        let CancelAsync {
            lane_pos,
            spawned_consumers,
            subscription_diff,
            new_children,
        } = cancel;

        self.perform_purge_async_work_local(spawned_consumers, subscription_diff, lane_pos);

        if let Some(new_children) = new_children {
            new_children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                child.remove_async_work_in_subtree(lane_pos)
            })
        }

        // Do not requeue anything here
    }
}

impl<E: FullElement> ElementNode<E> {
    fn remove_async_work_in_subtree(self: &Arc<Self>, lane_pos: LanePos) {
        let no_mailbox_update = !self.context.mailbox_lanes().contains(lane_pos);
        let no_consumer_root = !self.context.consumer_lanes().contains(lane_pos);
        let no_descendant_lanes = !self.context.descendant_lanes().contains(lane_pos);

        if no_mailbox_update && no_consumer_root && no_descendant_lanes {
            return;
        }

        let remove = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_reborrow = &mut *snapshot;
            self.prepare_remove_async_work(snapshot_reborrow, lane_pos)
        };

        let RemoveAsync { purge, start } = remove;

        match purge {
            Ok(purge) => self.perform_cancel_async_work(purge),
            Err(Some(children)) => children
                .par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                    child.remove_async_work_in_subtree(lane_pos)
                }),
            Err(None) => {}
        }

        if let Some(start) = start {
            let node = self.clone();
            node.execute_reconcile_node_async_detached(start);
        }
    }

    pub(super) fn prepare_remove_async_work(
        self: &Arc<Self>,
        snapshot: &mut ElementSnapshot<E>,
        lane_pos: LanePos,
    ) -> RemoveAsync<E> {
        match &mut snapshot.inner {
            ElementSnapshotInner::AsyncInflating(async_inflating) => {
                let purge =
                    Self::prepare_purge_async_work_async_inflating(async_inflating, lane_pos);
                RemoveAsync { purge, start: None }
            }
            ElementSnapshotInner::Mainline(mainline) => {
                self.prepare_remove_async_work_mainline(mainline, &snapshot.widget, lane_pos)
            }
        }
    }

    pub(super) fn prepare_remove_async_work_mainline(
        self: &Arc<Self>,
        mainline: &mut Mainline<E>,
        old_widget: &E::ArcWidget,
        lane_pos: LanePos,
    ) -> RemoveAsync<E> {
        let purge = Self::prepare_purge_async_work_mainline(mainline, lane_pos);
        let rebuild = self.prepare_execute_backqueue(mainline, old_widget);
        RemoveAsync {
            purge,
            start: rebuild,
        }
    }
}

impl<E: FullElement> ElementNode<E> {
    fn purge_async_work_in_subtree(self: &Arc<Self>, lane_pos: LanePos) {
        let no_mailbox_update = !self.context.mailbox_lanes().contains(lane_pos);
        let no_consumer_root = !self.context.consumer_lanes().contains(lane_pos);
        let no_descendant_lanes = !self.context.descendant_lanes().contains(lane_pos);

        if no_mailbox_update && no_consumer_root && no_descendant_lanes {
            return;
        }

        let purge = {
            let mut snapshot = self.snapshot.lock();
            Self::prepare_purge_async_work(&mut *snapshot, lane_pos)
        };

        match purge {
            Ok(remove) => self.perform_purge_async_work(remove),
            Err(Some(children)) => children
                .par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                    child.purge_async_work_in_subtree(lane_pos)
                }),
            Err(None) => {}
        }
    }

    pub(super) fn prepare_purge_async_work(
        snapshot: &mut ElementSnapshot<E>,
        lane_pos: LanePos,
    ) -> Result<
        CancelAsync<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
        Option<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
    > {
        match &mut snapshot.inner {
            ElementSnapshotInner::AsyncInflating(async_inflating) => {
                Self::prepare_purge_async_work_async_inflating(async_inflating, lane_pos)
            }
            ElementSnapshotInner::Mainline(mainline) => {
                Self::prepare_purge_async_work_mainline(mainline, lane_pos)
            }
        }
    }

    pub(super) fn prepare_purge_async_work_async_inflating(
        async_inflating: &mut AsyncInflating<E>,
        lane_pos: LanePos,
    ) -> Result<
        CancelAsync<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
        Option<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
    > {
        let AsyncInflating {
            work_context,
            stash:
                AsyncStash {
                    handle,
                    subscription_diff,
                    spawned_consumers,
                    output,
                },
        } = async_inflating;
        debug_assert!(spawned_consumers.is_none());
        assert!(
            work_context.lane_pos == lane_pos,
            "A tree walk should not witness unmounted nodes from other lanes"
        );
        handle.abort();
        let subscription_diff = std::mem::take(subscription_diff);
        // Replace with an invalid empty value
        let output = std::mem::replace(
            output,
            AsyncOutput::Suspended {
                suspend: None,
                barrier: None,
            },
        );
        use AsyncOutput::*;
        Ok(CancelAsync {
            lane_pos,
            spawned_consumers: None,
            subscription_diff,
            new_children: match output {
                Completed(results) => Some(results.children),
                _ => None,
            },
        })
    }

    pub(in super::super) fn prepare_purge_async_work_mainline(
        mainline: &mut Mainline<E>,
        lane_pos: LanePos,
    ) -> Result<
        CancelAsync<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
        Option<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
    > {
        let Mainline { state, async_queue } = mainline;
        use AsyncDequeueResult::*;
        use AsyncOutput::*;
        match async_queue.try_remove(lane_pos) {
            FoundCurrent(AsyncQueueCurrentEntry {
                stash:
                    AsyncStash {
                        handle,
                        subscription_diff,
                        spawned_consumers,
                        output,
                    },
                ..
            }) => {
                handle.abort();
                Ok(CancelAsync {
                    lane_pos,
                    spawned_consumers,
                    subscription_diff,
                    new_children: match output {
                        Completed(results) => Some(results.children),
                        _ => None,
                    },
                })
            }
            FoundBackqueue(_) => Err(state
                .as_ref()
                .expect("A mainline tree walk should not encounter another sync work.")
                .children_cloned()),
            NotFound => Err(None),
        }
    }

    pub(super) fn perform_purge_async_work_local(
        self: &Arc<Self>,
        spawned_consumers: Option<Vec<(AweakElementContextNode, ConsumerWorkSpawnToken)>>,
        subscription_diff: SubscriptionDiff,
        lane_pos: LanePos,
    ) {
        if <E as Element>::Impl::PROVIDE_ELEMENT {
            if let Some(spawned_consumers) = spawned_consumers {
                self.context.unreserve_write_async(lane_pos);
                // We choose relaxed lane marking without unmarking
                for (spawned_consumer, spawn_token) in spawned_consumers {
                    spawned_consumer
                        .upgrade()
                        .expect("Readers should be alive")
                        .unmark_consumer(lane_pos, spawn_token);
                }
            }
        } else {
            debug_assert!(
                spawned_consumers.is_none(),
                "An Element without declaring provider should not write and spawn consumer work"
            )
        }

        for reserved in subscription_diff.reserve {
            reserved.unreserve_read(&(Arc::downgrade(self) as _), lane_pos)
        }
    }

    pub(in super::super) fn perform_purge_async_work(
        self: &Arc<Self>,
        cancel: CancelAsync<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
    ) {
        let CancelAsync {
            lane_pos,
            spawned_consumers,
            subscription_diff,
            new_children,
        } = cancel;

        if let Some(new_children) = new_children {
            new_children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                child.purge_async_work_in_subtree(lane_pos)
            })
        }

        // Reverse-order
        self.perform_purge_async_work_local(spawned_consumers, subscription_diff, lane_pos);
    }
}

impl<E: FullElement> ElementNode<E> {
    pub fn remove_async_work_and_lane_in_subtree(self: &Arc<Self>, lane_pos: LanePos) {
        let no_mailbox_update = !self.context.mailbox_lanes().contains(lane_pos);
        let no_consumer_root = !self.context.consumer_lanes().contains(lane_pos);
        let no_descendant_lanes = !self.context.descendant_lanes().contains(lane_pos);

        if no_mailbox_update && no_consumer_root && no_descendant_lanes {
            return;
        }
        let remove = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_reborrow = &mut *snapshot;
            self.prepare_remove_async_work(snapshot_reborrow, lane_pos)
        };

        let RemoveAsync { purge, start } = remove;

        match purge {
            Ok(purge) => {
                let CancelAsync {
                    lane_pos,
                    spawned_consumers,
                    subscription_diff,
                    new_children,
                } = purge;

                if let Some(new_children) = new_children {
                    new_children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                        child.remove_async_work_and_lane_in_subtree(lane_pos)
                    })
                }

                // Reverse-order
                self.perform_purge_async_work_local(spawned_consumers, subscription_diff, lane_pos);
            }
            Err(Some(children)) => children
                .par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                    child.remove_async_work_and_lane_in_subtree(lane_pos)
                }),
            Err(None) => {}
        }

        if let Some(start) = start {
            let node = self.clone();
            node.execute_reconcile_node_async_detached(start);
        }
    }
}

impl<E: FullElement> ElementNode<E> {
    pub(in crate::sync::build) fn setup_unmount_async_work(
        snapshot: &mut ElementSnapshot<E>,
    ) -> Option<CancelAsync<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>>
    {
        match &mut snapshot.inner {
            ElementSnapshotInner::AsyncInflating(async_inflating) => {
                Self::setup_unmount_async_work_async_inflating(async_inflating)
            }
            ElementSnapshotInner::Mainline(mainline) => {
                Self::setup_unmount_async_work_mainline(mainline)
            }
        }
    }

    pub(super) fn setup_unmount_async_work_async_inflating(
        async_inflating: &mut AsyncInflating<E>,
    ) -> Option<CancelAsync<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>>
    {
        let AsyncInflating {
            work_context,
            stash:
                AsyncStash {
                    handle,
                    subscription_diff,
                    spawned_consumers,
                    output,
                },
        } = async_inflating;
        debug_assert!(spawned_consumers.is_none());
        handle.abort();
        let subscription_diff = std::mem::take(subscription_diff);
        // Replace with an invalid empty value
        let output = std::mem::replace(
            output,
            AsyncOutput::Suspended {
                suspend: None,
                barrier: None,
            },
        );
        use AsyncOutput::*;
        Some(CancelAsync {
            lane_pos: work_context.lane_pos,
            spawned_consumers: None,
            subscription_diff,
            new_children: match output {
                Completed(results) => Some(results.children),
                _ => None,
            },
        })
    }

    pub(in super::super) fn setup_unmount_async_work_mainline(
        mainline: &mut Mainline<E>,
    ) -> Option<CancelAsync<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>>
    {
        // We must take out the entire async queue during unmount, to make sure the commit barrier in backqueue is actually dropped
        // We cannot rely on the ref-counting `Drop::drop` behavior since there can be easily a leak.
        let (current, backqueue) =
            std::mem::replace(&mut mainline.async_queue, AsyncWorkQueue::new_empty())
                .current_and_backqueue();

        drop(backqueue); // To show that we really dropped the backqueue

        let current = current?;
        let AsyncQueueCurrentEntry {
            widget: _,
            work_context,
            stash:
                AsyncStash {
                    handle,
                    subscription_diff,
                    spawned_consumers,
                    output,
                },
        } = current;
        handle.abort();
        Some(CancelAsync {
            lane_pos: work_context.lane_pos,
            spawned_consumers: spawned_consumers,
            subscription_diff,
            new_children: match output {
                AsyncOutput::Completed(results) => Some(results.children),
                _ => None,
            },
        })
    }

    pub(in super::super) fn execute_unmount_async_work<'batch>(
        self: &Arc<Self>,
        cancel: CancelAsync<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) {
        let CancelAsync {
            lane_pos,
            spawned_consumers,
            subscription_diff,
            new_children,
        } = cancel;

        // Because we are going to unmount, there is no point to maintain consumer root lane marking consistency
        drop(spawned_consumers);

        // if <E as Element>::Impl::PROVIDE_ELEMENT {
        //     if let Some(spawned_consumers) = spawned_consumers {
        //         self.context.unreserve_write_async(lane_pos);
        //         // We choose relaxed lane marking without unmarking
        //         for write_affected_consumer in spawned_consumers {
        //             todo!();
        //             // let deactivated = write_affected_consumer
        //             //     .upgrade()
        //             //     .expect("Readers should be alive")
        //             //     .dec_secondary_root(lane_pos);
        //             // if deactivated {
        //             //     todo!("Record deactivated secondary root")
        //             // }
        //         }
        //     }
        // } else {
        //     debug_assert!(
        //         spawned_consumers.is_none(),
        //         "An Element without declaring provider should not reserve a write"
        //     )
        // }
        for reserved in subscription_diff.reserve {
            reserved.unreserve_read(&(Arc::downgrade(self) as _), lane_pos)
        }

        if let Some(new_children) = new_children {
            new_children
                .into_iter()
                .for_each(|child| scope.spawn(|scope| child.unmount(scope, lane_scheduler)));
            // new_children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
            //     child.unmount(scope, lane_scheduler)
            // })
        }
    }
}

pub(crate) mod cancel_private {
    use super::*;

    pub trait AnyElementNodeAsyncCancelExt {
        fn remove_async_work_in_subtree(
            self: Arc<Self>,
            lane_pos: LanePos,
            // modifications: Asc<TreeModifications>,
        );

        fn remove_async_work_and_lane_in_subtree(
            self: Arc<Self>,
            lane_pos: LanePos,
            // modifications: Asc<TreeModifications>,
        );

        fn purge_async_work_in_subtree(self: Arc<Self>, lane_pos: LanePos);
    }

    impl<E: FullElement> AnyElementNodeAsyncCancelExt for ElementNode<E> {
        fn remove_async_work_in_subtree(self: Arc<Self>, lane_pos: LanePos) {
            Self::remove_async_work_in_subtree(&self, lane_pos)
        }

        fn remove_async_work_and_lane_in_subtree(self: Arc<Self>, lane_pos: LanePos) {
            Self::remove_async_work_and_lane_in_subtree(&self, lane_pos)
        }

        fn purge_async_work_in_subtree(self: Arc<Self>, lane_pos: LanePos) {
            Self::purge_async_work_in_subtree(&self, lane_pos)
        }
    }
}
