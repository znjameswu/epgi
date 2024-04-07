use std::marker::PhantomData;

use crate::{
    foundation::{Arc, AsIterator, Asc, Canvas, Key, LayerProtocol, PaintContext, Protocol},
    sync::{ImplAdopterLayer, ImplHitTest, ImplLayout, ImplPaint},
    tree::{
        ArcChildRenderObject, ChildLayerProducingIterator, ContainerOf, HitTestBehavior,
        HitTestResults, ImplRenderObject, LayerCompositionConfig, PaintResults, Render, TreeNode,
    },
};

use super::{
    HasCachedCompositeImpl, HasCompositeImpl, HasDryLayoutImpl, HasHitTestImpl, HasLayerPaintImpl,
    HasLayoutImpl, HasOrphanLayerImpl, HasPaintImpl, ImplRender,
};

pub struct RenderImpl<
    R: Render,
    const DRY_LAYOUT: bool,
    const LAYER_PAINT: bool,
    const CACHED_COMPOSITE: bool,
    const ORPHAN_LAYER: bool,
>(PhantomData<R>);

impl<
        R: Render,
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
pub trait Layout: Render {
    fn perform_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        children: &ContainerOf<Self, ArcChildRenderObject<Self::ChildProtocol>>,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo);
}

pub trait DryLayout: Render {
    fn compute_dry_layout(
        &self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
    ) -> <Self::ParentProtocol as Protocol>::Size;

    fn compute_layout_memo(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &<Self::ParentProtocol as Protocol>::Size,
        children: &ContainerOf<Self, ArcChildRenderObject<Self::ChildProtocol>>,
    ) -> Self::LayoutMemo;
}

impl<
        R: Render,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > HasLayoutImpl<R> for RenderImpl<R, false, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Layout,
{
    fn perform_layout(
        render: &mut R,
        constraints: &<<R>::ParentProtocol as Protocol>::Constraints,
        children: &ContainerOf<R, ArcChildRenderObject<<R>::ChildProtocol>>,
    ) -> (<<R>::ParentProtocol as Protocol>::Size, <R>::LayoutMemo) {
        R::perform_layout(render, constraints, children)
    }
}

impl<
        R: Render,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > HasDryLayoutImpl<R> for RenderImpl<R, true, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: DryLayout,
{
    fn compute_dry_layout(
        render: &R,
        constraints: &<<R>::ParentProtocol as Protocol>::Constraints,
    ) -> <<R>::ParentProtocol as Protocol>::Size {
        R::compute_dry_layout(render, constraints)
    }

    fn compute_layout_memo(
        render: &mut R,
        constraints: &<<R>::ParentProtocol as Protocol>::Constraints,
        size: &<<R>::ParentProtocol as Protocol>::Size,
        children: &ContainerOf<R, ArcChildRenderObject<<R>::ChildProtocol>>,
    ) -> <R>::LayoutMemo {
        R::compute_layout_memo(render, constraints, size, children)
    }
}

pub trait Paint: Render {
    fn perform_paint(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &ContainerOf<Self, ArcChildRenderObject<Self::ChildProtocol>>,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    );

    // fn hit_test_children(
    //     &self,
    //     size: &<Self::ParentProtocol as Protocol>::Size,
    //     offset: &<Self::ParentProtocol as Protocol>::Offset,
    //     memo: &Self::LayoutMemo,
    //     children: &ContainerOf<Self, ArcChildRenderObject<Self::ChildProtocol>>,
    //     results: &mut HitTestResults<<Self::ParentProtocol as Protocol>::Canvas>,
    // ) -> bool;

    // #[allow(unused_variables)]
    // fn hit_test_self(
    //     &self,
    //     position: &<<Self::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
    //     size: &<Self::ParentProtocol as Protocol>::Size,
    //     offset: &<Self::ParentProtocol as Protocol>::Offset,
    //     memo: &Self::LayoutMemo,
    // ) -> Option<HitTestBehavior> {
    //     <Self::ParentProtocol as Protocol>::position_in_shape(position, offset, size)
    //         .then_some(HitTestBehavior::DeferToChild)
    // }
}

