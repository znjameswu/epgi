use std::{any::TypeId, ops::DerefMut};

use epgi_2d::{BoxProtocol, Point2d};
use epgi_core::{
    foundation::{AnyRawPointer, Asc, Canvas, PaintContext, Protocol, SyncMutex},
    interface_query_entry,
    nodes::{
        ProxyWidget, SingleChildRenderObject, SingleChildRenderObjectElement,
        SingleChildRenderObjectWidget,
    },
    tree::{
        ArcChildRenderObject, ArcChildWidget, Element, HitTestConfig, RenderAction,
        TransformedHitTestEntry, Widget,
    },
};
use hashbrown::HashMap;

use crate::{
    AnyTransformedGestureRecognizer, ArcCallback, GestureRecognizer, GestureRecognizerTeamPolicy,
    PointerEvent, TapGestureRecognizer, TransformedPointerEventHandler,
};

#[derive(Debug)]
pub struct RawGestureDetector {
    device_pixel_ratio: f32,
    recognizer_factories: Vec<GestureRecognizerFactory>,
    child: ArcChildWidget<BoxProtocol>,
}

pub struct GestureRecognizerFactory {
    type_id: TypeId,
    create:
        Box<dyn Fn() -> Asc<SyncMutex<dyn GestureRecognizer<HitPosition = Point2d>>> + Send + Sync>,
    update: Box<dyn Fn(&mut dyn GestureRecognizer<HitPosition = Point2d>) + Send + Sync>,
}

impl GestureRecognizerFactory {
    fn new<T: GestureRecognizer<HitPosition = Point2d>>(
        create: impl Fn() -> T + Send + Sync + 'static,
        update: impl Fn(&mut T) + Send + Sync + 'static,
    ) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            create: Box::new(move || Asc::new(SyncMutex::new(create()))),
            update: Box::new(move |recognizer| {
                let recognizer = recognizer
                    .as_any_mut()
                    .downcast_mut::<T>()
                    .expect("The received recognizer should be of correct type");
                update(recognizer)
            }),
        }
    }
}

impl std::fmt::Debug for GestureRecognizerFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GestureRecognizerFactory")
            .field("type_id", &self.type_id)
            .finish()
    }
}

pub struct RawGestureDetectorState {
    recognizers: HashMap<TypeId, Asc<SyncMutex<dyn GestureRecognizer<HitPosition = Point2d>>>>,
}

impl Widget for RawGestureDetector {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Element = SingleChildRenderObjectElement<RawGestureDetector>;

    fn into_arc_widget(
        self: std::sync::Arc<Self>,
    ) -> <Self::Element as epgi_core::tree::Element>::ArcWidget {
        todo!()
    }
}

impl ProxyWidget for RawGestureDetector {
    type Protocol = BoxProtocol;

    type RenderState = RawGestureDetectorState;

    fn child(&self) -> &ArcChildWidget<Self::Protocol> {
        &self.child
    }

    fn create_render_state(&self) -> Self::RenderState {
        RawGestureDetectorState {
            recognizers: self
                .recognizer_factories
                .iter()
                .map(|factory| (factory.type_id, (factory.create)()))
                .collect(),
        }
    }

    fn update_render_state(&self, render_state: &mut Self::RenderState) -> RenderAction {
        let new_recognizers = self
            .recognizer_factories
            .iter()
            .map(|factory| {
                if let Some(recognizer) = render_state.recognizers.remove(&factory.type_id) {
                    (factory.update)(recognizer.lock().deref_mut());
                    (factory.type_id, recognizer)
                } else {
                    (factory.type_id, (factory.create)())
                }
            })
            .collect();
        let old_recognizers = std::mem::replace(&mut render_state.recognizers, new_recognizers);
        old_recognizers
            .values()
            .for_each(|recognizer| recognizer.lock().on_detach());
        RenderAction::None
    }

    fn detach_render_state(render_state: &mut Self::RenderState) {
        render_state
            .recognizers
            .values()
            .for_each(|recognizer| recognizer.lock().on_detach());
    }

    type LayoutMemo = ();

    type LayerOrUnit = ();

    fn all_hit_test_interfaces() -> &'static [(
        TypeId,
        fn(*mut TransformedHitTestEntry<SingleChildRenderObject<Self>>) -> AnyRawPointer,
    )] {
        &[interface_query_entry!(dyn TransformedPointerEventHandler)]
    }
}

const GESTURE_DETECTOR_INTERFACE_QUERY_TABLE = [interface_query_entry!(dyn TransformedPointerEventHandler)];

impl TransformedPointerEventHandler
    for TransformedHitTestEntry<SingleChildRenderObject<RawGestureDetector>>
{
    fn handle_pointer_event(&self, event: &PointerEvent) {}

    fn all_gesture_recognizers(&self) -> Option<(GestureRecognizerTeamPolicy, Vec<TypeId>)> {
        let Some(render_object) = self.render_object.upgrade() else {
            return None;
        };
        Some((GestureRecognizerTeamPolicy::Competing, todo!()))
    }

    fn get_gesture_recognizer(
        &self,
        type_id: TypeId,
    ) -> Option<Box<dyn AnyTransformedGestureRecognizer>> {
        todo!()
    }
}
