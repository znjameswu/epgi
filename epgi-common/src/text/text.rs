use std::{borrow::Cow, ops::Deref, sync::Arc};

use epgi_2d::{BoxProtocol, LocalTextStyle, MultiLineProtocol, TextAlign, TextSpan, TextStyle};
use epgi_core::{
    foundation::{Asc, AscProvideExt, InlinableDwsizeVec, Provide, SmallVecExt, TypeKey},
    nodes::{ComponentElement, ComponentWidget, ConsumerElement, ConsumerWidget},
    read_providers,
    tree::{ArcChildWidget, BuildContext, ElementBase, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::{BoxMultiLineAdapter, RichText};

#[derive(Clone, Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Text>))]
pub struct Text {
    /// Single item optimization. If `text` is filled, then `text_spans` will be ignored
    #[builder(default, setter(strip_option, into))]
    pub text: Option<Cow<'static, str>>,
    /// If `text` is filled, then `text_spans` will be ignored
    #[builder(default)]
    pub text_spans: Vec<TextSpan>,
    #[builder(default, setter(strip_option))]
    pub style: Option<LocalTextStyle>,
    #[builder(default, setter(strip_option))]
    pub text_align: Option<TextAlign>,
}

impl Widget for Text {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type Element = ComponentElement<BoxProtocol>;

    fn into_arc_widget(self: Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

impl ComponentWidget<BoxProtocol> for Text {
    fn build(&self, _ctx: &mut BuildContext<'_>) -> ArcChildWidget<BoxProtocol> {
        BoxMultiLineAdapter!(
            child = Asc::new(MultiLineText {
                text: self.text.clone(),
                text_spans: self.text_spans.clone(),
                style: self.style.clone(),
                text_align: self.text_align,
            })
        )
    }
}

#[derive(Clone, Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<MultiLineText>))]
pub struct MultiLineText {
    /// Single item optimization. If `text` is filled, then `text_spans` will be ignored
    #[builder(default, setter(strip_option, into))]
    pub text: Option<Cow<'static, str>>,
    /// If `text` is filled, then `text_spans` will be ignored
    #[builder(default)]
    pub text_spans: Vec<TextSpan>,
    #[builder(default, setter(strip_option))]
    pub style: Option<LocalTextStyle>,
    #[builder(default, setter(strip_option))]
    pub text_align: Option<TextAlign>,
}

impl Widget for MultiLineText {
    type ParentProtocol = MultiLineProtocol;
    type ChildProtocol = MultiLineProtocol;
    type Element = ConsumerElement<MultiLineProtocol>;

    fn into_arc_widget(self: Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

lazy_static::lazy_static! {
    static ref MULTI_LINE_TEXT_CONSUMED_TYPES: [TypeKey; 1] = [
        TypeKey::of::<TextStyle>(),
    ];
}

impl ConsumerWidget<MultiLineProtocol> for MultiLineText {
    fn get_consumed_types(&self) -> Cow<[TypeKey]> {
        MULTI_LINE_TEXT_CONSUMED_TYPES.deref().into()
    }

    fn build(
        &self,
        _ctx: &mut BuildContext,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> ArcChildWidget<MultiLineProtocol> {
        let default_text_style = read_providers!(provider_values, TextStyle);
        let mut effective_text_style = default_text_style.as_ref().clone();
        if let Some(style) = self.style.as_ref() {
            effective_text_style = effective_text_style.merge(style.clone())
        }

        // TODO: mediaquery bold text
        // TODO: figure out the TextAlign mess

        Asc::new(RichText {
            text: self.text.as_ref().map(|text| TextSpan {
                text: text.clone(),
                style: None,
            }),
            text_spans: self.text_spans.clone(),
            style: effective_text_style,
            text_align: TextAlign::Start,
        })
    }
}
