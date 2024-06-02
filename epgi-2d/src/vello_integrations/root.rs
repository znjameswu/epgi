use epgi_core::{
    foundation::{Arc, Asc, BuildSuspendedError, Canvas, InlinableDwsizeVec, Provide},
    template::ImplByTemplate,
    tree::{
        ArcChildWidget, BuildContext, ChildLayerProducingIterator, HitTestContext,
        LayerCompositionConfig, RecordedChildLayer, RenderAction, Widget,
    },
};

use crate::{
    Affine2dCanvas, Affine2dEncoding, ArcBoxRenderObject, ArcBoxWidget, BoxConstraints, BoxOffset,
    BoxProtocol, BoxSingleChildCachedComposite, BoxSingleChildDryLayout, BoxSingleChildElement,
    BoxSingleChildElementTemplate, BoxSingleChildHitTest, BoxSingleChildLayerPaint,
    BoxSingleChildRender, BoxSingleChildRenderElement, BoxSingleChildRenderTemplate, BoxSize,
};

pub struct RootView {
    pub child: ArcBoxWidget,
}

impl std::fmt::Debug for RootView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RootView")
            // .field("child", &self.child)
            .finish()
    }
}

impl Widget for RootView {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Element = RootElement;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> Asc<RootView> {
        self
    }
}

#[derive(Clone)]
pub struct RootElement {}

impl ImplByTemplate for RootElement {
    type Template = BoxSingleChildElementTemplate<true, false>;
}

impl BoxSingleChildElement for RootElement {
    type ArcWidget = Asc<RootView>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<BoxProtocol>, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {}
    }
}

impl BoxSingleChildRenderElement for RootElement {
    type Render = RenderRoot;

    fn create_render(&self, _widget: &Self::ArcWidget) -> Self::Render {
        RenderRoot {}
    }

    fn update_render(_render: &mut Self::Render, _widget: &Self::ArcWidget) -> RenderAction {
        RenderAction::None
    }
}

pub struct RenderRoot {}

impl ImplByTemplate for RenderRoot {
    type Template = BoxSingleChildRenderTemplate<true, true, true, false>;
}

impl BoxSingleChildRender for RenderRoot {
    type LayoutMemo = ();
}

impl BoxSingleChildDryLayout for RenderRoot {
    fn compute_dry_layout(&self, _constraints: &BoxConstraints) -> BoxSize {
        BoxSize::INFINITY
    }

    fn perform_layout(
        &mut self,
        constraints: &BoxConstraints,
        _size: &BoxSize,
        child: &ArcBoxRenderObject,
    ) -> Self::LayoutMemo {
        child.layout(constraints)
    }
}

impl BoxSingleChildLayerPaint for RenderRoot {}

impl BoxSingleChildCachedComposite for RenderRoot {
    type CompositionMemo = Arc<Affine2dEncoding>;

    fn composite_into_memo(
        &self,
        child_iterator: &mut ChildLayerProducingIterator<Affine2dCanvas>,
    ) -> Self::CompositionMemo {
        let mut result = Affine2dEncoding::new();
        use epgi_core::tree::ChildLayerOrFragmentRef::*;
        child_iterator.for_each(|child| match child {
            Fragment(encoding) => {
                Affine2dCanvas::composite_encoding(&mut result, encoding, None);
                Vec::new()
            }
            Child(layer) | AdoptedChild(layer) => {
                layer.layer.composite_to(&mut result, &layer.config)
            }
        });
        return Arc::new(result);
    }

    fn composite_from_cache_to(
        &self,
        _encoding: &mut Affine2dEncoding,
        _memo: &Self::CompositionMemo,
        _composition_config: &LayerCompositionConfig<Affine2dCanvas>,
    ) {
        unreachable!()
    }
}

impl BoxSingleChildHitTest for RenderRoot {
    fn hit_test_child(
        &self,
        ctx: &mut HitTestContext<Affine2dCanvas>,
        _size: &BoxSize,
        _offset: &BoxOffset,
        _memo: &Self::LayoutMemo,
        child: &ArcBoxRenderObject,
        _adopted_children: &[RecordedChildLayer<Affine2dCanvas>],
    ) -> bool {
        ctx.hit_test(child.clone())
    }
}
