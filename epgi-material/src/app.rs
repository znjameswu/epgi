use epgi_2d::{ArcBoxWidget, BoxProtocol, BoxProvider};
use epgi_common::{AnimationFrame, FrameInfo};
use epgi_core::{
    foundation::{Arc, Asc, AscProvideExt, InlinableDwsizeVec, Provide, SmallVecExt, TypeKey},
    nodes::{ConsumerElement, ConsumerWidget},
    read_providers,
    tree::{BuildContext, Widget},
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
    type Element = ConsumerElement<BoxProtocol>;

    fn into_arc_widget(self: Arc<Self>) -> Arc<dyn ConsumerWidget<BoxProtocol>> {
        self
    }
}

lazy_static::lazy_static!(
    static ref MATERIAL_CONSUMED_TYPES: [TypeKey; 1] = [
        TypeKey::of::<FrameInfo>()
    ];
);

impl ConsumerWidget<BoxProtocol> for MaterialApp {
    fn get_consumed_types(&self) -> std::borrow::Cow<[TypeKey]> {
        MATERIAL_CONSUMED_TYPES.as_ref().into()
    }

    fn build(
        &self,
        _ctx: &mut BuildContext,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> ArcBoxWidget {
        let frame_info = read_providers!(provider_values, FrameInfo);
        let animation_frame = AnimationFrame {
            time: frame_info.instant,
        };
        let theme_data = ThemeData::light();
        let child = self.child.clone();
        let child = BoxProvider::value_inner(animation_frame, child);
        let child = BoxProvider::value_inner(theme_data.progress_indicator_theme.clone(), child);
        let child = BoxProvider::value_inner(theme_data.text_theme.body_medium.clone(), child);
        let child = BoxProvider::value_inner(theme_data, child);
        child
    }
}
