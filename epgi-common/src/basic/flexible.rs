use std::{any::Any, marker::PhantomData};

use epgi_core::{
    foundation::{Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Protocol, Provide},
    template::{ImplByTemplate, SingleChildElement, SingleChildElementTemplate},
    tree::{ArcChildWidget, BuildContext, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

/// How the child is inscribed into the available space.
///
/// See also:
///
///  * [RenderFlex], the flex render object.
///  * [Column], [Row], and [Flex], the flex widgets.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum FlexFit {
    /// The child is forced to fill the available space.
    ///
    // /// The [Expanded] widget assigns this kind of [FlexFit] to its child.
    Tight,

    /// The child can be at most as large as the available space (but is
    /// allowed to be smaller).
    ///
    // /// The [Flexible] widget assigns this kind of [FlexFit] to its child.
    Loose,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct FlexibleConfig {
    pub flex: u32,
    pub fit: FlexFit,
}

impl Default for FlexibleConfig {
    fn default() -> Self {
        Self {
            flex: 0,
            fit: FlexFit::Tight,
        }
    }
}

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Flexible<P>>))]
pub struct Flexible<P: Protocol> {
    #[builder(default = 1)]
    pub flex: u32,
    #[builder(default=FlexFit::Loose)]
    pub fit: FlexFit,
    pub child: ArcChildWidget<P>,
}

impl<P: Protocol> Flexible<P> {
    pub fn get_config(&self) -> FlexibleConfig {
        FlexibleConfig {
            flex: self.flex,
            fit: self.fit,
        }
    }
}

impl<P: Protocol> Widget for Flexible<P> {
    type ParentProtocol = P;
    type ChildProtocol = P;
    type Element = FlexibleElement<P>;

    fn into_arc_widget(self: Asc<Self>) -> Asc<Self> {
        self
    }
}

#[derive(Clone, Debug)]
pub struct FlexibleElement<P: Protocol> {
    flexible_config: Option<FlexibleConfig>,
    phantom: PhantomData<P>,
}

impl<P: Protocol> ImplByTemplate for FlexibleElement<P> {
    type Template = SingleChildElementTemplate<false, false>;
}

impl<P: Protocol> SingleChildElement for FlexibleElement<P> {
    type ParentProtocol = P;
    type ChildProtocol = P;
    type ArcWidget = Asc<Flexible<P>>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<P>, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {
            flexible_config: None,
            phantom: PhantomData,
        }
    }

    fn generate_parent_data(
        &mut self,
        widget: &Self::ArcWidget,
    ) -> Option<(Asc<dyn Any + Send + Sync>, Option<RenderAction>)> {
        let new_flexible_config = widget.get_config();
        let needs_update_parent_data = !self
            .flexible_config
            .is_some_and(|flexible_config| flexible_config == new_flexible_config);

        return needs_update_parent_data.then(|| {
            (
                Asc::new(new_flexible_config) as _,
                Some(RenderAction::Relayout),
            )
        });
    }
}
