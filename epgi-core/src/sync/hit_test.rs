use crate::{
    foundation::{Arc, Canvas, ConstBool, False, Protocol, True},
    tree::{
        HitTest, HitTestBehavior, HitTestResults, OrphanComposite, Paint, Render, RenderNew,
        RenderObject, RenderObjectOld, TreeNode,
    },
};

use super::SelectLayerAdoptImpl;

pub trait ChildRenderObjectHitTestExt<PP: Protocol> {
    fn hit_test(self: Arc<Self>, results: &mut HitTestResults<PP::Canvas>) -> bool;
}

pub trait SelectHitTestImpl<OrphanComposite: ConstBool>: TreeNode {
    fn hit_test_from_parent(
        render_object: Arc<RenderObject<Self>>,
        results: &mut HitTestResults<<Self::ParentProtocol as Protocol>::Canvas>,
    ) -> bool
    where
        Self: RenderNew + Sized;

    fn hit_test_from_adopter(
        render_object: Arc<RenderObject<Self>>,
        results: &mut HitTestResults<Self::AdopterCanvas>,
    ) -> bool
    where
        Self: RenderNew + Sized;
}

impl<R> SelectHitTestImpl<False> for R
where
    R: HitTest,
    R: RenderNew<OrphanComposite = False>,
{
    fn hit_test_from_parent(
        render_object: Arc<RenderObject<Self>>,
        results: &mut HitTestResults<<Self::ParentProtocol as Protocol>::Canvas>,
    ) -> bool
    where
        Self: RenderNew + Sized,
    {
        // TODO: for detached layers, skip the hit test, since this method is called by its parent, not its adopter.
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

    fn hit_test_from_adopter(
        render_object: Arc<RenderObject<Self>>,
        results: &mut HitTestResults<
            <Self as SelectLayerAdoptImpl<<Self as RenderNew>::OrphanComposite>>::AdopterCanvas,
        >,
    ) -> bool
    where
        Self: RenderNew + Sized,
    {
        R::hit_test_from_parent(render_object, results)
    }
}

impl<R> SelectHitTestImpl<True> for R
where
    R: OrphanComposite,
    R: RenderNew<OrphanComposite = True>,
{
    fn hit_test_from_parent(
        render_object: Arc<RenderObject<Self>>,
        results: &mut HitTestResults<<Self::ParentProtocol as Protocol>::Canvas>,
    ) -> bool
    where
        Self: RenderNew + Sized,
    {
        return false;
    }

    fn hit_test_from_adopter(
        render_object: Arc<RenderObject<Self>>,
        results: &mut HitTestResults<
            <Self as SelectLayerAdoptImpl<<Self as RenderNew>::OrphanComposite>>::AdopterCanvas,
        >,
    ) -> bool
    where
        Self: RenderNew + Sized,
    {
        todo!()
    }
}

impl<R> ChildRenderObjectHitTestExt<R::ParentProtocol> for RenderObject<R>
where
    R: RenderNew + SelectHitTestImpl<R::OrphanComposite>,
{
    fn hit_test(
        self: Arc<Self>,
        results: &mut HitTestResults<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        R::hit_test_from_parent(self, results)
    }
}

impl<R> ChildRenderObjectHitTestExt<R::ParentProtocol> for RenderObjectOld<R>
where
    R: Render,
{
    fn hit_test(
        self: Arc<Self>,
        results: &mut HitTestResults<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        // TODO: for detached layers, skip the hit test, since this method is called by its parent, not its adopter.
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

        let self_has_interface = results.interface_exist_on_old::<R>();
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

pub trait ChildLayerRenderObjectHitTestExt<C: Canvas> {
    fn hit_test_layer(self: Arc<Self>, results: &mut HitTestResults<C>) -> bool;
}

impl<R> ChildLayerRenderObjectHitTestExt<R::AdopterCanvas> for RenderObject<R>
where
    R: RenderNew + SelectHitTestImpl<R::OrphanComposite>,
{
    fn hit_test_layer(self: Arc<Self>, results: &mut HitTestResults<R::AdopterCanvas>) -> bool {
        R::hit_test_from_adopter(self, results)
    }
}

impl<R> ChildLayerRenderObjectHitTestExt<<R::ParentProtocol as Protocol>::Canvas>
    for RenderObjectOld<R>
where
    R: Render,
{
    fn hit_test_layer(
        self: Arc<Self>,
        results: &mut HitTestResults<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        // For most layers, just directly hit test
        ChildRenderObjectHitTestExt::hit_test(self, results)
        // TODO: for detached layers, impl the real hit test logic
    }
}
