use crate::{
    foundation::{
        Arc, Aweak, Canvas, InlinableDwsizeVec, InlinableVec, LayerProtocol, PaintResults,
        Protocol, SyncMutex,
    },
    nodes::{RenderRepaintBoundary, RepaintBoundaryElement},
    tree::{ArcChildRenderObject, AscRenderContextNode, RenderObject},
};

use super::{ArcAnyLayer, ArcChildLayer, AweakParentLayer, ChildLayer};

/// A transparent, unretained internal layer.
pub struct ComponentLayer<P: LayerProtocol> {
    context: AscRenderContextNode,
    render_object: Aweak<RenderObject<RenderRepaintBoundary<P>>>,
    inner: SyncMutex<ComponentLayerInner<P>>,
}

struct ComponentLayerInner<P: Protocol> {
    paint_cache: Option<PaintResults<P::Canvas>>,
}

impl<P> ChildLayer for ComponentLayer<P>
where
    P: LayerProtocol,
{
    type ParentCanvas = P::Canvas;

    fn composite_to(&self, encoding: &mut <Self::ParentCanvas as Canvas>::Encoding) {
        todo!()
    }

    fn clear(&self) {
        todo!()
    }

    fn paint(&self) {
        if !self.context.needs_paint() {
            return;
        }
        let render_object = self
            .render_object
            .upgrade()
            .expect("Layer should hold reference to a living render object");

        let child = { render_object.inner.lock().render.child.clone() };

        let results = P::Canvas::paint_render_object(child);

        {
            self.inner.lock().paint_cache = Some(results)
        }
        todo!()
    }
}
