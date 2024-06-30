use std::any::TypeId;

use epgi_core::{
    foundation::{
        AnyRawPointer, Arc, Asc, Canvas, Key, LayerProtocol, PaintContext, Protocol, VecContainer,
    },
    template::{
        ImplByTemplate, TemplateCachedComposite, TemplateComposite, TemplateHitTest,
        TemplateLayerPaint, TemplateLayout, TemplateLayoutByParent, TemplateOrphanLayer,
        TemplatePaint, TemplateRender, TemplateRenderBase,
    },
    tree::{
        ArcChildRenderObject, ChildLayerProducingIterator, HitTestContext, HitTestResult,
        ImplRender, LayerCompositionConfig, PaintResults, RecordedChildLayer, Render, RenderBase,
        RenderImpl, RenderObject,
    },
};

use crate::{Affine2dCanvas, Affine2dEncoding, Point2d};

pub struct Affine2dMultiChildRenderTemplate<
    const SIZED_BY_PARENT: bool,
    const LAYER_PAINT: bool,
    const CACHED_COMPOSITE: bool,
    const ORPHAN_LAYER: bool,
>;

pub trait Affine2dMultiChildRender: Send + Sync + Sized + 'static {
    type ParentProtocol: Protocol<Canvas = Affine2dCanvas>;
    type ChildProtocol: Protocol<Canvas = Affine2dCanvas>;
    type LayoutMemo: Send + Sync;

    fn detach(&mut self) {}
    const NOOP_DETACH: bool = false;
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateRenderBase<R>
    for Affine2dMultiChildRenderTemplate<
        SIZED_BY_PARENT,
        LAYER_PAINT,
        CACHED_COMPOSITE,
        ORPHAN_LAYER,
    >
