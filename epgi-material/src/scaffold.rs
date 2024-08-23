use std::borrow::Cow;

use epgi_2d::{ArcBoxWidget, BoxProtocol, Color};
use epgi_common::ColoredBox;
use epgi_core::{
    foundation::{Arc, Asc, AscProvideExt, InlinableDwsizeVec, Provide, SmallVecExt, TypeKey},
    nodes::{ConsumerElement, ConsumerWidget},
    read_providers,
    tree::{BuildContext, ElementBase, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::ThemeData;

/// This is a placeholder scaffold impl only!!!
#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Scaffold>))]
pub struct Scaffold {
    #[builder(default)]
    background_color: Option<Color>,
    body: ArcBoxWidget,
}

impl Widget for Scaffold {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type Element = ConsumerElement<BoxProtocol>;

    fn into_arc_widget(self: Asc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

lazy_static::lazy_static! {
    static ref SCAFFOLD_CONSUMED_TYPES: [TypeKey; 1] = [TypeKey::of::<ThemeData>()];
}

impl ConsumerWidget<BoxProtocol> for Scaffold {
    fn get_consumed_types(&self) -> Cow<[TypeKey]> {
        SCAFFOLD_CONSUMED_TYPES.as_ref().into()
    }

    fn build(
        &self,
        _ctx: &mut BuildContext,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> ArcBoxWidget {
        let theme_data = read_providers!(provider_values, ThemeData);

        ColoredBox!(
            color = self
                .background_color
                .unwrap_or(theme_data.scaffold_background_color),
            child = self.body.clone()
        )
    }
}
