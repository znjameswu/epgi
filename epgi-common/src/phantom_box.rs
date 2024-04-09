use std::sync::Arc;

use epgi_2d::{Affine2dCanvas, BoxConstraints, BoxOffset, BoxProtocol, BoxSize};
use epgi_core::{
    foundation::{Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide},
    template::{ImplByTemplate, LeafElement, LeafElementTemplate, LeafRender, LeafRenderTemplate},
    tree::{ElementBase, Widget},
};

lazy_static! {
    static ref ARC_PHANTOM_BOX: Asc<PhantomBox> = Asc::new(PhantomBox {});
}

#[derive(Clone, Copy, Debug)]
pub struct PhantomBox {}

impl Widget for PhantomBox {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Element = PhantomBoxElement;

    fn into_arc_widget(self: Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PhantomBoxElement {}

impl ImplByTemplate for PhantomBoxElement {
    type Template = LeafElementTemplate;
}

impl LeafElement for PhantomBoxElement {
    type Protocol = BoxProtocol;

    type ArcWidget = Asc<PhantomBox>;

    type Render = RenderPhantomBox;

    fn create_element(
        widget: &Self::ArcWidget,
        ctx: epgi_core::tree::BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<Self, BuildSuspendedError> {
        Ok(Self)
    }

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderPhantomBox
    }

    const NOOP_UPDATE_RENDER_OBJECT: bool = true;
}

pub struct RenderPhantomBox;

impl ImplByTemplate for RenderPhantomBox {
    type Template = LeafRenderTemplate;
}

impl LeafRender for RenderPhantomBox {
    type Protocol = BoxProtocol;

    fn perform_layout(&mut self, constraints: &BoxConstraints) -> BoxSize {
        constraints.smallest()
    }

    fn perform_paint(
        &self,
        size: &BoxSize,
        offset: &BoxOffset,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
    }
}
