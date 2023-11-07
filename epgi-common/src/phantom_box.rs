use std::sync::Arc;

use epgi_2d::{Affine2d, Affine2dCanvas, BoxConstraints, BoxProtocol, BoxSize};
use epgi_core::{
    foundation::{BuildSuspendedError, InlinableDwsizeVec, Never, PaintContext, Provide},
    tree::{
        ArcChildElementNode, ArcChildRenderObject, DryLayout, Element, Reconciler, Render,
        RenderElement, RenderObjectUpdateResult, Widget,
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

    type Provided = Never;

    fn perform_rebuild_element(
        self,
        _widget: &Arc<PhantomBox>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        _reconciler: impl Reconciler<BoxProtocol>,
    ) -> Result<Self, (Self, BuildSuspendedError)> {
        Ok(self)
    }

    fn perform_inflate_element(
        _widget: &Arc<PhantomBox>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        _reconciler: impl Reconciler<BoxProtocol>, // TODO: A specialized reconciler for inflate, to save passing &JobIds
    ) -> Result<Self, BuildSuspendedError> {
        Ok(Self {})
    }

    type ChildIter = [ArcChildElementNode<BoxProtocol>; 0];

    fn children(&self) -> Self::ChildIter {
        []
    }

    type RenderOrUnit = RenderPhantomBox;
}

impl RenderElement<RenderPhantomBox> for PhantomBoxElement {
    fn try_create_render_object(&self, _widget: &Arc<PhantomBox>) -> Option<RenderPhantomBox> {
        Some(RenderPhantomBox {})
    }

    fn update_render(
        _render_object: &mut RenderPhantomBox,
        _widget: &Arc<PhantomBox>,
    ) -> RenderObjectUpdateResult {
        RenderObjectUpdateResult::None
    }

    const NOOP_UPDATE_RENDER_OBJECT: bool = true;

    fn try_update_render_object_children(
        &self,
        _render_object: &mut RenderPhantomBox,
    ) -> Result<(), ()> {
        Ok(())
    }

    const NOOP_UPDATE_RENDER_OBJECT_CHILDREN: bool = true;
}

pub struct RenderPhantomBox {}

impl Render for RenderPhantomBox {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type ChildIter = [ArcChildRenderObject<BoxProtocol>; 0];

    fn children(&self) -> Self::ChildIter {
        []
    }

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
