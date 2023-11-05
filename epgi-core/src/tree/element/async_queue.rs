use crate::{
    foundation::{Asc, InlinableUsizeVec, TryResult, TryResult::*, VecPushLastExt},
    scheduler::LanePos,
    sync::CommitBarrier,
    tree::{
        ArcElementContextNode, BuildResults, BuildSuspendResults, Element, Work, WorkContext,
        WorkHandle,
    },
};

use super::{ArcChildElementNode, ContainerOf};

pub(crate) struct AsyncWorkQueue<E: Element> {
    pub(crate) inner: Option<Box<AsyncWorkQueueInner<E>>>,
}

pub(crate) struct AsyncWorkQueueInner<E: Element> {
    pub(crate) current: Option<AsyncQueueCurrentEntry<E>>,
    pub(crate) backqueue: Vec<AsyncQueueBackqueueEntry<E::ArcWidget>>,
}

pub(crate) struct AsyncQueueCurrentEntry<E: Element> {
    pub(crate) work: Work<E::ArcWidget>,
    pub(crate) stash: AsyncStash<E>,
}

pub(crate) struct AsyncInflating<E: Element> {
    pub(crate) work_context: Asc<WorkContext>,
    pub(crate) stash: AsyncStash<E>,
}

pub(crate) struct AsyncStash<E: Element> {
    /// This handle can be used to:
    /// 1. Prevent further write-backs
    /// 2. Prevent further wake calls.
    /// 3. Prevent spawning staled child work.
    pub(crate) handle: WorkHandle,
    pub(crate) subscription_diff: SubscriptionDiff,
    pub(crate) reserved_provider_write: bool,
    pub(crate) output: AsyncOutput<E>,
}

pub(crate) struct AsyncQueueBackqueueEntry<ArcWidget> {
    pub(crate) work: Work<ArcWidget>,
    pub(crate) barrier: CommitBarrier,
}

impl<E> AsyncWorkQueue<E>
where
    E: Element,
{
    pub(crate) fn new_empty() -> Self {
        Self { inner: None }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.inner.is_none()
    }

    pub(crate) fn is_free(&self) -> bool {
        match &self.inner {
            None => true,
            Some(inner) => inner.current.is_none(),
        }
    }

    // fn get_stash_mut(&mut self, lane_pos: LanePos) -> Option<&mut AsyncStash<E>> {
    //     self.inner.as_mut().and_then(|inner| {
    //         inner.current.as_mut().and_then(|current| {
    //             (current.work.context.lane_pos == lane_pos).then_some(&mut current.1)
    //         })
    //     })
    // }

    pub(crate) fn current(&self) -> Option<&AsyncQueueCurrentEntry<E>> {
        self.inner.as_ref().and_then(|inner| inner.current.as_ref())
    }
    pub(crate) fn current_mut(&mut self) -> Option<&mut AsyncQueueCurrentEntry<E>> {
        self.inner.as_mut().and_then(|inner| inner.current.as_mut())
    }

    pub(crate) fn remove_current(&mut self) -> Option<AsyncQueueCurrentEntry<E>> {
        self.inner
            .as_mut()
            .and_then(|inner| (&mut inner.current).take()) // rust-analyzer#14933
    }

    pub(crate) fn push_backqueue(&mut self, work: Work<E::ArcWidget>, barrier: CommitBarrier) {
        let inner = self.get_inner_or_create();
        inner
            .backqueue
            .push(AsyncQueueBackqueueEntry { work, barrier });
    }

    pub(crate) fn backqueue_mut(
        &mut self,
    ) -> Option<&mut Vec<AsyncQueueBackqueueEntry<E::ArcWidget>>> {
        self.inner.as_mut().map(|inner| &mut inner.backqueue)
    }

    pub(crate) fn current_and_backqueue_mut(
        &mut self,
    ) -> (
        Option<&mut AsyncQueueCurrentEntry<E>>,
        Option<&mut Vec<AsyncQueueBackqueueEntry<E::ArcWidget>>>,
    ) {
        let Some(inner) = self.inner.as_mut() else {
            return (None, None);
        };
        (inner.current.as_mut(), Some(&mut inner.backqueue))
    }

    fn contains(&self, lane_pos: LanePos) -> bool {
        if let Some(inner) = &self.inner {
            if let Some(current) = &inner.current {
                if current.work.context.lane_pos == lane_pos {
                    return true;
                }
            }
            if inner
                .backqueue
                .iter()
                .find(|entry| entry.work.context.lane_pos == lane_pos)
                .is_some()
            {
                return true;
            }
        }
        return false;
    }

    pub(super) fn backqueue_current_if<
        F: FnOnce(&AsyncQueueCurrentEntry<E>) -> Option<CommitBarrier>,
    >(
        &mut self,
        predicate: F,
    ) -> Option<AsyncQueueCurrentEntry<E>> {
        if let Some(inner) = &mut self.inner {
            if let Some(current) = &mut inner.current {
                if let Some(barrier) = predicate(current) {
                    let taken = (&mut inner.current).take().expect("Impossible to fail"); // rust-analyzer#14933
                    let backqueued_entry = inner.backqueue.push_last(AsyncQueueBackqueueEntry {
                        work: taken.work.clone(),
                        barrier,
                    });
                    return Some(taken);
                }
            }
        }
        return None;
    }

    pub(super) fn backqueue_current<F: FnOnce(&AsyncQueueCurrentEntry<E>) -> CommitBarrier>(
        &mut self,
        get_commit_barrier: F,
    ) -> Option<AsyncQueueCurrentEntry<E>> {
        if let Some(inner) = &mut self.inner {
            if let Some(taken) = (&mut inner.current).take() {
                // rust-analyzer#14933
                let barrier = get_commit_barrier(&taken);
                let backqueued_entry = inner.backqueue.push_last(AsyncQueueBackqueueEntry {
                    work: taken.work.clone(),
                    barrier,
                });
                return Some(taken);
            }
        }
        return None;
    }

    pub(crate) fn try_push_front(
        &mut self,
        work: &Work<E::ArcWidget>,
        barrier: &CommitBarrier,
        subscription_diff: SubscriptionDiff,
        reserved_provider_write: bool,
    ) -> TryResult<WorkHandle, ()> {
        let inner = self.get_inner_or_create();

        if let Some(current) = &inner.current {
            if current.work.context.batch.priority <= work.context.batch.priority {
                Yielded(())
            } else {
                Blocked(())
            }
        } else {
            let output = AsyncOutput::Uninitiated {
                barrier: barrier.clone(),
            };
            let handle = WorkHandle::new();
            self.inner = Some(Box::new(AsyncWorkQueueInner {
                current: Some(AsyncQueueCurrentEntry {
                    work: work.clone(),
                    stash: AsyncStash {
                        handle: handle.clone(),
                        subscription_diff,
                        reserved_provider_write,
                        output,
                    },
                }),
                backqueue: Default::default(),
            }));
            Success(handle)
        }
    }

    fn get_inner_or_create(&mut self) -> &mut Box<AsyncWorkQueueInner<E>> {
        self.inner.get_or_insert(Box::new(AsyncWorkQueueInner {
            current: None,
            backqueue: Default::default(),
        }))
    }

    // Cancels given lane in this queue. If the current active work is cancelled, return the children it has spawned.
    // Return error if the given lane was not found.
    pub(crate) fn try_remove(&mut self, lane_pos: LanePos) -> AsyncDequeueResult<E> {
        let Some(inner) = &mut self.inner else {
            return AsyncDequeueResult::NotFound;
        };
        if let Some(entry) = &inner.current {
            if entry.work.context.lane_pos == lane_pos {
                let current = (&mut inner.current).take().expect("Impossible to fail"); // rust-analyzer#14933
                return AsyncDequeueResult::FoundCurrent(current);
            }
        }
        if let Some(index) = inner
            .backqueue
            .iter()
            .position(|entry| entry.work.context.lane_pos == lane_pos)
        {
            let result = inner.backqueue.swap_remove(index);
            if inner.current.is_none() && inner.backqueue.is_empty() {
                self.inner = None
            }
            return AsyncDequeueResult::FoundBackqueue(result);
        }
        if inner.current.is_none() && inner.backqueue.is_empty() {
            self.inner = None
        }
        return AsyncDequeueResult::NotFound;
    }
}

