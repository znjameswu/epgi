use std::any::TypeId;
use std::borrow::Cow;

use epgi_core::foundation::{VecContainer, EMPTY_CONSUMED_TYPES};
use epgi_core::template::TemplateRender;
use epgi_core::tree::{default_reconcile_vec, ImplRender, RenderBase, RenderImpl};
use epgi_core::{
    foundation::{
        AnyRawPointer, Arc, Asc, BuildSuspendedError, Canvas, InlinableDwsizeVec, Key,
        PaintContext, Protocol, Provide, TypeKey,
    },
    template::{
        ImplByTemplate, TemplateCachedComposite, TemplateComposite, TemplateElement,
        TemplateElementBase, TemplateHitTest, TemplateLayerPaint, TemplateLayout,
        TemplateLayoutByParent, TemplateOrphanLayer, TemplatePaint, TemplateProvideElement,
        TemplateRenderBase, TemplateRenderElement,
    },
    tree::{
        ArcChildElementNode, ArcWidget, BuildContext, ChildLayerProducingIterator,
        ChildRenderObjectsUpdateCallback, ElementBase, ElementImpl, ElementReconcileItem,
        FullRender, HitTestContext, HitTestResult, ImplElement, LayerCompositionConfig,
        PaintResults, RecordedChildLayer, Render, RenderAction, RenderObject,
    },
};

use crate::{
    Affine2dCanvas, Affine2dEncoding, ArcBoxElementNode, ArcBoxRenderObject, ArcBoxWidget,
    BoxConstraints, BoxOffset, BoxProtocol, BoxSize, Point2d,
};

pub struct BoxMultiChildElementTemplate<const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool>;

