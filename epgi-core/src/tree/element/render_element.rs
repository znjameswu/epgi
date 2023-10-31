use crate::{
    foundation::Arc,
    tree::{
        render_has_layer, ArcChildRenderObject, Render, RenderContextNode, RenderObject,
        RenderObjectUpdateResult,
    },
};

use super::{
    ArcChildElementNode, ArcElementContextNode, Element, RenderElement, SingleChildElement,
    SuspenseElementFunctionTable,
};

pub trait RenderOrUnit<E: Element> {
    type ArcRenderObject: Clone + Send + Sync + 'static;
    const RENDER_ELEMENT_FUNCTION_TABLE: RenderElementFunctionTable<E>;
    fn lock_with(
        render_object: &Self::ArcRenderObject,
        op: impl FnOnce(&mut E::RenderOrUnit, &RenderContextNode),
    );
}

pub enum RenderElementFunctionTable<E: Element> {
    RenderObject {
        into_arc_child_render_object:
            fn(ArcRenderObjectOf<E>) -> ArcChildRenderObject<E::ParentProtocol>,
        try_create_render_object:
            fn(&E, &E::ArcWidget, &ArcElementContextNode) -> Option<ArcRenderObjectOf<E>>,
        update_render_object:
            Option<fn(&mut E::RenderOrUnit, &E::ArcWidget) -> RenderObjectUpdateResult>,
        try_update_render_object_children: Option<fn(&E, &mut E::RenderOrUnit) -> Result<(), ()>>,
        detach_render_object: Option<fn(&mut E::RenderOrUnit)>,
        get_suspense: Option<SuspenseElementFunctionTable<E>>,
        has_layer: bool,
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
            try_create_render_object: |element, widget, element_context| {
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
            update_render_object: if E::NOOP_UPDATE_RENDER_OBJECT {
                None
            } else {
                Some(E::update_render_object)
            },
            try_update_render_object_children: if E::NOOP_UPDATE_RENDER_OBJECT_CHILDREN {
                None
            } else {
                Some(E::try_update_render_object_children)
            },
            detach_render_object: if R::NOOP_DETACH {
                None
            } else {
                Some(R::detach)
            },
            get_suspense: E::SUSPENSE_ELEMENT_FUNCTION_TABLE,
            has_layer: render_has_layer::<R>(),
        };

    fn lock_with(
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

    fn lock_with(
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
