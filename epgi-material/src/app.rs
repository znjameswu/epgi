use epgi_2d::{ArcBoxWidget, BoxProtocol};
use epgi_common::{AnimationFrame, FrameInfo};
use epgi_core::{
    foundation::{Arc, Asc, TypeKey},
    nodes::{ComponentElement, ComponentWidget},
    tree::{BuildContext, Widget},
    Consumer, Provider,
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::ThemeData;

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<MaterialApp>))]
pub struct MaterialApp {
    pub child: ArcBoxWidget,
}

impl Widget for MaterialApp {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type Element = ComponentElement<BoxProtocol>;

    fn into_arc_widget(self: Arc<Self>) -> Arc<dyn ComponentWidget<BoxProtocol>> {
        self
    }
}

lazy_static::lazy_static!(
    static ref MATERIAL_CONSUMED_TYPES: [TypeKey; 1] = [
        TypeKey::of::<FrameInfo>()
    ];
);

impl ComponentWidget<BoxProtocol> for MaterialApp {
    fn build(&self, _ctx: &mut BuildContext) -> ArcBoxWidget {
        let theme_data = ThemeData::light();
        let child = self.child.clone();
        let child = Consumer!(
            builder = move |_ctx, frame_info: Asc<FrameInfo>| Provider!(
                value = AnimationFrame {
                    time: frame_info.instant,
                },
                child = child.clone()
            )
        );
        let child = Provider!(value = theme_data.progress_indicator_theme.clone(), child);
        let child = Provider!(value = theme_data.text_theme.body_medium.clone(), child);
        let child = Provider!(value = theme_data, child);
        child
    }
}
