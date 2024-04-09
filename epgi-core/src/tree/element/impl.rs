use std::marker::PhantomData;

use crate::{
    foundation::{Arc, ArrayContainer, Asc, ContainerOf, Protocol, Provide, TypeKey},
    nodes::{RenderSuspense, SuspenseElement},
    sync::ImplReconcileCommit,
    tree::{ArcAnyRenderObject, ArcChildRenderObject, RenderObject},
};

use super::{ArcChildElementNode, ElementBase, ProvideElement, RenderElement};

pub trait ImplElement:
    ImplElementNode<Self::Element> + ImplProvide<Self::Element> + ImplReconcileCommit<Self::Element>
{
    type Element: ElementBase;
}

pub struct ElementImpl<E: ElementBase, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool>(
    PhantomData<E>,
);

impl<E: ElementBase, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> ImplElement
    for ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    Self: ImplElementNode<E>,
    // Self: ImplReconcile<E>,
    Self: ImplProvide<E>,
    Self: ImplReconcileCommit<E>,
{
    type Element = E;
}

pub trait ImplProvide<E: ElementBase> {
    const PROVIDE_ELEMENT: bool;
    fn option_get_provided_key_value_pair(
        widget: &E::ArcWidget,
    ) -> Option<(Arc<dyn Provide>, TypeKey)>;

    fn diff_provided_value(
        old_widget: &E::ArcWidget,
        new_widget: &E::ArcWidget,
    ) -> Option<Arc<dyn Provide>>;
}

impl<E: ElementBase, const RENDER_ELEMENT: bool> ImplProvide<E>
    for ElementImpl<E, RENDER_ELEMENT, false>
{
    const PROVIDE_ELEMENT: bool = false;

    fn option_get_provided_key_value_pair(
        _widget: &E::ArcWidget,
    ) -> Option<(Arc<dyn Provide>, TypeKey)> {
        None
    }

    fn diff_provided_value(
        _old_widget: &E::ArcWidget,
        _new_widget: &E::ArcWidget,
    ) -> Option<Arc<dyn Provide>> {
        None
    }
}

impl<E: ElementBase, const RENDER_ELEMENT: bool> ImplProvide<E>
    for ElementImpl<E, RENDER_ELEMENT, true>
where
    E: ProvideElement,
{
    const PROVIDE_ELEMENT: bool = true;

    fn option_get_provided_key_value_pair(
        widget: &E::ArcWidget,
    ) -> Option<(Arc<dyn Provide>, TypeKey)> {
        Some((E::get_provided_value(widget), TypeKey::of::<E::Provided>()))
    }

    fn diff_provided_value(
        old_widget: &E::ArcWidget,
        new_widget: &E::ArcWidget,
    ) -> Option<Arc<dyn Provide>> {
        let old_provided_value = E::get_provided_value(&old_widget);
        let new_provided_value = E::get_provided_value(new_widget);
        if !Asc::ptr_eq(&old_provided_value, &new_provided_value)
            && !old_provided_value.eq_sized(new_provided_value.as_ref())
        {
            Some(new_provided_value)
        } else {
            None
        }
    }
}

pub trait ImplElementNode<E: ElementBase> {
    type OptionArcRenderObject: Default + Clone + Send + Sync;
    fn get_current_subtree_render_object(
        render_object: &Self::OptionArcRenderObject,
        children: &ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
    ) -> Option<ArcChildRenderObject<E::ParentProtocol>>;

    const GET_RENDER_OBJECT_AS_ANY: Option<
        fn(&Self::OptionArcRenderObject) -> Option<ArcAnyRenderObject>,
    >;
}

impl<E: ElementBase, const PROVIDE_ELEMENT: bool> ImplElementNode<E>
    for ElementImpl<E, false, PROVIDE_ELEMENT>
where
    E: ElementBase<
        ChildContainer = ArrayContainer<1>,
        ChildProtocol = <E as ElementBase>::ParentProtocol,
    >,
{
    type OptionArcRenderObject = ();

    fn get_current_subtree_render_object(
        _render_object: &(),
        [child]: &[ArcChildElementNode<E::ChildProtocol>; 1],
    ) -> Option<ArcChildRenderObject<E::ParentProtocol>> {
        child.get_current_subtree_render_object()
    }

    const GET_RENDER_OBJECT_AS_ANY: Option<
        fn(&Self::OptionArcRenderObject) -> Option<ArcAnyRenderObject>,
    > = None;
}

impl<E: ElementBase, const PROVIDE_ELEMENT: bool> ImplElementNode<E>
    for ElementImpl<E, true, PROVIDE_ELEMENT>
where
    E: RenderElement,
{
    type OptionArcRenderObject = Option<Arc<RenderObject<E::Render>>>;

    fn get_current_subtree_render_object(
        render_object: &Self::OptionArcRenderObject,
        _children: &ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
    ) -> Option<ArcChildRenderObject<E::ParentProtocol>> {
        render_object
            .as_ref()
            .map(|render_object| render_object.clone() as _)
    }

    const GET_RENDER_OBJECT_AS_ANY: Option<
        fn(&Self::OptionArcRenderObject) -> Option<ArcAnyRenderObject>,
    > = Some(|render_object| {
        render_object
            .as_ref()
            .map(|render_object| render_object.clone() as _)
    });
}

impl<P: Protocol, const PROVIDE_ELEMENT: bool> ImplElementNode<SuspenseElement<P>>
    for ElementImpl<SuspenseElement<P>, true, PROVIDE_ELEMENT>
{
    type OptionArcRenderObject = Option<Arc<RenderObject<RenderSuspense<P>>>>;

    fn get_current_subtree_render_object(
        render_object: &Self::OptionArcRenderObject,
        _children: &ContainerOf<
            <SuspenseElement<P> as ElementBase>::ChildContainer,
            ArcChildElementNode<P>,
        >,
    ) -> Option<ArcChildRenderObject<P>> {
        render_object
            .as_ref()
            .map(|render_object| render_object.clone() as _)
    }

    const GET_RENDER_OBJECT_AS_ANY: Option<
        fn(&Self::OptionArcRenderObject) -> Option<ArcAnyRenderObject>,
    > = Some(|render_object| {
        render_object
            .as_ref()
            .map(|render_object| render_object.clone() as _)
    });
}
