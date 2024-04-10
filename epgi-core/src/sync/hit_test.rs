use crate::{
    foundation::{Arc, Canvas, Protocol},
    tree::{
        FullRender, HitTest, HitTestBehavior, HitTestResults, Render, RenderImpl, RenderObject,
    },
};

use super::ImplAdopterLayer;

pub trait ChildRenderObjectHitTestExt<PP: Protocol> {
    fn hit_test(self: Arc<Self>, results: &mut HitTestResults<PP::Canvas>) -> bool;
}

impl<R> ChildRenderObjectHitTestExt<R::ParentProtocol> for RenderObject<R>
where
    R: FullRender,
{
    fn hit_test(
        self: Arc<Self>,
        results: &mut HitTestResults<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        <R as Render>::Impl::hit_test(self, results)
    }
}

pub trait ChildLayerRenderObjectHitTestExt<C: Canvas> {
    // fn hit_test_layer(self: Arc<Self>, results: &mut HitTestResults<C>) -> bool;
}

pub trait ImplHitTest<R: Render> {
    fn hit_test(
        render_object: Arc<RenderObject<R>>,
        results: &mut HitTestResults<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> bool;
}

impl<R: Render, const DRY_LAYOUT: bool, const LAYER_PAINT: bool, const CACHED_COMPOSITE: bool>
    ImplHitTest<R> for RenderImpl<DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, false>
where
    R::Impl: ImplAdopterLayer<R, AdopterCanvas = <R::ParentProtocol as Protocol>::Canvas>,
    R: HitTest,
{
    fn hit_test(
        render_object: Arc<RenderObject<R>>,
        results: &mut HitTestResults<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        let inner = render_object.inner.lock();
        let no_relayout_token = render_object.mark.assume_not_needing_layout(); // TODO: Do we really need to check this
        let layout_cache = inner
            .cache
            .layout_cache_ref(no_relayout_token)
            .expect("Hit test should not occur before layout");
        let offset = layout_cache
            .paint_offset
            .as_ref()
            .expect("Hit test should not occur before paint");

        let hit_within_shape = inner.render.hit_test_self(
            results.curr_position(),
            &layout_cache.layout_results.size,
            offset,
            &layout_cache.layout_results.memo,
        );

        let Some(behavior) = hit_within_shape else {
            return false;
        };

        let hit_children = inner.render.hit_test_children(
            &layout_cache.layout_results.size,
            offset,
            &layout_cache.layout_results.memo,
            &inner.children,
            results,
        );
        drop(inner);

        let self_has_interface = results.interface_exist_on::<R>();
        if self_has_interface {
            if hit_children
                || matches!(
                    behavior,
                    HitTestBehavior::Opaque | HitTestBehavior::Transparent
                )
            {
                results.push(render_object as _);
            }
            return hit_children || matches!(behavior, HitTestBehavior::Opaque);
        } else {
            return hit_children;
        }
    }
}

impl<R: Render, const DRY_LAYOUT: bool, const LAYER_PAINT: bool, const CACHED_COMPOSITE: bool>
    ImplHitTest<R> for RenderImpl<DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, true>
where
    R: HitTest,
{
    fn hit_test(
        render_object: Arc<RenderObject<R>>,
        results: &mut HitTestResults<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        false
    }
}
