use std::marker::PhantomData;

use crate::foundation::{Arc, Asc, Never, Protocol};

use super::{ArcChildElementNode, Element, Render, RenderElement, RenderObject, Widget};

pub trait SingleChildRenderObjectWidget: Widget {
    type Render: Render<Element = Self::Element>;
    fn update_render_object(&self, render_object: &Arc<RenderObject<Self::Render>>);
}

pub struct SingleChildRenderObjectElement<P, W> {
    child: ArcChildElementNode<P>,
    widget: PhantomData<W>,
}

impl<P, W> Clone for SingleChildRenderObjectElement<P, W>
where
    P: Protocol,
{
    fn clone(&self) -> Self {
        Self {
            child: self.child.clone(),
            widget: self.widget.clone(),
        }
    }
}

impl<P, W> Element for SingleChildRenderObjectElement<P, W>
where
    P: Protocol,
    W: SingleChildRenderObjectWidget<Element = Self>,
{
    type ArcWidget = Asc<W>;

    type ParentProtocol = P;

    type ChildProtocol = P;

    type Provided = Never;

    fn perform_rebuild_element(
        self,
        widget: &Self::ArcWidget,
        provider_values: crate::foundation::InlinableDwsizeVec<
            crate::foundation::Arc<dyn crate::foundation::Provide>,
        >,
        reconciler: impl super::Reconciler<Self::ChildProtocol>,
    ) -> Result<Self, (Self, crate::foundation::BuildSuspendedError)> {
        todo!()
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        provider_values: crate::foundation::InlinableDwsizeVec<
            crate::foundation::Arc<dyn crate::foundation::Provide>,
        >,
        reconciler: impl super::Reconciler<Self::ChildProtocol>, // TODO: A specialized reconciler for inflate, to save passing &JobIds
    ) -> Result<Self, crate::foundation::BuildSuspendedError> {
        todo!()
    }

    type ChildIter = [ArcChildElementNode<P>; 1];

    fn children(&self) -> Self::ChildIter {
        [self.child.clone()]
    }

    type ArcRenderObject = Arc<RenderObject<W::Render>>;
}

impl<P, W> RenderElement for SingleChildRenderObjectElement<P, W>
where
    P: Protocol,
    W: SingleChildRenderObjectWidget<Element = Self>,
{
    type Render = W::Render;

    fn try_create_render_object(
        &self,
        widget: &Self::ArcWidget,
    ) -> Option<Arc<RenderObject<Self::Render>>> {
        todo!()
    }

    fn update_render_object_widget(
        widget: &Self::ArcWidget,
        render_object: &Arc<RenderObject<Self::Render>>,
    ) {
        W::update_render_object(widget, render_object)
    }

    fn try_update_render_object_children(
        &self,
        render_object: &Arc<RenderObject<Self::Render>>,
    ) -> Result<(), ()> {
        todo!()
    }

    fn detach_render_object(render_object: &Arc<RenderObject<Self::Render>>) {
        todo!()
    }
}
