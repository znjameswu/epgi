use crate::sync::{ImplAdopterLayer, ImplHitTest, ImplLayout, ImplPaint};

use super::{ImplRenderObject, RenderBase};

pub trait ImplRender<R: RenderBase>:
    ImplRenderObject<R> + ImplLayout<R> + ImplPaint<R> + ImplHitTest<R> + ImplAdopterLayer<R>
{
}

pub struct RenderImpl<
    const DRY_LAYOUT: bool,
    const LAYER_PAINT: bool,
    const CACHED_COMPOSITE: bool,
    const ORPHAN_LAYER: bool,
>;

impl<
        R: RenderBase,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ImplRender<R> for RenderImpl<DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    Self: ImplLayout<R>,
    Self: ImplPaint<R>,
    Self: ImplHitTest<R>,
    Self: ImplAdopterLayer<R>,
    Self: ImplRenderObject<R>,
{
}
