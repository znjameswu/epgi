mod mark;
pub use mark::*;

mod node;
pub use node::*;

use std::{any::TypeId, marker::PhantomData};

use crate::{
    foundation::{
        default_cast_interface_by_table_raw, default_cast_interface_by_table_raw_mut,
        default_query_interface_arc, default_query_interface_box, default_query_interface_ref,
        AnyRawPointer, Arc, AsIterator, Aweak, Canvas, CastInterfaceByRawPtr, HktContainer, Key,
        LayerProtocol, PaintContext, Protocol, SyncMutex, Transform, TransformHitPosition,
    },
    sync::{
        ImplComposite, ImplLayout, ImplPaint, SelectHitTestImpl, SelectLayoutImpl, SelectPaintImpl,
    },
};

use super::{
    ArcAnyLayerRenderObject, ArcElementContextNode, AweakAnyLayerRenderObject,
    ChildLayerProducingIterator, ContainerOf, ElementContextNode, LayerCache,
    LayerCompositionConfig, LayerMark, PaintResults,
};

pub type ArcChildRenderObject<P> = Arc<dyn ChildRenderObject<P>>;
pub type ArcAnyRenderObject = Arc<dyn AnyRenderObject>;
pub type AweakAnyRenderObject = Aweak<dyn AnyRenderObject>;
pub type AweakParentRenderObject<P> = Arc<dyn ParentRenderObject<P>>;
pub type ArcChildRenderObjectWithCanvas<C> = Arc<dyn ChildRenderObjectWithCanvas<C>>;

pub trait TreeNode: Send + Sync {
    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;
    type ChildContainer: HktContainer;
}

pub trait Render:
    TreeNode
    + HasLayoutMemo
    + SelectOrphanLayer<
        false,
        AdopterCanvas = <<Self as TreeNode>::ParentProtocol as Protocol>::Canvas,
    > + Sized
    + 'static
{
    // type DryLayout: ConstBool;
    // type LayerPaint: ConstBool;
    // type CachedComposite: ConstBool;
    // type OrphanLayer: ConstBool;

    type RenderImpl: ImplRender;

    type RenderObject: ImplRenderObjectReconcile<Self> + ChildRenderObject<Self::ParentProtocol>;

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut Self::RenderObject) -> AnyRawPointer)]
    {
        &[]
    }

    fn detach(&mut self) {}
    const NOOP_DETACH: bool = false;
}

pub trait ImplRender: ImplLayout<Self::Render> + ImplPaint<Self::Render> {
    type Render: Render;
}

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
{
    type Render = R;
}

pub trait ImplRenderBySuper: ImplLayout<Self::Render> + ImplPaint<Self::Render> {
    type Render: Render;
    type Super: ImplRender<Render = Self::Render>;
}

impl<T> ImplRender for T
where
    T: ImplRenderBySuper,
{
    type Render = T::Render;
}

pub trait HasLayoutMemo {
    type LayoutMemo: Send + Sync;
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
pub trait Layout: TreeNode + HasLayoutMemo {
    fn perform_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo);
}

pub trait DryLayout: TreeNode + HasLayoutMemo {
    fn compute_dry_layout(
        &self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
    ) -> <Self::ParentProtocol as Protocol>::Size;

