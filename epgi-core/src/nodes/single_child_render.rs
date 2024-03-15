use std::any::TypeId;
use std::marker::PhantomData;

use crate::foundation::{
    AnyRawPointer, Arc, ArrayContainer, Asc, BuildSuspendedError, InlinableDwsizeVec, Never,
    PaintContext, Protocol, Provide,
};

use crate::tree::{
    ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, BuildContext,
    ChildRenderObjectsUpdateCallback, DryLayoutFunctionTable, Element, ElementReconcileItem,
    HitTestContext, LayerOrUnit, Render, RenderAction, RenderElement, RenderObject, Widget,
};

pub trait SingleChildRenderObjectWidget:
    Widget<Element = SingleChildRenderObjectElement<Self>> + Sized
{
    type RenderState: Send + Sync;

    fn child(&self) -> &ArcChildWidget<Self::ChildProtocol>;

    fn create_render_state(&self) -> Self::RenderState;

    fn update_render_state(&self, render_state: &mut Self::RenderState) -> RenderAction;

    const NOOP_UPDATE_RENDER_OBJECT: bool = false;

    fn detach_render_state(render_state: &mut Self::RenderState);

    const NOOP_DETACH: bool = false;

    type LayoutMemo: Send + Sync + 'static;

    fn perform_layout(
        state: &Self::RenderState,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        child: &ArcChildRenderObject<Self::ChildProtocol>,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo);

    /// If this is not None, then [`Self::perform_layout`]'s implementation will be ignored.
    const DRY_LAYOUT_FUNCTION_TABLE: Option<DryLayoutFunctionTable<SingleChildRenderObject<Self>>> =
        None;

    // We don't make perform paint into an associated constant because it has an generic paramter
    // Then we have to go to associated generic type, which makes the boilerplate explodes.
    fn perform_paint(
        state: &Self::RenderState,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        child: &ArcChildRenderObject<Self::ChildProtocol>,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    );

    fn hit_test_children(
        state: &Self::RenderState,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        child: &ArcChildRenderObject<Self::ChildProtocol>,
        context: &mut HitTestContext<<Self::ParentProtocol as Protocol>::Canvas>,
    ) -> bool;

    type LayerOrUnit: LayerOrUnit<SingleChildRenderObject<Self>>;

    fn all_hit_test_interfaces() -> &'static [(
        TypeId,
        fn(*mut RenderObject<SingleChildRenderObject<Self>>) -> AnyRawPointer,
    )] {
        &[]
    }
}

pub trait SingleChildDryLayout: SingleChildRenderObjectWidget {
    const DRY_LAYOUT_FUNCTION_TABLE: Option<DryLayoutFunctionTable<SingleChildRenderObject<Self>>> =
        Some(DryLayoutFunctionTable {
            compute_dry_layout: |render, constraints| {
                Self::compute_dry_layout(&render.state, constraints)
            },
            compute_layout_memo: |render, constraints, size, children| {
                let [child] = children;
                Self::compute_layout_memo(&render.state, constraints, size, child)
            },
        });

    fn compute_dry_layout(
        state: &Self::RenderState,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
    ) -> <Self::ParentProtocol as Protocol>::Size;

    fn compute_layout_memo(
        state: &Self::RenderState,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &<Self::ParentProtocol as Protocol>::Size,
        child: &ArcChildRenderObject<Self::ChildProtocol>,
    ) -> Self::LayoutMemo;
}

pub struct SingleChildRenderObjectElement<W: SingleChildRenderObjectWidget>(PhantomData<W>);

