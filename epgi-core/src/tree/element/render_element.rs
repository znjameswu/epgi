use crate::{
    foundation::{Arc, ArrayContainer, HktContainer, Never, Protocol},
    sync::SubtreeRenderObjectChange,
    tree::{
        render_has_layer, ArcChildRenderObject, Render, RenderAction, RenderContextNode,
        RenderObject,
    },
};

use super::{
    ArcChildElementNode, ArcElementContextNode, ChildRenderObjectsUpdateCallback, ContainerOf,
    Element, ElementContextNode, ElementNode, RenderElement, SuspenseElementFunctionTable,
};

pub trait RenderOrUnit<E: Element> {
    type ArcRenderObject: Clone + Send + Sync + 'static;
    const RENDER_ELEMENT_FUNCTION_TABLE: RenderElementFunctionTable<E>;
    fn with_inner<T>(
        render_object: &Self::ArcRenderObject,
        op: impl FnOnce(
            &mut E::RenderOrUnit,
            &mut ContainerOf<E, ArcChildRenderObject<E::ChildProtocol>>,
            &RenderContextNode,
        ) -> T,
    ) -> T;

    fn visit_commit(
        element_node: &ElementNode<E>,
        render_object: Option<Self::ArcRenderObject>,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
        self_rebuild_suspended: bool,
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

    fn inflate_success_commit(
        element: &E,
        widget: &E::ArcWidget,
        element_context: &ArcElementContextNode,
        render_object_changes: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
    ) -> (
        Option<Self::ArcRenderObject>,
        SubtreeRenderObjectChange<<E as Element>::ParentProtocol>,
    );
}

pub enum RenderElementFunctionTable<E: Element> {
    RenderObject {
        into_arc_child_render_object:
            fn(ArcRenderObjectOf<E>) -> ArcChildRenderObject<E::ParentProtocol>,
        create_render: fn(&E, &E::ArcWidget) -> E::RenderOrUnit,
        update_render: Option<fn(&mut E::RenderOrUnit, &E::ArcWidget) -> RenderAction>,
        detach_render: Option<fn(&mut E::RenderOrUnit)>,
        suspense: Option<SuspenseElementFunctionTable<E>>,
        has_layer: bool,
        create_render_object: fn(
            E::RenderOrUnit,
            ContainerOf<E, ArcChildRenderObject<E::ChildProtocol>>,
            ArcElementContextNode,
        ) -> ArcRenderObjectOf<E>,
        mark_render_action: fn(
            &ArcRenderObjectOf<E>,
            self_render_action: RenderAction,
            subtree_render_action: RenderAction,
        ),
        boundary_type: fn(&ArcRenderObjectOf<E>) -> RenderAction,
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

// pub enum MaybeSuspendChildRenderObject<P: Protocol> {
//     Ready(ArcChildRenderObject<P>),
//     ElementSuspended(ArcChildRenderObject<P>),
//     Detached,
// }

// impl<P> MaybeSuspendChildRenderObject<P>
// where
//     P: Protocol,
// {
//     pub fn is_ready(&self) -> bool {
//         matches!(self, MaybeSuspendChildRenderObject::Ready(_))
//     }

//     #[inline(always)]
//     pub(crate) fn merge_with(self, change: SubtreeRenderObjectChange<P>) -> Self {
//         use MaybeSuspendChildRenderObject::*;
//         use SubtreeRenderObjectChange::*;
//         match (self, change) {
//             (child, Keep { .. }) => child,
//             (_, New(child)) => Ready(child),
//             (_, SuspendNew(child)) => ElementSuspended(child),
//             (_, Detach) => Detached,
//             (Ready(child) | ElementSuspended(child), SuspendKeep) => ElementSuspended(child),
//             (Detached, SuspendKeep) => Detached,
//         }
//     }
// }

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
            create_render: E::create_render,
            create_render_object: |render, children, element_context| {
                assert!(
                    element_context.has_render,
                    concat!(
                        "ElementNodes with RenderObject must be registered in its ElementContextNode. \n",
                        "If this assertion failed, you have encountered a framework bug."
                    )
                );
                Arc::new(RenderObject::new(render, children, element_context.clone()))
            },
            update_render: if E::NOOP_UPDATE_RENDER_OBJECT {
                None
            } else {
                Some(E::update_render)
            },
            detach_render: if R::NOOP_DETACH {
                None
            } else {
                Some(R::detach)
            },
            suspense: E::SUSPENSE_ELEMENT_FUNCTION_TABLE,
            has_layer: render_has_layer::<R>(),
            mark_render_action: |render_object, self_render_action, child_render_action| todo!(),
            boundary_type: |render_object| todo!(),
        };

    fn with_inner<T>(
        render_object: &Self::ArcRenderObject,
        op: impl FnOnce(
            &mut E::RenderOrUnit,
            &mut ContainerOf<E, ArcChildRenderObject<E::ChildProtocol>>,
            &RenderContextNode,
        ) -> T,
    ) -> T {
        let mut inner = render_object.inner.lock();
        let inner = &mut *inner;
        op(
            &mut inner.render,
            &mut inner.children,
            &render_object.context,
        )
    }

