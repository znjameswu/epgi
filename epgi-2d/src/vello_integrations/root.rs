use epgi_core::{
    foundation::{
        Arc, Asc, BuildSuspendedError, Canvas, InlinableDwsizeVec, OptionContainer, Protocol,
        Provide,
    },
    tree::{
        ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, BuildContext, CachedComposite,
        ChildLayerProducingIterator, ChildRenderObjectsUpdateCallback, DryLayout, Element,
        ElementImpl, ElementReconcileItem, HasArcWidget, HasLayoutMemo, HitTest, HitTestResults,
        LayerCompositionConfig, LayerPaint, Reconcile, Render, RenderAction, RenderElement,
        RenderImpl, RenderObjectSlots, TreeNode, Widget,
    },
};

use crate::{Affine2dCanvas, Affine2dEncoding, BoxOffset, BoxProtocol, BoxSize, Point2d};

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

    fn into_arc_widget(self: std::sync::Arc<Self>) -> Asc<RootView> {
        self
    }
}

#[derive(Clone)]
pub struct RootElement {}

impl TreeNode for RootElement {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type ChildContainer = OptionContainer;
}

impl HasArcWidget for RootElement {
    type ArcWidget = Asc<RootView>;
}

impl Element for RootElement {
    type ElementImpl = ElementImpl<Self, true, false>;
}

impl Reconcile for RootElement {
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
        let (item, shuffle) = match (child_widget, children) {
            (None, None) => (None, None),
            (None, Some(child)) => {
                nodes_needing_unmount.push(child.clone());
                (None, Some(Box::new(|_| None) as _))
            }
            (Some(child_widget), None) => (
                Some(ElementReconcileItem::new_inflate(child_widget)),
                Some(Box::new(|_| Some(RenderObjectSlots::Inflate)) as _),
            ),
            (Some(child_widget), Some(child)) => {
                let item = match child.can_rebuild_with(child_widget) {
                    Ok(item) => Some(item),
                    Err((child, child_widget)) => {
                        nodes_needing_unmount.push(child);
                        Some(ElementReconcileItem::new_inflate(child_widget))
                    }
                };
                (item, None)
            }
        };
        Ok((item, shuffle))
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(Self, Option<ArcChildWidget<Self::ChildProtocol>>), BuildSuspendedError> {
        let child_widget = (widget.build)(ctx);
        Ok((RootElement {}, child_widget))
    }
}

impl RenderElement for RootElement {
    type Render = RenderRoot;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        todo!()
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> RenderAction {
        todo!()
    }

    // fn create_render(&self, widget: &Self::ArcWidget) -> RenderRoot {
    //     todo!()
    // }

    // fn update_render(render_object: &mut RenderRoot, widget: &Self::ArcWidget) -> RenderAction {
    //     todo!()
    // }

    // fn element_render_children_mapping<T: Send + Sync>(
    //     &self,
    //     element_children: <Self::ChildContainer as epgi_core::foundation::HktContainer>::Container<
    //         T,
    //     >,
    // ) -> <<RenderRoot as Render>::ChildContainer as epgi_core::foundation::HktContainer>::Container<T>
    // {
    //     todo!()
    // }
}

pub struct RenderRoot {
    pub child: Option<ArcChildRenderObject<BoxProtocol>>,
}

impl TreeNode for RenderRoot {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type ChildContainer = OptionContainer;
}

impl HasLayoutMemo for RenderRoot {
    type LayoutMemo = ();
}

impl DryLayout for RenderRoot {
    fn compute_dry_layout(
        &self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
    ) -> <Self::ParentProtocol as Protocol>::Size {
        todo!()
    }

    fn compute_layout_memo(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &<Self::ParentProtocol as Protocol>::Size,
        children: &<Self::ChildContainer as epgi_core::foundation::HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> Self::LayoutMemo {
        todo!()
    }
}

impl LayerPaint for RenderRoot {}

impl CachedComposite for RenderRoot {
    type CompositionCache = Arc<Affine2dEncoding>;

