mod mark;
pub use mark::*;

mod node;
pub use node::*;

mod r#impl;
pub use r#impl::*;

use std::any::TypeId;

use crate::{
    foundation::{
        default_cast_interface_by_table_raw, default_cast_interface_by_table_raw_mut,
        default_query_interface_arc, default_query_interface_box, default_query_interface_ref,
        AnyRawPointer, Arc, AsIterator, Asc, Aweak, Canvas, CastInterfaceByRawPtr, ContainerOf,
        HktContainer, Key, LayerProtocol, PaintContext, Protocol, SyncMutex, Transform,
        TransformHitPosition,
    },
    sync::ImplAdopterLayer,
    tree::{ChildLayerProducingIterator, LayerCompositionConfig, PaintResults},
};

use super::{
    ArcAnyLayerRenderObject, ArcElementContextNode, AweakAnyLayerRenderObject, ElementContextNode,
};

pub type ArcChildRenderObject<P> = Arc<dyn ChildRenderObject<P>>;
pub type ArcAnyRenderObject = Arc<dyn AnyRenderObject>;
pub type AweakAnyRenderObject = Aweak<dyn AnyRenderObject>;
pub type AweakParentRenderObject<P> = Arc<dyn ParentRenderObject<P>>;
pub type ArcChildRenderObjectWithCanvas<C> = Arc<dyn ChildRenderObjectWithCanvas<C>>;

pub type ContainerOfRender<E, T> =
    <<E as RenderBase>::ChildContainer as HktContainer>::Container<T>;

pub trait RenderBase: Send + Sync + Sized + 'static {
    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;
    type ChildContainer: HktContainer;

    type LayoutMemo: Send + Sync;

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<Self>) -> AnyRawPointer)]
    where
        Self: Render,
    {
        &[]
    }

    fn detach(&mut self) {}
    const NOOP_DETACH: bool = false;
}

pub trait Render: RenderBase {
    type RenderImpl: ImplRender<Self>;
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

    fn compute_layout_memo(
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

    // fn transform_config(
    //     self_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
    //     child_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    // ) -> LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>;

    fn layer_key(&self) -> Option<&Arc<dyn Key>> {
        None
    }
}

pub trait Composite<
    AdopterCanvas: Canvas = <<Self as RenderBase>::ParentProtocol as Protocol>::Canvas,
>: RenderBase
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
    AdopterCanvas: Canvas = <<Self as RenderBase>::ParentProtocol as Protocol>::Canvas,
>: RenderBase
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

/// Orphan layers can skip this implementation
pub trait HitTest: RenderBase {
    fn hit_test_children(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &ContainerOf<Self::ChildContainer, ArcChildRenderObject<Self::ChildProtocol>>,
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
        Self::ParentProtocol::position_in_shape(position, offset, size)
            .then_some(HitTestBehavior::DeferToChild)
    }
}

// We COULD orthogonalize the Orphan/Structured vs Noncached/cached trait set,
// but that would inevitably bake directly into library user's code an explicit AdopterCanvas type
// either somewhere in an associated type or somewhere as a generic trait paramter.
// As an unproven idea, I would like to make orphan layer mechanism optional and not bake into anything more than necessary.
// Edit: We actually did orthogonalize these traits.
pub trait OrphanLayer: LayerPaint
where
    Self::ParentProtocol: LayerProtocol,
    Self::ChildProtocol: LayerProtocol,
{
    fn adopter_key(&self) -> &Asc<dyn Key>;
}

pub enum HitTestBehavior {
    Transparent,
    DeferToChild,
    Opaque,
}

pub struct HitTestResults<C: Canvas> {
    position: C::HitPosition,
    curr_transform: C::Transform,
    curr_position: C::HitPosition,
    pub targets: Vec<(C::Transform, ArcChildRenderObjectWithCanvas<C>)>,
    trait_type_id: TypeId,
}

impl<C> HitTestResults<C>
where
    C: Canvas,
{
    pub fn new(position: C::HitPosition, trait_type_id: TypeId) -> Self {
        Self {
            position: position.clone(),
            curr_transform: <C::Transform as Transform<_>>::identity(),
            curr_position: position,
            targets: Vec::new(),
            trait_type_id,
        }
    }
    pub fn curr_position(&self) -> &C::HitPosition {
        &self.curr_position
    }

    pub fn interface_exist_on<R: Render>(&self) -> bool {
        R::all_hit_test_interfaces()
            .iter()
            .any(|(type_id, _)| self.trait_type_id == *type_id)
    }

    pub fn push(&mut self, render_object: ArcChildRenderObjectWithCanvas<C>) {
        self.targets
            .push((self.curr_transform.clone(), render_object));
    }

    #[inline(always)]
    pub fn hit_test_with_raw_transform<P: Protocol<Canvas = C>>(
        &mut self,
        render_object: ArcChildRenderObject<P>,
        transform: Option<&C::Transform>,
    ) -> bool {
        if let Some(transform) = transform {
            let new_transform = Transform::mul(&self.curr_transform, transform);
            let old_position = std::mem::replace(
                &mut self.curr_position,
                new_transform.transform(&self.position),
            );
            let old_transform = std::mem::replace(&mut self.curr_transform, new_transform);
            let subtree_has_absorbed = render_object.hit_test(self);
            self.curr_transform = old_transform;
            self.curr_position = old_position;
            return subtree_has_absorbed;
        } else {
            return render_object.hit_test(self);
        }
    }

    #[inline(always)]
    pub fn hit_test<P: Protocol<Canvas = C>>(
        &mut self,
        render_object: ArcChildRenderObject<P>,
    ) -> bool {
        return render_object.hit_test(self);
    }
}

impl<R> CastInterfaceByRawPtr for RenderObject<R>
where
    R: Render,
{
    fn cast_interface_raw(&self, trait_type_id: TypeId) -> Option<AnyRawPointer> {
        default_cast_interface_by_table_raw(self, trait_type_id, R::all_hit_test_interfaces())
    }

    fn cast_interface_raw_mut(&mut self, trait_type_id: TypeId) -> Option<AnyRawPointer> {
        default_cast_interface_by_table_raw_mut(self, trait_type_id, R::all_hit_test_interfaces())
    }
}

#[macro_export]
macro_rules! hit_test_interface_query_table {
    ($name: ident, $type: ty, $($trait: ty),* $(,)?) => {
        epgi_core::interface_query_table!($name, RenderObject<$type>, $($trait,)*);
    };
}

pub use hit_test_interface_query_table;

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

pub trait ImplRenderObjectReconcile<R: RenderBase>: Send + Sync + Sized {
    fn new(
        render: R,
        children: ContainerOf<R::ChildContainer, ArcChildRenderObject<R::ChildProtocol>>,
        context: ArcElementContextNode,
    ) -> Self;

