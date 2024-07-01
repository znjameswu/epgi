use epgi_2d::{ArcBoxWidget, BoxConstraints, BoxProtocol, Color};
use epgi_core::{
    foundation::Asc,
    nodes::{ComponentElement, ComponentWidget},
    tree::{BuildContext, ElementBase, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::{Align, Alignment, ColoredBox, ConstrainedBox, ARC_PHANTOM_BOX};

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Container>))]
pub struct Container {
    #[builder(default, setter(strip_option, into))]
    pub alignment: Option<Alignment>,
    // pub padding: Padding,
    #[builder(default, setter(strip_option, into))]
    pub color: Option<Color>,
    // TODO: Decoration
    #[builder(default, setter(strip_option, into))]
    pub width: Option<f32>,
    #[builder(default, setter(strip_option, into))]
    pub height: Option<f32>,
    #[builder(default, setter(strip_option, into))]
    pub constraints: Option<BoxConstraints>,
    // pub margin: Padding,
    // TODO: transform
    #[builder(default=ARC_PHANTOM_BOX.clone())]
    pub child: ArcBoxWidget,
    // TODO: Clip behavior
}

impl Widget for Container {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type Element = ComponentElement<BoxProtocol>;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

impl ComponentWidget<BoxProtocol> for Container {
    fn build(&self, _ctx: &mut BuildContext<'_>) -> ArcBoxWidget {
        let mut child = self.child.clone();
        if let Some(alignment) = self.alignment {
            child = Align!(alignment, child);
        }

        if let Some(color) = self.color {
            child = ColoredBox!(color, child);
        }

        let effective_constraints = if self.width.is_some() || self.height.is_some() {
            Some(
                self.constraints
                    .map(|constraints| constraints.tighten(self.width, self.height))
                    .unwrap_or(BoxConstraints::new_tight_for(self.width, self.height)),
            )
        } else {
            self.constraints
        };
        if let Some(constraints) = effective_constraints {
            child = ConstrainedBox!(constraints, child)
        }
        return child;
    }
}
