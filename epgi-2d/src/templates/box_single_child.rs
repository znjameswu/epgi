use std::any::TypeId;

use epgi_core::template::TemplateRender;
use epgi_core::tree::{ImplRender, RenderBase, RenderImpl};
use epgi_core::{
    foundation::{
        AnyRawPointer, Arc, ArrayContainer, Asc, BuildSuspendedError, Canvas, InlinableDwsizeVec,
        Key, PaintContext, Protocol, Provide, TypeKey,
    },
    template::{
        ImplByTemplate, TemplateCachedComposite, TemplateComposite, TemplateElement,
        TemplateElementBase, TemplateHitTest, TemplateLayerPaint, TemplateLayout,
        TemplateLayoutByParent, TemplateOrphanLayer, TemplatePaint, TemplateProvideElement,
        TemplateRenderBase, TemplateRenderElement,
    },
    tree::{
        ArcChildElementNode, ArcChildWidget, ArcWidget, BuildContext, ChildLayerProducingIterator,
        ChildRenderObjectsUpdateCallback, ElementBase, ElementImpl, ElementReconcileItem,
        FullRender, HitTestBehavior, HitTestContext, HitTestResult, ImplElement,
        LayerCompositionConfig, PaintResults, RecordedChildLayer, Render, RenderAction,
        RenderObject,
    },
};

use crate::{
    Affine2dCanvas, Affine2dEncoding, ArcBoxRenderObject, BoxConstraints, BoxOffset, BoxProtocol,
    BoxSize, Point2d,
};

pub struct BoxSingleChildElementTemplate<const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool>;