    fn update<T>(
        &self,
        op: impl FnOnce(
            &mut R,
            &mut ContainerOf<R::ChildContainer, ArcChildRenderObject<R::ChildProtocol>>,
        ) -> T,
    ) -> T;
}

impl<R> ImplRenderObjectReconcile<R> for RenderObject<R>
where
    R: Render,
{
    fn new(
        render: R,
        children: ContainerOf<R::ChildContainer, ArcChildRenderObject<R::ChildProtocol>>,
        context: ArcElementContextNode,
    ) -> Self {
        Self {
            element_context: context,
            mark: RenderMark::new(),
            layer_mark: Default::default(),
            inner: SyncMutex::new(RenderObjectInner {
                cache: RenderCache::new(),
                render,
                children,
            }),
        }
    }

    fn update<T>(
        &self,
        op: impl FnOnce(
            &mut R,
            &mut ContainerOf<R::ChildContainer, ArcChildRenderObject<R::ChildProtocol>>,
        ) -> T,
    ) -> T {
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        op(&mut inner_reborrow.render, &mut inner_reborrow.children)
    }
}

pub trait ChildRenderObject<PP: Protocol>:
    AnyRenderObject
    + crate::sync::ChildRenderObjectLayoutExt<PP>
    + crate::sync::ChildRenderObjectPaintExt<PP>
    + crate::sync::ChildRenderObjectHitTestExt<PP>
    + Send
    + Sync
{
    fn as_arc_any_render_object(self: Arc<Self>) -> ArcAnyRenderObject;
}

impl<R> ChildRenderObject<R::ParentProtocol> for RenderObject<R>
where
    R: Render,
{
    fn as_arc_any_render_object(self: Arc<Self>) -> ArcAnyRenderObject {
        self
    }
}

pub trait AnyRenderObject: crate::sync::AnyRenderObjectLayoutExt + Send + Sync {
    fn element_context(&self) -> &ElementContextNode;
    fn detach_render_object(&self);
    fn downcast_arc_any_layer_render_object(self: Arc<Self>) -> Option<ArcAnyLayerRenderObject>;

    fn mark_render_action(
        &self,
        child_render_action: RenderAction,
        subtree_has_action: RenderAction,
    ) -> RenderAction;

    fn try_as_aweak_any_layer_render_object(
        render_object: &Arc<Self>,
    ) -> Option<AweakAnyLayerRenderObject>
    where
        Self: Sized;
}

impl<R> AnyRenderObject for RenderObject<R>
where
    R: Render,
{
    fn element_context(&self) -> &ElementContextNode {
        todo!()
    }

    fn detach_render_object(&self) {
        todo!()
    }

    fn downcast_arc_any_layer_render_object(self: Arc<Self>) -> Option<ArcAnyLayerRenderObject> {
        todo!()
    }

    fn mark_render_action(
        &self,
        child_render_action: RenderAction,
        subtree_has_action: RenderAction,
    ) -> RenderAction {
        todo!()
    }

    fn try_as_aweak_any_layer_render_object(
        render_object: &Arc<Self>,
    ) -> Option<AweakAnyLayerRenderObject>
    where
        Self: Sized,
    {
        todo!()
    }
}

pub trait ParentRenderObject<CP: Protocol>: Send + Sync + 'static {}

pub trait ChildRenderObjectWithCanvas<C: Canvas>:
    CastInterfaceByRawPtr + Send + Sync + 'static
{
}

impl<R> ChildRenderObjectWithCanvas<<R::RenderImpl as ImplAdopterLayer<R>>::AdopterCanvas>
    for RenderObject<R>
where
    R: Render,
{
}

impl<C> dyn ChildRenderObjectWithCanvas<C>
where
    C: Canvas,
{
    pub fn query_interface_ref<T: ?Sized + 'static>(&self) -> Option<&T> {
        default_query_interface_ref(self)
    }

    pub fn query_interface_box<T: ?Sized + 'static>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
        default_query_interface_box(self)
    }

    pub fn query_interface_arc<T: ?Sized + 'static>(self: Arc<Self>) -> Result<Arc<T>, Arc<Self>> {
        default_query_interface_arc(self)
    }
}
