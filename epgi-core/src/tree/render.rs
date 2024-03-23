mod layer_or_unit;
pub use layer_or_unit::*;

mod mark;
pub use mark::*;

mod node;
pub use node::*;

use std::{any::TypeId, marker::PhantomData};

use crate::{
    foundation::{
        default_cast_interface_by_table_raw, default_cast_interface_by_table_raw_mut,
        default_query_interface_arc, default_query_interface_box, default_query_interface_ref,
        AnyRawPointer, Arc, AsIterator, Aweak, Canvas, CastInterfaceByRawPtr, ConstBool, False,
        HktContainer, Key, LayerProtocol, PaintContext, Protocol, Transform, TransformHitPosition,
        True,
    },
    sync::{
        SelectCompositeImpl, SelectCompositionCacheImpl, SelectHitTestImpl, SelectLayerAdoptImpl,
    },
};

use super::{
    ArcAnyLayerRenderObject, ArcElementContextNode, ChildLayerProducingIterator,
    ElementContextNode, LayerCache, LayerCompositionConfig, LayerMark, PaintResults,
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

pub trait RenderNew:
    TreeNode
    + HasLayoutMemo
    + SelectLayoutImpl<Self::DryLayout>
    + SelectPaintImpl<Self::LayerPaint, Self::OrphanComposite>
    + SelectCompositeImpl<Self::CachedComposite, Self::OrphanComposite>
    + SelectCompositionCacheImpl<Self::CachedComposite>
    + SelectLayerAdoptImpl<Self::OrphanComposite>
    + SelectHitTestImpl<Self::OrphanComposite>
    + Sized
    + 'static
{
    type DryLayout: ConstBool;
    type LayerPaint: ConstBool;
    type CachedComposite: ConstBool;
    type OrphanComposite: ConstBool;

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<Self>) -> AnyRawPointer)]
    {
        &[]
    }
}

pub trait HasLayoutMemo {
    type LayoutMemo: Send + Sync;
}

pub trait Layout: TreeNode + HasLayoutMemo + SelectLayoutImpl<False> {
    fn perform_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo);
}

pub trait DryLayout: TreeNode + HasLayoutMemo + SelectLayoutImpl<True> {
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
    TreeNode + HasLayoutMemo + SelectPaintImpl<False, False,  LayerMark = (), HktLayerCache = HktUnit>
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
    + SelectPaintImpl<
        True,
        LayerMark = LayerMark,
        HktLayerCache = HktLayerCache<<<Self as TreeNode>::ChildProtocol as Protocol>::Canvas>,
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

