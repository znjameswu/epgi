use std::any::TypeId;

use crate::{
    foundation::{
        default_cast_interface_by_table_raw, default_cast_interface_by_table_raw_mut,
        default_query_interface_box, default_query_interface_ref, AnyRawPointer, Arc, Aweak,
        Canvas, CastInterfaceByRawPtr, Protocol, TransformHitPosition,
    },
    tree::{AweakAnyRenderObject, Render, RenderObject},
};

use super::TransformedHitTestTarget;

pub trait AnyTransformedHitTestEntry: CastInterfaceByRawPtr {}

impl dyn AnyTransformedHitTestEntry {
    pub fn query_interface_ref<T: ?Sized + 'static>(&self) -> Option<&T> {
        default_query_interface_ref(self)
    }

    pub fn query_interface_box<T: ?Sized + 'static>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
        default_query_interface_box(self)
    }
}

pub struct TransformedHitTestEntry<R: Render> {
    pub render_object: Aweak<RenderObject<R>>,
    pub hit_position: <<R::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
    pub transform: <R::ParentProtocol as Protocol>::Offset,
}

impl<R> CastInterfaceByRawPtr for TransformedHitTestEntry<R>
where
    R: Render,
{
    fn cast_interface_raw(&self, raw_ptr_type_id: TypeId) -> Option<AnyRawPointer> {
        default_cast_interface_by_table_raw(self, raw_ptr_type_id, R::all_hit_test_interfaces())
    }

    fn cast_interface_raw_mut(&mut self, raw_ptr_type_id: TypeId) -> Option<AnyRawPointer> {
        default_cast_interface_by_table_raw_mut(self, raw_ptr_type_id, R::all_hit_test_interfaces())
    }
}

impl<R> AnyTransformedHitTestEntry for TransformedHitTestEntry<R> where R: Render {}

pub trait ChildHitTestEntry<C: Canvas>: Send + Sync {
    fn prepend_transform(&mut self, transform: &C::Transform);

    fn with_position(&self, hit_position: C::HitPosition) -> Box<dyn AnyTransformedHitTestEntry>;

    fn render_object(&self) -> AweakAnyRenderObject;
}

pub struct InLayerHitTestEntry<R: Render> {
    target: TransformedHitTestTarget<R>,
    transform: Option<<<R::ParentProtocol as Protocol>::Canvas as Canvas>::Transform>,
}

impl<R> InLayerHitTestEntry<R>
where
    R: Render,
{
    pub(super) fn new(
        target: TransformedHitTestTarget<R>,
        transform: Option<<<R::ParentProtocol as Protocol>::Canvas as Canvas>::Transform>,
    ) -> Self {
        Self { target, transform }
    }
}

impl<R> ChildHitTestEntry<<R::ParentProtocol as Protocol>::Canvas> for InLayerHitTestEntry<R>
where
    R: Render,
{
    fn prepend_transform(
        &mut self,
        transform: &<<R::ParentProtocol as Protocol>::Canvas as Canvas>::Transform,
    ) {
        let new_prepend_transform = self
            .transform
            .as_ref()
            .map(|self_transform| {
                <<R::ParentProtocol as Protocol>::Canvas as Canvas>::mul_transform_ref(
                    self_transform,
                    transform,
                )
            })
            .unwrap_or_else(|| transform.clone());
        self.transform = Some(new_prepend_transform);
    }

    fn with_position(
        &self,
        hit_position: <<R::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
    ) -> Box<dyn AnyTransformedHitTestEntry> {
        let hit_position = if let Some(transform) = self.transform.as_ref() {
            transform.transform(&hit_position)
        } else {
            hit_position
        };
        Box::new(TransformedHitTestEntry {
            render_object: self.target.render_object.clone(),
            hit_position,
            transform: self.target.transform.clone(),
        })
    }

    fn render_object(&self) -> AweakAnyRenderObject {
        self.target.render_object.clone()
    }
}

pub struct LinkedHitTestEntry<PC: Canvas, CC: Canvas> {
    prepend_transform: Option<PC::Transform>,
    transform: Arc<dyn TransformHitPosition<PC, CC>>,
    next: Box<dyn ChildHitTestEntry<CC>>,
}

impl<PC, CC> LinkedHitTestEntry<PC, CC>
where
    PC: Canvas,
    CC: Canvas,
{
    pub(super) fn new(
        prepend_transform: Option<PC::Transform>,
        transform: Arc<dyn TransformHitPosition<PC, CC>>,
        next: Box<dyn ChildHitTestEntry<CC>>,
    ) -> Self {
        Self {
            prepend_transform,
            transform,
            next,
        }
    }
}

impl<PC, CC> ChildHitTestEntry<PC> for LinkedHitTestEntry<PC, CC>
where
    PC: Canvas,
    CC: Canvas,
{
    fn prepend_transform(&mut self, transform: &<PC as Canvas>::Transform) {
        let new_prepend_transform = self
            .prepend_transform
            .as_ref()
            .map(|self_prepend_transform| PC::mul_transform_ref(self_prepend_transform, transform))
            .unwrap_or_else(|| transform.clone());
        self.prepend_transform = Some(new_prepend_transform);
    }

    fn with_position(
        &self,
        hit_position: <PC as Canvas>::HitPosition,
    ) -> Box<dyn AnyTransformedHitTestEntry> {
        let hit_position = if let Some(prepend_transform) = self.prepend_transform.as_ref() {
            prepend_transform.transform(&hit_position)
        } else {
            hit_position
        };
        let hit_position = self.transform.transform(&hit_position);
        self.next.with_position(hit_position)
    }

    fn render_object(&self) -> AweakAnyRenderObject {
        self.next.render_object()
    }
}
