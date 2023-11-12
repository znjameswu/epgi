use crate::{
    foundation::{Arc, ArrayContainer, HktContainer, Never, Protocol},
    sync::SubtreeRenderObjectChange,
    tree::{
        render_has_layer, ArcChildRenderObject, Render, RenderContextNode, RenderObject,
        RerenderAction,
    },
};

use super::{
    ArcChildElementNode, ArcElementContextNode, ContainerOf, Element, ElementContextNode,
    RenderElement, SuspenseElementFunctionTable,
};

pub trait RenderOrUnit<E: Element> {
    type ArcRenderObject: Clone + Send + Sync + 'static;
    type RenderChildren: Send + Sync;
    const RENDER_ELEMENT_FUNCTION_TABLE: RenderElementFunctionTable<E>;
    fn with_inner<T>(
        render_object: &Self::ArcRenderObject,
        op: impl FnOnce(
            &mut E::RenderOrUnit,
            &mut ContainerOf<E, ArcChildRenderObject<E::ChildProtocol>>,
            &RenderContextNode,
        ) -> T,
    ) -> T;
}

pub enum RenderElementFunctionTable<E: Element> {
    RenderObject {
        into_arc_child_render_object:
            fn(ArcRenderObjectOf<E>) -> ArcChildRenderObject<E::ParentProtocol>,
        create_render: fn(&E, &E::ArcWidget) -> E::RenderOrUnit,
        update_render: Option<fn(&mut E::RenderOrUnit, &E::ArcWidget) -> RerenderAction>,
        detach_render: Option<fn(&mut E::RenderOrUnit)>,
        suspense: Option<SuspenseElementFunctionTable<E>>,
        has_layer: bool,
        create_render_object: fn(
            E::RenderOrUnit,
            ContainerOf<E, ArcChildRenderObject<E::ChildProtocol>>,
            ArcElementContextNode,
        ) -> ArcRenderObjectOf<E>,
    },
    None {
        as_child: fn(
            &ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
        ) -> &ArcChildElementNode<E::ParentProtocol>,
        into_subtree_update: fn(
            ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
        ) -> SubtreeRenderObjectChange<E::ParentProtocol>,
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

pub enum MaybeSuspendChildRenderObject<P: Protocol> {
    Ready(ArcChildRenderObject<P>),
    ElementSuspended(ArcChildRenderObject<P>),
    Detached,
}

impl<E, R> RenderOrUnit<E> for R
where
    E: RenderElement<Self>,
    R: Render<
        ParentProtocol = E::ParentProtocol,
        ChildProtocol = E::ChildProtocol,
        ChildContainer = E::ChildContainer,
    >,
{
    type ArcRenderObject = Arc<RenderObject<R>>;
    // We assume suspend is relatively rare. Suspended state should not bloat the node size
    type RenderChildren = Box<ContainerOf<E, MaybeSuspendChildRenderObject<E::ChildProtocol>>>;

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
                let render_context = &element_context.nearest_render_context;
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
    // std::mem::size_of::<Result<Never, ()>>() == 0
    type RenderChildren = ();

    const RENDER_ELEMENT_FUNCTION_TABLE: RenderElementFunctionTable<E> =
        RenderElementFunctionTable::None {
            as_child: |children| &children[0],
            into_subtree_update: |x| {
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
}

pub(crate) type ArcRenderObjectOf<E: Element> =
    <E::RenderOrUnit as RenderOrUnit<E>>::ArcRenderObject;

pub(crate) type RenderChildrenOf<E: Element> = <E::RenderOrUnit as RenderOrUnit<E>>::RenderChildren;

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
