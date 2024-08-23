use epgi_2d::{ArcBoxWidget, BoxProtocol, BoxSingleChildElement, BoxSingleChildElementTemplate};
use epgi_core::{
    foundation::{Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Provide},
    template::ImplByTemplate,
    tree::{BuildContext, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

#[derive(PartialEq, Clone, Default, Debug)]
pub struct PositionedConfig {
    pub l: Option<f32>,
    pub r: Option<f32>,
    pub t: Option<f32>,
    pub b: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

impl PositionedConfig {
    pub fn is_positioned(&self) -> bool {
        self.l.is_some()
            || self.r.is_some()
            || self.t.is_some()
            || self.b.is_some()
            || self.width.is_some()
            || self.height.is_some()
    }
}

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Positioned>))]
pub struct Positioned {
    #[builder(default, setter(strip_option))]
    pub l: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub r: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub t: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub b: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub width: Option<f32>,
    #[builder(default, setter(strip_option))]
    pub height: Option<f32>,
    pub child: ArcBoxWidget,
}

impl Positioned {
    pub fn is_positioned(&self) -> bool {
        self.get_config().is_positioned()
    }
    fn get_config(&self) -> PositionedConfig {
        PositionedConfig {
            l: self.l,
            r: self.r,
            t: self.t,
            b: self.b,
            width: self.width,
            height: self.height,
        }
    }
}

impl Widget for Positioned {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type Element = PositionedElement;

    fn into_arc_widget(self: Asc<Self>) -> Asc<Self> {
        self
    }
}

#[derive(Clone, Debug)]
pub struct PositionedElement {
    positioned_config: Option<PositionedConfig>,
}

impl ImplByTemplate for PositionedElement {
    type Template = BoxSingleChildElementTemplate<false, false>;
}

impl BoxSingleChildElement for PositionedElement {
    type ArcWidget = Asc<Positioned>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcBoxWidget, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {
            positioned_config: None,
        }
    }

    fn generate_parent_data(
        &mut self,
        widget: &Self::ArcWidget,
    ) -> Option<(Asc<dyn std::any::Any + Send + Sync>, Option<RenderAction>)> {
        let new_positioned_config = widget.get_config();
        let needs_update_parent_data = !self
            .positioned_config
            .as_ref()
            .is_some_and(|positioned_config| positioned_config == &new_positioned_config);
        return needs_update_parent_data.then(|| {
            (
                Asc::new(new_positioned_config) as _,
                Some(RenderAction::Relayout),
            )
        });
    }
}
