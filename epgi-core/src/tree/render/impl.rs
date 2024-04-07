mod render_impl;
pub use render_impl::*;

mod impl_layout;
pub use impl_layout::*;

mod impl_paint;
pub use impl_paint::*;

mod impl_composite;
pub use impl_composite::*;

mod impl_hit_test;
pub use impl_hit_test::*;

mod impl_orphan;
pub use impl_orphan::*;

use crate::sync::{ImplAdopterLayer, ImplHitTest, ImplLayout, ImplPaint};

use super::{ImplRenderObject, Render};

pub trait ImplRender:
    ImplRenderObject<Self::Render>
    + ImplLayout<Self::Render>
    + ImplPaint<Self::Render>
    + ImplHitTest<Self::Render>
    + ImplAdopterLayer<Self::Render>
{
    type Render: Render;
}

pub trait ImplRenderBySuper:
    ImplRenderObject<Self::Render>
    + ImplLayout<Self::Render>
    + ImplPaint<Self::Render>
    + ImplHitTest<Self::Render>
    + ImplAdopterLayer<Self::Render>
{
    type Render: Render;
    type Super: ImplRender<Render = Self::Render>;
}

impl<T> ImplRender for T
where
    T: ImplRenderBySuper,
{
    type Render = T::Render;
}
