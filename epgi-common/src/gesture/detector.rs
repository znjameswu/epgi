use std::{any::TypeId, ops::DerefMut, sync::Arc};

use epgi_2d::{BoxOffset, BoxProtocol, BoxSize, Point2d};
use epgi_core::{
    foundation::{AnyRawPointer, Asc, SyncMutex},
    hit_test_interface_query_table,
    nodes::{
        ComponentElement, ComponentWidget, ProxyWidget, SingleChildRenderObject,
        SingleChildRenderObjectElement,
    },
    tree::{
        ArcChildRenderObject, ArcChildWidget, BuildContext, Element, HitTestConfig, RenderAction,
        TransformedHitTestEntry, Widget,
    },
};
use hashbrown::HashMap;

use crate::{
    AnyTransformedGestureRecognizer, ArcCallback, GestureRecognizer, GestureRecognizerTeamPolicy,
    PointerEvent, TapGestureRecognizer, TransformedGestureRecognizer,
    TransformedPointerEventHandler,
};

pub struct GestureDetector {
    pub on_tap: Option<ArcCallback>,
    pub child: ArcChildWidget<BoxProtocol>,
}

impl std::fmt::Debug for GestureDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GestureDetector")
            .field("on_tap", &self.on_tap.as_ref().map(|_| ()))
            .field("child", &self.child)
            .finish()
    }
}

impl Widget for GestureDetector {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Element = ComponentElement<BoxProtocol>;

    fn into_arc_widget(self: Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        self as _
    }
}

impl ComponentWidget<BoxProtocol> for GestureDetector {
    fn build(&self, _ctx: BuildContext) -> ArcChildWidget<BoxProtocol> {
        let mut recognizer_factories = Vec::new();
        if let Some(on_tap) = &self.on_tap {
            recognizer_factories.push(GestureRecognizerFactory::new::<TapGestureRecognizer>(
                {
                    let on_tap = on_tap.clone();
                    move || TapGestureRecognizer {
                        device_pixel_ratio: 1.0, //TODO
                        on_tap: on_tap.clone(),
                    }
                },
                {
                    let on_tap = on_tap.clone();
                    move |recognizer| {
                        recognizer.device_pixel_ratio = 1.0;
                        recognizer.on_tap = on_tap.clone();
                    }
                },
            ));
        }
        Asc::new(RawGestureDetector {
            recognizer_factories,
            child: self.child.clone(),
        })
    }
}

#[derive(Debug)]
pub struct RawGestureDetector {
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
        self
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

    fn compute_hit_test(
        render_state: &Self::RenderState,
        position: &Point2d,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &Self::LayoutMemo,
        child: &ArcChildRenderObject<BoxProtocol>,
    ) -> HitTestConfig<Self::ParentProtocol, Self::ChildProtocol> {
        HitTestConfig::new_single_in_layer(true, child.clone(), offset.clone(), None)
    }

    fn all_hit_test_interfaces() -> &'static [(
        TypeId,
        fn(*mut TransformedHitTestEntry<SingleChildRenderObject<Self>>) -> AnyRawPointer,
    )] {
        RAW_GESTURE_DETECTOR_HIT_TEST_INTERFACE_TABLE.as_slice()
    }
}

hit_test_interface_query_table!(
    RAW_GESTURE_DETECTOR_HIT_TEST_INTERFACE_TABLE,
    SingleChildRenderObject<RawGestureDetector>,
    dyn TransformedPointerEventHandler,
);

impl TransformedPointerEventHandler
    for TransformedHitTestEntry<SingleChildRenderObject<RawGestureDetector>>
{
    fn handle_pointer_event(&self, _event: &PointerEvent) {}

    fn all_gesture_recognizers(&self) -> Option<(GestureRecognizerTeamPolicy, Vec<TypeId>)> {
        let Some(render_object) = self.render_object.upgrade() else {
            return None;
        };
        Some((
            GestureRecognizerTeamPolicy::Competing,
            render_object
                .modify_render_with(|render| render.state.recognizers.keys().cloned().collect()),
        ))
    }

    fn get_gesture_recognizer(
        &self,
        type_id: TypeId,
    ) -> Option<Box<dyn AnyTransformedGestureRecognizer>> {
        let Some(render_object) = self.render_object.upgrade() else {
            return None;
        };

        render_object
            .modify_render_with(|render| render.state.recognizers.get(&type_id).cloned())
            .map(|recognizer| {
                Box::new(TransformedGestureRecognizer {
                    recognizer,
                    hit_position: self.hit_position,
                }) as _
            })
    }
}
