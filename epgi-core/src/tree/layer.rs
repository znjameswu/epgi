mod context;
mod fragment;

pub use context::*;
pub use fragment::*;

use std::{any::Any, ops::Mul};

use crate::foundation::{Arc, Aweak, Canvas, Encoding, InlinableDwsizeVec, Protocol, SyncMutex};

use super::{ArcElementContextNode, AscRenderContextNode, Element, Render};

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

pub trait Layer: Send + Sync {
    type ParentCanvas: Canvas;
    type ChildCanvas: Canvas;

    fn context(&self) -> &AscLayerContextNode;

    fn composite_to(
        &self,
        encoding: &mut <Self::ParentCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::ParentCanvas>,
    );

    fn repaint(&self);
    // /// Clear all contents to prepare for repaint.
    // ///
    // /// For [LayerFragment]s, they will clear their recorded encodings.
    // /// For [LayerScope]s, they will clear their structured children (?).
    // fn clear(&self);

    // fn transform_abs(&self) -> C::Transform;

    fn as_arc_child_layer(
        self: Arc<Self>,
    ) -> Arc<dyn ChildLayer<ParentCanvas = Self::ParentCanvas>>;
    fn as_arc_parent_layer(
        self: Arc<Self>,
    ) -> Arc<dyn ParentLayer<ChildCanvas = Self::ChildCanvas>>;
    fn as_arc_any_layer(self: Arc<Self>) -> Arc<dyn AnyLayer>;
}

pub trait ChildLayer: Send + Sync {
    type ParentCanvas: Canvas;

    fn paint(&self);

    fn composite_to(
        &self,
        encoding: &mut <Self::ParentCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::ParentCanvas>,
    );
    // /// Clear all contents to prepare for repaint.
    // ///
    // /// For [LayerFragment]s, they will clear their recorded encodings.
    // /// For [LayerScope]s, they will clear their structured children (?).
    // fn clear(&self);
}

pub trait ParentLayer: Send + Sync {
    type ChildCanvas: Canvas;
}

pub trait AnyLayer: Send + Sync {
    fn composite_to(&self, encoding: &mut dyn Any, composition_config: &dyn Any);
    fn composite_self(&self) -> Arc<dyn Any + Send + Sync>;
}

impl<T> ChildLayer for T
where
    T: Layer,
{
    type ParentCanvas = T::ParentCanvas;

    fn paint(&self) {
        todo!()
    }

    fn composite_to(
        &self,
        encoding: &mut <Self::ParentCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::ParentCanvas>,
    ) {
        Layer::composite_to(self, encoding, composition_config)
    }

    // fn clear(&self) {
    //     T::clear(self)
    // }
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
    fn composite_to(&self, encoding: &mut dyn Any, composition_config: &dyn Any) {
        let encoding = encoding
            .downcast_mut::<<<Self as Layer>::ParentCanvas as Canvas>::Encoding>()
            .expect(
                "A Layer should always receives the correct type of encoding in order to composite",
            );
        let composition_config = composition_config
            .downcast_ref::<LayerCompositionConfig<T::ParentCanvas>>()
            .expect(
                "A Layer should always receives the correct type of encoding in order to composite",
            );
        Layer::composite_to(self, encoding, composition_config)
    }

    fn composite_self(&self) -> Arc<dyn Any + Send + Sync> {
        todo!()
    }
}

pub struct PaintResults<C: Canvas> {
    pub structured_children: Vec<ChildLayerOrFragment<C>>,
    pub detached_children: Vec<ComposableChildLayer<C>>,
}

impl<C> PaintResults<C>
where
    C: Canvas,
{
    pub fn composite_to(
        &self,
        encoding: &mut C::Encoding,
        composition_config: &LayerCompositionConfig<C>,
    ) {
        self.structured_children
            .iter()
            .for_each(|child| match child {
                ChildLayerOrFragment::Fragment(fragment_encoding) => C::composite(
                    encoding,
                    fragment_encoding,
                    composition_config.transform(),
                    composition_config.clip(),
                ),
                ChildLayerOrFragment::Layer(ComposableChildLayer {
                    config: child_config,
                    layer,
                }) => layer.composite_to(encoding, composition_config * child_config),
            })
    }
}

impl<C> Default for PaintResults<C>
where
    C: Canvas,
{
    fn default() -> Self {
        Self {
            structured_children: Default::default(),
            detached_children: Default::default(),
        }
    }
}

pub enum ChildLayerOrFragment<C: Canvas> {
    Fragment(C::Encoding),
    Layer(ComposableChildLayer<C>),
}

pub struct ComposableChildLayer<C: Canvas> {
    pub config: LayerCompositionConfig<C>,
    pub layer: ArcChildLayer<C>,
}

#[non_exhaustive]
pub struct LayerCompositionConfig<C: Canvas> {
    pub transform: C::Transform,
}

impl<C> LayerCompositionConfig<C>
where
    C: Canvas,
{
    pub fn transform(&self) -> Option<&C::Transform> {
        Some(&self.transform)
    }

    pub fn clip(&self) -> Option<&C::Clip> {
        None //TODO
    }
}

impl<'a, C> Mul<&'a LayerCompositionConfig<C>> for &'a LayerCompositionConfig<C>
where
    C: Canvas,
{
    type Output = &'a LayerCompositionConfig<C>;

    fn mul(self, rhs: &'a LayerCompositionConfig<C>) -> Self::Output {
        todo!()
    }
}

impl<C> Mul<LayerCompositionConfig<C>> for LayerCompositionConfig<C>
where
    C: Canvas,
{
    type Output = LayerCompositionConfig<C>;

    fn mul(self, rhs: LayerCompositionConfig<C>) -> Self::Output {
        todo!()
    }
}
