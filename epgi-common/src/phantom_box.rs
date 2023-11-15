use std::sync::Arc;

use epgi_2d::{Affine2d, Affine2dCanvas, BoxConstraints, BoxProtocol, BoxSize};
use epgi_core::{
    foundation::{
        ArrayContainer, BuildSuspendedError, InlinableDwsizeVec, Never, PaintContext, Provide,
    },
    tree::{
        ArcChildElementNode, ArcChildWidget, DryLayout, Element, Render, RenderAction,
        RenderElement, Widget,
    },
};

lazy_static! {
    static ref ARC_PHANTOM_BOX: Arc<PhantomBox> = Arc::new(PhantomBox {});
}

#[derive(Clone, Copy, Debug)]
pub struct PhantomBox {}

impl Widget for PhantomBox {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Element = PhantomBoxElement;

    fn into_arc_widget(self: Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PhantomBoxElement {}

impl Element for PhantomBoxElement {
    type ArcWidget = Arc<PhantomBox>;

    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type ChildContainer = ArrayContainer<0>;

    type Provided = Never;

    fn perform_rebuild_element(
        &mut self,
        _widget: &Self::ArcWidget,
        _ctx: epgi_core::tree::BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<epgi_core::foundation::Arc<dyn Provide>>,
        _children: [ArcChildElementNode<Self::ChildProtocol>; 0],
        _nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::ChildProtocol>>,
    ) -> Result<
        (
            [epgi_core::tree::ElementReconcileItem<Self::ChildProtocol>; 0],
            Option<epgi_core::tree::ChildRenderObjectsUpdateCallback<Self>>,
        ),
        (
            [ArcChildElementNode<Self::ChildProtocol>; 0],
            BuildSuspendedError,
        ),
    > {
        Ok(([], None))
    }

    fn perform_inflate_element(
        _widget: &Self::ArcWidget,
        _ctx: epgi_core::tree::BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<epgi_core::foundation::Arc<dyn Provide>>,
    ) -> Result<(Self, [ArcChildWidget<Self::ChildProtocol>; 0]), BuildSuspendedError> {
        Ok((PhantomBoxElement {}, []))
    }

    type RenderOrUnit = RenderPhantomBox;
}

impl RenderElement for PhantomBoxElement {
    type Render = RenderPhantomBox;

    fn create_render(&self, _widget: &Arc<PhantomBox>) -> RenderPhantomBox {
        RenderPhantomBox {}
    }

    fn update_render(
        _render_object: &mut RenderPhantomBox,
        _widget: &Arc<PhantomBox>,
    ) -> RenderAction {
        RenderAction::None
    }

    const NOOP_UPDATE_RENDER_OBJECT: bool = true;

    fn element_render_children_mapping<T: Send + Sync>(
        &self,
        _element_children: <Self::ChildContainer as epgi_core::foundation::HktContainer>::Container<T>,
    ) -> <<RenderPhantomBox as Render>::ChildContainer as epgi_core::foundation::HktContainer>::Container<T>{
        todo!()
    }
}

pub struct RenderPhantomBox {}

impl Render for RenderPhantomBox {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type ChildContainer = ArrayContainer<0>;

    const NOOP_DETACH: bool = true;

    type LayoutMemo = ();

    fn perform_layout(&self, _constraints: &BoxConstraints) -> (BoxSize, Self::LayoutMemo) {
        unreachable!()
    }

    fn perform_paint(
        &self,
        _size: &BoxSize,
        _transform: &Affine2d,
        _memo: &Self::LayoutMemo,
        _paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
    }

    type LayerOrUnit = ();
}

impl DryLayout for RenderPhantomBox {
    fn compute_dry_layout(&self, constraints: &BoxConstraints) -> BoxSize {
        constraints.smallest()
    }

    fn compute_layout_memo<'a, 'layout>(
        &'a self,
        _constraints: &'a BoxConstraints,
        _size: &'a BoxSize,
    ) {
    }
}
