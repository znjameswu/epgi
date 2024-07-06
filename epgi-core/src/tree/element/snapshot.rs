use crate::{
    foundation::{ContainerOf, HktContainer},
    tree::{AsyncInflating, HooksWithCleanups},
};

use super::{
    ArcChildElementNode, ArcSuspendWaker, AsyncWorkQueue, Element, ImplElementNode, SuspendWaker,
};

pub(crate) struct ElementSnapshot<E: Element> {
    pub(crate) widget: E::ArcWidget,
    // pub(super) subtree_suspended: bool,
    pub(crate) inner: ElementSnapshotInner<E>,
    pub(crate) element_lock_held: ElementLockHeldToken,
}

pub(crate) struct ElementLockHeldToken(());

impl<E: Element> ElementSnapshot<E> {
    pub(crate) fn new(widget: E::ArcWidget, inner: ElementSnapshotInner<E>) -> Self {
        Self {
            widget,
            inner,
            element_lock_held: ElementLockHeldToken(()),
        }
    }
}

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

    // pub(crate) fn async_inflating_ref(&self) -> Option<&AsyncInflating<E>> {
    //     match self {
    //         ElementSnapshotInner::AsyncInflating(x) => Some(x),
    //         ElementSnapshotInner::Mainline(_) => None,
    //     }
    // }

    pub(crate) fn async_inflating_mut(&mut self) -> Option<&mut AsyncInflating<E>> {
        match self {
            ElementSnapshotInner::AsyncInflating(x) => Some(x),
            ElementSnapshotInner::Mainline(_) => None,
        }
    }

    pub(crate) fn is_async_inflating(&self) -> bool {
        match self {
            ElementSnapshotInner::AsyncInflating(_) => true,
            ElementSnapshotInner::Mainline(_) => false,
        }
    }
}

pub(crate) struct Mainline<E: Element> {
    pub(crate) state: Option<MainlineState<E, HooksWithCleanups>>,
    pub(crate) async_queue: AsyncWorkQueue<E>,
}

pub(crate) enum MainlineState<E: Element, H> {
    Ready {
        element: E,
        hooks: H,
        children: ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        render_object: <E::Impl as ImplElementNode<E>>::OptionArcRenderObject,
    },
    InflateSuspended {
        suspended_hooks: H,
        waker: ArcSuspendWaker,
    }, // The hooks may be partially initialized
    RebuildSuspended {
        element: E,
        suspended_hooks: H,
        children: ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        waker: ArcSuspendWaker,
    }, // The element is stale. The hook state is valid but may only have partial transparent build effects.
}

impl<E: Element, H> MainlineState<E, H> {
    // pub(crate) fn is_suspended(&self) -> bool {
    //     match self {
    //         MainlineState::Ready { .. } => false,
    //         MainlineState::InflateSuspended { .. } | MainlineState::RebuildSuspended { .. } => true,
    //     }
    // }

    // pub(crate) fn element(self) -> Option<E> {
    //     match self {
    //         MainlineState::InflateSuspended { .. } => None,
    //         MainlineState::Ready { element, .. }
    //         | MainlineState::RebuildSuspended { element, .. } => Some(element),
    //     }
    // }

    // pub(crate) fn element_ref(&self) -> Option<&E> {
    //     match self {
    //         MainlineState::InflateSuspended { .. } => None,
    //         MainlineState::Ready { element, .. }
    //         | MainlineState::RebuildSuspended { element, .. } => Some(element),
    //     }
    // }

    // pub(crate) fn children_ref(
    //     &self,
    // ) -> Option<&ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>> {
    //     match self {
    //         MainlineState::InflateSuspended { .. } => None,
    //         MainlineState::Ready { children, .. }
    //         | MainlineState::RebuildSuspended { children, .. } => Some(children),
    //     }
    // }

    pub(crate) fn children_cloned(
        &self,
    ) -> Option<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>> {
        match self {
            MainlineState::InflateSuspended { .. } => None,
            MainlineState::Ready { children, .. }
            | MainlineState::RebuildSuspended { children, .. } => {
                Some(E::ChildContainer::clone_container(children))
            }
        }
    }

    // pub(crate) fn hooks(self) -> Option<H> {
    //     match self {
    //         MainlineState::InflateSuspended { .. } => None,
    //         MainlineState::Ready { hooks, .. }
    //         | MainlineState::RebuildSuspended {
    //             suspended_hooks: hooks,
    //             ..
    //         } => Some(hooks),
    //     }
    // }

    // pub(crate) fn hooks_ref(&self) -> Option<&H> {
    //     match self {
    //         MainlineState::InflateSuspended { .. } => None,
    //         MainlineState::Ready { hooks, .. }
    //         | MainlineState::RebuildSuspended {
    //             suspended_hooks: hooks,
    //             ..
    //         } => Some(hooks),
    //     }
    // }

    // pub(crate) fn waker(self) -> Option<ArcSuspendWaker> {
    //     match self {
    //         MainlineState::Ready { .. } => None,
    //         MainlineState::InflateSuspended { waker, .. }
    //         | MainlineState::RebuildSuspended { waker, .. } => Some(waker),
    //     }
    // }

    pub(crate) fn waker_ref(&self) -> Option<&SuspendWaker> {
        match self {
            MainlineState::Ready { .. } => None,
            MainlineState::InflateSuspended { waker, .. }
            | MainlineState::RebuildSuspended { waker, .. } => Some(waker),
        }
    }
}
