use crate::{
    foundation::{Arc, Canvas, Protocol},
    tree::{
        ArcChildRenderObject, HitTestLayerTransform, HitTestNode, HitTestNodeChild,
        HitTestNodeWithLayerTransform, Render, RenderObject, TransformedHitTestTarget,
    },
};

pub trait ChildRenderObjectHitTestExt<PP: Protocol> {
    fn hit_test(
        self: Arc<Self>,
        position: &<PP::Canvas as Canvas>::HitPosition,
        transform: &PP::Transform,
    ) -> Option<HitTestNode<PP::Canvas>>;
}

impl<R> ChildRenderObjectHitTestExt<R::ParentProtocol> for RenderObject<R>
where
    R: Render,
{
    fn hit_test(
        self: Arc<Self>,
        hit_position: &<<R::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
        transform: &<R::ParentProtocol as Protocol>::Transform,
    ) -> Option<HitTestNode<<R::ParentProtocol as Protocol>::Canvas>> {
        let inner = self.inner.lock();
        let no_relayout_token = self.mark.assume_not_needing_layout(); // TODO: Do we really need to check this
        let layout_cache = inner
            .cache
            .layout_cache_ref(no_relayout_token)
            .expect("Hit test should not occur before layout");
        // let a = layout_cache.paint_cache;
        let config = inner.render.compute_hit_test(
            hit_position,
            &layout_cache.layout_results.size,
            transform,
            &layout_cache.layout_results.memo,
            &inner.children,
        );
        drop(inner);

        if config.is_empty() {
            return None;
        }

        #[inline(always)]
        fn hit_test_child<CP: Protocol>(
            child: ArcChildRenderObject<CP>,
            protocol_transform: CP::Transform,
            canvas_transform: &Option<<CP::Canvas as Canvas>::Transform>,
            hit_position: &<CP::Canvas as Canvas>::HitPosition,
        ) -> Option<HitTestNode<CP::Canvas>> {
            if let Some(canvas_transform) = &canvas_transform {
                child.hit_test(
                    &<CP::Canvas as Canvas>::transform_hit_position(canvas_transform, hit_position),
                    &protocol_transform,
                )
            } else {
                child.hit_test(hit_position, &protocol_transform)
            }
        }

        let children = match config.layer_transform {
            HitTestLayerTransform::None {
                cast_hit_position_ref,
                cast_hit_test_node_child,
            } => config
                .children
                .into_iter()
                .filter_map(|(child, protocol_transform, canvas_transform)| {
                    let child_hit_test_node = hit_test_child(
                        child,
                        protocol_transform,
                        &canvas_transform,
                        cast_hit_position_ref(hit_position),
                    );
                    child_hit_test_node.map(|child| {
                        cast_hit_test_node_child(HitTestNodeChild::InLayer(child, canvas_transform))
                    })
                })
                .collect::<Vec<_>>(),
            HitTestLayerTransform::Layer { transform } => config
                .children
                .into_iter()
                .filter_map(|(child, protocol_transform, canvas_transform)| {
                    let child_hit_test_node = hit_test_child(
                        child,
                        protocol_transform,
                        &canvas_transform,
                        &transform.transform(hit_position),
                    );
                    child_hit_test_node.map(|child| {
                        HitTestNodeChild::NewLayer(Box::new(HitTestNodeWithLayerTransform {
                            child,
                            transform: transform.clone(),
                        }))
                    })
                })
                .collect::<Vec<_>>(),
        };

        return Some(HitTestNode {
            target: Box::new(TransformedHitTestTarget {
                render_object: Arc::downgrade(&self),
                transform: transform.clone(),
            }),
            children,
        });
    }
}