pub trait LayerPaint: TreeNode
where
    Self::ParentProtocol: LayerProtocol,
    Self::ChildProtocol: LayerProtocol,
{
    fn paint_layer(
        &self,
        children: &ContainerOf<Self, ArcChildRenderObject<Self::ChildProtocol>>,
    ) -> PaintResults<<Self::ChildProtocol as Protocol>::Canvas> {
        <<Self::ChildProtocol as Protocol>::Canvas as Canvas>::paint_render_objects(
            children.as_iter().cloned(),
        )
    }

    // fn transform_config(
    //     self_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
    //     child_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    // ) -> LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>;

    fn layer_key(&self) -> Option<&Arc<dyn Key>> {
        None
    }
}

impl<R: Render, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool, const ORPHAN_LAYER: bool>
    HasPaintImpl<R> for RenderImpl<R, DRY_LAYOUT, false, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Paint,
{
    fn perform_paint(
        render: &R,
        size: &<<R>::ParentProtocol as Protocol>::Size,
        offset: &<<R>::ParentProtocol as Protocol>::Offset,
        memo: &<R>::LayoutMemo,
        children: &ContainerOf<R, ArcChildRenderObject<<R>::ChildProtocol>>,
        paint_ctx: &mut impl PaintContext<Canvas = <<R>::ParentProtocol as Protocol>::Canvas>,
    ) {
        R::perform_paint(render, size, offset, memo, children, paint_ctx)
    }
}

impl<R: Render, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool, const ORPHAN_LAYER: bool>
    HasLayerPaintImpl<R> for RenderImpl<R, DRY_LAYOUT, true, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn paint_layer(
        render: &R,
        children: &ContainerOf<R, ArcChildRenderObject<<R>::ChildProtocol>>,
    ) -> PaintResults<<<R>::ChildProtocol as Protocol>::Canvas> {
        R::paint_layer(render, children)
    }

    fn layer_key(render: &R) -> Option<&Arc<dyn Key>> {
        R::layer_key(render)
    }
}

pub trait Composite<
    AdopterCanvas: Canvas = <<Self as TreeNode>::ParentProtocol as Protocol>::Canvas,
