use std::any::TypeId;

use crate::{
    common::ReconcileItem,
    foundation::{
        Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Key, Never, Protocol, Provide,
    },
};

use super::{
    ArcAnyWidget, ArcChildElementNode, ArcChildWidget, ArcWidget, BuildContext, Element,
    Reconciler, SingleChildElement, Widget, WidgetExt,
};

// ComponentWidget and Consumer are separated due to the virtual call overhead in get_consumed_types
// ComponentWidget and Provider are separated due to type inconsistencies in Element::Provided
pub trait ComponentWidget<P: Protocol>: Widget<Element = ComponentElement<P>> + WidgetExt {
    fn build_with(&self, ctx: BuildContext) -> ArcChildWidget<P>;
}

impl<P> ArcWidget for Asc<dyn ComponentWidget<P>>
where
    P: Protocol,
{
    type Element = ComponentElement<P>;

    fn into_any_widget(self) -> ArcAnyWidget {
        self.as_arc_any_widget()
    }

    fn into_child_widget(self) -> ArcChildWidget<P> {
        self.as_arc_child_widget()
    }

    fn widget_type_id(&self) -> TypeId {
        WidgetExt::widget_type_id(self.as_ref())
    }

    fn key(&self) -> Option<&dyn Key> {
        Widget::key(self.as_ref())
    }
}

#[derive(Clone)]
pub struct ComponentElement<P: Protocol> {
    child: ArcChildElementNode<P>,
}

impl<P> Element for ComponentElement<P>
where
    P: Protocol,
{
    type ArcWidget = Asc<dyn ComponentWidget<P>>;

    type ParentProtocol = P;

    type ChildProtocol = P;

    type Provided = Never;

    #[inline(always)]
    fn perform_rebuild_element(
        self,
        widget: &Self::ArcWidget,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        mut reconciler: impl Reconciler<Self::ChildProtocol>,
    ) -> Result<Self, (Self, BuildSuspendedError)> {
        let child_widget = widget.build_with(reconciler.build_context_mut());
        match self.child.can_rebuild_with(child_widget) {
            Ok(item) => {
                let [child] = reconciler.into_reconcile([item]);
                Ok(Self { child })
            }
            Err((child, child_widget)) => {
                reconciler.nodes_needing_unmount_mut().push(child);
                let [child] = reconciler.into_reconcile([ReconcileItem::new_inflate(child_widget)]);
                Ok(Self { child })
            }
        }
    }

    #[inline(always)]
    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        mut reconciler: impl Reconciler<Self::ChildProtocol>, // TODO: A specialized reconciler for inflate, to save passing &JobIds
    ) -> Result<Self, BuildSuspendedError> {
        let child_widget = widget.build_with(reconciler.build_context_mut());
        let [child] = reconciler.into_reconcile([ReconcileItem::new_inflate(child_widget)]);
        Ok(Self { child })
    }

    type ChildIter = [ArcChildElementNode<P>; 1];

    fn children(&self) -> Self::ChildIter {
        [self.child.clone()]
    }

    type ArcRenderObject = Never;
}

impl<P> SingleChildElement for ComponentElement<P>
where
    P: Protocol,
{
    fn child(&self) -> &ArcChildElementNode<Self::ParentProtocol> {
        &self.child
    }
}

pub struct Function<F: Fn(BuildContext) -> ArcChildWidget<P> + Send + Sync + 'static, P: Protocol>(
    pub F,
);

impl<F, P> std::fmt::Debug for Function<F, P>
where
    P: Protocol,
    F: Fn(BuildContext) -> ArcChildWidget<P> + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Function").finish()
    }
}

impl<F, P> Widget for Function<F, P>
where
    P: Protocol,
    F: Fn(BuildContext) -> ArcChildWidget<P> + Send + Sync + 'static,
{
    type Element = ComponentElement<P>;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        self
    }
}

impl<F, P> ComponentWidget<P> for Function<F, P>
where
    P: Protocol,
    F: Fn(BuildContext) -> ArcChildWidget<P> + Send + Sync + 'static,
{
    fn build_with(&self, ctx: BuildContext) -> ArcChildWidget<P> {
        (self.0)(ctx)
    }
}
