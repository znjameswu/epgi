use std::{any::TypeId, marker::PhantomData};

use crate::{
    foundation::{
        Arc, ArrayContainer, Asc, BuildSuspendedError, InlinableDwsizeVec, Key, Never, Protocol,
        Provide,
    },
    tree::{
        ArcAnyWidget, ArcChildElementNode, ArcChildWidget, ArcWidget, BuildContext,
        ChildRenderObjectsUpdateCallback, Element, ElementReconcileItem, ReconcileItem, Reconciler,
        Widget, WidgetExt,
    },
};

// ComponentWidget and Consumer are separated due to the virtual call overhead in get_consumed_types
// ComponentWidget and Provider are separated due to type inconsistencies in Element::Provided
pub trait ComponentWidget<P: Protocol>:
    Widget<Element = ComponentElement<P>, ParentProtocol = P, ChildProtocol = P> + WidgetExt
{
    fn build(&self, ctx: BuildContext) -> ArcChildWidget<P>;
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

#[derive(Default, Clone)]
pub struct ComponentElement<P: Protocol>(PhantomData<P>);

impl<P> Element for ComponentElement<P>
where
    P: Protocol,
{
    type ArcWidget = Asc<dyn ComponentWidget<P>>;

    type ParentProtocol = P;

    type ChildProtocol = P;

    type ChildContainer = ArrayContainer<1>;

    type Provided = Never;

    #[inline(always)]
    fn perform_rebuild_element(
        &mut self,
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        children: [ArcChildElementNode<Self::ChildProtocol>; 1],
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::ChildProtocol>>,
    ) -> Result<
        (
            [ElementReconcileItem<Self::ChildProtocol>; 1],
            Option<ChildRenderObjectsUpdateCallback<Self>>,
        ),
        (
            [ArcChildElementNode<Self::ChildProtocol>; 1],
            BuildSuspendedError,
        ),
    > {
        let child_widget = widget.build(ctx);
        let [child] = children;
        match child.can_rebuild_with(child_widget) {
            Ok(item) => Ok(([item], None)),
            Err((child, child_widget)) => {
                nodes_needing_unmount.push(child);
                Ok(([ElementReconcileItem::new_inflate(child_widget)], None))
            }
        }
    }

    #[inline(always)]
    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(Self, [ArcChildWidget<Self::ChildProtocol>; 1]), BuildSuspendedError> {
        let child_widget = widget.build(ctx);
        Ok((Self(PhantomData), [child_widget]))
    }

    type RenderOrUnit = ();
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
    type ParentProtocol = P;

    type ChildProtocol = P;

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
    fn build(&self, ctx: BuildContext) -> ArcChildWidget<P> {
        (self.0)(ctx)
    }
}
