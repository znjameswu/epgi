use std::any::TypeId;

use crate::{
    foundation::{
        AnyRawPointer, Arc, Asc, Canvas, ContainerOf, HktContainer, Key, LayerProtocol,
        PaintContext, Protocol,
    },
    tree::{
        ArcChildRenderObject, CachedComposite, ChildLayerProducingIterator, Composite, DryLayout,
        HitTest, HitTestBehavior, HitTestResults, ImplRender, LayerCompositionConfig, LayerPaint,
        Layout, OrphanLayer, Paint, PaintResults, Render, RenderBase, RenderObject,
    },
};

use super::ImplByTemplate;

pub trait TemplateRenderBase<R> {
    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;
    type ChildContainer: HktContainer;

    type LayoutMemo: Send + Sync;

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<R>) -> AnyRawPointer)]
    where
        R: Render;

    fn detach(render: &mut R);
    const NOOP_DETACH: bool;
}

impl<R> RenderBase for R
where
    R: ImplByTemplate,
    R::Template: TemplateRenderBase<R>,
    R: Send + Sync + Sized + 'static,
{
    type ParentProtocol = <R::Template as TemplateRenderBase<R>>::ParentProtocol;
    type ChildProtocol = <R::Template as TemplateRenderBase<R>>::ChildProtocol;
    type ChildContainer = <R::Template as TemplateRenderBase<R>>::ChildContainer;

    type LayoutMemo = <R::Template as TemplateRenderBase<R>>::LayoutMemo;

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<Self>) -> AnyRawPointer)]
    where
        Self: Render,
    {
        R::Template::all_hit_test_interfaces()
    }

    fn detach(&mut self) {
        R::Template::detach(self)
    }
    const NOOP_DETACH: bool = R::Template::NOOP_DETACH;
}

pub trait TemplateRender<R: RenderBase> {
    type RenderImpl: ImplRender<R>;
}

impl<R> Render for R
where
    R: ImplByTemplate,
    R::Template: TemplateRender<R>,
    R: RenderBase,
{
    type Impl = <R::Template as TemplateRender<R>>::RenderImpl;
}

pub trait TemplateLayout<R: RenderBase> {
    fn perform_layout(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        children: &ContainerOf<R::ChildContainer, ArcChildRenderObject<R::ChildProtocol>>,
    ) -> (<R::ParentProtocol as Protocol>::Size, R::LayoutMemo);
}

impl<R> Layout for R
where
    R: ImplByTemplate,
    R::Template: TemplateLayout<R>,
    R: RenderBase,
{
    fn perform_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        children: &ContainerOf<Self::ChildContainer, ArcChildRenderObject<Self::ChildProtocol>>,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo) {
        R::Template::perform_layout(self, constraints, children)
    }
}

pub trait TemplateDryLayout<R: RenderBase> {
    fn compute_dry_layout(
        render: &R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
    ) -> <R::ParentProtocol as Protocol>::Size;

    fn compute_layout_memo(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        size: &<R::ParentProtocol as Protocol>::Size,
        children: &ContainerOf<R::ChildContainer, ArcChildRenderObject<R::ChildProtocol>>,
    ) -> R::LayoutMemo;
}

impl<R> DryLayout for R
where
    R: ImplByTemplate,
    R::Template: TemplateDryLayout<R>,
    R: RenderBase,
{
    fn compute_dry_layout(
        &self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
    ) -> <Self::ParentProtocol as Protocol>::Size {
        R::Template::compute_dry_layout(self, constraints)
    }

    fn perform_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &<Self::ParentProtocol as Protocol>::Size,
        children: &ContainerOf<Self::ChildContainer, ArcChildRenderObject<Self::ChildProtocol>>,
    ) -> Self::LayoutMemo {
        R::Template::compute_layout_memo(self, constraints, size, children)
    }
}

pub trait TemplatePaint<R: RenderBase> {
    fn perform_paint(
        render: &R,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
        children: &ContainerOf<R::ChildContainer, ArcChildRenderObject<R::ChildProtocol>>,
        paint_ctx: &mut impl PaintContext<Canvas = <R::ParentProtocol as Protocol>::Canvas>,
    );

    // fn hit_test_children(
    //     render: &R,
    //     size: &<R::ParentProtocol as Protocol>::Size,
    //     offset: &<R::ParentProtocol as Protocol>::Offset,
    //     memo: &R::LayoutMemo,
    //     children: &ContainerOf<R, ArcChildRenderObject<R::ChildProtocol>>,
    //     results: &mut HitTestResults<<R::ParentProtocol as Protocol>::Canvas>,
    // ) -> bool;

    // #[allow(unused_variables)]
    // fn hit_test_self(
    //     render: &R,
    //     position: &<<R::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
    //     size: &<R::ParentProtocol as Protocol>::Size,
    //     offset: &<R::ParentProtocol as Protocol>::Offset,
    //     memo: &R::LayoutMemo,
    // ) -> Option<HitTestBehavior> {
    //     <R::ParentProtocol as Protocol>::position_in_shape(position, offset, size)
    //         .then_some(HitTestBehavior::DeferToChild)
    // }
}

