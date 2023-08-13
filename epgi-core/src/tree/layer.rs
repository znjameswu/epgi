use std::any::Any;

use crate::foundation::{Arc, Aweak, Canvas, InlinableDwsizeVec, Protocol, SyncMutex};

use super::{ArcElementContextNode, Element, Render};

pub type ArcChildLayer<C> = Arc<dyn ChildLayer<ParentCanvas = C>>;
pub type ArcParentLayer<C> = Arc<dyn ParentLayer<ChildCanvas = C>>;
pub type AweakParentLayer<C> = Aweak<dyn ParentLayer<ChildCanvas = C>>;
pub type ArcAnyLayer = Arc<dyn AnyLayer>;
#[allow(type_alias_bounds)]
pub type ArcLayerOf<R: Render> = Arc<
    dyn Layer<
        ParentCanvas = <<R::Element as Element>::ParentProtocol as Protocol>::Canvas,
        ChildCanvas = <<R::Element as Element>::ChildProtocol as Protocol>::Canvas,
    >,
>;

/// A transparent, unretained internal layer.
pub struct LayerScope<C: Canvas> {
    detached_parent: Option<AweakParentLayer<C>>,
    element_context: ArcElementContextNode,
    inner: SyncMutex<LayerScopeInner<C>>,
}

struct LayerScopeInner<C: Canvas> {
    transform_abs: C::Transform,
    structured_children: InlinableDwsizeVec<ArcChildLayer<C>>,
    detached_children: InlinableDwsizeVec<ArcChildLayer<C>>,
}

/// Fragments are ephemeral. Scopes are persistent.
pub struct LayerFragment<C: Canvas> {
    inner: SyncMutex<LayerFragmentInner<C>>,
}

struct LayerFragmentInner<C: Canvas> {
    transform_abs: C::Transform,
    encoding: C::Encoding,
}

pub trait Layer: Send + Sync {
    type ParentCanvas: Canvas;
    type ChildCanvas: Canvas;

    fn composite_to(&self, encoding: &mut <Self::ParentCanvas as Canvas>::Encoding);
    /// Clear all contents to prepare for repaint.
    ///
    /// For [LayerFragment]s, they will clear their recorded encodings.
    /// For [LayerScope]s, they will clear their structured children (?).
    fn clear(&self);

    // fn transform_abs(&self) -> C::Transform;

    fn as_child_layer_arc(
        self: Arc<Self>,
    ) -> Arc<dyn ChildLayer<ParentCanvas = Self::ParentCanvas>>;
    fn as_parent_layer_arc(
        self: Arc<Self>,
    ) -> Arc<dyn ParentLayer<ChildCanvas = Self::ChildCanvas>>;
    fn as_any_layer_arc(self: Arc<Self>) -> Arc<dyn AnyLayer>;
}

pub trait ChildLayer: Send + Sync {
    type ParentCanvas: Canvas;

    fn composite_to(&self, encoding: &mut <Self::ParentCanvas as Canvas>::Encoding);
    /// Clear all contents to prepare for repaint.
    ///
    /// For [LayerFragment]s, they will clear their recorded encodings.
    /// For [LayerScope]s, they will clear their structured children (?).
    fn clear(&self);
}

pub trait ParentLayer: Send + Sync {
    type ChildCanvas: Canvas;
}

pub trait AnyLayer {
    fn composite_to(&self, encoding: &mut dyn Any);
    fn composite_self(&self) -> Arc<dyn Any + Send + Sync>;
}

impl<T> ChildLayer for T
where
    T: Layer,
{
    type ParentCanvas = T::ParentCanvas;

    fn composite_to(&self, encoding: &mut <Self::ParentCanvas as Canvas>::Encoding) {
        Layer::composite_to(self, encoding)
    }

    fn clear(&self) {
        T::clear(self)
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

    fn composite_self(&self) -> Arc<dyn Any + Send + Sync> {
        todo!()
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

    fn clear(&self) {
        C::clear(&mut self.inner.lock().encoding)
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

    fn clear(&self) {
        let mut inner = self.inner.lock();
        inner.structured_children.clear();
        // inner.detached_children.clear();
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

impl<C> LayerScope<C>
where
    C: Canvas,
{
    pub fn new_structured(
        element_context: ArcElementContextNode,
        transform_abs: C::Transform,
    ) -> Self {
        Self {
            detached_parent: None,
            element_context,
            inner: SyncMutex::new(LayerScopeInner {
                transform_abs,
                structured_children: Default::default(),
                detached_children: Default::default(),
            }),
        }
    }

    // pub fn new_detached(
    //     parent_layer: ArcParentLayer<C>,
    //     element_context: ArcElementContextNode,
    // ) -> Self {
    //     Self {
    //         detached_parent: None,
    //         element_context,
    //         inner: SyncMutex::new(LayerScopeInner {
    //             transform_abs: todo!(),
    //             structured_children: Default::default(),
    //             detached_children: Default::default(),
    //         }),
    //     }
    // }
}
