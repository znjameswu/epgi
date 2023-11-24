use std::marker::PhantomData;

use either::Either;

use crate::{
    foundation::{
        Arc, ArrayContainer, Asc, BuildSuspendedError, Canvas, EitherContainer, EitherParallel,
        InlinableDwsizeVec, Key, Never, PaintContext, Protocol, Provide,
    },
    tree::{
        ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, BuildContext,
        ChildRenderObjectsUpdateCallback, Element, ElementReconcileItem, Render, Widget,
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
        children: EitherParallel<
            [ArcChildElementNode<Self::ChildProtocol>; 1],
            [ArcChildElementNode<Self::ChildProtocol>; 2],
        >,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::ChildProtocol>>,
    ) -> Result<
        (
            EitherParallel<
                [ElementReconcileItem<Self::ChildProtocol>; 1],
                [ElementReconcileItem<Self::ChildProtocol>; 2],
            >,
            Option<ChildRenderObjectsUpdateCallback<Self>>,
        ),
        (
            EitherParallel<
                [ArcChildElementNode<Self::ChildProtocol>; 1],
                [ArcChildElementNode<Self::ChildProtocol>; 2],
            >,
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
            EitherParallel<
                [ArcChildWidget<Self::ChildProtocol>; 1],
                [ArcChildWidget<Self::ChildProtocol>; 2],
            >,
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
        constraints: &'a <Self::ParentProtocol as Protocol>::Constraints,
        children: &[ArcChildRenderObject<P>; 1],
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo) {
        unreachable!()
    }

    fn perform_paint(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        transform: &<Self::ParentProtocol as Protocol>::Transform,
        memo: &Self::LayoutMemo,
        children: &[ArcChildRenderObject<P>; 1],
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) {
        todo!()
    }

    fn hit_test(
        &self,
        results: &mut crate::tree::HitTestResults,
        coord: &<<Self::ParentProtocol as Protocol>::Canvas as Canvas>::HitTestCoordinate,
        children: &[ArcChildRenderObject<Self::ChildProtocol>; 1],
    ) {
        let [child] = children;
        child.hit_test(results, coord)
    }

    type LayerOrUnit = ();
}
