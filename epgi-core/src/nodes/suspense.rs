use std::marker::PhantomData;

use either::Either;

use crate::{
    foundation::{
        Arc, ArrayContainer, Asc, BuildSuspendedError, EitherContainer, EitherParallel,
        InlinableDwsizeVec, Key, Protocol, Provide,
    },
    template::{ImplByTemplate, ProxyRender, ProxyRenderTemplate},
    tree::{
        ArcChildElementNode, ArcChildWidget, BuildContext, ChildRenderObjectsUpdateCallback,
        Element, ElementBase, ElementImpl, ElementReconcileItem, Widget,
    },
};

#[derive(Debug)]
pub struct Suspense<P: Protocol> {
    pub child: ArcChildWidget<P>,
    pub fallback: ArcChildWidget<P>,
    pub key: Option<Box<dyn Key>>,
}

impl<P: Protocol> Widget for Suspense<P> {
    type ParentProtocol = P;
    type ChildProtocol = P;
    type Element = SuspenseElement<P>;

    fn key(&self) -> Option<&dyn Key> {
        self.key.as_deref()
    }

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

#[derive(Clone)]
pub struct SuspenseElement<P: Protocol> {
    _phantom: PhantomData<P>, // pub(crate) fallback_widget: ArcChildWidget<P>,
                              // pub(crate) fallback: Option<ArcChildElementNode<P>>,
}

impl<P: Protocol> ElementBase for SuspenseElement<P> {
    type ArcWidget = Asc<Suspense<P>>;

    type ParentProtocol = P;
    type ChildProtocol = P;
    type ChildContainer = EitherContainer<ArrayContainer<1>, ArrayContainer<2>>;

    fn perform_rebuild_element(
        &mut self,
        widget: &Self::ArcWidget,
        _ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        children: EitherParallel<[ArcChildElementNode<P>; 1], [ArcChildElementNode<P>; 2]>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<P>>,
    ) -> Result<
        (
            EitherParallel<[ElementReconcileItem<P>; 1], [ElementReconcileItem<P>; 2]>,
            Option<ChildRenderObjectsUpdateCallback<Self::ChildContainer, Self::ChildProtocol>>,
        ),
        (
            EitherParallel<[ArcChildElementNode<P>; 1], [ArcChildElementNode<P>; 2]>,
            BuildSuspendedError,
        ),
    > {
        use Either::*;
        match children.0 {
            Left([child]) => {
                let item = match child.can_rebuild_with(widget.child.clone()) {
                    Ok(pair) => pair,
                    Err((child, child_widget)) => {
                        nodes_needing_unmount.push(child);
                        ElementReconcileItem::Inflate(child_widget)
                    }
                };
                return Ok((EitherParallel::new_left([item]), None));
            }
            Right([child, fallback]) => {
                let child_item = match child.can_rebuild_with(widget.child.clone()) {
                    Ok(pair) => pair,
                    Err((child, child_widget)) => {
                        nodes_needing_unmount.push(child);
                        ElementReconcileItem::Inflate(child_widget)
                    }
                };
                let fallback_item = match fallback.can_rebuild_with(widget.fallback.clone()) {
                    Ok(pair) => pair,
                    Err((fallback, fallback_widget)) => {
                        nodes_needing_unmount.push(fallback);
                        ElementReconcileItem::Inflate(fallback_widget)
                    }
                };
                return Ok((EitherParallel::new_right([child_item, fallback_item]), None));
            }
        }
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        _ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<
        (
            Self,
            EitherParallel<[ArcChildWidget<P>; 1], [ArcChildWidget<P>; 2]>,
        ),
        BuildSuspendedError,
    > {
        Ok((
            Self {
                _phantom: PhantomData,
            },
            EitherParallel::new_left([widget.child.clone()]),
        ))
    }
}

impl<P: Protocol> Element for SuspenseElement<P> {
    type Impl = ElementImpl<true, false>;
}

pub struct RenderSuspense<P: Protocol> {
    pub is_suspended: bool,
    phantom_data: PhantomData<P>,
}

impl<P: Protocol> RenderSuspense<P> {
    pub fn new(is_suspended: bool) -> Self {
        Self {
            is_suspended,
            phantom_data: PhantomData,
        }
    }
}

impl<P: Protocol> ImplByTemplate for RenderSuspense<P> {
    type Template = ProxyRenderTemplate;
}

impl<P: Protocol> ProxyRender for RenderSuspense<P> {
    type Protocol = P;
}
