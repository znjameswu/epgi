use epgi_core::{
    foundation::{
        Arc, Asc, BuildSuspendedError, Canvas, InlinableDwsizeVec, Never, PaintContext, Protocol,
        Provide,
    },
    tree::{
        ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, ArcElementContextNode,
        ArcLayerOf, BuildContext, DryLayout, Element, LayerPaint, ReconcileItem, Reconciler,
        Render, RenderObject, Widget,
    },
};

use crate::BoxProtocol;

pub struct RootView {
    pub build: Box<dyn Fn(BuildContext) -> Option<ArcChildWidget<BoxProtocol>> + Send + Sync>,
    // pub child: Option<ArcChildWidget<BoxProtocol>>,
}

impl std::fmt::Debug for RootView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RootView")
            // .field("child", &self.child)
            .finish()
    }
}

impl Widget for RootView {
    type Element = RootViewElement;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        todo!()
    }
}

#[derive(Clone)]
pub struct RootViewElement {
    pub child: Option<ArcChildElementNode<BoxProtocol>>,
}

impl Element for RootViewElement {
    type ArcWidget = Asc<RootView>;

    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Provided = Never;

    fn perform_rebuild_element(
        // Rational for a moving self: Allows users to destructure the self without needing to fill in a placeholder value.
        self,
        widget: &Self::ArcWidget,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        mut reconciler: impl Reconciler<Self::ChildProtocol>,
    ) -> Result<Self, (Self, BuildSuspendedError)> {
        let child_widget = (widget.build)(reconciler.build_context());
        match (child_widget, self.child) {
            (None, None) => Ok(Self { child: None }),
            (None, Some(child)) => {
                reconciler.nodes_needing_unmount_mut().push(child.clone());
                Ok(Self { child: None })
            }
            (Some(child_widget), None) => {
                let [child] = reconciler.into_reconcile([ReconcileItem::new_inflate(child_widget)]);
                Ok(Self { child: Some(child) })
            }
            (Some(child_widget), Some(child)) => match child.can_rebuild_with(child_widget) {
                Ok(item) => {
                    let [child] = reconciler.into_reconcile([item]);
                    Ok(Self { child: Some(child) })
                }
                Err((child, child_widget)) => {
                    reconciler.nodes_needing_unmount_mut().push(child);
                    let [child] =
                        reconciler.into_reconcile([ReconcileItem::new_inflate(child_widget)]);
                    Ok(Self { child: Some(child) })
                }
            },
        }
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        mut reconciler: impl Reconciler<Self::ChildProtocol>, // TODO: A specialized reconciler for inflate, to save passing &JobIds
    ) -> Result<Self, BuildSuspendedError> {
        let child_widget = (widget.build)(reconciler.build_context());
        if let Some(child_widget) = child_widget {
            let [child] = reconciler.into_reconcile([ReconcileItem::new_inflate(child_widget)]);
            Ok(Self { child: Some(child) })
        } else {
            Ok(Self { child: None })
        }
    }

    type ChildIter = Option<ArcChildElementNode<BoxProtocol>>;

    fn children(&self) -> Self::ChildIter {
        todo!()
    }

    type ArcRenderObject = Arc<RenderObject<RenderRootView>>;
}

pub struct RenderRootView {
    child: Option<ArcChildRenderObject<BoxProtocol>>,
}

impl Render for RenderRootView {
    type Element = RootViewElement;

    type ChildIter = Option<ArcChildRenderObject<BoxProtocol>>;

    fn children(&self) -> Self::ChildIter {
        todo!()
    }

    fn try_create_render_object_from_element(
        element: &Self::Element,
        widget: &<Self::Element as Element>::ArcWidget,
    ) -> Option<Self> {
        todo!()
    }

    fn update_render_object(
        &mut self,
        widget: &<Self::Element as Element>::ArcWidget,
    ) -> epgi_core::tree::RenderObjectUpdateResult {
        todo!()
    }

    fn try_update_render_object_children(&mut self, element: &Self::Element) -> Result<(), ()> {
        todo!()
    }

    type LayoutMemo = ();

    fn perform_layout<'a, 'layout>(
        &'a self,
        _constraints: &'a <<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
    ) -> (
        <<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        Self::LayoutMemo,
    ) {
        unreachable!()
    }

    const PERFORM_DRY_LAYOUT: Option<epgi_core::tree::PerformDryLayout<Self>> =
        Some(<Self as DryLayout>::PERFORM_DRY_LAYOUT);

    fn perform_paint(
        &self,
        _size: &<<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        _transform: &<<Self::Element as Element>::ParentProtocol as Protocol>::Transform,
        _memo: &Self::LayoutMemo,
        _paint_ctx: &mut impl PaintContext<
            Canvas = <<Self::Element as Element>::ParentProtocol as Protocol>::Canvas,
        >,
    ) {
        unreachable!()
    }
}

impl DryLayout for RenderRootView {
    fn compute_dry_layout(
        &self,
        constraints: &<<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
    ) -> <<Self::Element as Element>::ParentProtocol as Protocol>::Size {
        todo!()
    }

    fn perform_layout<'a, 'layout>(
        &'a self,
        constraints: &'a <<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
        size: &'a <<Self::Element as Element>::ParentProtocol as Protocol>::Size,
    ) -> Self::LayoutMemo {
        // self.render_ctx.resize_surface(&mut self.surface, size.width, size.height)
    }
}

impl LayerPaint for RenderRootView {
    fn get_layer_or_insert(
        &mut self,
        size: &<<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        transform: &<<Self::Element as Element>::ParentProtocol as Protocol>::Transform,
        memo: &Self::LayoutMemo,
        element_context: &ArcElementContextNode,
        transform_parent: &<<<Self::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::Transform,
    ) -> &ArcLayerOf<Self> {
        unimplemented!("Root layer design has not been finalized")
    }

    fn get_layer(&mut self) -> &ArcLayerOf<Self> {
        todo!()
    }
}