    fn composite_into_cache(
        &self,
        child_iterator: &mut impl ChildLayerProducingIterator<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> Self::CompositionCache {
        todo!()
    }

    fn composite_from_cache_to(
        &self,
        encoding: &mut <<Self::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        cache: &Self::CompositionCache,
        composition_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
    ) {
        todo!()
    }

    fn transform_config(
        self_config: &LayerCompositionConfig<
            <<Self as TreeNode>::ParentProtocol as Protocol>::Canvas,
        >,
        child_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<<Self as TreeNode>::ParentProtocol as Protocol>::Canvas> {
        todo!()
    }
}

impl HitTest for RenderRoot {
    fn hit_test_children(
        &self,
        _size: &BoxSize,
        _offset: &BoxOffset,
        _memo: &Self::LayoutMemo,
        children: &Option<ArcChildRenderObject<BoxProtocol>>,
        results: &mut HitTestResults<Affine2dCanvas>,
    ) -> bool {
        children
            .as_ref()
            .map(|child| results.hit_test(child.clone()))
            .unwrap_or_default()
    }

    fn hit_test_self(
        &self,
        position: &Point2d,
        _size: &BoxSize,
        _offset: &BoxOffset,
        _memo: &Self::LayoutMemo,
    ) -> Option<epgi_core::tree::HitTestBehavior> {
        todo!()
    }
}

impl Render for RenderRoot {
    type RenderImpl = RenderImpl<Self, true, true, true, false>;
}

// impl Render for RenderRoot {
//     type ParentProtocol = BoxProtocol;

//     type ChildProtocol = BoxProtocol;

//     type ChildContainer = OptionContainer;

//     type LayoutMemo = ();

//     fn perform_layout<'a, 'layout>(
//         &'a mut self,
//         _constraints: &'a BoxConstraints,
//         _children: &Option<ArcChildRenderObject<BoxProtocol>>,
//     ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo) {
//         unreachable!()
//     }

//     const DRY_LAYOUT_FUNCTION_TABLE: Option<DryLayoutFunctionTable<Self>> =
//         <Self as DryLayoutOld>::DRY_LAYOUT_FUNCTION_TABLE;

//     fn perform_paint(
//         &self,
//         _size: &BoxSize,
//         _offset: &BoxOffset,
//         _memo: &Self::LayoutMemo,
//         _children: &Option<ArcChildRenderObject<BoxProtocol>>,
//         _paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
//     ) {
//         unreachable!()
//     }

//     fn hit_test_children(
//         &self,
//         _size: &BoxSize,
//         _offset: &BoxOffset,
//         _memo: &Self::LayoutMemo,
//         children: &Option<ArcChildRenderObject<BoxProtocol>>,
//         results: &mut HitTestResults<Affine2dCanvas>,
//     ) -> bool {
//         children
//             .as_ref()
//             .map(|child| results.hit_test(child.clone()))
//             .unwrap_or_default()
//     }

//     type LayerOrUnit = RenderRoot;
// }

// impl DryLayoutOld for RenderRoot {
//     fn compute_dry_layout(&self, constraints: &BoxConstraints) -> BoxSize {
//         constraints.biggest()
//     }

//     fn compute_layout_memo(
//         &self,
//         constraints: &BoxConstraints,
//         _size: &BoxSize,
//         children: &Option<ArcChildRenderObject<BoxProtocol>>,
//     ) -> Self::LayoutMemo {
//         if let Some(child) = children {
//             child.layout(constraints)
//         }
//         ()
//     }
// }

// impl LayerRender for RenderRoot {
//     fn composite_to(
//         encoding: &mut Affine2dEncoding,
//         child_iterator: &mut impl ChildLayerProducingIterator<Affine2dCanvas>,
//         composition_config: &LayerCompositionConfig<Affine2dCanvas>,
//     ) {
//         todo!()
//     }

//     fn transform_config(
//         self_config: &LayerCompositionConfig<Affine2dCanvas>,
//         child_config: &LayerCompositionConfig<Affine2dCanvas>,
//     ) -> LayerCompositionConfig<Affine2dCanvas> {
//         todo!()
//     }

//     fn transform_hit_test(
//         &self,
//         position: &<Affine2dCanvas as Canvas>::HitPosition,
//     ) -> <Affine2dCanvas as Canvas>::HitPosition {
//         todo!()
//     }

//     fn key(&self) -> Option<&Arc<dyn Key>> {
//         None
//     }

//     type CachedComposition = Arc<Affine2dEncoding>;

//     const CACHED_COMPOSITION_FUNCTION_TABLE: Option<CachedCompositionFunctionTable<Self>> =
//         <Self as CachedLayer>::PERFORM_CACHED_COMPOSITION;
// }

// impl CachedLayer for RenderRoot {
//     fn composite_into_cache(
//         child_iterator: &mut impl ChildLayerProducingIterator<Affine2dCanvas>,
//     ) -> Self::CachedComposition {
//         let mut result = Affine2dEncoding::new();
//         use epgi_core::tree::ChildLayerOrFragmentRef::*;
//         child_iterator.for_each(|child| match child {
//             Fragment(encoding) => {
//                 Affine2dCanvas::composite_encoding(&mut result, encoding, None);
//                 Vec::new()
//             }
//             StructuredChild(ComposableChildLayer { config, layer }) => {
//                 layer.composite_to(&mut result, config)
//             }
//             AdoptedChild(ComposableAdoptedLayer { config, layer }) => {
//                 layer.composite_to(&mut result, config)
//             }
//         });
//         return Arc::new(result);
//     }

//     fn composite_from_cache_to(
//         encoding: &mut Affine2dEncoding,
//         cache: &Self::CachedComposition,
//         composition_config: &LayerCompositionConfig<Affine2dCanvas>,
//     ) {
//         todo!()
//     }
// }
