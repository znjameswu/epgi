use crate::{
    foundation::{Arc, Aweak, LayerProtocol},
    sync::{ImplAdopterLayer, ImplComposite, ImplHitTest, ImplLayout, ImplPaint},
    tree::{ArcAnyLayerRenderObject, AweakAnyLayerRenderObject},
};

use super::{ImplRenderObject, LayerPaint, Render, RenderBase, RenderObject};

pub trait ImplRender<R: RenderBase>:
    ImplRenderObject<R> + ImplLayout<R> + ImplAdopterLayer<R>
{
}

impl<I, R: RenderBase> ImplRender<R> for I where
    I: ImplRenderObject<R> + ImplLayout<R> + ImplAdopterLayer<R>
{
}

pub trait ImplFullRender<R: Render<Impl = Self>>:
    ImplRender<R> + ImplLayerRenderObjectCast<R> + ImplPaint<R> + ImplHitTest<R>
{
}

impl<I, R: Render<Impl = Self>> ImplFullRender<R> for I where
    I: ImplRender<R> + ImplLayerRenderObjectCast<R> + ImplPaint<R> + ImplHitTest<R>
{
}

pub struct RenderImpl<
    const DRY_LAYOUT: bool,
    const LAYER_PAINT: bool,
    const CACHED_COMPOSITE: bool,
    const ORPHAN_LAYER: bool,
>;

pub trait ImplLayerRenderObjectCast<R: Render<Impl = Self>> {
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
}

impl<
        R: Render<Impl = Self>,
        const DRY_LAYOUT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ImplLayerRenderObjectCast<R> for RenderImpl<DRY_LAYOUT, false, CACHED_COMPOSITE, ORPHAN_LAYER>
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
        render_object: Aweak<RenderObject<R>>,
    ) -> AweakAnyLayerRenderObject
    where
        Self: ImplFullRender<R>,
    {
        unreachable!()
    }
}

impl<
        R: Render<Impl = Self>,
        const DRY_LAYOUT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ImplLayerRenderObjectCast<R> for RenderImpl<DRY_LAYOUT, true, CACHED_COMPOSITE, ORPHAN_LAYER>
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
}
