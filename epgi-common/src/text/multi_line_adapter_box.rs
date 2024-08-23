use epgi_2d::{
    Affine2dCanvas, ArcBoxRenderObject, ArcBoxWidget, BoxConstraints, BoxOffset, BoxProtocol,
    MultiLineConstraints, MultiLineIntrinsics, MultiLineOffset, MultiLineProtocol, MultiLineSize,
    SingleLineSize,
};
use epgi_core::{
    foundation::{Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide},
    template::{
        AdapterRender, AdapterRenderTemplate, ImplByTemplate, SingleChildElement,
        SingleChildElementTemplate, SingleChildRenderElement,
    },
    tree::{BuildContext, ElementBase, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<MultiLineAdapterBox>))]
pub struct MultiLineAdapterBox {
    child: ArcBoxWidget,
}

impl Widget for MultiLineAdapterBox {
    type ParentProtocol = MultiLineProtocol;
    type ChildProtocol = BoxProtocol;
    type Element = MultiLineAdapterBoxElement;

    fn into_arc_widget(self: Asc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

#[derive(Clone, Debug)]
pub struct MultiLineAdapterBoxElement {}

impl ImplByTemplate for MultiLineAdapterBoxElement {
    type Template = SingleChildElementTemplate<true, false>;
}

impl SingleChildElement for MultiLineAdapterBoxElement {
    type ParentProtocol = MultiLineProtocol;
    type ChildProtocol = BoxProtocol;
    type ArcWidget = Asc<MultiLineAdapterBox>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcBoxWidget, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {}
    }
}

impl SingleChildRenderElement for MultiLineAdapterBoxElement {
    type Render = RenderMultiLineBAdapterBox;

    fn create_render(&self, _widget: &Self::ArcWidget) -> Self::Render {
        RenderMultiLineBAdapterBox {}
    }

    fn update_render(
        _render: &mut Self::Render,
        _widget: &Self::ArcWidget,
    ) -> Option<RenderAction> {
        None
    }
}

pub struct RenderMultiLineBAdapterBox {}

impl ImplByTemplate for RenderMultiLineBAdapterBox {
    type Template = AdapterRenderTemplate;
}

impl AdapterRender for RenderMultiLineBAdapterBox {
    type ParentProtocol = MultiLineProtocol;
    type ChildProtocol = BoxProtocol;
    type LayoutMemo = ();

    fn perform_layout(
        &mut self,
        constraints: &MultiLineConstraints,
        child: &ArcBoxRenderObject,
    ) -> (MultiLineSize, ()) {
        let size = child.layout_use_size(&BoxConstraints {
            min_width: 0.0,
            max_width: constraints.max_width - constraints.first_line_existing_advance,
            min_height: 0.0,
            max_height: constraints.max_height,
        });
        let single_line_size = SingleLineSize {
            advance: size.width,
            above: size.height,
            below: 0.0,
        };
        (
            MultiLineSize {
                sizes: vec![single_line_size],
            },
            (),
        )
    }

    fn perform_paint(
        &self,
        size: &MultiLineSize,
        offset: &MultiLineOffset,
        _memo: &(),
        child: &ArcBoxRenderObject,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        let [first_offset] = offset.offsets.as_slice() else {
            panic!("Multi-line object with a single line should only receive a single offset");
        };
        let [first_size] = size.sizes.as_slice() else {
            panic!("Multi-line object with a single line should only report a single size");
        };
        paint_ctx.paint(
            child,
            &BoxOffset {
                x: first_offset.advance,
                y: first_offset.baseline - first_size.above,
            },
        );
    }

    fn compute_intrinsics(
        &mut self,
        child: &ArcBoxRenderObject,
        intrinsics: &mut MultiLineIntrinsics,
    ) {
        unimplemented!()
    }

    const NOOP_DETACH: bool = true;
}
