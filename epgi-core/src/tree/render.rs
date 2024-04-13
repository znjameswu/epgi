mod mark;
pub use mark::*;

mod render_object;
pub use render_object::*;

mod r#impl;
pub use r#impl::*;

mod render_cache;
pub use render_cache::*;

mod layer_cache;
pub use layer_cache::*;

mod layer_iterator;
pub use layer_iterator::*;

mod layer_child;
pub use layer_child::*;

mod hit_test;
pub use hit_test::*;

use crate::foundation::{
    Arc, AsIterator, Asc, Canvas, ContainerOf, HktContainer, Key, LayerProtocol, PaintContext,
    Protocol,
};

use super::ArcElementContextNode;

pub type ContainerOfRender<E, T> =
    <<E as RenderBase>::ChildContainer as HktContainer>::Container<T>;

pub trait RenderBase: Send + Sync + Sized + 'static {
    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;
    type ChildContainer: HktContainer;

    type LayoutMemo: Send + Sync;

    fn detach(&mut self) {}
    const NOOP_DETACH: bool = false;
}

pub trait Render: RenderBase + HitTest {
    type Impl: ImplRender<Self>;
}

pub trait FullRender: Render<Impl = <Self as FullRender>::Impl> {
    type Impl: ImplFullRender<Self>;
}

impl<R> FullRender for R
where
    R: Render,
    R::Impl: ImplFullRender<R>,
{
    type Impl = R::Impl;
}

/// Dry layout means that under all circumstances, this render object's size is solely determined
/// by the constraints given by its parents.
///
/// Since the size of its children does not affect its own size,
/// this render object will always serves as a relayout boundary.
///
/// Contrary to what you may assume, dry-layout itself does not bring
/// any additional optimization during the actual layout visit.
/// It still needs to layout its children if dirty or receiving a new constraints.
/// It merely serves a boundary to halt relayout propagation.
pub trait Layout: RenderBase {
    fn perform_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        children: &ContainerOf<Self::ChildContainer, ArcChildRenderObject<Self::ChildProtocol>>,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo);
}

pub trait DryLayout: RenderBase {
    fn compute_dry_layout(
        &self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
    ) -> <Self::ParentProtocol as Protocol>::Size;

    fn perform_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &<Self::ParentProtocol as Protocol>::Size,
        children: &ContainerOf<Self::ChildContainer, ArcChildRenderObject<Self::ChildProtocol>>,
    ) -> Self::LayoutMemo;
}

pub trait Paint: RenderBase {
    fn perform_paint(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &ContainerOf<Self::ChildContainer, ArcChildRenderObject<Self::ChildProtocol>>,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    );
}

pub trait LayerPaint: RenderBase
where
    Self::ParentProtocol: LayerProtocol,
    Self::ChildProtocol: LayerProtocol,
{
    fn paint_layer(
        &self,
        children: &ContainerOf<Self::ChildContainer, ArcChildRenderObject<Self::ChildProtocol>>,
    ) -> PaintResults<<Self::ChildProtocol as Protocol>::Canvas> {
        <<Self::ChildProtocol as Protocol>::Canvas as Canvas>::paint_render_objects(
            children.as_iter().cloned(),
        )
    }

    fn transform_config(
        self_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
        child_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>;

    fn layer_key(&self) -> Option<&Arc<dyn Key>> {
        None
    }
}

pub trait Composite: RenderBase {
    fn composite_to(
        &self,
        encoding: &mut <<Self::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        child_iterator: &mut ChildLayerProducingIterator<<Self::ChildProtocol as Protocol>::Canvas>,
        composition_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
    );
}

pub trait CachedComposite: RenderBase {
    type CompositionMemo: Send + Sync + Clone + 'static;

    fn composite_into_memo(
        &self,
        child_iterator: &mut ChildLayerProducingIterator<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> Self::CompositionMemo;

    fn composite_from_cache_to(
        &self,
        encoding: &mut <<Self::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        memo: &Self::CompositionMemo,
        composition_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
    );
}

pub trait OrphanLayer: LayerPaint
where
    Self::ParentProtocol: LayerProtocol,
    Self::ChildProtocol: LayerProtocol,
{
    fn adopter_key(&self) -> &Asc<dyn Key>;
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub enum RenderAction {
    None,
    Recomposite,
    Repaint,
    Relayout,
}

impl Default for RenderAction {
    fn default() -> Self {
        RenderAction::None
    }
}

impl RenderAction {
    pub fn downgrade(self) -> Self {
        use RenderAction::*;
        match self {
            None => None,
            Recomposite => None,
            Repaint => Recomposite,
            Relayout => Repaint,
        }
    }

    pub fn absorb_relayout(self) -> Self {
        use RenderAction::*;
        match self {
            Relayout => Repaint,
            other => other,
        }
    }

    pub fn absorb_repaint(self) -> Self {
        use RenderAction::*;
        match self {
            Repaint => Recomposite,
            other => other,
        }
    }

    pub fn absorb_recomposite(self) -> Self {
        use RenderAction::*;
        match self {
            Recomposite => None,
            other => other,
        }
    }
}