pub(crate) enum AsyncDequeueResult<E: Element> {
    FoundCurrent(AsyncQueueCurrentEntry<E>),
    FoundBackqueue(AsyncQueueBackqueueEntry<E::ArcWidget>),
    NotFound,
}

impl<E> Default for AsyncWorkQueue<E>
where
    E: Element,
{
    fn default() -> Self {
        Self { inner: None }
    }
}

pub(crate) enum AsyncOutput<E: Element> {
    Uninitiated {
        barrier: CommitBarrier,
    },
    Suspended {
        /// None means a work from the same lane has taken the results and is currently processing it.
        suspend: Option<BuildSuspendResults>,
        /// None means this async work is allowed to be commited as suspended.
        barrier: Option<CommitBarrier>,
    },
    Completed {
        results: BuildResults<E>,
        children: ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
    },
}

///
/// ```text
///       ┌─New Consumed─┐
///       │     reserve  │
/// ┌─────┼───────────┐  │
/// │     │   register│  │
/// │     │    ┌──────┴──┼────┐
/// │     │    │read-only│    │
/// │     └────┼──────┬──┘    │
/// │          │remove│       │
/// └─Reserved─┼──────┘ remove│
///            │              │
///            └─Old Consumed─┘
/// ```
#[derive(Default)]
pub(crate) struct SubscriptionDiff {
    /// New subscriptions introduced during this build,
    /// but its value is already covered by a ancestor work node from the same lane.
    pub(crate) register: InlinableUsizeVec<ArcElementContextNode>,
    /// New subscriptions introduced during this build,
    /// and its value is not covered by a ancestor work node from the same lane.
    /// Therefore, it has reserved a temporary subscription in the subscribed node.
    /// The subscription needs to be cleared in the event of a commit or a cancellation.
    pub(crate) reserve: InlinableUsizeVec<ArcElementContextNode>,
    pub(crate) remove: InlinableUsizeVec<ArcElementContextNode>,
}

impl SubscriptionDiff {
    fn new_uninflated(subscriptions: InlinableUsizeVec<ArcElementContextNode>) -> Self {
        Self {
            register: subscriptions,
            reserve: Default::default(),
            remove: Default::default(),
        }
    }
}
