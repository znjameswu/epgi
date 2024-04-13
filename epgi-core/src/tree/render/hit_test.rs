use std::any::TypeId;

use crate::foundation::{
    default_cast_interface_by_table_raw, default_cast_interface_by_table_raw_mut, AnyRawPointer,
    Canvas, CastInterfaceByRawPtr, ContainerOf, Protocol, Transform, TransformHitPosition,
};

use super::{
    ArcChildLayerRenderObject, ArcChildRenderObject, ArcChildRenderObjectWithCanvas,
    RecordedChildLayer, Render, RenderBase, RenderObject,
};

pub trait HitTest: RenderBase {
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
        children: &ContainerOf<Self::ChildContainer, ArcChildRenderObject<Self::ChildProtocol>>,
        adopted_children: &[RecordedChildLayer<<Self::ChildProtocol as Protocol>::Canvas>],
    ) -> HitTestResult {
        use HitTestResult::*;
        let hit_self = self.hit_test_self(ctx.curr_position(), size, offset, memo);
        if !hit_self {
            // Stop hit-test children if the hit is outside of parent
            return NotHit;
        }

        let hit_children =
            self.hit_test_children(ctx, size, offset, memo, children, adopted_children);
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
    fn hit_test_children(
        &self,
        ctx: &mut HitTestContext<<Self::ParentProtocol as Protocol>::Canvas>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &ContainerOf<Self::ChildContainer, ArcChildRenderObject<Self::ChildProtocol>>,
        adopted_children: &[RecordedChildLayer<<Self::ChildProtocol as Protocol>::Canvas>],
    ) -> bool;

    // The reason we separate hit_test_self from hit_test_children is that we do not wish to leak hit_position into hit_test_children
    // Therefore preventing implementer to perform transform on hit_position rather than recording it in
    #[allow(unused_variables)]
    fn hit_test_self(
        &self,
        position: &<<Self::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
    ) -> bool {
        Self::ParentProtocol::position_in_shape(position, offset, size)
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

#[derive(PartialEq, Eq)]
pub enum HitTestResult {
    NotHit,
    /// Add self to hit list, but pretend to the parent that the hit inside subtree has never happened
    HitThroughSelf,
    Hit,
}

pub enum HitTestBehavior {
    Transparent,
    DeferToChild,
    Opaque,
}

pub struct HitTestContext<C: Canvas> {
    position: C::HitPosition,
    curr_transform: C::Transform,
    curr_position: C::HitPosition,
    pub targets: Vec<(C::Transform, ArcChildRenderObjectWithCanvas<C>)>,
    trait_type_id: TypeId,
}

impl<C> HitTestContext<C>
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
    pub fn with_raw_transform<T>(
        &mut self,
        raw_transform: &C::Transform,
        op: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let new_transform = Transform::mul(&self.curr_transform, raw_transform);
        let old_position = std::mem::replace(
            &mut self.curr_position,
            new_transform.transform(&self.position),
        );
        let old_transform = std::mem::replace(&mut self.curr_transform, new_transform);
        let result = op(self);
        self.curr_transform = old_transform;
        self.curr_position = old_position;
        return result;
    }

    #[inline(always)]
    pub fn with_paint_transform<T>(
        &mut self,
        paint_transform: &C::Transform,
        op: impl FnOnce(&mut Self) -> T,
    ) -> Option<T> {
        paint_transform
            .inv()
            .map(|raw_transform| self.with_raw_transform(&raw_transform, op))
    }

    pub fn hit_test<P: Protocol<Canvas = C>>(
        &mut self,
        render_object: ArcChildRenderObject<P>,
    ) -> bool {
        return render_object.hit_test_with(self);
    }

    pub fn hit_test_adopted_layer(&mut self, render_object: ArcChildLayerRenderObject<C>) -> bool {
        return render_object.hit_test_from_adopter_with(self);
    }

    pub fn hit_test_with_raw_transform<P: Protocol<Canvas = C>>(
        &mut self,
        render_object: ArcChildRenderObject<P>,
        raw_transform: &C::Transform,
    ) -> bool {
        self.with_raw_transform(raw_transform, |ctx| ctx.hit_test(render_object))
    }

    pub fn hit_test_with_paint_transform<P: Protocol<Canvas = C>>(
        &mut self,
        render_object: ArcChildRenderObject<P>,
        paint_transform: &C::Transform,
    ) -> bool {
        self.with_paint_transform(paint_transform, |ctx| ctx.hit_test(render_object))
            .unwrap_or(false) // If the inverse cannot be found, it means the object has lost it size during transformation, and we consider this a no-hit
    }

    pub fn hit_test_adopted_layer_with_raw_transform(
        &mut self,
        render_object: ArcChildLayerRenderObject<C>,
        raw_transform: &C::Transform,
    ) -> bool {
        self.with_raw_transform(raw_transform, |ctx| {
            ctx.hit_test_adopted_layer(render_object)
        })
    }

    pub fn hit_test_w_adopted_layerith_paint_transform(
        &mut self,
        render_object: ArcChildLayerRenderObject<C>,
        paint_transform: &C::Transform,
    ) -> bool {
        self.with_paint_transform(paint_transform, |ctx| {
            ctx.hit_test_adopted_layer(render_object)
        })
        .unwrap_or(false) // If the inverse cannot be found, it means the object has lost it size during transformation, and we consider this a no-hit
    }
}

impl<R: Render> CastInterfaceByRawPtr for RenderObject<R> {
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