>: TreeNode
{
    fn composite_to(
        &self,
        encoding: &mut AdopterCanvas::Encoding,
        child_iterator: &mut impl ChildLayerProducingIterator<<Self::ChildProtocol as Protocol>::Canvas>,
        composition_config: &LayerCompositionConfig<AdopterCanvas>,
    );

    fn transform_config(
        self_config: &LayerCompositionConfig<AdopterCanvas>,
        child_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<AdopterCanvas>;
}

pub trait CachedComposite<
    AdopterCanvas: Canvas = <<Self as TreeNode>::ParentProtocol as Protocol>::Canvas,
>: TreeNode
{
    type CompositionCache: Send + Sync + Clone + 'static;

    fn composite_into_cache(
        &self,
        child_iterator: &mut impl ChildLayerProducingIterator<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> Self::CompositionCache;

    fn composite_from_cache_to(
        &self,
        encoding: &mut AdopterCanvas::Encoding,
        cache: &Self::CompositionCache,
        composition_config: &LayerCompositionConfig<AdopterCanvas>,
    );

    fn transform_config(
        self_config: &LayerCompositionConfig<AdopterCanvas>,
        child_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<AdopterCanvas>;
}

impl<
        R: Render,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const ORPHAN_LAYER: bool,
        AdopterCanvas: Canvas,
    > HasCompositeImpl<R, AdopterCanvas>
    for RenderImpl<R, DRY_LAYOUT, LAYER_PAINT, false, ORPHAN_LAYER>
where
    R: Composite<AdopterCanvas>,
{
    fn composite_to(
        render: &R,
        encoding: &mut <AdopterCanvas as Canvas>::Encoding,
        child_iterator: &mut impl ChildLayerProducingIterator<<<R>::ChildProtocol as Protocol>::Canvas>,
        composition_config: &LayerCompositionConfig<AdopterCanvas>,
    ) {
        R::composite_to(render, encoding, child_iterator, composition_config)
    }

    fn transform_config(
        self_config: &LayerCompositionConfig<AdopterCanvas>,
        child_config: &LayerCompositionConfig<<<R>::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<AdopterCanvas> {
        R::transform_config(self_config, child_config)
    }
}

impl<
        R: Render,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const ORPHAN_LAYER: bool,
        AdopterCanvas: Canvas,
    > HasCachedCompositeImpl<R, AdopterCanvas>
    for RenderImpl<R, DRY_LAYOUT, LAYER_PAINT, true, ORPHAN_LAYER>
where
    R: CachedComposite<AdopterCanvas>,
{
    type CompositionCache = R::CompositionCache;

    fn composite_into_cache(
        render: &R,
        child_iterator: &mut impl ChildLayerProducingIterator<<<R>::ChildProtocol as Protocol>::Canvas>,
    ) -> Self::CompositionCache {
        R::composite_into_cache(render, child_iterator)
    }

    fn composite_from_cache_to(
        render: &R,
        encoding: &mut <AdopterCanvas as Canvas>::Encoding,
        cache: &Self::CompositionCache,
        composition_config: &LayerCompositionConfig<AdopterCanvas>,
    ) {
        R::composite_from_cache_to(render, encoding, cache, composition_config)
    }

    fn transform_config(
        self_config: &LayerCompositionConfig<AdopterCanvas>,
        child_config: &LayerCompositionConfig<<<R>::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<AdopterCanvas> {
        R::transform_config(self_config, child_config)
    }
}

/// Orphan layers can skip this implementation
pub trait HitTest: Render {
    fn hit_test_children(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &ContainerOf<Self, ArcChildRenderObject<Self::ChildProtocol>>,
        results: &mut HitTestResults<<Self::ParentProtocol as Protocol>::Canvas>,
    ) -> bool;

    #[allow(unused_variables)]
    fn hit_test_self(
        &self,
        position: &<<Self::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
    ) -> Option<HitTestBehavior> {
        <Self::ParentProtocol as Protocol>::position_in_shape(position, offset, size)
            .then_some(HitTestBehavior::DeferToChild)
    }
}

impl<
        R: Render,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > HasHitTestImpl<R> for RenderImpl<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: HitTest,
{
    fn hit_test_children(
        render: &R,
        size: &<<R>::ParentProtocol as Protocol>::Size,
        offset: &<<R>::ParentProtocol as Protocol>::Offset,
        memo: &<R>::LayoutMemo,
        children: &ContainerOf<R, ArcChildRenderObject<<R>::ChildProtocol>>,
        results: &mut HitTestResults<<<R>::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        R::hit_test_children(render, size, offset, memo, children, results)
    }

    fn hit_test_self(
        render: &R,
        position: &<<<R>::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
        size: &<<R>::ParentProtocol as Protocol>::Size,
        offset: &<<R>::ParentProtocol as Protocol>::Offset,
        memo: &<R>::LayoutMemo,
    ) -> Option<HitTestBehavior> {
        R::hit_test_self(render, position, size, offset, memo)
    }
}

// We COULD orthogonalize the Orphan/Structured vs Noncached/cached trait set,
// but that would inevitably bake directly into library user's code an explicit AdopterCanvas type
// either somewhere in an associated type or somewhere as a generic trait paramter.
// As an unproven idea, I would like to make orphan layer mechanism optional and not bake into anything more than necessary.
// Edit: We actually did orthogonalize these traits.
pub trait OrphanLayer: TreeNode + LayerPaint
where
    Self::ParentProtocol: LayerProtocol,
    Self::ChildProtocol: LayerProtocol,
{
    fn adopter_key(&self) -> &Asc<dyn Key>;
}

impl<R: Render, const DRY_LAYOUT: bool, const LAYER_PAINT: bool, const CACHED_COMPOSITE: bool>
    HasOrphanLayerImpl<R> for RenderImpl<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, true>
where
    R: OrphanLayer,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn adopter_key(render: &R) -> &Asc<dyn Key> {
        R::adopter_key(render)
    }
}
