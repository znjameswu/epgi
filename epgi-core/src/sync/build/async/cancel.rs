/// Basic concepts:
///
/// 1. Backqueue: Means after we remove the current work, we place it inside the backqueue. Otherwise, we just discard it
/// 2. Requeue: Means after we remove the current work, we check the backqueue to fire the highest priority work. It corresponds to the eager batch dispatch strategy
/// 3. Interrupt: Cancel with backqueue but not requeue
use crate::{
    foundation::{Arc, Container, ContainerOf},
    r#async::AsyncReconcile,
    scheduler::{get_current_scheduler, LaneMask, LanePos},
    sync::LaneScheduler,
    tree::{
        ArcChildElementNode, AsyncDequeueResult, AsyncInflating, AsyncOutput,
        AsyncQueueCurrentEntry, AsyncStash, AsyncWorkQueue, AweakElementContextNode,
        ConsumerWorkSpawnToken, Element, ElementBase, ElementNode, ElementSnapshot,
        ElementSnapshotInner, FullElement, ImplProvide, Mainline, SubscriptionDiff,
    },
};

pub trait AnyElementNodeAsyncCancelExt {
    // Lane purging cannot be done in this function.
    // This function visit the async version of the tree.
    // While lane marking and purging should be done on the mainline version of the tree.
    // The impact of this difference is that by visiting the async version of the tree, we may miss some to-be-unmounted mainline nodes
    // If we visit *both* the mainline children and the async children, then we are creating an exponential visitt explosion by potentially visit one node twice.
    // (Though the exponential explosion is avoided by the atomic flag checking)
    fn cancel_async_work(self: Arc<Self>, lane_pos: LanePos, requeue: bool);
}

impl<E: FullElement> AnyElementNodeAsyncCancelExt for ElementNode<E> {
    fn cancel_async_work(self: Arc<Self>, lane_pos: LanePos, requeue: bool) {
        self.cancel_async_work_impl(lane_pos, requeue)
    }
}

pub(in crate::sync) struct AsyncCancel<I> {
    pub(super) lane_pos: LanePos,
    pub(super) spawned_consumers: Option<Vec<(AweakElementContextNode, ConsumerWorkSpawnToken)>>,
    pub(super) subscription_diff: SubscriptionDiff,
    pub(super) async_children: Option<I>,
}

struct AsyncCancelAndRestart<E: ElementBase> {
    cancel: Result<
        AsyncCancel<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
        Option<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
    >,
    start: Option<AsyncReconcile<E>>,
}

impl<E: FullElement> ElementNode<E> {
    pub fn cancel_async_work_impl(
        self: Arc<Self>,
        lane_pos: LanePos,
        requeue: bool,
        // purge_lane_mark: bool,
    ) {
        let no_mailbox_update = !self.context.mailbox_lanes().contains(lane_pos);
        let no_consumer_root = !self.context.consumer_lanes().contains(lane_pos);
        let no_descendant_lanes = !self.context.descendant_lanes().contains(lane_pos);

        if no_mailbox_update && no_consumer_root && no_descendant_lanes {
            return;
        }
        let remove = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_reborrow = &mut *snapshot;
            self.setup_cancel_async_work(snapshot_reborrow, lane_pos, requeue)
        };

        let AsyncCancelAndRestart { cancel, start } = remove;

        match cancel {
            Ok(cancel) => {
                let AsyncCancel {
                    lane_pos,
                    spawned_consumers,
                    subscription_diff,
                    async_children,
                } = cancel;

                if let Some(new_children) = async_children {
                    new_children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                        child.cancel_async_work(lane_pos, requeue)
                    })
                }

