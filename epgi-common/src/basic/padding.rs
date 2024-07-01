use epgi_2d::{
    ArcBoxRenderObject, ArcBoxWidget, BoxConstraints, BoxOffset, BoxProtocol,
    BoxSingleChildElement, BoxSingleChildElementTemplate, BoxSingleChildRenderElement, BoxSize,
    ShiftedBoxRender, ShiftedBoxRenderTemplate,
};
use epgi_core::{
    foundation::{set_if_changed, Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Provide},
    max,
    template::ImplByTemplate,
    tree::{ArcChildWidget, BuildContext, ElementBase, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Padding>))]
pub struct Padding {
    pub padding: EdgeInsets,
    pub child: ArcBoxWidget,
}

#[derive(PartialEq, Default, Clone, Copy, Debug)]
pub struct EdgeInsets {
    pub l: f32,
    pub r: f32,
    pub t: f32,
    pub b: f32,
}

impl EdgeInsets {
    pub fn new() -> Self {
        Self {
            l: 0.0,
            r: 0.0,
            t: 0.0,
            b: 0.0,
        }
    }

    pub fn l(mut self, l: f32) -> Self {
        self.l = l;
        self
    }
    pub fn r(mut self, r: f32) -> Self {
        self.r = r;
        self
    }
    pub fn t(mut self, t: f32) -> Self {
        self.t = t;
        self
    }
    pub fn b(mut self, b: f32) -> Self {
        self.b = b;
        self
    }

    pub fn new_ltrb(l: f32, r: f32, t: f32, b: f32) -> Self {
        Self { l, r, t, b }
    }

    pub fn new_all(value: f32) -> Self {
        Self {
            l: value,
            r: value,
            t: value,
            b: value,
        }
    }

    pub fn new_symmetric(vertical: f32, horizontal: f32) -> Self {
        Self {
            l: horizontal,
            r: horizontal,
            t: vertical,
            b: vertical,
        }
    }
}

pub trait BoxGeometryEdgeInsetsExt {
    fn deflate(&self, edges: EdgeInsets) -> Self;
    fn inflate(&self, edges: EdgeInsets) -> Self;
}

impl BoxGeometryEdgeInsetsExt for BoxConstraints {
    fn deflate(&self, edges: EdgeInsets) -> Self {
        let horizontal = edges.l + edges.r;
        let vertical = edges.t + edges.b;
        BoxConstraints {
            min_width: self.min_width - horizontal,
            max_width: self.max_width - horizontal,
            min_height: self.min_height - vertical,
            max_height: self.max_height - vertical,
        }
    }

    fn inflate(&self, edges: EdgeInsets) -> Self {
        let horizontal = edges.l + edges.r;
        let vertical = edges.t + edges.b;
        BoxConstraints {
            min_width: self.min_width + horizontal,
            max_width: self.max_width + horizontal,
            min_height: self.min_height + vertical,
            max_height: self.max_height + vertical,
        }
    }
}

impl BoxGeometryEdgeInsetsExt for BoxSize {
    fn deflate(&self, edges: EdgeInsets) -> Self {
        let horizontal = edges.l + edges.r;
        let vertical = edges.t + edges.b;
        BoxSize {
            width: self.width - horizontal,
            height: self.height - vertical,
        }
    }

    fn inflate(&self, edges: EdgeInsets) -> Self {
        let horizontal = edges.l + edges.r;
        let vertical = edges.t + edges.b;
        BoxSize {
            width: self.width + horizontal,
            height: self.height + vertical,
        }
    }
}

impl Widget for Padding {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type Element = PaddingElement;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

#[derive(Clone)]
pub struct PaddingElement {}

impl ImplByTemplate for PaddingElement {
    type Template = BoxSingleChildElementTemplate<true, false>;
}

impl BoxSingleChildElement for PaddingElement {
    type ArcWidget = Asc<Padding>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<epgi_2d::BoxProtocol>, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {}
    }
}

impl BoxSingleChildRenderElement for PaddingElement {
    type Render = RenderPadding;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderPadding {
            padding: widget.padding,
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        max!(set_if_changed(&mut render.padding, widget.padding).then_some(RenderAction::Relayout))
    }
}

pub struct RenderPadding {
    pub padding: EdgeInsets,
}

impl ImplByTemplate for RenderPadding {
    type Template = ShiftedBoxRenderTemplate;
}

impl ShiftedBoxRender for RenderPadding {
    type LayoutMemo = ();

    fn get_child_offset(&self, _size: &BoxSize, offset: &BoxOffset, _memo: &()) -> BoxOffset {
        *offset
            + BoxOffset {
                x: self.padding.l,
                y: self.padding.t,
            }
    }

    fn perform_layout(
        &mut self,
        constraints: &BoxConstraints,
        child: &ArcBoxRenderObject,
    ) -> (BoxSize, Self::LayoutMemo) {
        let child_size = child.layout_use_size(&constraints.deflate(self.padding));
        let size = constraints.constrain(child_size.inflate(self.padding));
        (size, ())
    }
}
