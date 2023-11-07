use crate::{
    foundation::Arc,
    tree::{
        render_has_layer, ArcChildRenderObject, Render, RenderContextNode, RenderObject,
        RenderObjectUpdateResult,
    },
};

use super::{
    ArcChildElementNode, ArcElementContextNode, ContainerOf, Element, ElementContextNode,
    RenderElement, SingleChildElement, SuspenseElementFunctionTable,
};

pub trait RenderOrUnit<E: Element> {
    type ArcRenderObject: Clone + Send + Sync + 'static;
    const RENDER_ELEMENT_FUNCTION_TABLE: RenderElementFunctionTable<E>;
    fn with_inner(
        render_object: &Self::ArcRenderObject,
        op: impl FnOnce(
            &mut E::RenderOrUnit,
            &mut ContainerOf<E, ArcChildRenderObject<E::ChildProtocol>>,
            &RenderContextNode,
        ),
    );
}

pub enum RenderElementFunctionTable<E: Element> {
    RenderObject {
        into_arc_child_render_object:
            fn(ArcRenderObjectOf<E>) -> ArcChildRenderObject<E::ParentProtocol>,
        create_render: fn(&E, &E::ArcWidget) -> E::RenderOrUnit,
        update_render: Option<fn(&mut E::RenderOrUnit, &E::ArcWidget) -> RenderObjectUpdateResult>,
        detach_render: Option<fn(&mut E::RenderOrUnit)>,
        suspense: Option<SuspenseElementFunctionTable<E>>,
        has_layer: bool,
        create_render_object: fn(
            E::RenderOrUnit,
            ContainerOf<E, ArcChildRenderObject<E::ChildProtocol>>,
            &ElementContextNode,
        ) -> ArcRenderObjectOf<E>,
    },
    None {
        child: fn(&E) -> &ArcChildElementNode<E::ParentProtocol>,
        into_arc_child_render_object:
            fn(ArcRenderObjectOf<E>) -> ArcChildRenderObject<E::ParentProtocol>,
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
    E: RenderElement<Self>,
    R: Render<ParentProtocol = E::ParentProtocol, ChildProtocol = E::ChildProtocol>,
{
    type ArcRenderObject = Arc<RenderObject<R>>;

    const RENDER_ELEMENT_FUNCTION_TABLE: RenderElementFunctionTable<E> =
        RenderElementFunctionTable::RenderObject {
            into_arc_child_render_object: |x| x,
            create_render: |element, widget, element_context| {
                assert!(
                element_context.has_render,
                concat!(
                    "ElementNodes with RenderObject must be registered in its ElementContextNode. \n",
                    "If this assertion failed, you have encountered a framework bug."
                )
            );
                let render_context = &element_context.nearest_render_context;
                let render = E::try_create_render_object(element, widget)?;
                Some(Arc::new(RenderObject::new(render, element_context.clone())))
            },
            create_render_object: todo!(),
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

    fn with_inner(
        render_object: &Self::ArcRenderObject,
        op: impl FnOnce(&mut <E as Element>::RenderOrUnit, &RenderContextNode),
    ) {
        let mut inner = render_object.inner.lock();
        op(&mut inner.render, &render_object.context)
    }
}

impl<E> RenderOrUnit<E> for ()
where
    E: SingleChildElement<RenderOrUnit = Self>,
{
    type ArcRenderObject = ArcChildRenderObject<E::ParentProtocol>;

    const RENDER_ELEMENT_FUNCTION_TABLE: RenderElementFunctionTable<E> =
        RenderElementFunctionTable::None {
            child: E::child,
            into_arc_child_render_object: |x| x,
        };

    fn with_inner(
        render_object: &Self::ArcRenderObject,
        op: impl FnOnce(&mut <E as Element>::RenderOrUnit, &RenderContextNode),
    ) {
        panic!("You should never unwrap non-RenderElement's render object")
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
