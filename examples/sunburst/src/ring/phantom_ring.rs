use std::sync::Arc;

use epgi_2d::Affine2dCanvas;
use epgi_core::{
    foundation::{Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide},
    template::{ImplByTemplate, LeafElement, LeafElementTemplate, LeafRender, LeafRenderTemplate},
    tree::{BuildContext, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use super::{RingConstraints, RingOffset, RingProtocol, RingSize};

lazy_static::lazy_static! {
    pub static ref ARC_PHANTOM_RING: Asc<PhantomRing> = Asc::new(PhantomRing {});
}

#[derive(Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<PhantomRing>))]
#[derive(Clone, Copy, Debug)]
pub struct PhantomRing {}

impl Widget for PhantomRing {
    type ParentProtocol = RingProtocol;
    type ChildProtocol = RingProtocol;
    type Element = PhantomRingElement;

    fn into_arc_widget(self: Arc<Self>) -> Asc<Self> {
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PhantomRingElement;

impl ImplByTemplate for PhantomRingElement {
    type Template = LeafElementTemplate;
}

impl LeafElement for PhantomRingElement {
    type Protocol = RingProtocol;
    type ArcWidget = Asc<PhantomRing>;
    type Render = RenderPhantomRing;

    fn create_element(
        _widget: &Self::ArcWidget,
        _ctx: &mut BuildContext,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<Self, BuildSuspendedError> {
        Ok(Self)
    }

    fn create_render(&self, _widget: &Self::ArcWidget) -> Self::Render {
        RenderPhantomRing {}
    }

    const NOOP_UPDATE_RENDER_OBJECT: bool = true;
}

pub struct RenderPhantomRing {}

impl ImplByTemplate for RenderPhantomRing {
    type Template = LeafRenderTemplate;
}

impl LeafRender for RenderPhantomRing {
    type Protocol = RingProtocol;

    fn perform_layout(&mut self, constraints: &RingConstraints) -> RingSize {
        constraints.smallest()
    }

    fn perform_paint(
        &self,
        _size: &RingSize,
        _offset: &RingOffset,
        _paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
    }
}