                // Reverse-order
                self.execute_cancel_async_work_local(
                    spawned_consumers,
                    subscription_diff,
                    lane_pos,
                );
            }
            Err(Some(children)) => children
                .par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                    child.cancel_async_work(lane_pos, requeue)
                }),
            Err(None) => {}
        }

        // if purge_lane_mark {
        //     self.context.purge_lane(lane_pos);
        // }
        debug_assert!(
            self.context.provider_object.is_none()
                || self
                    .context
                    .provider_object
                    .as_ref()
                    .is_some_and(|provider| !provider
                        .contains_reservation_from_lanes(LaneMask::new_single(lane_pos))),
            "The cancel left residues inside this provider object"
        );

        if let Some(start) = start {
            self.execute_reconcile_node_async_detached(start);
        }
    }

    fn setup_cancel_async_work(
        self: &Arc<Self>,
        snapshot: &mut ElementSnapshot<E>,
        lane_pos: LanePos,
        requeue: bool,
    ) -> AsyncCancelAndRestart<E> {
        match &mut snapshot.inner {
            ElementSnapshotInner::AsyncInflating(async_inflating) => {
                let cancel =
                    Self::setup_cancel_async_work_async_inflating(async_inflating, lane_pos);
                AsyncCancelAndRestart {
                    cancel: Ok(cancel),
                    start: None,
                }
            }
            ElementSnapshotInner::Mainline(mainline) => {
                let cancel = Self::setup_cancel_async_work_mainline(mainline, lane_pos, None);

                let start = if requeue {
                    self.setup_execute_backqueue(
                        mainline,
                        &snapshot.widget,
                        &snapshot.element_lock_held,
                    )
                } else {
                    None
                };
                AsyncCancelAndRestart { cancel, start }
            }
        }
    }

    pub(in crate::sync) fn setup_interrupt_async_work(
        mainline: &mut Mainline<E>,
        lane_pos: LanePos,
        lane_scheduler: &LaneScheduler,
    ) -> Result<
        AsyncCancel<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
        Option<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
    > {
        Self::setup_cancel_async_work_mainline(mainline, lane_pos, Some(lane_scheduler))
    }

    pub(in crate::sync) fn setup_cancel_async_work_async_inflating(
        async_inflating: &mut AsyncInflating<E>,
        lane_pos: LanePos,
    ) -> AsyncCancel<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>> {
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
        let output = std::mem::replace(output, AsyncOutput::Gone);
        use AsyncOutput::*;
        AsyncCancel {
            lane_pos,
            spawned_consumers: None,
            subscription_diff,
            async_children: match output {
                Completed(results) => Some(results.children),
                _ => None,
            },
        }
    }

    fn setup_cancel_async_work_mainline(
        mainline: &mut Mainline<E>,
        lane_pos: LanePos,
        backqueue_with_lane_scehduler: Option<&LaneScheduler>,
    ) -> Result<
        AsyncCancel<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
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
                if let Some(lane_scheduler) = backqueue_with_lane_scehduler {
                    async_queue.push_backqueue(
                        widget,
                        work_context,
                        lane_scheduler
                            .get_commit_barrier_for(lane_pos)
                            .expect("CommitBarrier should exist"),
                    );
                }
                Ok(AsyncCancel {
                    lane_pos,
                    spawned_consumers,
                    subscription_diff,
                    async_children: match output {
                        Completed(results) => Some(results.children),
                        _ => None,
                    },
                })
            }
            NotFound => Err(state
                .as_ref()
                .expect("A mainline tree walk should not encounter another sync work.")
                .children_cloned()),
            // Should we return None or the mainline children?
            // If we can guarantee no active work under a backqueued work, then we can return None
            FoundBackqueue(_) => Err(None),
        }
    }

    pub(in crate::sync) fn execute_cancel_async_work(
        self: &Arc<Self>,
        cancel: AsyncCancel<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
        requeue: bool,
    ) {
        let AsyncCancel {
            lane_pos,
            spawned_consumers,
            subscription_diff,
            async_children,
        } = cancel;

        if let Some(async_children) = async_children {
            async_children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                child.cancel_async_work(lane_pos, requeue)
            })
        }

        self.execute_cancel_async_work_local(spawned_consumers, subscription_diff, lane_pos);
        // Do not requeue anything here
    }

    fn execute_cancel_async_work_local(
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

        // Since we are holding sync scheduler lock, can we use scheduler ordering instead of element lock to guarantee side effect reversal?
        // We need to prove reservation the same lane either happens-before us acquired the element lock, or after the sync walk is completed.
        for reserved in subscription_diff.reserve {
            reserved.unreserve_read(&(Arc::downgrade(self) as _), lane_pos)
        }
    }
}

impl<E: FullElement> ElementNode<E> {
    pub(in super::super) fn setup_unmount_async_work_mainline(
        mainline: &mut Mainline<E>,
    ) -> Option<AsyncCancel<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>>
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
        Some(AsyncCancel {
            lane_pos: work_context.lane_pos,
            spawned_consumers,
            subscription_diff,
            async_children: match output {
                AsyncOutput::Completed(results) => Some(results.children),
                _ => None,
            },
        })
    }

    #[inline]
    pub(in super::super) fn execute_unmount_async_work<'batch>(
        self: &Arc<Self>,
        cancel: AsyncCancel<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>,
        scope: &rayon::Scope<'batch>,
        single_child_optimization: bool,
    ) {
        let AsyncCancel {
            lane_pos,
            spawned_consumers,
            subscription_diff,
            async_children,
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

        if let Some(async_children) = async_children {
            if single_child_optimization {
                async_children
                    .into_iter()
                    .for_each(|child| scope.spawn(|scope| child.unmount_if_async_inflating(scope)));
            } else {
                let mut it = async_children.into_iter();
                // Single child optimization
                if it.len() == 1 {
                    let child = it.next().unwrap();
                    child.unmount_if_async_inflating(scope)
                } else {
                    it.for_each(|child| scope.spawn(|s| child.unmount_if_async_inflating(s)))
                }
            }
        }
    }
}
