use crate::{
    foundation::{Arc, Canvas, LayerProtocol, Protocol},
    sync::SubtreeRenderObjectChange,
    tree::{ArcAnyLayerNode, ArcChildLayerNode, AscLayerContextNode, Layer, LayerNode},
};

use super::{LayerRender, Render, RenderAction};

pub trait LayerOrUnit<R: Render>: Send + Sync + 'static {
    type ArcLayerNode: Clone + Send + Sync + 'static;
    const LAYER_RENDER_FUNCTION_TABLE: LayerRenderFunctionTable<R>;

    fn mark_render_action(
        child_render_action: RenderAction,
        subtree_has_action: RenderAction,
    ) -> RenderAction;
}

impl<R, L> LayerOrUnit<R> for L
where
    R: LayerRender<L>,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
    L: Layer<
        ParentCanvas = <R::ParentProtocol as Protocol>::Canvas,
        ChildCanvas = <R::ChildProtocol as Protocol>::Canvas,
    >,
{
    type ArcLayerNode = Arc<LayerNode<L>>;

    const LAYER_RENDER_FUNCTION_TABLE: LayerRenderFunctionTable<R> =
        LayerRenderFunctionTable::LayerNode {
            as_arc_any_layer_node: |x| x,
            as_arc_child_layer_node: |x| x,
            create_arc_layer_node: |render, layer_context| todo!(),
            get_canvas_transform_ref: |x| x,
            get_canvas_transform: |x| x,
        };

    fn mark_render_action(
        child_render_action: RenderAction,
        subtree_has_action: RenderAction,
    ) -> RenderAction {
        todo!()
    }
}

impl<R> LayerOrUnit<R> for ()
where
    R: Render<LayerOrUnit = Self>,
{
    type ArcLayerNode = ();

    const LAYER_RENDER_FUNCTION_TABLE: LayerRenderFunctionTable<R> =
        LayerRenderFunctionTable::None { create: || () };

    fn mark_render_action(
        child_render_action: RenderAction,
        _subtree_has_action: RenderAction,
    ) -> RenderAction {
        child_render_action
    }
}

pub enum LayerRenderFunctionTable<R: Render> {
    LayerNode {
        as_arc_any_layer_node: fn(ArcLayerNodeOf<R>) -> ArcAnyLayerNode,
        as_arc_child_layer_node:
            fn(ArcLayerNodeOf<R>) -> ArcChildLayerNode<<R::ParentProtocol as Protocol>::Canvas>,
        create_arc_layer_node: fn(&R, &AscLayerContextNode) -> ArcLayerNodeOf<R>,
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