where
    R: ImplByTemplate<Template = Self>,
    R: Affine2dMultiChildRender,
{
    type ParentProtocol = R::ParentProtocol;
    type ChildProtocol = R::ChildProtocol;
    type ChildContainer = VecContainer;

    type LayoutMemo = R::LayoutMemo;

    fn detach(render: &mut R) {
        R::detach(render)
    }

    const NOOP_DETACH: bool = R::NOOP_DETACH;
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateRender<R>
    for Affine2dMultiChildRenderTemplate<
        SIZED_BY_PARENT,
        LAYER_PAINT,
        CACHED_COMPOSITE,
        ORPHAN_LAYER,
    >
where
    R: RenderBase,
    RenderImpl<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>: ImplRender<R>,
{
    type RenderImpl = RenderImpl<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>;
}

/// Layout-by-parent means that under all circumstances, this render object's size is solely determined
/// by the constraints given by its parents.
///
/// Since the size of its children does not affect its own size,
/// this render object will always serves as a relayout boundary.
///
/// Contrary to what you may assume, layout-by-parent itself does not bring
/// any additional optimization during the actual layout visit.
/// It still needs to layout its children if dirty or receiving a new constraints.
/// It merely serves a boundary to halt relayout propagation.
pub trait Affine2dMultiChildLayout: Affine2dMultiChildRender {
    fn perform_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        children: &Vec<ArcChildRenderObject<Self::ChildProtocol>>,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo);
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateLayout<R>
    for Affine2dMultiChildRenderTemplate<
        SIZED_BY_PARENT,
        LAYER_PAINT,
        CACHED_COMPOSITE,
        ORPHAN_LAYER,
    >
where
    R: ImplByTemplate<Template = Self>,
    R: Affine2dMultiChildLayout,
{
    fn perform_layout(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        children: &Vec<ArcChildRenderObject<R::ChildProtocol>>,
    ) -> (<R::ParentProtocol as Protocol>::Size, R::LayoutMemo) {
        R::perform_layout(render, constraints, children)
    }
}

pub trait Affine2dMultiChildLayoutByParent: Affine2dMultiChildRender {
    fn compute_size_by_parent(
        &self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
    ) -> <Self::ParentProtocol as Protocol>::Size;

    fn perform_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &<Self::ParentProtocol as Protocol>::Size,
        children: &Vec<ArcChildRenderObject<Self::ChildProtocol>>,
    ) -> Self::LayoutMemo;
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateLayoutByParent<R>
    for Affine2dMultiChildRenderTemplate<
        SIZED_BY_PARENT,
        LAYER_PAINT,
        CACHED_COMPOSITE,
        ORPHAN_LAYER,
    >
where
    R: ImplByTemplate<Template = Self>,
    R: Affine2dMultiChildLayoutByParent,
{
    fn compute_size_by_parent(
        render: &R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
    ) -> <R::ParentProtocol as Protocol>::Size {
        R::compute_size_by_parent(render, constraints)
    }

    fn perform_layout(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        size: &<R::ParentProtocol as Protocol>::Size,
        children: &Vec<ArcChildRenderObject<R::ChildProtocol>>,
    ) -> R::LayoutMemo {
        R::perform_layout(render, constraints, size, children)
    }
}

pub trait Affine2dMultiChildPaint: Affine2dMultiChildRender {
    fn perform_paint(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &Vec<ArcChildRenderObject<Self::ChildProtocol>>,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    );
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplatePaint<R>
    for Affine2dMultiChildRenderTemplate<
        SIZED_BY_PARENT,
        LAYER_PAINT,
        CACHED_COMPOSITE,
        ORPHAN_LAYER,
    >
where
    R: ImplByTemplate<Template = Self>,
    R: Affine2dMultiChildPaint,
{
    fn perform_paint(
        render: &R,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
        children: &Vec<ArcChildRenderObject<R::ChildProtocol>>,
        paint_ctx: &mut impl PaintContext<Canvas = <R::ParentProtocol as Protocol>::Canvas>,
    ) {
        R::perform_paint(render, size, offset, memo, children, paint_ctx)
    }
}

pub trait Affine2dMultiChildLayerPaint: Affine2dMultiChildRender
where
    Self::ParentProtocol: LayerProtocol,
    Self::ChildProtocol: LayerProtocol,
{
    fn paint_layer(
        &self,
        children: &Vec<ArcChildRenderObject<Self::ChildProtocol>>,
    ) -> PaintResults<<Self::ParentProtocol as Protocol>::Canvas> {
        <Self::ParentProtocol as Protocol>::Canvas::paint_render_objects(children.clone())
    }

    fn transform_config(
        self_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
        child_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas> {
        unimplemented!()
    }

    fn layer_key(&self) -> Option<&Arc<dyn Key>> {
        None
    }
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateLayerPaint<R>
    for Affine2dMultiChildRenderTemplate<
        SIZED_BY_PARENT,
        LAYER_PAINT,
        CACHED_COMPOSITE,
        ORPHAN_LAYER,
    >
where
    R: ImplByTemplate<Template = Self>,
    R: Affine2dMultiChildLayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn paint_layer(
        render: &R,
        children: &Vec<ArcChildRenderObject<R::ChildProtocol>>,
    ) -> PaintResults<<R::ChildProtocol as Protocol>::Canvas> {
        R::paint_layer(render, children)
    }

    fn transform_config(
        self_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
        child_config: &LayerCompositionConfig<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas> {
        R::transform_config(self_config, child_config)
    }

    fn layer_key(render: &R) -> Option<&Arc<dyn Key>> {
        R::layer_key(render)
    }
}

pub trait Affine2dMultiChildComposite: Affine2dMultiChildRender {
    fn composite_to(
        &self,
        encoding: &mut Affine2dEncoding,
        child_iterator: &mut ChildLayerProducingIterator<
            <Self::ParentProtocol as Protocol>::Canvas,
        >,
        composition_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
    );
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateComposite<R>
    for Affine2dMultiChildRenderTemplate<
        SIZED_BY_PARENT,
        LAYER_PAINT,
        CACHED_COMPOSITE,
        ORPHAN_LAYER,
    >
where
    R: ImplByTemplate<Template = Self>,
    R: Affine2dMultiChildComposite,
{
    fn composite_to(
        render: &R,
        encoding: &mut Affine2dEncoding,
        child_iterator: &mut ChildLayerProducingIterator<<R::ParentProtocol as Protocol>::Canvas>,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
    ) {
        R::composite_to(render, encoding, child_iterator, composition_config)
    }
}

pub trait Affine2dMultiChildCachedComposite: Affine2dMultiChildRender {
    type CompositionMemo: Send + Sync + Clone + 'static;

    fn composite_into_memo(
        &self,
        child_iterator: &mut ChildLayerProducingIterator<
            <Self::ParentProtocol as Protocol>::Canvas,
        >,
    ) -> Self::CompositionMemo;

    fn composite_from_cache_to(
        &self,
        encoding: &mut Affine2dEncoding,
        memo: &Self::CompositionMemo,
        composition_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
    );
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateCachedComposite<R>
    for Affine2dMultiChildRenderTemplate<
        SIZED_BY_PARENT,
        LAYER_PAINT,
        CACHED_COMPOSITE,
        ORPHAN_LAYER,
    >
where
    R: ImplByTemplate<Template = Self>,
    R: Affine2dMultiChildCachedComposite,
{
    type CompositionMemo = R::CompositionMemo;

    fn composite_into_memo(
        render: &R,
        child_iterator: &mut ChildLayerProducingIterator<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> R::CompositionMemo {
        R::composite_into_memo(render, child_iterator)
    }

    fn composite_from_cache_to(
        render: &R,
        encoding: &mut Affine2dEncoding,
        memo: &R::CompositionMemo,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
    ) {
        R::composite_from_cache_to(render, encoding, memo, composition_config)
    }
}

pub trait Affine2dMultiChildOrphanLayer: Affine2dMultiChildLayerPaint
where
    Self::ParentProtocol: LayerProtocol,
    Self::ChildProtocol: LayerProtocol,
{
    fn adopter_key(&self) -> &Asc<dyn Key>;
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateOrphanLayer<R>
    for Affine2dMultiChildRenderTemplate<
        SIZED_BY_PARENT,
        LAYER_PAINT,
        CACHED_COMPOSITE,
        ORPHAN_LAYER,
    >
where
    R: ImplByTemplate<Template = Self>,
    R: Affine2dMultiChildOrphanLayer,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn adopter_key(render: &R) -> &Asc<dyn Key> {
        R::adopter_key(render)
    }
}

pub trait Affine2dMultiChildHitTest: Affine2dMultiChildRender {
    /// The actual method that was invoked for hit-testing.
    ///
    /// Note however, this method is hard to impl directly. Therefore, if not for rare edge cases,
    /// it is recommended to implement [HitTest::hit_test_children], [HitTest::hit_test_self],
    /// and [HitTest::hit_test_behavior] instead. This method has a default impl that is composed on top of those method.
    ///
    /// If you do indeed overwrite the default impl of this method without using the other methods,
    /// you can assume the other methods mentioned above are `unreachable!()`.
    fn hit_test(
        &self,
        ctx: &mut HitTestContext<<Self::ParentProtocol as Protocol>::Canvas>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &Vec<ArcChildRenderObject<Self::ChildProtocol>>,
        adopted_children: &[RecordedChildLayer<<Self::ParentProtocol as Protocol>::Canvas>],
    ) -> HitTestResult {
        use HitTestResult::*;
        let hit_in_bound =
            Self::ParentProtocol::position_in_shape(ctx.curr_position(), offset, size);
        if !hit_in_bound {
            return NotHit;
        }

        let hit_children =
            self.hit_test_children(ctx, size, offset, memo, children, adopted_children);
        if hit_children {
            return Hit;
        }
        // We have not hit any children. Now it up to us ourself.
        let hit_self = self.hit_test_self(ctx.curr_position(), size, offset, memo);
        return hit_self;
    }

    /// Returns: If a child has claimed the hit
    #[allow(unused_variables)]
    fn hit_test_children(
        &self,
        ctx: &mut HitTestContext<<Self::ParentProtocol as Protocol>::Canvas>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &Vec<ArcChildRenderObject<Self::ChildProtocol>>,
        adopted_children: &[RecordedChildLayer<<Self::ParentProtocol as Protocol>::Canvas>],
    ) -> bool {
        for child in children.iter() {
            if ctx.hit_test(child.clone()) {
                return true;
            }
        }
        return false;
    }

    // The reason we separate hit_test_self from hit_test_children is that we do not wish to leak hit_position into hit_test_children
    // Therefore preventing implementer to perform transform on hit_position rather than recording it in
    #[allow(unused_variables)]
    fn hit_test_self(
        &self,
        position: &Point2d,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
    ) -> HitTestResult {
        HitTestResult::NotHit
    }

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<Self>) -> AnyRawPointer)]
    where
        Self: Render,
    {
        &[]
    }
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateHitTest<R>
    for Affine2dMultiChildRenderTemplate<
        SIZED_BY_PARENT,
        LAYER_PAINT,
        CACHED_COMPOSITE,
        ORPHAN_LAYER,
    >
where
    R: ImplByTemplate<Template = Self>,
    R: Affine2dMultiChildHitTest,
{
    fn hit_test(
        render: &R,
        ctx: &mut HitTestContext<<R::ParentProtocol as Protocol>::Canvas>,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
        children: &Vec<ArcChildRenderObject<R::ChildProtocol>>,
        adopted_children: &[RecordedChildLayer<<R::ParentProtocol as Protocol>::Canvas>],
    ) -> HitTestResult {
        R::hit_test(render, ctx, size, offset, memo, children, adopted_children)
    }

    /// Returns: If a child has claimed the hit
    fn hit_test_children(
        _render: &R,
        _ctx: &mut HitTestContext<<R::ParentProtocol as Protocol>::Canvas>,
        _size: &<R::ParentProtocol as Protocol>::Size,
        _offset: &<R::ParentProtocol as Protocol>::Offset,
        _memo: &R::LayoutMemo,
        _children: &Vec<ArcChildRenderObject<R::ChildProtocol>>,
        _adopted_children: &[RecordedChildLayer<<R::ParentProtocol as Protocol>::Canvas>],
    ) -> bool {
        unreachable!(
            "TemplatePaint has already provided a hit_test implementation, \
            but hit_test_children is still invoked somehow. This indicates a framework bug."
        )
    }

    fn hit_test_self(
        _render: &R,
        _position: &Point2d,
        _size: &<R::ParentProtocol as Protocol>::Size,
        _offset: &<R::ParentProtocol as Protocol>::Offset,
        _memo: &R::LayoutMemo,
    ) -> HitTestResult {
        unreachable!(
            "TemplatePaint has already provided a hit_test implementation, \
            but hit_test_self is still invoked somehow. This indicates a framework bug."
        )
    }

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<R>) -> AnyRawPointer)]
    where
        R: Render,
    {
        <R as Affine2dMultiChildHitTest>::all_hit_test_interfaces()
    }
}
