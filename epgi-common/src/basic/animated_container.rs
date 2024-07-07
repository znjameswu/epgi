use std::time::Duration;

use epgi_2d::{ArcBoxWidget, BoxConstraints, BoxProtocol, Color};
use epgi_core::{
    foundation::Asc,
    nodes::{ComponentElement, ComponentWidget},
    tree::{BuildContext, ElementBase, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::{
    Alignment, BuildContextImplicitAnimationExt, Container, EdgeInsets, ImplicitlyAnimated, Tween,
    ARC_PHANTOM_BOX, FAST_OUT_SLOW_IN,
};

lazy_static::lazy_static! {
    static ref ARC_FAST_OUT_SLOW_IN: Asc<dyn Tween<Output = f32> + Send + Sync> = Asc::new(FAST_OUT_SLOW_IN);
}

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<AnimatedContainer>))]
pub struct AnimatedContainer {
    duration: Duration,
    #[builder(default = Some(ARC_FAST_OUT_SLOW_IN.clone()), setter(strip_option, into))]
    curve: Option<Asc<dyn Tween<Output = f32> + Send + Sync>>,
    #[builder(default, setter(strip_option, into))]
    pub alignment: Option<Alignment>,
    #[builder(default, setter(strip_option, into))]
    pub padding: Option<EdgeInsets>,
    #[builder(default, setter(strip_option, into))]
    pub color: Option<Color>,
    // TODO: Decoration
    #[builder(default, setter(strip_option, into))]
    pub width: Option<f32>,
    #[builder(default, setter(strip_option, into))]
    pub height: Option<f32>,
    #[builder(default, setter(strip_option, into))]
    pub constraints: Option<BoxConstraints>,
    #[builder(default, setter(strip_option, into))]
    pub margin: Option<EdgeInsets>,
    // TODO: transform
    #[builder(default=ARC_PHANTOM_BOX.clone())]
    pub child: ArcBoxWidget,
    // TODO: Clip behavior
}

impl Widget for AnimatedContainer {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type Element = ComponentElement<BoxProtocol>;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

impl ComponentWidget<BoxProtocol> for AnimatedContainer {
    fn build(&self, ctx: &mut BuildContext<'_>) -> ArcBoxWidget {
        // This bahaves differently from Flutter!
        // In Flutter, only non-null param will create tweens upon initState.
        // Then for the rest of its lifetime, it will simply ignore all other param updates. No change, no animation, nothing.
        // Throughout the lifetime, tweens will only panic out if receives a null update, but never reconstructed.
        // For ease of implmenation and runtime cost consideration, we interp everything
        let value = ctx.use_implicitly_animated_value(
            &(
                self.alignment,
                self.padding,
                self.color,
                self.width,
                self.height,
                self.constraints,
                self.margin,
            ),
            self.duration,
            self.curve.as_ref(),
        );

        let child = self.child.clone();
        ImplicitlyAnimated!(
            value,
            builder =
                move |_ctx, (alignment, padding, color, width, height, constraints, margin)| {
                    Asc::new(Container {
                        alignment,
                        padding,
                        color,
                        width,
                        height,
                        constraints,
                        margin,
                        child: child.clone(),
                    })
                }
        )
    }
}
