use crate::{
    foundation::{Arc, Canvas, LayerProtocol, Protocol},
    scheduler::get_current_scheduler,
    tree::{
        ArcAnyLayerRenderObject, ArcChildLayerRenderObject, AweakAnyLayerRenderObject, LayerMark,
        LayerRender, PaintCache,
    },
};

use super::{Render, RenderAction, RenderObject};

pub trait LayerOrUnit<R: Render>: Send + Sync + 'static {
    const LAYER_RENDER_FUNCTION_TABLE: LayerRenderFunctionTable<R>;

    type LayerMark: Send + Sync + 'static;

    type PaintResults: Send + Sync + 'static;

    fn create_layer_mark() -> Self::LayerMark;

    fn layer_mark_render_action(
        render_object: &Arc<RenderObject<R>>,
        child_render_action: RenderAction,
        subtree_has_action: RenderAction,
    ) -> RenderAction;

    fn downcast_arc_any_layer_render_object(
        render_object: Arc<RenderObject<R>>,
    ) -> Option<ArcAnyLayerRenderObject>;
}

impl<R> LayerOrUnit<R> for R
where
    R: LayerRender,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
{
    type LayerMark = LayerMark;

    type PaintResults = PaintCache<<R::ChildProtocol as Protocol>::Canvas, R::CachedComposition>;

    const LAYER_RENDER_FUNCTION_TABLE: LayerRenderFunctionTable<R> =
        LayerRenderFunctionTable::LayerRender {
            as_aweak_any_layer_render_object: |x| Arc::downgrade(x) as _,
            into_arc_child_layer_render_object: |x| x,
            get_canvas_transform_ref: |x| x,
            get_canvas_transform: |x| x,
        };

    fn create_layer_mark() -> LayerMark {
        LayerMark::new()
    }

    fn downcast_arc_any_layer_render_object(
        render_object: Arc<RenderObject<R>>,
    ) -> Option<ArcAnyLayerRenderObject> {
        Some(render_object as _)
    }

    fn layer_mark_render_action(
        render_object: &Arc<RenderObject<R>>,
        mut child_render_action: RenderAction,
        subtree_has_action: RenderAction,
    ) -> RenderAction {
        if child_render_action == RenderAction::Repaint {
            get_current_scheduler()
                .push_layer_render_objects_needing_paint(Arc::downgrade(render_object) as _);
            child_render_action = RenderAction::Recomposite
        }
        if child_render_action == RenderAction::Recomposite {
            render_object.layer_mark.set_needs_composite()
        }
        // if subtree_has_action == RenderAction::Recomposite
        return child_render_action;
    }
}

impl<R> LayerOrUnit<R> for ()
where
    R: Render<LayerOrUnit = Self>,
{
    type LayerMark = ();

    type PaintResults = ();

    const LAYER_RENDER_FUNCTION_TABLE: LayerRenderFunctionTable<R> =
        LayerRenderFunctionTable::None {};

    fn create_layer_mark() -> () {
        ()
    }

    fn downcast_arc_any_layer_render_object(
        render_object: Arc<RenderObject<R>>,
    ) -> Option<ArcAnyLayerRenderObject> {
        None
    }

    fn layer_mark_render_action(
        render_object: &Arc<RenderObject<R>>,
        child_render_action: RenderAction,
        subtree_has_action: RenderAction,
    ) -> RenderAction {
        child_render_action
    }
}

pub enum LayerRenderFunctionTable<R: Render> {
    LayerRender {
        as_aweak_any_layer_render_object: fn(&Arc<RenderObject<R>>) -> AweakAnyLayerRenderObject,
        into_arc_child_layer_render_object:
            fn(
                Arc<RenderObject<R>>,
            ) -> ArcChildLayerRenderObject<<R::ParentProtocol as Protocol>::Canvas>,
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
    None {},
}

impl<R> LayerRenderFunctionTable<R>
where
    R: Render,
{
    pub const fn is_some(&self) -> bool {
        matches!(self, LayerRenderFunctionTable::LayerRender { .. })
    }
}

// #[allow(type_alias_bounds)]
// pub(crate) type ArcLayerNodeOf<R: Render> = <R::LayerOrUnit as LayerOrUnit<R>>::ArcLayerNode;

pub(crate) const fn layer_render_function_table_of<R: Render>() -> LayerRenderFunctionTable<R> {
    <R::LayerOrUnit as LayerOrUnit<R>>::LAYER_RENDER_FUNCTION_TABLE
}

pub(crate) const fn render_has_layer<R: Render>() -> bool {
    <R::LayerOrUnit as LayerOrUnit<R>>::LAYER_RENDER_FUNCTION_TABLE.is_some()
}
