use std::{any::TypeId, borrow::Cow};

use crate::{
    foundation::{
        AnyRawPointer, Arc, Asc, BuildSuspendedError, Canvas, InlinableDwsizeVec, Key,
        LayerProtocol, PaintContext, Protocol, Provide, TypeKey, VecContainer,
        EMPTY_CONSUMED_TYPES,
    },
    tree::{
        default_reconcile_vec, ArcChildElementNode, ArcChildRenderObject, ArcChildWidget,
        ArcWidget, BuildContext, ChildLayerProducingIterator, ChildRenderObjectsUpdateCallback,
        ElementBase, ElementImpl, ElementReconcileItem, FullRender, HitTestContext, HitTestResult,
        ImplElement, ImplRender, LayerCompositionConfig, PaintResults, RecordedChildLayer, Render,
        RenderAction, RenderBase, RenderImpl, RenderObject,
    },
};

use super::{
    ImplByTemplate, TemplateCachedComposite, TemplateComposite, TemplateElement,
    TemplateElementBase, TemplateHitTest, TemplateLayerPaint, TemplateLayout,
    TemplateLayoutByParent, TemplateOrphanLayer, TemplatePaint, TemplateProvideElement,
    TemplateRender, TemplateRenderBase, TemplateRenderElement,
};

/// Multi-child element must also be a RenderElement
pub struct MultiChildElementTemplate<const PROVIDE_ELEMENT: bool>;

