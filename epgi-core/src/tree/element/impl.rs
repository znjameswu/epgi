mod impl_reconcile;
pub use impl_reconcile::*;

mod impl_provide_element;
pub use impl_provide_element::*;

mod impl_render_element;
pub use impl_render_element::*;

mod element_impl;
pub use element_impl::*;

use crate::sync::ImplReconcileCommit;

use super::Element;

pub trait ImplElement:
    ImplElementNode<Self::Element> + ImplProvide<Self::Element> + ImplReconcileCommit<Self::Element>
{
    type Element: Element;
}

pub trait ImplElementBySuper {
    type Super: ImplElement;
}

impl<I, E: Element> ImplElement for I
where
    I: ImplElementBySuper,
    I::Super: ImplElement<Element = E>,
    Self: ImplElementNode<E>,
    Self: ImplProvide<E>,
    Self: ImplReconcileCommit<E>,
{
    type Element = E;
}