    fn transform_config(
        self_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
        child_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>;

    fn transform_hit_test(
        &self,
        position: &<<Self::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
    ) -> <<Self::ChildProtocol as Protocol>::Canvas as Canvas>::HitPosition;

    fn key(&self) -> Option<&Arc<dyn Key>> {
        None
    }
}

pub trait Composite:
    TreeNode
    + SelectCompositeImpl<
        False,
        False,
        AdopterCanvas = <<Self as TreeNode>::ParentProtocol as Protocol>::Canvas,
        CompositionCache = (),
    >
{
    fn composite_to(
        encoding: &mut <<Self::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        child_iterator: &mut impl ChildLayerProducingIterator<<Self::ChildProtocol as Protocol>::Canvas>,
        composition_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
    );
}

pub trait CachedComposite:
    TreeNode
    + SelectCompositeImpl<
        True,
        False,
        AdopterCanvas = <<Self as TreeNode>::ParentProtocol as Protocol>::Canvas,
        CompositionCache = <Self as CachedComposite>::CompositionCache,
    >
{
    type CompositionCache: Send + Sync + Clone + 'static;

    fn composite_into_cache(
        child_iterator: &mut impl ChildLayerProducingIterator<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> <Self as CachedComposite>::CompositionCache;

    fn composite_from_cache_to(
        encoding: &mut <<Self::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        cache: &<Self as CachedComposite>::CompositionCache,
        composition_config: &LayerCompositionConfig<<Self::ParentProtocol as Protocol>::Canvas>,
    );
}

pub trait HitTest:
    TreeNode
    + HasLayoutMemo
    + SelectLayerAdoptImpl<
        False,
        AdopterCanvas = <<Self as TreeNode>::ParentProtocol as Protocol>::Canvas,
    >
{
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
pub trait OrphanComposite:
    TreeNode
    + SelectCompositeImpl<
        False,
        True,
        AdopterCanvas = <<Self as TreeNode>::ChildProtocol as Protocol>::Canvas,
        CompositionCache = (),
    >
{
    fn composite_orphan_to(
        encoding: &mut <<Self::ChildProtocol as Protocol>::Canvas as Canvas>::Encoding,
        child_iterator: &mut impl ChildLayerProducingIterator<<Self::ChildProtocol as Protocol>::Canvas>,
        composition_config: &LayerCompositionConfig<<Self::ChildProtocol as Protocol>::Canvas>,
    );

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

pub trait SelectLayoutImpl<DryLayout: ConstBool>: TreeNode + HasLayoutMemo {
    fn perform_layout_without_resize(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &mut <Self::ParentProtocol as Protocol>::Size,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> Self::LayoutMemo;
    fn perform_wet_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo);
}

impl<T> SelectLayoutImpl<False> for T
where
    T: Layout,
{
    fn perform_layout_without_resize(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &mut <Self::ParentProtocol as Protocol>::Size,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> Self::LayoutMemo {
        let (new_size, memo) = self.perform_layout(constraints, children);
        *size = new_size;
        memo
    }

    fn perform_wet_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo) {
        self.perform_layout(constraints, children)
    }
}

impl<T> SelectLayoutImpl<True> for T
where
    T: DryLayout,
{
    fn perform_layout_without_resize(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &mut <Self::ParentProtocol as Protocol>::Size,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> Self::LayoutMemo {
        self.compute_layout_memo(constraints, size, children)
    }

    fn perform_wet_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo) {
        let size = self.compute_dry_layout(constraints);
        let memo = self.compute_layout_memo(constraints, &size, children);
        (size, memo)
    }
}

pub trait SelectPaintImpl<LayerPaint: ConstBool, OrphanComposite: ConstBool>: TreeNode + HasLayoutMemo {
    type LayerMark: Send + Sync;
    type HktLayerCache: Hkt;

    fn option_perform_paint(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    );

    fn option_paint_self_as_child_layer(
        render_object: &Arc<RenderObject<Self>>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) where
        Self: RenderNew + Sized;
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

impl<T> SelectPaintImpl<False, False> for T
where
    T: Paint,
{
    type LayerMark = ();
    type HktLayerCache = HktUnit;

    fn option_perform_paint(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) {
        self.perform_paint(size, offset, memo, children, paint_ctx)
    }

    fn option_paint_self_as_child_layer(
        render_object: &Arc<RenderObject<Self>>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) where
        Self: RenderNew + Sized,
    {
        // no-op
    }
}

pub struct HktLayerCache<C>(PhantomData<C>);

impl<C: Canvas> Hkt for HktLayerCache<C> {
    type T<T> = LayerCache<C, T> where T: Send + Sync;
}

impl<R> SelectPaintImpl<True, False> for R
where
    R: RenderNew<LayerPaint = True>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    type LayerMark = LayerMark;
    type HktLayerCache = HktLayerCache<<R::ChildProtocol as Protocol>::Canvas>;

    fn option_perform_paint(
        &self,
        _size: &<Self::ParentProtocol as Protocol>::Size,
        _offset: &<Self::ParentProtocol as Protocol>::Offset,
        _memo: &Self::LayoutMemo,
        _children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
        _paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) {
    }

    fn option_paint_self_as_child_layer(
        render_object: &Arc<RenderObject<Self>>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) where
        Self: RenderNew + Sized,
    {
        paint_ctx.add_layer(render_object.clone(), |transform| {
            <R::ParentProtocol as LayerProtocol>::compute_layer_transform(offset, transform)
        })
    }
}

pub trait Render: Sized + Send + Sync + 'static {
    // type Element: Element<ArcRenderObject = Arc<RenderObject<Self>>>;

    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;

    // type RenderObject: ChildRenderObject<Self::ParentProtocol>
    //     + ParentRenderObject<Self::ChildProtocol>
    //     + ChildRenderObjectWithCanvas<<Self::ParentProtocol as Protocol>::Canvas>;

    type ChildContainer: HktContainer;

    fn detach(&mut self) {}

    /// Whether [Render::detach] is a no-op
    ///
    /// When set to true, [Render::detach]'s implementation will be ignored,
    /// Certain optimizations to reduce mutex usages will be applied during the commit phase.
    /// However, if [Render::detach] is actually not no-op, doing this will cause unexpected behaviors.
    ///
    /// Setting to false will always guarantee the correct behavior. And this is why it's left as false.
    /// Render objects that do not manage any external resources may consider setting this to true.
    const NOOP_DETACH: bool = false;

    type LayoutMemo: Send + Sync + 'static;

    fn perform_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo);

    /// If this is not None, then [`Self::perform_layout`]'s implementation will be ignored.
    const DRY_LAYOUT_FUNCTION_TABLE: Option<DryLayoutFunctionTable<Self>> = None;

    // We don't make perform paint into an associated constant because it has an generic paramter
    // Then we have to go to associated generic type, which makes the boilerplate explodes.
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

    type LayerOrUnit: LayerOrUnit<Self>;

    // Use RenderObject<R> as receiver instead of a self receiver.
    // Because the receiver type must be at least no lower than the last polymorphic boundary,
    // if we wish to extend interface from external code.
    fn all_hit_test_interfaces(
    ) -> &'static [(TypeId, fn(*mut RenderObjectOld<Self>) -> AnyRawPointer)] {
        &[]
    }
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

    pub fn interface_exist_on<R: RenderNew>(&self) -> bool {
        R::all_hit_test_interfaces()
            .iter()
            .any(|(type_id, _)| self.trait_type_id == *type_id)
    }

    pub fn interface_exist_on_old<R: Render>(&self) -> bool {
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
    R: RenderNew,
{
    fn cast_interface_raw(&self, trait_type_id: TypeId) -> Option<AnyRawPointer> {
        default_cast_interface_by_table_raw(self, trait_type_id, R::all_hit_test_interfaces())
    }

    fn cast_interface_raw_mut(&mut self, trait_type_id: TypeId) -> Option<AnyRawPointer> {
        default_cast_interface_by_table_raw_mut(self, trait_type_id, R::all_hit_test_interfaces())
    }
}

impl<R> CastInterfaceByRawPtr for RenderObjectOld<R>
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
        epgi_core::interface_query_table!($name, RenderObjectOld<$type>, $($trait,)*);
    };
}

pub use hit_test_interface_query_table;

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
pub trait DryLayoutOld: Render {
    const DRY_LAYOUT_FUNCTION_TABLE: Option<DryLayoutFunctionTable<Self>> =
        Some(DryLayoutFunctionTable {
            compute_dry_layout: Self::compute_dry_layout,
            compute_layout_memo: Self::compute_layout_memo,
        });

