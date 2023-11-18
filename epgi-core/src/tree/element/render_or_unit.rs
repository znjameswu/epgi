use crate::{
    foundation::{Arc, ArrayContainer, Never, Protocol},
    nodes::{RenderSuspense, SuspenseElement},
    scheduler::TreeScheduler,
    sync::SubtreeRenderObjectChange,
    tree::{ArcChildRenderObject, LayerOrUnit, Render, RenderObject},
};

use super::{
    ArcChildElementNode, ArcElementContextNode, ChildRenderObjectsUpdateCallback, ContainerOf,
    Element, ElementNode, RenderElement,
};

pub trait RenderOrUnit<E: Element> {
    type ArcRenderObject: Clone + Send + Sync + 'static;
    const RENDER_ELEMENT_FUNCTION_TABLE: RenderElementFunctionTable<E>;

    fn visit_commit(
        element_node: &ElementNode<E>,
        render_object: Option<Self::ArcRenderObject>,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
        self_rebuild_suspended: bool,
        scope: &rayon::Scope<'_>,
        tree_scheduler: &TreeScheduler,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol>;

    fn rebuild_success_commit(
        element: &E,
        widget: &E::ArcWidget,
        shuffle: Option<ChildRenderObjectsUpdateCallback<E>>,
        children: &ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
        render_object: Option<Self::ArcRenderObject>,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
        element_context: &ArcElementContextNode,
        is_new_widget: bool,
    ) -> (
        Option<Self::ArcRenderObject>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    );

    fn rebuild_suspend_commit(
        render_object: Option<Self::ArcRenderObject>,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol>;

    fn inflate_success_commit(
        element: &E,
        widget: &E::ArcWidget,
        element_context: &ArcElementContextNode,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
    ) -> (
        Option<Self::ArcRenderObject>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    );
}

pub enum RenderElementFunctionTable<E: Element> {
    RenderObject {
        into_arc_child_render_object:
            fn(ArcRenderObjectOf<E>) -> ArcChildRenderObject<E::ParentProtocol>,
    },
    None {
        as_child: fn(
            &ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
        ) -> &ArcChildElementNode<E::ParentProtocol>,
        into_subtree_render_object_change: fn(
            ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
        )
            -> SubtreeRenderObjectChange<E::ParentProtocol>,
    },
}

impl<E> RenderElementFunctionTable<E>
where
    E: Element,
{
    pub const fn is_none(&self) -> bool {
        match self {
            RenderElementFunctionTable::RenderObject { .. } => false,
            RenderElementFunctionTable::None { .. } => true,
        }
    }

    pub const fn is_some(&self) -> bool {
        !self.is_none()
    }
}

impl<E, R> RenderOrUnit<E> for R
where
    E: RenderElement<Render = R>,
    R: Render<
        ParentProtocol = E::ParentProtocol,
        ChildProtocol = E::ChildProtocol,
        ChildContainer = E::ChildContainer,
    >,
{
    type ArcRenderObject = Arc<RenderObject<R>>;

    const RENDER_ELEMENT_FUNCTION_TABLE: RenderElementFunctionTable<E> =
        RenderElementFunctionTable::RenderObject {
            into_arc_child_render_object: |x| x,
        };

    #[inline(always)]
    fn visit_commit(
        element_node: &ElementNode<E>,
        render_object: Option<Self::ArcRenderObject>,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
        self_rebuild_suspended: bool,
        _scope: &rayon::Scope<'_>,
        _tree_scheduler: &TreeScheduler,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        element_node.visit_commit(render_object, render_object_changes, self_rebuild_suspended)
    }

    #[inline(always)]
    fn rebuild_success_commit(
        element: &E,
        widget: &E::ArcWidget,
        shuffle: Option<ChildRenderObjectsUpdateCallback<E>>,
        children: &ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
        render_object: Option<Self::ArcRenderObject>,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
        element_context: &ArcElementContextNode,
        is_new_widget: bool,
    ) -> (
        Option<Self::ArcRenderObject>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        ElementNode::<E>::rebuild_success_commit(
            element,
            widget,
            shuffle,
            children,
            render_object,
            render_object_changes,
            element_context,
            is_new_widget,
        )
    }

    fn rebuild_suspend_commit(
        render_object: Option<Self::ArcRenderObject>,
    ) -> SubtreeRenderObjectChange<<E as Element>::ParentProtocol> {
        ElementNode::<E>::rebuild_suspend_commit(render_object)
    }

    #[inline(always)]
    fn inflate_success_commit(
        element: &E,
        widget: &E::ArcWidget,
        element_context: &ArcElementContextNode,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
    ) -> (
        Option<Self::ArcRenderObject>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        ElementNode::<E>::inflate_success_commit(
            element,
            widget,
            element_context,
            render_object_changes,
        )
    }
}

impl<E> RenderOrUnit<E> for ()
where
    E: Element<
        ChildProtocol = <E as Element>::ParentProtocol,
        ChildContainer = ArrayContainer<1>,
        RenderOrUnit = Self,
    >,
{
    type ArcRenderObject = Never;

    const RENDER_ELEMENT_FUNCTION_TABLE: RenderElementFunctionTable<E> =
        RenderElementFunctionTable::None {
            as_child: |children| &children[0],
            into_subtree_render_object_change: |x| {
                let [x] = x;
                x
            },
        };

    #[inline(always)]
    fn visit_commit(
        _element_node: &ElementNode<E>,
        _render_object: Option<Never>,
        [change]: [SubtreeRenderObjectChange<E::ChildProtocol>; 1],
        _self_rebuild_suspended: bool,
        _scope: &rayon::Scope<'_>,
        _tree_scheduler: &TreeScheduler,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        return change;
    }

    #[inline(always)]
    fn rebuild_success_commit(
        _element: &E,
        _widget: &E::ArcWidget,
        _shuffle: Option<ChildRenderObjectsUpdateCallback<E>>,
        _children: &[ArcChildElementNode<E::ChildProtocol>; 1],
        _render_object: Option<Never>,
        [change]: [SubtreeRenderObjectChange<E::ChildProtocol>; 1],
        _element_context: &ArcElementContextNode,
        _is_new_widget: bool,
    ) -> (
        Option<Self::ArcRenderObject>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        (None, change)
    }

    #[inline(always)]
    fn rebuild_suspend_commit(
        render_object: Option<Self::ArcRenderObject>,
    ) -> SubtreeRenderObjectChange<<E as Element>::ParentProtocol> {
        SubtreeRenderObjectChange::Suspend
    }

    #[inline(always)]
    fn inflate_success_commit(
        _element: &E,
        _widget: &E::ArcWidget,
        _element_context: &ArcElementContextNode,
        [change]: [SubtreeRenderObjectChange<E::ChildProtocol>; 1],
    ) -> (
        Option<Self::ArcRenderObject>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        (None, change)
    }
}

impl<P> RenderOrUnit<SuspenseElement<P>> for RenderSuspense<P>
where
    P: Protocol,
{
    type ArcRenderObject = Arc<RenderObject<RenderSuspense<P>>>;

    const RENDER_ELEMENT_FUNCTION_TABLE: RenderElementFunctionTable<SuspenseElement<P>> =
        RenderElementFunctionTable::RenderObject {
            into_arc_child_render_object: |x| x,
        };

    fn visit_commit(
        element_node: &ElementNode<SuspenseElement<P>>,
        render_object: Option<Self::ArcRenderObject>,
        render_object_changes: ContainerOf<SuspenseElement<P>, SubtreeRenderObjectChange<P>>,
        self_rebuild_suspended: bool,
        scope: &rayon::Scope<'_>,
        tree_scheduler: &TreeScheduler,
    ) -> SubtreeRenderObjectChange<P> {
        debug_assert!(!self_rebuild_suspended, "Suspense can not suspend itself");
        crate::sync::suspense_element::suspense_visit_commit(
            element_node,
            render_object,
            render_object_changes,
            scope,
            tree_scheduler,
        )
    }

    fn rebuild_success_commit(
        element: &SuspenseElement<P>,
        widget: &<SuspenseElement<P> as Element>::ArcWidget,
        shuffle: Option<ChildRenderObjectsUpdateCallback<SuspenseElement<P>>>,
        children: &ContainerOf<SuspenseElement<P>, ArcChildElementNode<P>>,
        render_object: Option<Self::ArcRenderObject>,
        render_object_changes: ContainerOf<SuspenseElement<P>, SubtreeRenderObjectChange<P>>,
        element_context: &ArcElementContextNode,
        is_new_widget: bool,
    ) -> (Option<Self::ArcRenderObject>, SubtreeRenderObjectChange<P>) {
        todo!()
    }

    fn rebuild_suspend_commit(
        _render_object: Option<Self::ArcRenderObject>,
    ) -> SubtreeRenderObjectChange<P> {
        panic!("Suspense can not suspend on itself")
    }

    fn inflate_success_commit(
        element: &SuspenseElement<P>,
        widget: &<SuspenseElement<P> as Element>::ArcWidget,
        element_context: &ArcElementContextNode,
        render_object_changes: ContainerOf<SuspenseElement<P>, SubtreeRenderObjectChange<P>>,
    ) -> (Option<Self::ArcRenderObject>, SubtreeRenderObjectChange<P>) {
        todo!()
    }
}

pub(crate) type ArcRenderObjectOf<E: Element> =
    <E::RenderOrUnit as RenderOrUnit<E>>::ArcRenderObject;

pub(crate) const fn render_element_function_table_of<E: Element>() -> RenderElementFunctionTable<E>
{
    <E::RenderOrUnit as RenderOrUnit<E>>::RENDER_ELEMENT_FUNCTION_TABLE
}
