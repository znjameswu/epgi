use std::marker::PhantomData;

use either::Either;

use crate::{
    foundation::{
        Arc, ArrayContainer, Asc, BuildSuspendedError, EitherContainer, EitherParallel,
        InlinableDwsizeVec, Key, Never, PaintContext, Protocol, Provide,
    },
    tree::{
        ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, BuildContext,
        ChildRenderObjectsUpdateCallback, Element, ElementReconcileItem, HitTestContext, Render,
        Widget,
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
        todo!()
    }

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        todo!()
    }
}

#[derive(Clone)]
pub struct SuspenseElement<P: Protocol> {
    _phantom: PhantomData<P>, // pub(crate) fallback_widget: ArcChildWidget<P>,
                              // pub(crate) fallback: Option<ArcChildElementNode<P>>,
}

impl<P: Protocol> Element for SuspenseElement<P> {
    type ArcWidget = Asc<Suspense<P>>;

    type ParentProtocol = P;

    type ChildProtocol = P;

    type ChildContainer = EitherContainer<ArrayContainer<1>, ArrayContainer<2>>;

    type Provided = Never;

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
            Option<ChildRenderObjectsUpdateCallback<Self>>,
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

    type RenderOrUnit = RenderSuspense<P>;
}

pub struct RenderSuspense<P: Protocol> {
    is_suspended: bool,
    phantom_data: PhantomData<P>,
}

impl<P: Protocol> Render for RenderSuspense<P> {
    type ParentProtocol = P;

    type ChildProtocol = P;

    type ChildContainer = ArrayContainer<1>;

    const NOOP_DETACH: bool = true;

    type LayoutMemo = ();

    fn perform_layout<'a, 'layout>(
        &'a mut self,
        constraints: &'a P::Constraints,
        children: &[ArcChildRenderObject<P>; 1],
    ) -> (P::Size, Self::LayoutMemo) {
        unreachable!()
    }

    fn perform_paint(
        &self,
        size: &P::Size,
        offset: &P::Offset,
        memo: &Self::LayoutMemo,
        children: &[ArcChildRenderObject<P>; 1],
        paint_ctx: &mut impl PaintContext<Canvas = P::Canvas>,
    ) {
        todo!()
    }

    fn hit_test_children(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &[ArcChildRenderObject<P>; 1],
        context: &mut HitTestContext<<Self::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        todo!()
    }

    type LayerOrUnit = ();
}