pub trait BoxSingleChildElement: Clone + Send + Sync + Sized + 'static {
    type ArcWidget: ArcWidget<Element = Self>;

    #[allow(unused_variables)]
    fn get_consumed_types(widget: &Self::ArcWidget) -> &[TypeKey] {
        &[]
    }

    fn get_child_widget(
        element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<BoxProtocol>, BuildSuspendedError>;

    /// A major limitation to the single child element template is that,
    /// we cannot provide consumed values and build context during the creation the Element itself.
    /// On top of that, since you can no longer access hooks when creating the Element itself,
    /// it also becomes impossible to suspend safely during the process, hence the "must-succeed" signature.
    /// We expect most people does not need provider or hooks during this process.
    /// If you do need, you can always perform relevant operations in the parent and pass it down in widget.
    fn create_element(widget: &Self::ArcWidget) -> Self;
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateElementBase<E>
    for BoxSingleChildElementTemplate<RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: BoxSingleChildElement,
{
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type ChildContainer = ArrayContainer<1>;

    type ArcWidget = E::ArcWidget;

    fn perform_rebuild_element(
        element: &mut E,
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        [child]: [ArcChildElementNode<BoxProtocol>; 1],
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<BoxProtocol>>,
    ) -> Result<
        (
            [ElementReconcileItem<BoxProtocol>; 1],
            Option<ChildRenderObjectsUpdateCallback<Self::ChildContainer, BoxProtocol>>,
        ),
        ([ArcChildElementNode<BoxProtocol>; 1], BuildSuspendedError),
    > {
        let child_widget = match E::get_child_widget(Some(element), widget, ctx, provider_values) {
            Err(error) => return Err(([child], error)),
            Ok(child_wdiget) => child_wdiget,
        };
        let item = match child.can_rebuild_with(child_widget) {
            Ok(item) => item,
            Err((child, child_widget)) => {
                nodes_needing_unmount.push(child);
                ElementReconcileItem::new_inflate(child_widget)
            }
        };
        Ok(([item], None))
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(E, [ArcChildWidget<BoxProtocol>; 1]), BuildSuspendedError> {
        let element = E::create_element(widget);
        let child_widget = E::get_child_widget(None, widget, ctx, provider_values)?;
        Ok((element, [child_widget]))
    }
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateElement<E>
    for BoxSingleChildElementTemplate<RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ElementBase,
    ElementImpl<RENDER_ELEMENT, PROVIDE_ELEMENT>: ImplElement<E>,
{
    type Impl = ElementImpl<RENDER_ELEMENT, PROVIDE_ELEMENT>;
}

pub trait BoxSingleChildRenderElement: BoxSingleChildElement {
    type Render: FullRender<
        ParentProtocol = BoxProtocol,
        ChildProtocol = BoxProtocol,
        ChildContainer = ArrayContainer<1>,
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
    for BoxSingleChildElementTemplate<RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: BoxSingleChildRenderElement,
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

pub trait BoxSingleChildProvideElement: BoxSingleChildElement {
    type Provided: Provide;
    fn get_provided_value(widget: &Self::ArcWidget) -> Arc<Self::Provided>;
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateProvideElement<E>
    for BoxSingleChildElementTemplate<RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: BoxSingleChildProvideElement,
{
    type Provided = E::Provided;

    fn get_provided_value(widget: &<E as ElementBase>::ArcWidget) -> Arc<Self::Provided> {
        E::get_provided_value(widget)
    }
}

pub struct BoxSingleChildRenderTemplate<
    const SIZED_BY_PARENT: bool,
    const LAYER_PAINT: bool,
    const CACHED_COMPOSITE: bool,
    const ORPHAN_LAYER: bool,
>;

pub trait BoxSingleChildRender: Send + Sync + Sized + 'static {
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
    for BoxSingleChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxSingleChildRender,
{
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type ChildContainer = ArrayContainer<1>;

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
    for BoxSingleChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
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
pub trait BoxSingleChildLayout: BoxSingleChildRender {
    fn perform_layout(
        &mut self,
        constraints: &BoxConstraints,
        child: &ArcBoxRenderObject,
    ) -> (BoxSize, Self::LayoutMemo);
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateLayout<R>
    for BoxSingleChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxSingleChildLayout,
{
    fn perform_layout(
        render: &mut R,
        constraints: &BoxConstraints,
        [child]: &[ArcBoxRenderObject; 1],
    ) -> (BoxSize, R::LayoutMemo) {
        R::perform_layout(render, constraints, child)
    }
}

pub trait BoxSingleChildLayoutByParent: BoxSingleChildRender {
    fn compute_size_by_parent(&self, constraints: &BoxConstraints) -> BoxSize;

    fn perform_layout(
        &mut self,
        constraints: &BoxConstraints,
        size: &BoxSize,
        child: &ArcBoxRenderObject,
    ) -> Self::LayoutMemo;
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateLayoutByParent<R>
    for BoxSingleChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxSingleChildLayoutByParent,
{
    fn compute_size_by_parent(render: &R, constraints: &BoxConstraints) -> BoxSize {
        R::compute_size_by_parent(render, constraints)
    }

    fn perform_layout(
        render: &mut R,
        constraints: &BoxConstraints,
        size: &BoxSize,
        [child]: &[ArcBoxRenderObject; 1],
    ) -> R::LayoutMemo {
        R::perform_layout(render, constraints, size, child)
    }
}

pub trait BoxSingleChildPaint: BoxSingleChildRender {
    fn perform_paint(
        &self,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &Self::LayoutMemo,
        child: &ArcBoxRenderObject,
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
    for BoxSingleChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxSingleChildPaint,
{
    fn perform_paint(
        render: &R,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &R::LayoutMemo,
        [child]: &[ArcBoxRenderObject; 1],
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        R::perform_paint(render, size, offset, memo, child, paint_ctx)
    }
}

pub trait BoxSingleChildLayerPaint: BoxSingleChildRender {
    fn paint_layer(&self, child: &ArcBoxRenderObject) -> PaintResults<Affine2dCanvas> {
        Affine2dCanvas::paint_render_objects([child.clone()])
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
    for BoxSingleChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxSingleChildLayerPaint,
{
    fn paint_layer(render: &R, [child]: &[ArcBoxRenderObject; 1]) -> PaintResults<Affine2dCanvas> {
        R::paint_layer(render, child)
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

pub trait BoxSingleChildComposite: BoxSingleChildRender {
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
    for BoxSingleChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxSingleChildComposite,
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

pub trait BoxSingleChildCachedComposite: BoxSingleChildRender {
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
    for BoxSingleChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxSingleChildCachedComposite,
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

pub trait BoxSingleChildOrphanLayer: BoxSingleChildLayerPaint {
    fn adopter_key(&self) -> &Asc<dyn Key>;
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateOrphanLayer<R>
    for BoxSingleChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxSingleChildOrphanLayer,
{
    fn adopter_key(render: &R) -> &Asc<dyn Key> {
        R::adopter_key(render)
    }
}

pub trait BoxSingleChildHitTest: BoxSingleChildRender {
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
        child: &ArcBoxRenderObject,
        adopted_children: &[RecordedChildLayer<Affine2dCanvas>],
    ) -> HitTestResult {
        use HitTestResult::*;
        let hit_self = self.hit_test_self(ctx.curr_position(), size, offset, memo);
        if !hit_self {
            // Stop hit-test children if the hit is outside of parent
            return NotHit;
        }

        let hit_children = self.hit_test_child(ctx, size, offset, memo, child, adopted_children);
        if hit_children {
            return Hit;
        }

        use HitTestBehavior::*;
        match self.hit_test_behavior() {
            DeferToChild => NotHit,
            Transparent => HitThroughSelf,
            Opaque => Hit,
        }
    }

    /// Returns: If a child has claimed the hit
    fn hit_test_child(
        &self,
        ctx: &mut HitTestContext<Affine2dCanvas>,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &Self::LayoutMemo,
        child: &ArcBoxRenderObject,
        adopted_children: &[RecordedChildLayer<Affine2dCanvas>],
    ) -> bool;

    // The reason we separate hit_test_self from hit_test_children is that we do not wish to leak hit_position into hit_test_children
    // Therefore preventing implementer to perform transform on hit_position rather than recording it in
    #[allow(unused_variables)]
    fn hit_test_self(
        &self,
        position: &Point2d,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &Self::LayoutMemo,
    ) -> bool {
        BoxProtocol::position_in_shape(position, offset, size)
    }

    fn hit_test_behavior(&self) -> HitTestBehavior {
        HitTestBehavior::DeferToChild
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
    for BoxSingleChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: BoxSingleChildHitTest,
{
    fn hit_test(
        render: &R,
        ctx: &mut HitTestContext<Affine2dCanvas>,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &R::LayoutMemo,
        [child]: &[ArcBoxRenderObject; 1],
        adopted_children: &[RecordedChildLayer<Affine2dCanvas>],
    ) -> HitTestResult {
        R::hit_test(render, ctx, size, offset, memo, child, adopted_children)
    }

    /// Returns: If a child has claimed the hit
    fn hit_test_children(
        render: &R,
        ctx: &mut HitTestContext<Affine2dCanvas>,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &R::LayoutMemo,
        [child]: &[ArcBoxRenderObject; 1],
        adopted_children: &[RecordedChildLayer<Affine2dCanvas>],
    ) -> bool {
        R::hit_test_child(render, ctx, size, offset, memo, child, adopted_children)
    }

    fn hit_test_self(
        render: &R,
        position: &Point2d,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &R::LayoutMemo,
    ) -> bool {
        R::hit_test_self(render, position, size, offset, memo)
    }

    fn hit_test_behavior(render: &R) -> HitTestBehavior {
        R::hit_test_behavior(render)
    }

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<R>) -> AnyRawPointer)]
    where
        R: Render,
    {
        <R as BoxSingleChildHitTest>::all_hit_test_interfaces()
    }
}
