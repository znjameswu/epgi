mod layer_or_unit;
pub use layer_or_unit::*;

mod mark;
pub use mark::*;

mod node;
pub use node::*;

use std::any::TypeId;

use crate::foundation::{
    default_cast_interface_by_table_raw, default_cast_interface_by_table_raw_mut,
    default_query_interface_arc, default_query_interface_box, default_query_interface_ref,
    AnyRawPointer, Arc, Aweak, Canvas, CastInterfaceByRawPtr, HktContainer, PaintContext, Protocol,
    Transform, TransformHitPosition,
};

use super::{ArcAnyLayeredRenderObject, ArcElementContextNode, ElementContextNode};

pub type ArcChildRenderObject<P> = Arc<dyn ChildRenderObject<P>>;
pub type ArcAnyRenderObject = Arc<dyn AnyRenderObject>;
pub type AweakAnyRenderObject = Aweak<dyn AnyRenderObject>;
pub type AweakParentRenderObject<P> = Arc<dyn ParentRenderObject<ChildProtocol = P>>;
pub type ArcChildRenderObjectWithCanvas<C> = Arc<dyn ChildRenderObjectWithCanvas<C>>;

pub trait Render: Sized + Send + Sync + 'static {
    // type Element: Element<ArcRenderObject = Arc<RenderObject<Self>>>;

    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;

    type ChildContainer: HktContainer;

    const IS_REPAINT_BOUNDARY: bool = false;

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
    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<Self>) -> AnyRawPointer)]
    {
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
pub trait DryLayout: Render {
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
    + 'static
{
    fn as_arc_any_render_object(self: Arc<Self>) -> ArcAnyRenderObject;
    fn detach(&self);
}

impl<R> ChildRenderObject<R::ParentProtocol> for RenderObject<R>
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

pub trait AnyRenderObject: crate::sync::AnyRenderObjectLayoutExt + Send + Sync + 'static {
    fn element_context(&self) -> &ElementContextNode;
    fn detach(&self);
    fn downcast_arc_any_layer_render_object(self: Arc<Self>) -> Option<ArcAnyLayeredRenderObject>;
}

impl<R> AnyRenderObject for RenderObject<R>
where
    R: Render,
{
    fn element_context(&self) -> &ElementContextNode {
        &self.element_context
    }

    fn detach(&self) {
        todo!()
    }

    fn downcast_arc_any_layer_render_object(self: Arc<Self>) -> Option<ArcAnyLayeredRenderObject> {
        <R::LayerOrUnit as LayerOrUnit<R>>::downcast_arc_any_layer_render_object(self)
    }
}

pub trait ParentRenderObject: Send + Sync + 'static {
    type ChildProtocol: Protocol;
}

pub trait ChildRenderObjectWithCanvas<C: Canvas>:
    CastInterfaceByRawPtr + Send + Sync + 'static
{
}

impl<R> ChildRenderObjectWithCanvas<<R::ParentProtocol as Protocol>::Canvas> for RenderObject<R> where
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
