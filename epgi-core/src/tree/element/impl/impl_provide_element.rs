use crate::{foundation::{Arc, Asc, Provide, TypeKey}, tree::Element};

use super::{ElementImpl, HasProvideElementImpl};

pub trait ImplProvide<E: Element> {
    const PROVIDE_ELEMENT: bool;
    fn option_get_provided_key_value_pair(
        widget: &E::ArcWidget,
    ) -> Option<(Arc<dyn Provide>, TypeKey)>;

    fn diff_provided_value(
        old_widget: &E::ArcWidget,
        new_widget: &E::ArcWidget,
    ) -> Option<Arc<dyn Provide>>;
}

impl<E: Element, const RENDER_ELEMENT: bool> ImplProvide<E>
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

impl<E: Element, const RENDER_ELEMENT: bool> ImplProvide<E> for ElementImpl<E, RENDER_ELEMENT, true>
where
    E::ElementImpl: HasProvideElementImpl<E>,
{
    const PROVIDE_ELEMENT: bool = true;

    fn option_get_provided_key_value_pair(
        widget: &E::ArcWidget,
    ) -> Option<(Arc<dyn Provide>, TypeKey)> {
        Some((
            E::ElementImpl::get_provided_value(widget),
            TypeKey::of::<<E::ElementImpl as HasProvideElementImpl<E>>::Provided>(),
        ))
    }

    fn diff_provided_value(
        old_widget: &E::ArcWidget,
        new_widget: &E::ArcWidget,
    ) -> Option<Arc<dyn Provide>> {
        let old_provided_value = E::ElementImpl::get_provided_value(&old_widget);
        let new_provided_value = E::ElementImpl::get_provided_value(new_widget);
        if !Asc::ptr_eq(&old_provided_value, &new_provided_value)
            && !old_provided_value.eq_sized(new_provided_value.as_ref())
        {
            Some(new_provided_value)
        } else {
            None
        }
    }
}