impl<W> Clone for SingleChildRenderObjectElement<W>
where
    W: SingleChildRenderObjectWidget,
{
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<W> Element for SingleChildRenderObjectElement<W>
where
    W: SingleChildRenderObjectWidget<Element = Self>,
{
    type ArcWidget = Asc<W>;

    type ParentProtocol = W::ParentProtocol;

    type ChildContainer = ArrayContainer<1>;

    type ChildProtocol = W::ChildProtocol;

    type Provided = Never;

    fn perform_rebuild_element(
        &mut self,
        widget: &Self::ArcWidget,
        _ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        children: [ArcChildElementNode<Self::ChildProtocol>; 1],
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::ChildProtocol>>,
    ) -> Result<
        (
            [ElementReconcileItem<Self::ChildProtocol>; 1],
            Option<ChildRenderObjectsUpdateCallback<Self>>,
        ),
        (
            [ArcChildElementNode<Self::ChildProtocol>; 1],
            BuildSuspendedError,
        ),
    > {
        let [child] = children;
        match child.can_rebuild_with(widget.child().clone()) {
            Ok(item) => Ok(([item], None)),
            Err((child, child_widget)) => {
                nodes_needing_unmount.push(child);
                Ok(([ElementReconcileItem::new_inflate(child_widget)], None))
            }
        }
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        _ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(Self, [ArcChildWidget<Self::ChildProtocol>; 1]), BuildSuspendedError> {
        let child_widget = widget.child().clone();
        Ok((Self(PhantomData), [child_widget]))
    }

    type RenderOrUnit = SingleChildRenderObject<W>;
}

impl<W> RenderElement for SingleChildRenderObjectElement<W>
where
    W: SingleChildRenderObjectWidget<Element = Self>,
{
    type Render = SingleChildRenderObject<W>;

    #[inline(always)]
    fn create_render(&self, widget: &Self::ArcWidget) -> SingleChildRenderObject<W> {
        SingleChildRenderObject {
            state: W::create_render_state(widget),
        }
    }

    #[inline(always)]
    fn update_render(
        render: &mut SingleChildRenderObject<W>,
        widget: &Self::ArcWidget,
    ) -> RenderAction {
        W::update_render_state(widget, &mut render.state)
    }
    const NOOP_UPDATE_RENDER_OBJECT: bool = W::NOOP_UPDATE_RENDER_OBJECT;

    fn element_render_children_mapping<T: Send + Sync>(
        &self,
        element_children: <Self::ChildContainer as crate::foundation::HktContainer>::Container<T>,
    ) -> <<SingleChildRenderObject<W> as Render>::ChildContainer as crate::foundation::HktContainer>::Container<T>{
        element_children
    }
}

pub struct SingleChildRenderObject<W: SingleChildRenderObjectWidget> {
    pub state: W::RenderState,
}

impl<W> Render for SingleChildRenderObject<W>
where
    W: SingleChildRenderObjectWidget,
{
    type ParentProtocol = W::ParentProtocol;

    type ChildProtocol = W::ChildProtocol;

    type ChildContainer = ArrayContainer<1>;

    const NOOP_DETACH: bool = W::NOOP_DETACH;

    type LayoutMemo = W::LayoutMemo;

    #[inline(always)]
    fn perform_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        children: &[ArcChildRenderObject<Self::ChildProtocol>; 1],
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo) {
        let [child] = children;
        if W::DRY_LAYOUT_FUNCTION_TABLE.is_some() {
            unreachable!()
        } else {
            return W::perform_layout(&self.state, constraints, child);
        }
    }

    #[inline(always)]
    fn perform_paint(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &[ArcChildRenderObject<Self::ChildProtocol>; 1],
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) {
        let [child] = children;
        if <W::LayerOrUnit as LayerOrUnit<Self>>::LAYER_RENDER_FUNCTION_TABLE.is_some() {
            unreachable!()
        } else {
            return W::perform_paint(&self.state, size, offset, memo, child, paint_ctx);
        }
    }

    type LayerOrUnit = ();

    fn hit_test_children(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &[ArcChildRenderObject<Self::ChildProtocol>; 1],
        context: &mut HitTestContext<<Self::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        let [child] = children;
        return W::hit_test_children(&self.state, size, offset, memo, child, context);
    }

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<Self>) -> AnyRawPointer)]
    {
        W::all_hit_test_interfaces()
    }
}
