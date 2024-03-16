use crate::{
    foundation::{Arc, Protocol},
    tree::{HitTestBehavior, HitTestResults, Render, RenderObject},
};

pub trait ChildRenderObjectHitTestExt<PP: Protocol> {
    fn hit_test(self: Arc<Self>, results: &mut HitTestResults<PP::Canvas>) -> bool;
}

impl<R> ChildRenderObjectHitTestExt<R::ParentProtocol> for RenderObject<R>
where
    R: Render,
{
    fn hit_test(
        self: Arc<Self>,
        results: &mut HitTestResults<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        let inner = self.inner.lock();
        let no_relayout_token = self.mark.assume_not_needing_layout(); // TODO: Do we really need to check this
        let layout_cache = inner
            .cache
            .layout_cache_ref(no_relayout_token)
            .expect("Hit test should not occur before layout");
        let offset = layout_cache
            .paint_offset
            .as_ref()
            .expect("Hit test should not occur before paint");
        // let a = layout_cache.paint_cache;

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
                results.push(self as _);
            }
            return hit_children || matches!(behavior, HitTestBehavior::Opaque);
        } else {
            return hit_children;
        }
    }
}
