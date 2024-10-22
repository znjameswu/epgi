use crate::{
    foundation::{Arc, Aweak, LayerProtocol},
    scheduler::get_current_scheduler,
    sync::{ImplComposite, ImplHitTest, ImplLayout, ImplPaint},
    tree::{ArcAnyLayerRenderObject, AweakAnyLayerRenderObject},
};

use super::{ImplRenderObject, LayerPaint, Render, RenderAction, RenderBase, RenderObject};

pub trait ImplRender<R: RenderBase>: ImplRenderObject<R> + ImplLayout<R> {}

impl<I, R: RenderBase> ImplRender<R> for I where I: ImplRenderObject<R> + ImplLayout<R> {}

pub trait ImplFullRender<R: Render<Impl = Self>>:
    ImplRender<R> + ImplMaybeLayer<R> + ImplPaint<R> + ImplHitTest<R>
{
}

impl<I, R: Render<Impl = Self>> ImplFullRender<R> for I where
    I: ImplRender<R> + ImplMaybeLayer<R> + ImplPaint<R> + ImplHitTest<R>
{
}

pub struct RenderImpl<
    const SIZED_BY_PARENT: bool,
    const LAYER_PAINT: bool,
    const CACHED_COMPOSITE: bool,
    const ORPHAN_LAYER: bool,
>;

pub trait ImplMaybeLayer<R: Render<Impl = Self>> {
    const IS_LAYER: bool;
    fn into_arc_any_layer_render_object(
        render_object: Arc<RenderObject<R>>,
    ) -> ArcAnyLayerRenderObject
    where
        Self: ImplFullRender<R>;

    fn into_aweak_any_layer_render_object(
        render_object: Aweak<RenderObject<R>>,
    ) -> AweakAnyLayerRenderObject
    where
        Self: ImplFullRender<R>;

    /// Returns the render action that should be passed to the parent.
    /// The render action is less or equal to the child_render_action,
    /// because some of the action may be absorbed by the corresponding boundaries.
    fn maybe_layer_mark_render_action(
        render_object: &Arc<RenderObject<R>>,
        propagated_render_action: Option<RenderAction>,
        descendant_has_action: Option<RenderAction>,
    ) -> Option<RenderAction>
    where
        Self: ImplFullRender<R>;
}

impl<
        R: Render<Impl = Self>,
        const SIZED_BY_PARENT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ImplMaybeLayer<R> for RenderImpl<SIZED_BY_PARENT, false, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    Self: ImplRender<R>,
    Self: ImplPaint<R>,
{
    const IS_LAYER: bool = false;

    fn into_arc_any_layer_render_object(
        _render_object: Arc<RenderObject<R>>,
    ) -> ArcAnyLayerRenderObject
    where
        Self: ImplFullRender<R>,
    {
        unreachable!()
    }

    fn into_aweak_any_layer_render_object(
        _render_object: Aweak<RenderObject<R>>,
    ) -> AweakAnyLayerRenderObject
    where
        Self: ImplFullRender<R>,
    {
        unreachable!()
    }

    fn maybe_layer_mark_render_action(
        _render_object: &Arc<RenderObject<R>>,
        propagated_render_action: Option<RenderAction>,
        _descendant_has_action: Option<RenderAction>,
    ) -> Option<RenderAction>
    where
        Self: ImplFullRender<R>,
    {
        propagated_render_action
    }
}

impl<
        R: Render<Impl = Self>,
        const SIZED_BY_PARENT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ImplMaybeLayer<R> for RenderImpl<SIZED_BY_PARENT, true, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    Self: ImplRender<R>,
    Self: ImplComposite<R>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    const IS_LAYER: bool = true;

    fn into_arc_any_layer_render_object(
        render_object: Arc<RenderObject<R>>,
    ) -> ArcAnyLayerRenderObject
    where
        Self: ImplFullRender<R>,
    {
        render_object
    }

    fn into_aweak_any_layer_render_object(
        render_object: Aweak<RenderObject<R>>,
    ) -> AweakAnyLayerRenderObject
    where
        Self: ImplFullRender<R>,
    {
        render_object
    }

    fn maybe_layer_mark_render_action(
        render_object: &Arc<RenderObject<R>>,
        mut propagated_render_action: Option<RenderAction>,
        _descendant_has_action: Option<RenderAction>,
    ) -> Option<RenderAction>
    where
        Self: ImplFullRender<R>,
    {
        if propagated_render_action == Some(RenderAction::Repaint) {
            get_current_scheduler()
                .push_layer_render_objects_needing_paint(Arc::downgrade(render_object) as _);
            propagated_render_action = Some(RenderAction::Recomposite)
        }
        if propagated_render_action == Some(RenderAction::Recomposite) {
            render_object.layer_mark.set_needs_composite()
        }
        return propagated_render_action;
    }
}
