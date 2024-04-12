use std::ops::Mul;

use crate::foundation::{Arc, Canvas, Key, Transform};

use super::{ArcAnyLayerRenderObject, ArcChildLayerRenderObject, ChildLayerOrFragmentRef};

pub enum StructuredChildLayerOrFragment<C: Canvas> {
    Fragment(C::Encoding),
    StructuredChild(ComposableChildLayer<C>),
}

impl<'a, C> Into<ChildLayerOrFragmentRef<'a, C>> for &'a StructuredChildLayerOrFragment<C>
where
    C: Canvas,
{
    fn into(self) -> ChildLayerOrFragmentRef<'a, C> {
        match self {
            StructuredChildLayerOrFragment::Fragment(x) => ChildLayerOrFragmentRef::Fragment(x),
            StructuredChildLayerOrFragment::StructuredChild(x) => {
                ChildLayerOrFragmentRef::StructuredChild(x)
            }
        }
    }
}

pub struct ComposableChildLayer<C: Canvas> {
    pub config: LayerCompositionConfig<C>,
    pub layer: ArcChildLayerRenderObject<C>,
}

#[derive(derivative::Derivative)]
#[derivative(Clone(bound = ""))]
pub struct ComposableUnadoptedLayer<C: Canvas> {
    pub config: LayerCompositionConfig<C>,
    pub adopter_key: Option<Arc<dyn Key>>,
    pub layer: ArcAnyLayerRenderObject,
}

#[derive(derivative::Derivative)]
#[derivative(Clone(bound = ""))]
// #[non_exhaustive]
pub struct LayerCompositionConfig<C: Canvas> {
    pub transform: C::Transform,
}

impl<C> LayerCompositionConfig<C>
where
    C: Canvas,
{
    pub fn new() -> Self {
        Self {
            transform: <C::Transform as Transform<_>>::identity(),
        }
    }

    pub fn transform(&self) -> Option<&C::Transform> {
        Some(&self.transform)
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