    fn compute_layout_memo(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &<Self::ParentProtocol as Protocol>::Size,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> Self::LayoutMemo;
}

pub trait Paint:
    TreeNode + HasLayoutMemo + SelectLayerPaint<false, LayerMark = (), HktLayerCache = HktUnit>
{
    fn perform_paint(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    );

    fn hit_test_children(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
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

pub trait LayerPaint:
    TreeNode
    + SelectLayerPaint<
        true,
        LayerMark = LayerMark,
        HktLayerCache = HktLayerCache<<<Self as TreeNode>::ChildProtocol as Protocol>::Canvas>,
    > + SelectCachedComposite<false, CompositionCache = ()>
    + SelectCachedComposite<true>
    + SelectOrphanLayer<
        false,
        AdopterCanvas = <<Self as TreeNode>::ParentProtocol as Protocol>::Canvas,
    >
where
    Self::ParentProtocol: LayerProtocol,
    Self::ChildProtocol: LayerProtocol,
{
    fn paint_layer(
        &self,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
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

pub trait CompositeOld<const ORPHAN_LAYER: bool>:
    TreeNode
    + LayerPaint
    + SelectOrphanLayer<
        false,
        AdopterCanvas = <<Self as TreeNode>::ParentProtocol as Protocol>::Canvas,
    > + SelectOrphanLayer<ORPHAN_LAYER>
where
    Self::ParentProtocol: LayerProtocol,
    Self::ChildProtocol: LayerProtocol,
{
    fn composite_to(
        &self,
        encoding: &mut <<Self as SelectOrphanLayer<ORPHAN_LAYER>>::AdopterCanvas as Canvas>::Encoding,
        child_iterator: &mut impl ChildLayerProducingIterator<<Self::ChildProtocol as Protocol>::Canvas>,
        composition_config: &LayerCompositionConfig<
            <Self as SelectOrphanLayer<ORPHAN_LAYER>>::AdopterCanvas,
        >,
    );
    fn transform_config(
        self_config: &LayerCompositionConfig<
            <Self as SelectOrphanLayer<ORPHAN_LAYER>>::AdopterCanvas,
        >,
        child_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<Self as SelectOrphanLayer<ORPHAN_LAYER>>::AdopterCanvas>;
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

pub trait CachedCompositeOld<const ORPHAN_LAYER: bool = false>:
    TreeNode
    + LayerPaint
    + SelectCachedComposite<
        true,
        CompositionCache = <Self as CachedCompositeOld<ORPHAN_LAYER>>::CompositionCache,
    > + SelectOrphanLayer<ORPHAN_LAYER>
where
    Self::ParentProtocol: LayerProtocol,
    Self::ChildProtocol: LayerProtocol,
{
    type CompositionCache: Send + Sync + Clone + 'static;

    fn composite_into_cache(
        &self,
        child_iterator: &mut impl ChildLayerProducingIterator<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> <Self as CachedCompositeOld<ORPHAN_LAYER>>::CompositionCache;

    fn composite_from_cache_to(
        &self,
        encoding: &mut <<Self as SelectOrphanLayer<ORPHAN_LAYER>>::AdopterCanvas as Canvas>::Encoding,
        cache: &<Self as CachedCompositeOld<ORPHAN_LAYER>>::CompositionCache,
        composition_config: &LayerCompositionConfig<
            <Self as SelectOrphanLayer<ORPHAN_LAYER>>::AdopterCanvas,
        >,
    );

    fn transform_config(
        self_config: &LayerCompositionConfig<
            <Self as SelectOrphanLayer<ORPHAN_LAYER>>::AdopterCanvas,
        >,
        child_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<Self as SelectOrphanLayer<ORPHAN_LAYER>>::AdopterCanvas>;
}

/// Orphan layers can skip this implementation
pub trait HitTest: TreeNode + HasLayoutMemo {
    fn hit_test_children(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
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

// We COULD orthogonalize the Orphan/Structured vs Noncached/cached trait set,
// but that would inevitably bake directly into library user's code an explicit AdopterCanvas type
// either somewhere in an associated type or somewhere as a generic trait paramter.
// As an unproven idea, I would like to make orphan layer mechanism optional and not bake into anything more than necessary.
pub trait OrphanLayer: TreeNode
    + LayerPaint
    + SelectOrphanLayer<true, AdopterCanvas = <<Self as TreeNode>::ChildProtocol as Protocol>::Canvas>
where
    Self::ParentProtocol: LayerProtocol,
    Self::ChildProtocol: LayerProtocol,
{
    fn adopter_key(&self) -> Option<&Arc<dyn Key>>;

    // fn hit_test_children(
    //     &self,
    //     size: &<Self::ParentProtocol as Protocol>::Size,
    //     offset: &<Self::ParentProtocol as Protocol>::Offset,
    //     memo: &Self::LayoutMemo,
    //     children: &<Self::ChildContainer as HktContainer>::Container<
    //         ArcChildRenderObject<Self::ChildProtocol>,
    //     >,
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

pub trait SelectLayerPaint<const LAYER_PAINT: bool>: TreeNode + HasLayoutMemo {
    type LayerMark: Default + Send + Sync;
    type HktLayerCache: Hkt;
}

pub trait Hkt {
    type T<T>: Send + Sync
    where
        T: Send + Sync;
}

pub struct HktUnit;

impl Hkt for HktUnit {
    type T<T> = ()  where T: Send + Sync;
}

impl<T> SelectLayerPaint<false> for T
where
    T: Paint,
{
    type LayerMark = ();
    type HktLayerCache = HktUnit;
}

pub struct HktLayerCache<C>(PhantomData<C>);

impl<C: Canvas> Hkt for HktLayerCache<C> {
    type T<T> = LayerCache<C, T> where T: Send + Sync;
}

impl<R> SelectLayerPaint<true> for R
where
    R: Render,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    type LayerMark = LayerMark;
    type HktLayerCache = HktLayerCache<<R::ChildProtocol as Protocol>::Canvas>;
}

pub trait SelectOrphanLayer<const ORPHAN_LAYER: bool>: TreeNode {
    type AdopterCanvas: Canvas;
}

impl<R> SelectOrphanLayer<false> for R
where
    R: TreeNode,
{
    type AdopterCanvas = <R::ParentProtocol as Protocol>::Canvas;
}

impl<R> SelectOrphanLayer<true> for R
where
    R: LayerPaint + OrphanLayer,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
{
    type AdopterCanvas = <R::ChildProtocol as Protocol>::Canvas;
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

impl<
        R,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > CastInterfaceByRawPtr
    for RenderObject<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Render<RenderObject = Self>
        + SelectLayerPaint<LAYER_PAINT>
        + SelectCachedComposite<CACHED_COMPOSITE>,
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
        epgi_core::interface_query_table!($name, RenderObjectOld<$type>, $($trait,)*);
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

pub trait ImplRenderObjectReconcile<R: TreeNode>: Send + Sync + Sized {
    fn new(
        render: R,
        children: ContainerOf<R, ArcChildRenderObject<R::ChildProtocol>>,
        context: ArcElementContextNode,
    ) -> Self;

    fn update<T>(
        &self,
        op: impl FnOnce(&mut R, &mut ContainerOf<R, ArcChildRenderObject<R::ChildProtocol>>) -> T,
    ) -> T;
}

impl<
        R,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ImplRenderObjectReconcile<R>
    for RenderObject<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Render<RenderObject = Self>
        + SelectLayerPaint<LAYER_PAINT>
        + SelectCachedComposite<CACHED_COMPOSITE>,
{
    fn new(
        render: R,
        children: ContainerOf<R, ArcChildRenderObject<<R>::ChildProtocol>>,
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
        op: impl FnOnce(&mut R, &mut ContainerOf<R, ArcChildRenderObject<<R>::ChildProtocol>>) -> T,
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

impl<
        R,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ChildRenderObject<R::ParentProtocol>
    for RenderObject<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Render<RenderObject = Self>
        + SelectLayerPaint<LAYER_PAINT>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: SelectLayoutImpl<DRY_LAYOUT>,
    R: SelectPaintImpl<LAYER_PAINT, ORPHAN_LAYER>,
    R: SelectHitTestImpl<ORPHAN_LAYER>,
{
    fn as_arc_any_render_object(self: Arc<Self>) -> ArcAnyRenderObject {
        self
    }
}

pub trait AnyRenderObject: crate::sync::AnyRenderObjectLayoutExt + Send + Sync {
    fn element_context(&self) -> &ElementContextNode;
    fn detach(&self);
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

// impl<R> AnyRenderObject for RenderObject<R>
// where
//     R: RenderNew,
// {
//     fn element_context(&self) -> &ElementContextNode {
//         todo!()
//     }

//     fn detach(&self) {
//         todo!()
//     }

//     fn downcast_arc_any_layer_render_object(self: Arc<Self>) -> Option<ArcAnyLayerRenderObject> {
//         todo!()
//     }
// }

impl<
        R,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > AnyRenderObject for RenderObject<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Render<RenderObject = Self>
        + SelectLayerPaint<LAYER_PAINT>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    Self: crate::sync::AnyRenderObjectLayoutExt,
{
    fn element_context(&self) -> &ElementContextNode {
        todo!()
    }

    fn detach(&self) {
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

// impl<R> ChildRenderObjectWithCanvas<<R::ParentProtocol as Protocol>::Canvas> for RenderObject<R> where
//     R: RenderNew
// {
// }

impl<
        R,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ChildRenderObjectWithCanvas<<R as SelectOrphanLayer<ORPHAN_LAYER>>::AdopterCanvas>
    for RenderObject<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Render<RenderObject = Self>
        + SelectLayerPaint<LAYER_PAINT>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: SelectOrphanLayer<ORPHAN_LAYER>,
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
