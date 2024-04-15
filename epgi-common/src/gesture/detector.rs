use std::{any::TypeId, sync::Arc};

use epgi_2d::{
    BoxOffset, BoxProtocol, BoxSingleChildElement, BoxSingleChildElementTemplate,
    BoxSingleChildRenderElement, BoxSize, Point2d,
};
use epgi_core::{
    foundation::{AnyRawPointer, Asc, BuildSuspendedError, InlinableDwsizeVec, Protocol, Provide},
    hit_test_interface_query_table,
    nodes::{ComponentElement, ComponentWidget},
    template::{ImplByTemplate, ProxyRender, ProxyRenderTemplate},
    tree::{
        ArcChildWidget, BuildContext, ElementBase, HitTestBehavior, RenderAction, RenderObject,
        Widget,
    },
};
use hashbrown::HashMap;

use crate::{
    ArcCallback, GestureRecognizer, GestureRecognizerTeamPolicy, PointerEvent, PointerEventHandler,
    TapGestureRecognizer,
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

    fn into_arc_widget(self: Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self as _
    }
}

impl ComponentWidget<BoxProtocol> for GestureDetector {
    fn build(&self, _ctx: BuildContext<'_>) -> ArcChildWidget<BoxProtocol> {
        let mut recognizer_factories = Vec::new();
        if let Some(on_tap) = &self.on_tap {
            recognizer_factories.push(GestureRecognizerFactory::new::<TapGestureRecognizer>(
                {
                    let on_tap = on_tap.clone();
                    move || TapGestureRecognizer {
                        on_tap: on_tap.clone(),
                    }
                },
                {
                    let on_tap = on_tap.clone();
                    move |recognizer| {
                        todo!();
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
    create: Box<dyn Fn() -> Asc<dyn GestureRecognizer> + Send + Sync>,
    update: Box<dyn Fn(&dyn GestureRecognizer) + Send + Sync>,
}

impl GestureRecognizerFactory {
    fn new<T: GestureRecognizer>(
        create: impl Fn() -> T + Send + Sync + 'static,
        update: impl Fn(&T) + Send + Sync + 'static,
    ) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            create: Box::new(move || Asc::new(create())),
            update: Box::new(move |recognizer| {
                let recognizer = recognizer
                    .as_any()
                    .downcast_ref::<T>()
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

impl Widget for RawGestureDetector {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Element = RawGestureDetectorElement;

    fn into_arc_widget(self: Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

#[derive(Clone)]
pub struct RawGestureDetectorElement;

impl ImplByTemplate for RawGestureDetectorElement {
    type Template = BoxSingleChildElementTemplate<true, false>;
}

impl BoxSingleChildElement for RawGestureDetectorElement {
    type ArcWidget = Asc<RawGestureDetector>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<BoxProtocol>, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self
    }
}

impl BoxSingleChildRenderElement for RawGestureDetectorElement {
    type Render = RenderRawGestureDetector;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderRawGestureDetector {
            recognizers: widget
                .recognizer_factories
                .iter()
                .map(|factory| (factory.type_id, (factory.create)()))
                .collect(),
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> RenderAction {
        let new_recognizers = widget
            .recognizer_factories
            .iter()
            .map(|factory| {
                if let Some(recognizer) = render.recognizers.remove(&factory.type_id) {
                    (factory.update)(recognizer.as_ref());
                    (factory.type_id, recognizer)
                } else {
                    (factory.type_id, (factory.create)())
                }
            })
            .collect();
        let old_recognizers = std::mem::replace(&mut render.recognizers, new_recognizers);
        old_recognizers
            .values()
            .for_each(|recognizer| recognizer.on_detach());
        RenderAction::None
    }
}

pub struct RenderRawGestureDetector {
    recognizers: HashMap<TypeId, Asc<dyn GestureRecognizer>>,
}

impl ImplByTemplate for RenderRawGestureDetector {
    type Template = ProxyRenderTemplate;
}

impl ProxyRender for RenderRawGestureDetector {
    type Protocol = BoxProtocol;

    fn hit_test_self(&self, position: &Point2d, size: &BoxSize, offset: &BoxOffset) -> bool {
        BoxProtocol::position_in_shape(position, offset, size)
    }

    fn hit_test_behavior(&self) -> HitTestBehavior {
        HitTestBehavior::Opaque
    }

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<Self>) -> AnyRawPointer)]
    {
        RAW_GESTURE_DETECTOR_HIT_TEST_INTERFACE_TABLE.as_slice()
    }
}

hit_test_interface_query_table!(
    RAW_GESTURE_DETECTOR_HIT_TEST_INTERFACE_TABLE,
    RenderRawGestureDetector,
    dyn PointerEventHandler,
);

impl PointerEventHandler for RenderObject<RenderRawGestureDetector> {
    fn handle_pointer_event(&self, transformed_position: Point2d, event: &PointerEvent) {}

    fn all_gesture_recognizers(
        &self,
    ) -> Option<(GestureRecognizerTeamPolicy, Vec<Asc<dyn GestureRecognizer>>)> {
        Some((
            GestureRecognizerTeamPolicy::Competing,
            self.update(|render, _| render.recognizers.values().cloned().collect()),
        ))
    }
}