    fn compute_dry_layout(
        &self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
    ) -> <Self::ParentProtocol as Protocol>::Size;

    fn compute_layout_memo(
        &self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &<Self::ParentProtocol as Protocol>::Size,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
    ) -> Self::LayoutMemo;
}

pub struct DryLayoutFunctionTable<R: Render> {
    pub compute_dry_layout: fn(
        &R,
        &<R::ParentProtocol as Protocol>::Constraints,
    ) -> <R::ParentProtocol as Protocol>::Size,

    pub compute_layout_memo: fn(
        &R,
        &<R::ParentProtocol as Protocol>::Constraints,
        &<R::ParentProtocol as Protocol>::Size,
        &<R::ChildContainer as HktContainer>::Container<ArcChildRenderObject<R::ChildProtocol>>,
    ) -> R::LayoutMemo,
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

pub trait ChildRenderObject<PP: Protocol>:
    AnyRenderObject
    + crate::sync::ChildRenderObjectLayoutExt<PP>
    + crate::sync::ChildRenderObjectPaintExt<PP>
    + crate::sync::ChildRenderObjectHitTestExt<PP>
    + Send
    + Sync
{
    fn as_arc_any_render_object(self: Arc<Self>) -> ArcAnyRenderObject;
    fn detach(&self);
}

impl<R> ChildRenderObject<R::ParentProtocol> for RenderObject<R>
where
    R: RenderNew,
{
    fn as_arc_any_render_object(self: Arc<Self>) -> ArcAnyRenderObject {
        todo!()
    }

    fn detach(&self) {
        todo!()
    }
}

impl<R> ChildRenderObject<R::ParentProtocol> for RenderObjectOld<R>
where
    R: Render,
{
    fn as_arc_any_render_object(self: Arc<Self>) -> ArcAnyRenderObject {
        self
    }

    fn detach(&self) {
        todo!()
    }
}

pub trait AnyRenderObject: crate::sync::AnyRenderObjectLayoutExt + Send + Sync {
    fn element_context(&self) -> &ElementContextNode;
    fn detach(&self);
    fn downcast_arc_any_layer_render_object(self: Arc<Self>) -> Option<ArcAnyLayerRenderObject>;
}

impl<R> AnyRenderObject for RenderObject<R>
where
    R: RenderNew,
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
}

impl<R> AnyRenderObject for RenderObjectOld<R>
where
    R: Render,
{
    fn element_context(&self) -> &ElementContextNode {
        &self.element_context
    }

    fn detach(&self) {
        todo!()
    }

    fn downcast_arc_any_layer_render_object(self: Arc<Self>) -> Option<ArcAnyLayerRenderObject> {
        <R::LayerOrUnit as LayerOrUnit<R>>::downcast_arc_any_layer_render_object(self)
    }
}

pub trait ParentRenderObject<CP: Protocol>: Send + Sync + 'static {}

pub trait ChildRenderObjectWithCanvas<C: Canvas>:
    CastInterfaceByRawPtr + Send + Sync + 'static
{
}

impl<R> ChildRenderObjectWithCanvas<<R::ParentProtocol as Protocol>::Canvas> for RenderObject<R> where
    R: RenderNew
{
}

impl<R> ChildRenderObjectWithCanvas<<R::ParentProtocol as Protocol>::Canvas> for RenderObjectOld<R> where
    R: Render
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
