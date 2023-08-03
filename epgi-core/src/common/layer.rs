use std::{any::Any, sync::atomic::AtomicBool};

use crate::foundation::{Arc, Asc, Aweak, Canvas, InlinableDwsizeVec, Protocol, SyncMutex};

use super::{Element, Render};

pub type ArcChildLayer<C> = Arc<dyn ChildLayer<ParentCanvas = C>>;
pub type ArcParentLayer<C> = Arc<dyn ParentLayer<ChildCanvas = C>>;
pub type ArcAnyLayer = Arc<dyn AnyLayer>;
#[allow(type_alias_bounds)]
pub type ArcLayerOf<R: Render> = Arc<
    dyn Layer<
        ParentCanvas = <<R::Element as Element>::ParentProtocol as Protocol>::Canvas,
        ChildCanvas = <<R::Element as Element>::ChildProtocol as Protocol>::Canvas,
    >,
>;

pub struct LayerScope<C: Canvas> {
    detached_parent: Option<Aweak<dyn ParentLayer<ChildCanvas = C>>>,
    self_needs_recompositing: Option<Asc<AtomicBool>>,
    parent_needs_recompositing: Asc<AtomicBool>,
    // needs_recompositing: AtomicBool,
    inner: SyncMutex<LayerScopeInner<C>>,
}

struct LayerScopeInner<C: Canvas> {
    transform_abs: C::Transform,
    structured_children: InlinableDwsizeVec<Arc<dyn ChildLayer<ParentCanvas = C>>>,
    detached_children: InlinableDwsizeVec<Arc<dyn ChildLayer<ParentCanvas = C>>>,
}

/// Fragments are ephemeral. Scopes are persistent.
pub struct LayerFragment<C: Canvas> {
    inner: SyncMutex<LayerFragmentInner<C>>,
}

struct LayerFragmentInner<C: Canvas> {
    transform_abs: C::Transform,
    encoding: C::Encoding,
}

pub trait Layer {
    type ParentCanvas: Canvas;
    type ChildCanvas: Canvas;

    fn composite_to(&self, encoding: &mut <Self::ParentCanvas as Canvas>::Encoding);

    fn as_child_layer_arc(
        self: Arc<Self>,
    ) -> Arc<dyn ChildLayer<ParentCanvas = Self::ParentCanvas>>;
    fn as_parent_layer_arc(
        self: Arc<Self>,
    ) -> Arc<dyn ParentLayer<ChildCanvas = Self::ChildCanvas>>;
    fn as_any_layer_arc(self: Arc<Self>) -> Arc<dyn AnyLayer>;
}

pub trait ChildLayer {
    type ParentCanvas: Canvas;

    fn composite_to(&self, encoding: &mut <Self::ParentCanvas as Canvas>::Encoding);
}

pub trait ParentLayer {
    type ChildCanvas: Canvas;
}

pub trait AnyLayer {
    fn composite_to(&self, encoding: &mut dyn Any);
}

impl<T> ChildLayer for T
where
    T: Layer,
{
    type ParentCanvas = T::ParentCanvas;

    fn composite_to(&self, encoding: &mut <Self::ParentCanvas as Canvas>::Encoding) {
        Layer::composite_to(self, encoding)
    }
}

impl<T> ParentLayer for T
where
    T: Layer,
{
    type ChildCanvas = T::ChildCanvas;
}

impl<T> AnyLayer for T
where
    T: Layer,
{
    fn composite_to(&self, encoding: &mut dyn Any) {
        let encoding = encoding
            .downcast_mut::<<<Self as Layer>::ParentCanvas as Canvas>::Encoding>()
            .expect(
                "A Layer should always receives the correct type of encoding in order to composite",
            );
        Layer::composite_to(self, encoding)
    }
}

impl<C> ChildLayer for LayerFragment<C>
where
    C: Canvas,
{
    type ParentCanvas = C;

    fn composite_to(&self, encoding: &mut <Self::ParentCanvas as Canvas>::Encoding) {
        let inner = &mut *self.inner.lock();
        C::composite(encoding, &inner.encoding, Some(&inner.transform_abs));
    }
}

impl<C> Layer for LayerScope<C>
where
    C: Canvas,
{
    type ParentCanvas = C;

    type ChildCanvas = C;

    fn composite_to(&self, encoding: &mut <Self::ParentCanvas as Canvas>::Encoding) {
        let (structured_children, detached_children) = {
            let inner = &mut *self.inner.lock();
            (
                inner.structured_children.clone(),
                inner.detached_children.clone(),
            )
        };
        // TODO: Parallel composite.
        for child in structured_children {
            child.composite_to(encoding)
        }
        for child in detached_children {
            child.composite_to(encoding)
        }
    }

    fn as_child_layer_arc(
        self: Arc<Self>,
    ) -> Arc<dyn ChildLayer<ParentCanvas = Self::ParentCanvas>> {
        self
    }

    fn as_parent_layer_arc(
        self: Arc<Self>,
    ) -> Arc<dyn ParentLayer<ChildCanvas = Self::ChildCanvas>> {
        self
    }

    fn as_any_layer_arc(self: Arc<Self>) -> Arc<dyn AnyLayer> {
        self
    }
}
