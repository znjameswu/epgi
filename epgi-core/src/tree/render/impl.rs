use std::marker::PhantomData;

use crate::sync::{ImplAdopterLayer, ImplHitTest, ImplLayout, ImplPaint};

use super::{ImplRenderObject, RenderBase};

pub trait ImplRender:
    ImplRenderObject<Self::Render>
    + ImplLayout<Self::Render>
    + ImplPaint<Self::Render>
    + ImplHitTest<Self::Render>
    + ImplAdopterLayer<Self::Render>
{
    type Render: RenderBase;
}

pub struct RenderImpl<
    R: RenderBase,
    const DRY_LAYOUT: bool,
    const LAYER_PAINT: bool,
    const CACHED_COMPOSITE: bool,
    const ORPHAN_LAYER: bool,
>(PhantomData<R>);

impl<
        R: RenderBase,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ImplRender for RenderImpl<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    Self: ImplLayout<R>,
    Self: ImplPaint<R>,
    Self: ImplHitTest<R>,
    Self: ImplAdopterLayer<R>,
    Self: ImplRenderObject<R>,
{
    type Render = R;
}
