use std::sync::atomic::{AtomicBool, Ordering::*};

use futures::task::ArcWake;

use crate::{
    foundation::{Asc, InlinableDwsizeVec, InlinableUsizeVec},
    scheduler::{BatchId, LanePos},
    tree::{AsyncInflating, Hook, HookContext},
};

use super::{
    ArcChildElementNode, ArcRenderObjectOf, AsyncWorkQueue, AweakAnyElementNode, ContainerOf,
    Element,
};

pub(crate) enum ElementSnapshotInner<E: Element> {
    /// Helper state for sync inflate and rebuild. This state exists solely due to the lack of Arc::new_cyclic_async and mem::replace_with
    // Uninitialized,
    AsyncInflating(AsyncInflating<E>),
    Mainline(Mainline<E>),
}

impl<E: Element> ElementSnapshotInner<E> {
    pub(crate) fn mainline_ref(&self) -> Option<&Mainline<E>> {
        match self {
            ElementSnapshotInner::Mainline(mainline) => Some(mainline),
            ElementSnapshotInner::AsyncInflating(_) => None,
        }
    }

    pub(crate) fn mainline_mut(&mut self) -> Option<&mut Mainline<E>> {
        match self {
            ElementSnapshotInner::Mainline(mainline) => Some(mainline),
            ElementSnapshotInner::AsyncInflating(_) => None,
        }
    }

    pub(crate) fn async_inflating_ref(&self) -> Option<&AsyncInflating<E>> {
        match self {
            ElementSnapshotInner::AsyncInflating(x) => Some(x),
            ElementSnapshotInner::Mainline(_) => None,
        }
    }

    pub(crate) fn async_inflating_mut(&mut self) -> Option<&mut AsyncInflating<E>> {
        match self {
            ElementSnapshotInner::AsyncInflating(x) => Some(x),
            ElementSnapshotInner::Mainline(_) => None,
        }
    }
}

pub(crate) struct Mainline<E: Element> {
    pub(crate) state: Option<MainlineState<E>>,
    pub(crate) async_queue: AsyncWorkQueue<E>,
}

pub(crate) enum MainlineState<E: Element> {
    Ready {
        element: E,
        hooks: Hooks,
        children: ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
        render_object: Option<ArcRenderObjectOf<E>>,
    },
    InflateSuspended {
        suspended_hooks: Hooks,
        waker: SuspendWaker,
    }, // The hooks may be partially initialized
    RebuildSuspended {
        element: E,
        children: ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
        suspended_hooks: Hooks,
        waker: SuspendWaker,
    }, // The element is stale. The hook state is valid but may only have partial transparent build effects.
}

#[derive(Clone, Default)]
pub struct Hooks {
    pub array_hooks: Vec<Box<dyn Hook>>,
}

impl<E: Element> MainlineState<E> {
    pub(crate) fn is_suspended(&self) -> bool {
        match self {
            MainlineState::Ready { .. } => false,
            MainlineState::InflateSuspended { .. } | MainlineState::RebuildSuspended { .. } => true,
        }
    }

    pub(crate) fn element(self) -> Option<E> {
        match self {
            MainlineState::InflateSuspended { .. } => None,
            MainlineState::Ready { element, .. }
            | MainlineState::RebuildSuspended { element, .. } => Some(element),
        }
    }

    pub(crate) fn element_ref(&self) -> Option<&E> {
        match self {
            MainlineState::InflateSuspended { .. } => None,
            MainlineState::Ready { element, .. }
            | MainlineState::RebuildSuspended { element, .. } => Some(element),
        }
    }

    pub(crate) fn children_ref(
        &self,
    ) -> Option<&ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>> {
        match self {
            MainlineState::InflateSuspended { .. } => None,
            MainlineState::Ready { children, .. }
            | MainlineState::RebuildSuspended { children, .. } => Some(children),
        }
    }

    pub(crate) fn hooks(self) -> Option<Hooks> {
        match self {
            MainlineState::InflateSuspended { .. } => None,
            MainlineState::Ready { hooks, .. }
            | MainlineState::RebuildSuspended {
                suspended_hooks: hooks,
                ..
            } => Some(hooks),
        }
    }

    pub(crate) fn hooks_ref(&self) -> Option<&Hooks> {
        match self {
            MainlineState::InflateSuspended { .. } => None,
            MainlineState::Ready { hooks, .. }
            | MainlineState::RebuildSuspended {
                suspended_hooks: hooks,
                ..
            } => Some(hooks),
        }
    }

    pub(crate) fn waker(self) -> Option<SuspendWaker> {
        match self {
            MainlineState::Ready { .. } => None,
            MainlineState::InflateSuspended { waker, .. }
            | MainlineState::RebuildSuspended { waker, .. } => Some(waker),
        }
    }

    pub(crate) fn waker_ref(&self) -> Option<&SuspendWaker> {
        match self {
            MainlineState::Ready { .. } => None,
            MainlineState::InflateSuspended { waker, .. }
            | MainlineState::RebuildSuspended { waker, .. } => Some(waker),
        }
    }
}

pub struct BuildResults<E: Element> {
    hooks: Hooks,
    element: E,
    nodes_needing_unmount: InlinableUsizeVec<ArcChildElementNode<E::ChildProtocol>>,
    effects: Vec<u32>,
    performed_inflate: bool,
}

impl<E> BuildResults<E>
where
    E: Element,
{
    pub fn from_pieces(
        hooks_iter: HookContext,
        element: E,
        nodes_needing_unmount: InlinableDwsizeVec<ArcChildElementNode<E::ChildProtocol>>,
    ) -> Self {
        todo!()
    }
}

// struct BuildSuspendHandle {
// }

pub(crate) struct BuildSuspendResults {
    // widget: E::ArcWidget,
    hooks: Hooks,
}

impl BuildSuspendResults {
    pub fn new(hooks_iter: HookContext) -> Self {
        todo!()
    }
}

#[derive(Clone)]
pub(crate) struct SuspendWaker {
    inner: std::sync::Arc<SuspendWakerInner>,
}

impl SuspendWaker {
    pub(crate) fn abort(&self) {
        self.inner.aborted.store(true, Relaxed);
    }

    fn new_sync(node: AweakAnyElementNode) -> Self {
        Self {
            inner: Asc::new(SuspendWakerInner {
                aborted: AtomicBool::new(false),
                node,
                is_async_suspense: None,
            }),
        }
    }

    fn new_async(node: AweakAnyElementNode, lane_pos: LanePos, batch_id: BatchId) -> Self {
        Self {
            inner: Asc::new(SuspendWakerInner {
                aborted: AtomicBool::new(false),
                node,
                is_async_suspense: Some((lane_pos, batch_id)),
            }),
        }
    }
}

impl ArcWake for SuspendWakerInner {
    fn wake_by_ref(arc_self: &std::sync::Arc<Self>) {
        todo!()
    }
}

struct SuspendWakerInner {
    aborted: AtomicBool,
    // poll_permit: AtomicBool, // This has no use.
    node: AweakAnyElementNode,
    // lane: LanePos,
    is_async_suspense: Option<(LanePos, BatchId)>,
}
