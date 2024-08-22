use std::any::Any;

use crate::{
    foundation::{Arc, Canvas, CastInterfaceByRawPtr, Protocol},
    sync::{
        AnyRenderObjectLayoutExt, ChildRenderObjectHitTestExt, ChildRenderObjectLayoutExt,
        ChildRenderObjectPaintExt,
    },
    tree::ElementContextNode,
};

use super::{
    AnyRenderObject, ArcAnyLayerRenderObject, ArcAnyRenderObject, ArcChildRenderObject,
    ChildRenderObject, RenderMark,
};

pub struct ChildRenderObjectParentDataWrapper<P: Protocol, T> {
    parent_data: T,
    render_object: ArcChildRenderObject<P>,
}

impl<P, T> ChildRenderObject<P> for ChildRenderObjectParentDataWrapper<P, T>
where
    P: Protocol,
    T: Send + Sync + 'static,
{
    fn as_arc_any_render_object(self: Arc<Self>) -> ArcAnyRenderObject {
        self
    }
}

impl<P, T> ChildRenderObjectLayoutExt<P> for ChildRenderObjectParentDataWrapper<P, T>
where
    P: Protocol,
    T: Send + Sync + 'static,
{
    fn layout_use_size(&self, constraints: &P::Constraints) -> P::Size {
        self.render_object.layout_use_size(constraints)
    }

    fn layout(&self, constraints: &P::Constraints) {
        self.render_object.layout(constraints)
    }

    fn get_intrinsics(&self, intrinsics: &mut P::Intrinsics) {
        self.render_object.get_intrinsics(intrinsics)
    }
}

impl<P, T> ChildRenderObjectPaintExt<P> for ChildRenderObjectParentDataWrapper<P, T>
where
    P: Protocol,
    T: Send + Sync + 'static,
{
    fn paint(
        self: Arc<Self>,
        offset: &P::Offset,
        paint_ctx: &mut <P::Canvas as Canvas>::PaintContext<'_>,
    ) {
        self.render_object.clone().paint(offset, paint_ctx)
    }

    fn paint_scan(
        self: Arc<Self>,
        offset: &P::Offset,
        paint_ctx: &mut <P::Canvas as Canvas>::PaintScanner<'_>,
    ) {
        self.render_object.clone().paint_scan(offset, paint_ctx)
    }
}

impl<P, T> ChildRenderObjectHitTestExt<P::Canvas> for ChildRenderObjectParentDataWrapper<P, T>
where
    P: Protocol,
    T: Send + Sync + 'static,
{
    fn hit_test_with(self: Arc<Self>, ctx: &mut super::HitTestContext<P::Canvas>) -> bool {
        self.render_object.clone().hit_test_with(ctx)
    }

    fn hit_test_from_adopter_with(
        self: Arc<Self>,
        ctx: &mut super::HitTestContext<P::Canvas>,
    ) -> bool {
        self.render_object.clone().hit_test_from_adopter_with(ctx)
    }
}
impl<P, T> CastInterfaceByRawPtr for ChildRenderObjectParentDataWrapper<P, T>
where
    P: Protocol,
    T: Send + Sync + 'static,
{
    fn cast_interface_raw(
        &self,
        trait_type_id: std::any::TypeId,
    ) -> Option<crate::foundation::AnyRawPointer> {
        self.render_object.cast_interface_raw(trait_type_id)
    }

    // fn cast_interface_raw_mut(
    //     &mut self,
    //     trait_type_id: std::any::TypeId,
    // ) -> Option<crate::foundation::AnyRawPointer> {
    //     self.render_object.cast_interface_raw_mut(trait_type_id)
    // }
}

impl<P, T> AnyRenderObject for ChildRenderObjectParentDataWrapper<P, T>
where
    P: Protocol,
    T: Send + Sync + 'static,
{
    fn get_parent_data_any(&self) -> Option<&dyn Any> {
        Some(&self.parent_data)
    }

    fn element_context(&self) -> &ElementContextNode {
        self.render_object.element_context()
    }

    fn render_mark(&self) -> &RenderMark {
        self.render_object.render_mark()
    }

    fn detach_render_object(&self) {
        self.render_object.detach_render_object()
    }

    fn downcast_arc_any_layer_render_object(self: Arc<Self>) -> Option<ArcAnyLayerRenderObject> {
        self.render_object
            .clone()
            .downcast_arc_any_layer_render_object()
    }

    fn as_any_arc_child(self: Arc<Self>) -> Box<dyn Any> {
        Box::new(self as ArcChildRenderObject<P>)
    }
}

impl<P: Protocol, T> AnyRenderObjectLayoutExt for ChildRenderObjectParentDataWrapper<P, T> {
    fn visit_and_layout(&self) {
        self.render_object.visit_and_layout()
    }
}
