use epgi_core::{
    foundation::{
        Arc, Asc, BuildSuspendedError, Canvas, InlinableDwsizeVec, Never, OptionContainer,
        PaintContext, Protocol, Provide,
    },
    tree::{
        ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, BuildContext,
        CachedCompositionFunctionTable, CachedLayer, ChildLayerProducingIterator,
        ChildRenderObjectsUpdateCallback, DryLayout, Element, ElementReconcileItem, Layer,
        LayerCompositionConfig, LayerRender, PaintResults, Render, RenderAction, RenderElement,
        Widget,
    },
};

use crate::{Affine2dCanvas, BoxProtocol, VelloEncoding};

pub struct RootView {
    pub build: Box<dyn Fn(BuildContext) -> Option<ArcChildWidget<BoxProtocol>> + Send + Sync>,
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

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        self
    }
}

#[derive(Clone)]
pub struct RootElement {}

impl Element for RootElement {
    type ArcWidget = Asc<RootView>;

    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type ChildContainer = OptionContainer;

    type Provided = Never;

    fn perform_rebuild_element(
        // Rational for a moving self: Allows users to destructure the self without needing to fill in a placeholder value.
        &mut self,
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        children: Option<ArcChildElementNode<Self::ChildProtocol>>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::ChildProtocol>>,
    ) -> Result<
        (
            Option<ElementReconcileItem<Self::ChildProtocol>>,
            Option<ChildRenderObjectsUpdateCallback<Self>>,
        ),
        (
            Option<ArcChildElementNode<Self::ChildProtocol>>,
            BuildSuspendedError,
        ),
    > {
        let child_widget = (widget.build)(ctx);
        let item = match (child_widget, children) {
            (None, None) => None,
            (None, Some(child)) => {
                nodes_needing_unmount.push(child.clone());
                None
            }
            (Some(child_widget), None) => Some(ElementReconcileItem::new_inflate(child_widget)),
            (Some(child_widget), Some(child)) => match child.can_rebuild_with(child_widget) {
                Ok(item) => Some(item),
                Err((child, child_widget)) => {
                    nodes_needing_unmount.push(child);
                    Some(ElementReconcileItem::new_inflate(child_widget))
                }
            },
        };
        Ok((item, None))
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(Self, Option<ArcChildWidget<Self::ChildProtocol>>), BuildSuspendedError> {
        let child_widget = (widget.build)(ctx);
        Ok((RootElement {}, child_widget))
    }

    type RenderOrUnit = RenderRoot;
}

impl RenderElement for RootElement {
    type Render = RenderRoot;

    fn create_render(&self, widget: &Self::ArcWidget) -> RenderRoot {
        todo!()
    }

    fn update_render(render_object: &mut RenderRoot, widget: &Self::ArcWidget) -> RenderAction {
        todo!()
    }

    fn element_render_children_mapping<T: Send + Sync>(
        &self,
        element_children: <Self::ChildContainer as epgi_core::foundation::HktContainer>::Container<
            T,
        >,
    ) -> <<RenderRoot as Render>::ChildContainer as epgi_core::foundation::HktContainer>::Container<T>
    {
        todo!()
    }
}

pub struct RenderRoot {
    pub child: Option<ArcChildRenderObject<BoxProtocol>>,
}

impl Render for RenderRoot {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type ChildContainer = OptionContainer;

    type LayoutMemo = ();

    fn perform_layout<'a, 'layout>(
        &'a self,
        _constraints: &'a <Self::ParentProtocol as Protocol>::Constraints,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo) {
        unreachable!()
    }

    const DRY_LAYOUT_FUNCTION_TABLE: Option<epgi_core::tree::DryLayoutFunctionTable<Self>> =
        <Self as DryLayout>::DRY_LAYOUT_FUNCTION_TABLE;

    fn perform_paint(
        &self,
        _size: &<Self::ParentProtocol as Protocol>::Size,
        _transform: &<Self::ParentProtocol as Protocol>::Transform,
        _memo: &Self::LayoutMemo,
        _paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) {
        unreachable!()
    }

    type LayerOrUnit = RootLayer;
}

impl DryLayout for RenderRoot {
    fn compute_dry_layout(
        &self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
    ) -> <Self::ParentProtocol as Protocol>::Size {
        todo!()
    }

    fn compute_layout_memo(
        &self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &<Self::ParentProtocol as Protocol>::Size,
    ) -> Self::LayoutMemo {
        // self.render_ctx.resize_surface(&mut self.surface, size.width, size.height)
    }
}

impl LayerRender<RootLayer> for RenderRoot {
    fn create_layer(&self) -> Self::LayerOrUnit {
        todo!()
    }
}

pub struct RootLayer {
    /// This field is nullable because we temporarily share implementation with RootLayer
    child_render_object: Option<ArcChildRenderObject<BoxProtocol>>,
}

impl RootLayer {
    pub fn new(child_render_object: Option<ArcChildRenderObject<BoxProtocol>>) -> Self {
        Self {
            child_render_object,
        }
    }

    pub fn update_child_render_object(
        &mut self,
        child_render_object: ArcChildRenderObject<BoxProtocol>,
    ) {
        self.child_render_object = Some(child_render_object);
    }
}

impl Layer for RootLayer {
    type ParentCanvas = Affine2dCanvas;

    type ChildCanvas = Affine2dCanvas;

    fn composite_to(
        &self,
        encoding: &mut <Self::ParentCanvas as Canvas>::Encoding,
        child_iterator: &mut impl ChildLayerProducingIterator<Self::ChildCanvas>,
        composition_config: &LayerCompositionConfig<Self::ParentCanvas>,
    ) {
        todo!()
    }

    fn transform_config(
        self_config: &LayerCompositionConfig<Self::ParentCanvas>,
        child_config: &LayerCompositionConfig<Self::ChildCanvas>,
    ) -> LayerCompositionConfig<Self::ParentCanvas> {
        todo!()
    }

    fn repaint(
        &self,
        old_results: Option<&PaintResults<Self::ChildCanvas>>,
    ) -> PaintResults<Self::ChildCanvas> {
        todo!()
    }

    fn key(&self) -> Option<&Arc<dyn epgi_core::foundation::Key>> {
        todo!()
    }

    type CachedComposition = Arc<VelloEncoding>;

    const CACHED_COMPOSITION_FUNCTION_TABLE: Option<CachedCompositionFunctionTable<Self>> =
        <Self as CachedLayer>::PERFORM_CACHED_COMPOSITION;
}

impl CachedLayer for RootLayer {
    fn composite_to(
        &self,
        encoding: &mut <Self::ParentCanvas as Canvas>::Encoding,
        child_iterator: &mut impl ChildLayerProducingIterator<Self::ChildCanvas>,
        composition_config: &LayerCompositionConfig<Self::ParentCanvas>,
    ) -> Self::CachedComposition {
        todo!()
    }

    fn composite_from_cache_to(
        &self,
        encoding: &mut <Self::ParentCanvas as Canvas>::Encoding,
        cache: &Self::CachedComposition,
        composition_config: &LayerCompositionConfig<Self::ParentCanvas>,
    ) {
        todo!()
    }
}
