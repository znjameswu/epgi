use crate::{
    foundation::{Arc, ArrayContainer, Canvas, LayerProtocol, Protocol},
    tree::{
        ArcAnyLayerNode, ArcChildLayerNode, AweakAnyLayerNode, Layer, LayerMark, LayerNode,
        PaintCache,
    },
};

use super::{Render, RenderAction};

pub trait LayerOrUnit<R: Render>: Send + Sync + 'static {
    type ArcLayerNode: Clone + Send + Sync + 'static;
    const LAYER_RENDER_FUNCTION_TABLE: LayerRenderFunctionTable<R>;

    type LayerMark: Send + Sync + 'static;

    type PaintResults: Send + Sync + 'static;

    fn mark_render_action(
        layer_node: &Self::ArcLayerNode,
        child_render_action: RenderAction,
        subtree_has_action: RenderAction,
    ) -> RenderAction;

    fn create_layer_mark() -> Self::LayerMark;
}

impl<R, L> LayerOrUnit<R> for L
where
    R: Render<LayerOrUnit = L>,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
    L: Layer<
        ParentCanvas = <R::ParentProtocol as Protocol>::Canvas,
        ChildCanvas = <R::ChildProtocol as Protocol>::Canvas,
    >,
{
    type ArcLayerNode = Arc<LayerNode<L>>;

    type LayerMark = LayerMark;

    type PaintResults = PaintCache<L::ChildCanvas, L::CachedComposition>;

    const LAYER_RENDER_FUNCTION_TABLE: LayerRenderFunctionTable<R> =
        LayerRenderFunctionTable::LayerNode {
            into_arc_any_layer_node: |x| x,
            as_aweak_any_layer_node: |x| Arc::downgrade(x) as _,
            into_arc_child_layer_node: |x| x,
            create_arc_layer_node: |render| todo!(),
            get_canvas_transform_ref: |x| x,
            get_canvas_transform: |x| x,
        };

    fn mark_render_action(
        layer_node: &Arc<LayerNode<L>>,
        child_render_action: RenderAction,
        subtree_has_action: RenderAction,
    ) -> RenderAction {
        layer_node.mark_render_action(child_render_action, subtree_has_action)
    }

    fn create_layer_mark() -> LayerMark {
        LayerMark::new()
    }
}

impl<R> LayerOrUnit<R> for ()
where
    R: Render<LayerOrUnit = Self>,
{
    type ArcLayerNode = ();

    type LayerMark = ();

    type PaintResults = ();

    const LAYER_RENDER_FUNCTION_TABLE: LayerRenderFunctionTable<R> =
        LayerRenderFunctionTable::None { create: || () };

    fn mark_render_action(
        _layer_node: &(),
        child_render_action: RenderAction,
        _subtree_has_action: RenderAction,
    ) -> RenderAction {
        child_render_action
    }

    fn create_layer_mark() -> () {
        ()
    }
}

pub enum LayerRenderFunctionTable<R: Render> {
    LayerNode {
        into_arc_any_layer_node: fn(ArcLayerNodeOf<R>) -> ArcAnyLayerNode,
        as_aweak_any_layer_node: fn(&ArcLayerNodeOf<R>) -> AweakAnyLayerNode,
        into_arc_child_layer_node:
            fn(ArcLayerNodeOf<R>) -> ArcChildLayerNode<<R::ParentProtocol as Protocol>::Canvas>,
        create_arc_layer_node: fn(&R) -> ArcLayerNodeOf<R>,
        get_canvas_transform_ref:
            fn(
                &<R::ParentProtocol as Protocol>::Transform,
            ) -> &<<R::ParentProtocol as Protocol>::Canvas as Canvas>::Transform,
        get_canvas_transform: fn(
            <R::ParentProtocol as Protocol>::Transform,
        )
            -> <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Transform,
    },
    // // pub update_layer_node: fn(&R, &R::ArcLayerNode) -> LayerNodeUpdateResult,
    None {
        create: fn() -> ArcLayerNodeOf<R>,
    },
}

impl<R> LayerRenderFunctionTable<R>
where
    R: Render,
{
    pub const fn is_some(&self) -> bool {
        matches!(self, LayerRenderFunctionTable::LayerNode { .. })
    }
}

#[allow(type_alias_bounds)]
pub(crate) type ArcLayerNodeOf<R: Render> = <R::LayerOrUnit as LayerOrUnit<R>>::ArcLayerNode;

pub(crate) const fn layer_render_function_table_of<R: Render>() -> LayerRenderFunctionTable<R> {
    <R::LayerOrUnit as LayerOrUnit<R>>::LAYER_RENDER_FUNCTION_TABLE
}

pub(crate) const fn render_has_layer<R: Render>() -> bool {
    <R::LayerOrUnit as LayerOrUnit<R>>::LAYER_RENDER_FUNCTION_TABLE.is_some()
}
