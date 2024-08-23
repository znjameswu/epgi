use epgi_2d::{
    Affine2d, Affine2dCanvas, BoxConstraints, BoxIntrinsics, BoxOffset, BoxProtocol, BoxSize,
    MultiLineConstraints, MultiLineOffset, MultiLineProtocol, SingleLineOffset,
};
use epgi_core::{
    foundation::{Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide},
    template::{
        AdapterRender, AdapterRenderTemplate, ImplByTemplate, SingleChildElement,
        SingleChildElementTemplate, SingleChildRenderElement,
    },
    tree::{ArcChildRenderObject, ArcChildWidget, BuildContext, ElementBase, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<BoxAdapterMultiLine>))]
pub struct BoxAdapterMultiLine {
    child: ArcChildWidget<MultiLineProtocol>,
}

impl Widget for BoxAdapterMultiLine {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = MultiLineProtocol;
    type Element = BoxAdapterMultiLineElement;

    fn into_arc_widget(self: Asc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

#[derive(Clone, Debug)]
pub struct BoxAdapterMultiLineElement {}

impl ImplByTemplate for BoxAdapterMultiLineElement {
    type Template = SingleChildElementTemplate<true, false>;
}

impl SingleChildElement for BoxAdapterMultiLineElement {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = MultiLineProtocol;
    type ArcWidget = Asc<BoxAdapterMultiLine>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<MultiLineProtocol>, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {}
    }
}

impl SingleChildRenderElement for BoxAdapterMultiLineElement {
    type Render = RenderBoxAdapterMultiLine;

    fn create_render(&self, _widget: &Self::ArcWidget) -> Self::Render {
        RenderBoxAdapterMultiLine {}
    }

    fn update_render(
        _render: &mut Self::Render,
        _widget: &Self::ArcWidget,
    ) -> Option<RenderAction> {
        None
    }
}

pub struct RenderBoxAdapterMultiLine {}

impl ImplByTemplate for RenderBoxAdapterMultiLine {
    type Template = AdapterRenderTemplate;
}

impl AdapterRender for RenderBoxAdapterMultiLine {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = MultiLineProtocol;
    type LayoutMemo = MultiLineOffset;

    fn perform_layout(
        &mut self,
        constraints: &BoxConstraints,
        child: &ArcChildRenderObject<MultiLineProtocol>,
    ) -> (BoxSize, MultiLineOffset) {
        let multi_line_size = child.layout_use_size(&MultiLineConstraints {
            first_line_existing_advance: 0.0,
            max_width: constraints.max_width,
            last_line_append_advance: 0.0,
            max_height: constraints.max_height,
        });
        let mut y = 0.0f32;
        let mut max_width = 0.0f32;
        let offsets = multi_line_size
            .sizes
            .into_iter()
            .map(|size| {
                let offset = SingleLineOffset {
                    advance: 0.0,
                    baseline: y + size.above,
                };
                max_width = max_width.max(size.advance);
                y += size.above + size.below;
                offset
            })
            .collect(); // We use collect to trigger same-size Vec copy specialization
        (
            BoxSize {
                width: max_width,
                height: y,
            },
            MultiLineOffset { offsets },
        )
    }

    fn perform_paint(
        &self,
        _size: &BoxSize,
        offset: &BoxOffset,
        memo: &MultiLineOffset,
        child: &ArcChildRenderObject<MultiLineProtocol>,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        paint_ctx.with_transform(Affine2d::from_translation(offset), |paint_ctx| {
            paint_ctx.paint(child, memo)
        });
    }

    fn compute_intrinsics(
        &mut self,
        child: &ArcChildRenderObject<MultiLineProtocol>,
        intrinsics: &mut BoxIntrinsics,
    ) {
        unimplemented!()
    }

    const NOOP_DETACH: bool = true;
}