pub trait MultiChildElement: Clone + Send + Sync + Sized + 'static {
    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;

    type ArcWidget: ArcWidget<Element = Self>;
    type Render: FullRender<
        ParentProtocol = Self::ParentProtocol,
        ChildProtocol = Self::ChildProtocol,
        ChildContainer = VecContainer,
    >;

    #[allow(unused_variables)]
    fn get_consumed_types(widget: &Self::ArcWidget) -> Cow<[TypeKey]> {
        EMPTY_CONSUMED_TYPES.into()
    }

    fn get_child_widgets(
        element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<Vec<ArcChildWidget<Self::ChildProtocol>>, BuildSuspendedError>;

    /// A major limitation to the Multi child element template is that,
    /// we cannot provide consumed values and build context during the creation the Element itself.
    /// On top of that, since you can no longer access hooks when creating the Element itself,
    /// it also becomes impossible to suspend safely during the process, hence the "must-succeed" signature.
    /// We expect most people does not need provider or hooks during this process.
    /// If you do need, you can always perform relevant operations in the parent and pass it down in widget.
    fn create_element(widget: &Self::ArcWidget) -> Self;

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

impl<E, const PROVIDE_ELEMENT: bool> TemplateElementBase<E>
    for MultiChildElementTemplate<PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: MultiChildElement,
{
    type ParentProtocol = E::ParentProtocol;
    type ChildProtocol = E::ChildProtocol;
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
        children: Vec<ArcChildElementNode<E::ChildProtocol>>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<E::ChildProtocol>>,
    ) -> Result<
        (
            Vec<ElementReconcileItem<E::ChildProtocol>>,
            Option<ChildRenderObjectsUpdateCallback<Self::ChildContainer, E::ChildProtocol>>,
        ),
        (
            Vec<ArcChildElementNode<E::ChildProtocol>>,
            BuildSuspendedError,
        ),
    > {
        let new_widgets = match E::get_child_widgets(Some(element), widget, ctx, provider_values) {
            Err(error) => return Err((children, error)),
            Ok(new_widgets) => new_widgets,
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
    ) -> Result<(E, Vec<ArcChildWidget<E::ChildProtocol>>), BuildSuspendedError> {
        let element = E::create_element(widget);
        let child_widgets = E::get_child_widgets(None, widget, ctx, provider_values)?;
        Ok((element, child_widgets))
    }
}

impl<E, const PROVIDE_ELEMENT: bool> TemplateElement<E>
    for MultiChildElementTemplate<PROVIDE_ELEMENT>
where
    E: ElementBase,
    ElementImpl<true, PROVIDE_ELEMENT>: ImplElement<E>,
{
    type Impl = ElementImpl<true, PROVIDE_ELEMENT>;
}

impl<E, const PROVIDE_ELEMENT: bool> TemplateRenderElement<E>
    for MultiChildElementTemplate<PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: MultiChildElement,
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

pub trait MultiChildProvideElement: MultiChildElement {
    type Provided: Provide;
    fn get_provided_value(widget: &Self::ArcWidget) -> &Arc<Self::Provided>;
}

impl<E, const PROVIDE_ELEMENT: bool> TemplateProvideElement<E>
    for MultiChildElementTemplate<PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: MultiChildProvideElement,
{
    type Provided = E::Provided;

    fn get_provided_value(widget: &<E as ElementBase>::ArcWidget) -> &Arc<Self::Provided> {
        E::get_provided_value(widget)
    }
}

/// This is different from MultiChildElement, because we require child protocol to be the same as parent protocol
/// Normally if the protocol changes, then the node is probably an adapter node which usually has only one child.
/// If you really have a multi-child node which changes protocol, you will probably be better-off to impl it from scratch,
/// because there won't be as many default method impls that could be provided by the template,
/// as there is simply too little we (template provider) can assume about what you want.
pub struct MultiChildRenderTemplate<
    const SIZED_BY_PARENT: bool,
    const LAYER_PAINT: bool,
    const CACHED_COMPOSITE: bool,
    const ORPHAN_LAYER: bool,
>;

pub trait MultiChildRender: Send + Sync + Sized + 'static {
    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol<Canvas = <Self::ParentProtocol as Protocol>::Canvas>;
    type LayoutMemo: Send + Sync;

    fn detach(&mut self) {}
    const NOOP_DETACH: bool = false;

    fn compute_intrinsics(
        &mut self,
        children: &Vec<ArcChildRenderObject<Self::ChildProtocol>>,
        intrinsics: &mut <Self::ParentProtocol as Protocol>::Intrinsics,
    );
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateRenderBase<R>
    for MultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: MultiChildRender,
{
    type ParentProtocol = R::ParentProtocol;
    type ChildProtocol = R::ChildProtocol;
    type ChildContainer = VecContainer;

    type LayoutMemo = R::LayoutMemo;

    fn detach(render: &mut R) {
        R::detach(render)
    }

    const NOOP_DETACH: bool = R::NOOP_DETACH;

    fn compute_intrinsics(
        render: &mut R,
        children: &Vec<ArcChildRenderObject<R::ChildProtocol>>,
        intrinsics: &mut <R::ParentProtocol as Protocol>::Intrinsics,
    ) {
        R::compute_intrinsics(render, children, intrinsics)
    }
}

impl<
        R,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > TemplateRender<R>
    for MultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
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
pub trait MultiChildLayout: MultiChildRender {
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
    for MultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: MultiChildLayout,
{
    fn perform_layout(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        children: &Vec<ArcChildRenderObject<R::ChildProtocol>>,
    ) -> (<R::ParentProtocol as Protocol>::Size, R::LayoutMemo) {
        R::perform_layout(render, constraints, children)
    }
}

pub trait MultiChildLayoutByParent: MultiChildRender {
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
    for MultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: MultiChildLayoutByParent,
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

pub trait MultiChildPaint: MultiChildRender {
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
    for MultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: MultiChildPaint,
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

pub trait MultiChildLayerPaint: MultiChildRender
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
    for MultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: MultiChildLayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn paint_layer(
        render: &R,
        children: &Vec<ArcChildRenderObject<R::ChildProtocol>>,
    ) -> PaintResults<<R::ParentProtocol as Protocol>::Canvas> {
        R::paint_layer(render, children)
    }

    fn transform_config(
        self_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
        child_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas> {
        R::transform_config(self_config, child_config)
    }

    fn layer_key(render: &R) -> Option<&Arc<dyn Key>> {
        R::layer_key(render)
    }
}

pub trait MultiChildComposite: MultiChildRender {
    fn composite_to(
        &self,
        encoding: &mut <<Self::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
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
    for MultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: MultiChildComposite,
{
    fn composite_to(
        render: &R,
        encoding: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        child_iterator: &mut ChildLayerProducingIterator<<R::ParentProtocol as Protocol>::Canvas>,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
    ) {
        R::composite_to(render, encoding, child_iterator, composition_config)
    }
}

pub trait MultiChildCachedComposite: MultiChildRender {
    type CompositionMemo: Send + Sync + Clone + 'static;

    fn composite_into_memo(
        &self,
        child_iterator: &mut ChildLayerProducingIterator<
            <Self::ParentProtocol as Protocol>::Canvas,
        >,
    ) -> Self::CompositionMemo;

    fn composite_from_cache_to(
        &self,
        encoding: &mut <<Self::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
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
    for MultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: MultiChildCachedComposite,
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
        encoding: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        memo: &R::CompositionMemo,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
    ) {
        R::composite_from_cache_to(render, encoding, memo, composition_config)
    }
}

pub trait MultiChildOrphanLayer: MultiChildLayerPaint
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
    for MultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: MultiChildOrphanLayer,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn adopter_key(render: &R) -> &Asc<dyn Key> {
        R::adopter_key(render)
    }
}

pub trait MultiChildHitTest: MultiChildRender {
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
        adopted_children: &[RecordedChildLayer<<Self::ChildProtocol as Protocol>::Canvas>],
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
        adopted_children: &[RecordedChildLayer<<Self::ChildProtocol as Protocol>::Canvas>],
    ) -> bool {
        for child in children.iter().rev() {
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
        position: &<<Self::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
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
    for MultiChildRenderTemplate<SIZED_BY_PARENT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: ImplByTemplate<Template = Self>,
    R: MultiChildHitTest,
{
    fn hit_test(
        render: &R,
        ctx: &mut HitTestContext<<R::ParentProtocol as Protocol>::Canvas>,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
        children: &Vec<ArcChildRenderObject<R::ChildProtocol>>,
        adopted_children: &[RecordedChildLayer<<R::ChildProtocol as Protocol>::Canvas>],
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
        _adopted_children: &[RecordedChildLayer<<R::ChildProtocol as Protocol>::Canvas>],
    ) -> bool {
        unreachable!(
            "TemplatePaint has already provided a hit_test implementation, \
            but hit_test_children is still invoked somehow. This indicates a framework bug."
        )
    }

    fn hit_test_self(
        _render: &R,
        _position: &<<R::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
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
        <R as MultiChildHitTest>::all_hit_test_interfaces()
    }
}
