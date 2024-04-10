use crate::sync::{ImplAdopterLayer, ImplHitTest, ImplLayout, ImplPaint};

use super::{ImplRenderObject, Render, RenderBase};

pub trait ImplRender<R: RenderBase>:
    ImplRenderObject<R> + ImplLayout<R> + ImplAdopterLayer<R>
{
}

impl<I, R: RenderBase> ImplRender<R> for I where
    I: ImplRenderObject<R> + ImplLayout<R> + ImplAdopterLayer<R>
{
}

pub trait ImplFullRender<R: Render>: ImplRender<R> + ImplPaint<R> + ImplHitTest<R> {}

impl<I, R: Render> ImplFullRender<R> for I where I: ImplRender<R> + ImplPaint<R> + ImplHitTest<R> {}

pub struct RenderImpl<
    const DRY_LAYOUT: bool,
    const LAYER_PAINT: bool,
    const CACHED_COMPOSITE: bool,
    const ORPHAN_LAYER: bool,
>;