pub trait BoxMultiChildElement: Clone + Send + Sync + Sized + 'static {
    type ArcWidget: ArcWidget<Element = Self>;

    #[allow(unused_variables)]
    fn get_consumed_types(widget: &Self::ArcWidget) -> Cow<[TypeKey]> {
        EMPTY_CONSUMED_TYPES.into()
    }

    fn get_child_widgets(
        element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<Vec<ArcBoxWidget>, BuildSuspendedError>;

    /// A major limitation to the Multi child element template is that,
    /// we cannot provide consumed values and build context during the creation the Element itself.
    /// On top of that, since you can no longer access hooks when creating the Element itself,
    /// it also becomes impossible to suspend safely during the process, hence the "must-succeed" signature.
    /// We expect most people does not need provider or hooks during this process.
    /// If you do need, you can always perform relevant operations in the parent and pass it down in widget.
    fn create_element(widget: &Self::ArcWidget) -> Self;
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateElementBase<E>
    for BoxMultiChildElementTemplate<RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: BoxMultiChildElement,
{
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type ChildContainer = VecContainer;

    type ArcWidget = E::ArcWidget;

    fn get_consumed_types(widget: &Self::ArcWidget) -> Cow<[TypeKey]> {
        E::get_consumed_types(widget)
    }

    fn perform_rebuild_element(
        element: &mut E,
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        children: Vec<ArcBoxElementNode>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<BoxProtocol>>,
    ) -> Result<
        (
            Vec<ElementReconcileItem<BoxProtocol>>,
            Option<ChildRenderObjectsUpdateCallback<Self::ChildContainer, BoxProtocol>>,
        ),
        (Vec<ArcBoxElementNode>, BuildSuspendedError),
    > {
        let new_widgets = match E::get_child_widgets(Some(element), widget, ctx, provider_values) {
            Err(error) => return Err((children, error)),
            Ok(child_wdiget) => child_wdiget,
        };
        Ok(default_reconcile_vec(
            children,
            new_widgets,
            nodes_needing_unmount,
        ))
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(E, Vec<ArcBoxWidget>), BuildSuspendedError> {
        let element = E::create_element(widget);
        let child_widgets = E::get_child_widgets(None, widget, ctx, provider_values)?;
        Ok((element, child_widgets))
    }
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateElement<E>
    for BoxMultiChildElementTemplate<RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ElementBase,
    ElementImpl<RENDER_ELEMENT, PROVIDE_ELEMENT>: ImplElement<E>,
{
    type Impl = ElementImpl<RENDER_ELEMENT, PROVIDE_ELEMENT>;
}

pub trait BoxMultiChildRenderElement: BoxMultiChildElement {
    type Render: FullRender<
        ParentProtocol = BoxProtocol,
        ChildProtocol = BoxProtocol,
        ChildContainer = VecContainer,
    >;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render;
    /// Update necessary properties of render object given by the widget
    ///
    /// Called during the commit phase, when the widget is updated.
    /// Always called after [RenderElement::try_update_render_object_children].
    /// If that call failed to update children (indicating suspense), then this call will be skipped.
    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction>;

    /// Whether [Render::update_render_object] is a no-op and always returns None
    ///
    /// When set to true, [Render::update_render_object]'s implementation will be ignored,
    /// Certain optimizations to reduce mutex usages will be applied during the commit phase.
    /// However, if [Render::update_render_object] is actually not no-op, doing this will cause unexpected behaviors.
    ///
    /// Setting to false will always guarantee the correct behavior.
    const NOOP_UPDATE_RENDER_OBJECT: bool = false;
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateRenderElement<E>
    for BoxMultiChildElementTemplate<RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: BoxMultiChildRenderElement,
{
    type Render = E::Render;

    fn create_render(element: &E, widget: &<E as ElementBase>::ArcWidget) -> Self::Render {
        E::create_render(element, widget)
    }

    fn update_render(
        render: &mut Self::Render,
        widget: &<E as ElementBase>::ArcWidget,
    ) -> Option<RenderAction> {
        E::update_render(render, widget)
    }

    const NOOP_UPDATE_RENDER_OBJECT: bool = E::NOOP_UPDATE_RENDER_OBJECT;
}

pub trait BoxMultiChildProvideElement: BoxMultiChildElement {
    type Provided: Provide;
    fn get_provided_value(widget: &Self::ArcWidget) -> Arc<Self::Provided>;
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateProvideElement<E>
    for BoxMultiChildElementTemplate<RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: BoxMultiChildProvideElement,
{
    type Provided = E::Provided;

    fn get_provided_value(widget: &<E as ElementBase>::ArcWidget) -> Arc<Self::Provided> {
        E::get_provided_value(widget)
    }
}

pub struct BoxMultiChildRenderTemplate<
    const SIZED_BY_PARENT: bool,
    const LAYER_PAINT: bool,
    const CACHED_COMPOSITE: bool,
    const ORPHAN_LAYER: bool,
>;

pub trait BoxMultiChildRender: Send + Sync + Sized + 'static {
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
    for BoxMultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxMultiChildRender,
{
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
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
    for BoxMultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
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
pub trait BoxMultiChildLayout: BoxMultiChildRender {
    fn perform_layout(
        &mut self,
        constraints: &BoxConstraints,
        children: &Vec<ArcBoxRenderObject>,
    ) -> (BoxSize, Self::LayoutMemo);
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateLayout<R>
    for BoxMultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxMultiChildLayout,
{
    fn perform_layout(
        render: &mut R,
        constraints: &BoxConstraints,
        children: &Vec<ArcBoxRenderObject>,
    ) -> (BoxSize, R::LayoutMemo) {
        R::perform_layout(render, constraints, children)
    }
}

pub trait BoxMultiChildLayoutByParent: BoxMultiChildRender {
    fn compute_size_by_parent(&self, constraints: &BoxConstraints) -> BoxSize;

    fn perform_layout(
        &mut self,
        constraints: &BoxConstraints,
        size: &BoxSize,
        children: &Vec<ArcBoxRenderObject>,
    ) -> Self::LayoutMemo;
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateLayoutByParent<R>
    for BoxMultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxMultiChildLayoutByParent,
{
    fn compute_size_by_parent(render: &R, constraints: &BoxConstraints) -> BoxSize {
        R::compute_size_by_parent(render, constraints)
    }

    fn perform_layout(
        render: &mut R,
        constraints: &BoxConstraints,
        size: &BoxSize,
        children: &Vec<ArcBoxRenderObject>,
    ) -> R::LayoutMemo {
        R::perform_layout(render, constraints, size, children)
    }
}

pub trait BoxMultiChildPaint: BoxMultiChildRender {
    fn perform_paint(
        &self,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &Self::LayoutMemo,
        children: &Vec<ArcBoxRenderObject>,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    );
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplatePaint<R>
    for BoxMultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxMultiChildPaint,
{
    fn perform_paint(
        render: &R,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &R::LayoutMemo,
        children: &Vec<ArcBoxRenderObject>,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        R::perform_paint(render, size, offset, memo, children, paint_ctx)
    }
}

pub trait BoxMultiChildLayerPaint: BoxMultiChildRender {
    fn paint_layer(&self, children: &Vec<ArcBoxRenderObject>) -> PaintResults<Affine2dCanvas> {
        Affine2dCanvas::paint_render_objects(children.clone())
    }

    fn transform_config(
        self_config: &LayerCompositionConfig<Affine2dCanvas>,
        child_config: &LayerCompositionConfig<Affine2dCanvas>,
    ) -> LayerCompositionConfig<Affine2dCanvas> {
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
    for BoxMultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxMultiChildLayerPaint,
{
    fn paint_layer(render: &R, children: &Vec<ArcBoxRenderObject>) -> PaintResults<Affine2dCanvas> {
        R::paint_layer(render, children)
    }

    fn transform_config(
        self_config: &LayerCompositionConfig<Affine2dCanvas>,
        child_config: &LayerCompositionConfig<Affine2dCanvas>,
    ) -> LayerCompositionConfig<Affine2dCanvas> {
        R::transform_config(self_config, child_config)
    }

    fn layer_key(render: &R) -> Option<&Arc<dyn Key>> {
        R::layer_key(render)
    }
}

pub trait BoxMultiChildComposite: BoxMultiChildRender {
    fn composite_to(
        &self,
        encoding: &mut Affine2dEncoding,
        child_iterator: &mut ChildLayerProducingIterator<Affine2dCanvas>,
        composition_config: &LayerCompositionConfig<Affine2dCanvas>,
    );
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateComposite<R>
    for BoxMultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxMultiChildComposite,
{
    fn composite_to(
        render: &R,
        encoding: &mut Affine2dEncoding,
        child_iterator: &mut ChildLayerProducingIterator<Affine2dCanvas>,
        composition_config: &LayerCompositionConfig<Affine2dCanvas>,
    ) {
        R::composite_to(render, encoding, child_iterator, composition_config)
    }
}

pub trait BoxMultiChildCachedComposite: BoxMultiChildRender {
    type CompositionMemo: Send + Sync + Clone + 'static;

    fn composite_into_memo(
        &self,
        child_iterator: &mut ChildLayerProducingIterator<Affine2dCanvas>,
    ) -> Self::CompositionMemo;

    fn composite_from_cache_to(
        &self,
        encoding: &mut Affine2dEncoding,
        memo: &Self::CompositionMemo,
        composition_config: &LayerCompositionConfig<Affine2dCanvas>,
    );
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateCachedComposite<R>
    for BoxMultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxMultiChildCachedComposite,
{
    type CompositionMemo = R::CompositionMemo;

    fn composite_into_memo(
        render: &R,
        child_iterator: &mut ChildLayerProducingIterator<Affine2dCanvas>,
    ) -> R::CompositionMemo {
        R::composite_into_memo(render, child_iterator)
    }

    fn composite_from_cache_to(
        render: &R,
        encoding: &mut Affine2dEncoding,
        memo: &R::CompositionMemo,
        composition_config: &LayerCompositionConfig<Affine2dCanvas>,
    ) {
        R::composite_from_cache_to(render, encoding, memo, composition_config)
    }
}

pub trait BoxMultiChildOrphanLayer: BoxMultiChildLayerPaint {
    fn adopter_key(&self) -> &Asc<dyn Key>;
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateOrphanLayer<R>
    for BoxMultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxMultiChildOrphanLayer,
{
    fn adopter_key(render: &R) -> &Asc<dyn Key> {
        R::adopter_key(render)
    }
}

pub trait BoxMultiChildHitTest: BoxMultiChildRender {
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
        ctx: &mut HitTestContext<Affine2dCanvas>,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &Self::LayoutMemo,
        children: &Vec<ArcBoxRenderObject>,
        adopted_children: &[RecordedChildLayer<Affine2dCanvas>],
    ) -> HitTestResult {
        use HitTestResult::*;
        let hit_in_bound = BoxProtocol::position_in_shape(ctx.curr_position(), offset, size);
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
        ctx: &mut HitTestContext<Affine2dCanvas>,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &Self::LayoutMemo,
        children: &Vec<ArcBoxRenderObject>,
        adopted_children: &[RecordedChildLayer<Affine2dCanvas>],
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
        size: &BoxSize,
        offset: &BoxOffset,
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
    for BoxMultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxMultiChildHitTest,
{
    fn hit_test(
        render: &R,
        ctx: &mut HitTestContext<Affine2dCanvas>,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &R::LayoutMemo,
        children: &Vec<ArcBoxRenderObject>,
        adopted_children: &[RecordedChildLayer<Affine2dCanvas>],
    ) -> HitTestResult {
        R::hit_test(render, ctx, size, offset, memo, children, adopted_children)
    }

    /// Returns: If a child has claimed the hit
    fn hit_test_children(
        _render: &R,
        _ctx: &mut HitTestContext<Affine2dCanvas>,
        _size: &BoxSize,
        _offset: &BoxOffset,
        _memo: &R::LayoutMemo,
        _children: &Vec<ArcBoxRenderObject>,
        _adopted_children: &[RecordedChildLayer<Affine2dCanvas>],
    ) -> bool {
        unreachable!(
            "TemplatePaint has already provided a hit_test implementation, \
            but hit_test_children is still invoked somehow. This indicates a framework bug."
        )
    }

    fn hit_test_self(
        _render: &R,
        _position: &Point2d,
        _size: &BoxSize,
        _offset: &BoxOffset,
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
        <R as BoxMultiChildHitTest>::all_hit_test_interfaces()
    }
}
