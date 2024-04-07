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
        AnyRawPointer, Arc, Aweak, Canvas, CastInterfaceByRawPtr, Protocol, SyncMutex, Transform,
        TransformHitPosition,
    },
    sync::ImplAdopterLayer,
};

use super::{
    ArcAnyLayerRenderObject, ArcElementContextNode, AweakAnyLayerRenderObject, ContainerOf,
    ElementContextNode, TreeNode,
};

pub type ArcChildRenderObject<P> = Arc<dyn ChildRenderObject<P>>;
pub type ArcAnyRenderObject = Arc<dyn AnyRenderObject>;
pub type AweakAnyRenderObject = Aweak<dyn AnyRenderObject>;
pub type AweakParentRenderObject<P> = Arc<dyn ParentRenderObject<P>>;
pub type ArcChildRenderObjectWithCanvas<C> = Arc<dyn ChildRenderObjectWithCanvas<C>>;

pub trait Render: TreeNode + Sized + 'static {
    type LayoutMemo: Send + Sync;
    type RenderImpl: ImplRender<Render = Self>;

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<Self>) -> AnyRawPointer)]
    {
        &[]
    }

    fn detach(&mut self) {}
    const NOOP_DETACH: bool = false;
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

impl<R> ImplRenderObjectReconcile<R> for RenderObject<R>
where
    R: Render,
{
    fn new(
        render: R,
        children: ContainerOf<R, ArcChildRenderObject<R::ChildProtocol>>,
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
        op: impl FnOnce(&mut R, &mut ContainerOf<R, ArcChildRenderObject<R::ChildProtocol>>) -> T,
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

impl<R> AnyRenderObject for RenderObject<R>
where
    R: Render,
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