impl<R> Paint for R
where
    R: ImplByTemplate,
    R::Template: TemplatePaint<R>,
    R: RenderBase,
{
    fn perform_paint(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &ContainerOf<Self::ChildContainer, ArcChildRenderObject<Self::ChildProtocol>>,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) {
        R::Template::perform_paint(self, size, offset, memo, children, paint_ctx)
    }
}

pub trait TemplateLayerPaint<R: RenderBase>
where
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn paint_layer(
        render: &R,
        children: &ContainerOf<R::ChildContainer, ArcChildRenderObject<R::ChildProtocol>>,
    ) -> PaintResults<<R::ChildProtocol as Protocol>::Canvas>;

    fn transform_config(
        self_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
        child_config: &LayerCompositionConfig<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>;

    fn layer_key(render: &R) -> Option<&Arc<dyn Key>>;
}

impl<R> LayerPaint for R
where
    R: ImplByTemplate,
    R::Template: TemplateLayerPaint<R>,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
    R: RenderBase,
{
    fn paint_layer(
        &self,
        children: &ContainerOf<Self::ChildContainer, ArcChildRenderObject<Self::ChildProtocol>>,
    ) -> PaintResults<<Self::ChildProtocol as Protocol>::Canvas> {
        R::Template::paint_layer(self, children)
    }

    fn transform_config(
        self_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
        child_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas> {
        R::Template::transform_config(self_config, child_config)
    }

    fn layer_key(&self) -> Option<&Arc<dyn Key>> {
        R::Template::layer_key(self)
    }
}

pub trait TemplateComposite<R: RenderBase> {
    fn composite_to(
        render: &R,
        encoding: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        child_iterator: &mut ChildLayerProducingIterator<<R::ChildProtocol as Protocol>::Canvas>,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
    );

    fn transform_config(
        self_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
        child_config: &LayerCompositionConfig<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>;
}

impl<R> Composite for R
where
    R: ImplByTemplate,
    R::Template: TemplateComposite<R>,
    R: RenderBase,
{
    fn composite_to(
        &self,
        encoding: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        child_iterator: &mut ChildLayerProducingIterator<<Self::ChildProtocol as Protocol>::Canvas>,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
    ) {
        R::Template::composite_to(self, encoding, child_iterator, composition_config)
    }

    fn transform_config(
        self_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
        child_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas> {
        R::Template::transform_config(self_config, child_config)
    }
}

pub trait TemplateCachedComposite<R: RenderBase> {
    type CompositionMemo: Send + Sync + Clone + 'static;

    fn composite_into_memo(
        render: &R,
        child_iterator: &mut ChildLayerProducingIterator<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> Self::CompositionMemo;

    fn composite_from_cache_to(
        render: &R,
        encoding: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        cache: &Self::CompositionMemo,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
    );

    fn transform_config(
        self_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
        child_config: &LayerCompositionConfig<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>;
}

impl<R> CachedComposite for R
where
    R: ImplByTemplate,
    R::Template: TemplateCachedComposite<R>,
    R: RenderBase,
{
    type CompositionMemo = <R::Template as TemplateCachedComposite<R>>::CompositionMemo;

    fn composite_into_memo(
        &self,
        child_iterator: &mut ChildLayerProducingIterator<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> Self::CompositionMemo {
        R::Template::composite_into_memo(self, child_iterator)
    }

    fn composite_from_cache_to(
        &self,
        encoding: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        cache: &Self::CompositionMemo,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
    ) {
        R::Template::composite_from_cache_to(self, encoding, cache, composition_config)
    }

    fn transform_config(
        self_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
        child_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas> {
        R::Template::transform_config(self_config, child_config)
    }
}

/// Orphan layers can skip this implementation
pub trait TemplateHitTest<R: RenderBase> {
    fn hit_test_children(
        render: &R,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
        children: &ContainerOf<R::ChildContainer, ArcChildRenderObject<R::ChildProtocol>>,
        results: &mut HitTestResults<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> bool;

    fn hit_test_self(
        render: &R,
        position: &<<R::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
    ) -> Option<HitTestBehavior>;
}

impl<R> HitTest for R
where
    R: ImplByTemplate,
    R::Template: TemplateHitTest<R>,
    R: RenderBase,
{
    fn hit_test_children(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &ContainerOf<Self::ChildContainer, ArcChildRenderObject<Self::ChildProtocol>>,
        results: &mut HitTestResults<<Self::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        R::Template::hit_test_children(self, size, offset, memo, children, results)
    }

    fn hit_test_self(
        &self,
        position: &<<Self::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
    ) -> Option<HitTestBehavior> {
        R::Template::hit_test_self(self, position, size, offset, memo)
    }
}

pub trait TemplateOrphanLayer<R: RenderBase>
where
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn adopter_key(render: &R) -> &Asc<dyn Key>;
}

impl<R> OrphanLayer for R
where
    R: ImplByTemplate,
    R::Template: TemplateOrphanLayer<R>,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
    R: LayerPaint,
{
    fn adopter_key(&self) -> &Asc<dyn Key> {
        R::Template::adopter_key(self)
    }
}