    #[inline(always)]
    fn visit_commit(
        element_node: &ElementNode<E>,
        render_object: Option<Self::ArcRenderObject>,
        render_object_changes: ContainerOf<
            E,
            SubtreeRenderObjectChange<<E as Element>::ChildProtocol>,
        >,
        self_rebuild_suspended: bool,
    ) -> SubtreeRenderObjectChange<<E as Element>::ParentProtocol> {
        todo!()
    }

    fn rebuild_success_commit(
        element: &E,
        widget: &<E as Element>::ArcWidget,
        shuffle: Option<ChildRenderObjectsUpdateCallback<E>>,
        children: &ContainerOf<E, ArcChildElementNode<<E as Element>::ChildProtocol>>,
        render_object: Option<Self::ArcRenderObject>,
        render_object_changes: ContainerOf<
            E,
            SubtreeRenderObjectChange<<E as Element>::ChildProtocol>,
        >,
        element_context: &ArcElementContextNode,
        is_new_widget: bool,
    ) -> (
        Option<Self::ArcRenderObject>,
        SubtreeRenderObjectChange<<E as Element>::ParentProtocol>,
    ) {
        todo!()
    }

    fn inflate_success_commit(
        element: &E,
        widget: &<E as Element>::ArcWidget,
        element_context: &ArcElementContextNode,
        render_object_changes: ContainerOf<
            E,
            SubtreeRenderObjectChange<<E as Element>::ChildProtocol>,
        >,
    ) -> (
        Option<Self::ArcRenderObject>,
        SubtreeRenderObjectChange<<E as Element>::ParentProtocol>,
    ) {
        todo!()
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

    fn with_inner<T>(
        render_object: &Self::ArcRenderObject,
        op: impl FnOnce(
            &mut E::RenderOrUnit,
            &mut ContainerOf<E, ArcChildRenderObject<E::ChildProtocol>>,
            &RenderContextNode,
        ) -> T,
    ) -> T {
        panic!("You should never unwrap non-RenderElement's render object")
    }

    #[inline(always)]
    fn visit_commit(
        element_node: &ElementNode<E>,
        render_object: Option<Self::ArcRenderObject>,
        render_object_changes: ContainerOf<
            E,
            SubtreeRenderObjectChange<<E as Element>::ChildProtocol>,
        >,
        self_rebuild_suspended: bool,
    ) -> SubtreeRenderObjectChange<<E as Element>::ParentProtocol> {
        todo!()
    }

    fn rebuild_success_commit(
        element: &E,
        widget: &<E as Element>::ArcWidget,
        shuffle: Option<ChildRenderObjectsUpdateCallback<E>>,
        children: &ContainerOf<E, ArcChildElementNode<<E as Element>::ChildProtocol>>,
        render_object: Option<Self::ArcRenderObject>,
        render_object_changes: ContainerOf<
            E,
            SubtreeRenderObjectChange<<E as Element>::ChildProtocol>,
        >,
        element_context: &ArcElementContextNode,
        is_new_widget: bool,
    ) -> (
        Option<Self::ArcRenderObject>,
        SubtreeRenderObjectChange<<E as Element>::ParentProtocol>,
    ) {
        todo!()
    }

    fn inflate_success_commit(
        element: &E,
        widget: &<E as Element>::ArcWidget,
        element_context: &ArcElementContextNode,
        render_object_changes: ContainerOf<
            E,
            SubtreeRenderObjectChange<<E as Element>::ChildProtocol>,
        >,
    ) -> (
        Option<Self::ArcRenderObject>,
        SubtreeRenderObjectChange<<E as Element>::ParentProtocol>,
    ) {
        todo!()
    }
}

pub(crate) type ArcRenderObjectOf<E: Element> =
    <E::RenderOrUnit as RenderOrUnit<E>>::ArcRenderObject;

pub(crate) const fn render_element_function_table_of<E: Element>() -> RenderElementFunctionTable<E>
{
    <E::RenderOrUnit as RenderOrUnit<E>>::RENDER_ELEMENT_FUNCTION_TABLE
}

pub(crate) const fn is_non_suspense_render_element<E: Element>() -> bool {
    match render_element_function_table_of::<E>() {
        RenderElementFunctionTable::RenderObject { suspense: None, .. } => true,
        _ => false,
    }
}

pub(crate) const fn is_suspense_element<E: Element>() -> bool {
    match render_element_function_table_of::<E>() {
        RenderElementFunctionTable::RenderObject {
            suspense: Some(_), ..
        } => true,
        _ => false,
    }
}

pub(crate) const fn is_non_render_element<E: Element>() -> bool {
    match render_element_function_table_of::<E>() {
        RenderElementFunctionTable::None { .. } => true,
        _ => false,
    }
}
