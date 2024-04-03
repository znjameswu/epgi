use crate::{
    foundation::{Arc, Canvas, LayerProtocol, Protocol},
    tree::{
        HitTest, HitTestBehavior, HitTestResults, OrphanLayer, Paint, Render, RenderImpl,
        RenderObject, SelectCachedComposite, SelectLayerPaint, SelectOrphanLayer, TreeNode,
    },
};

pub trait ChildRenderObjectHitTestExt<PP: Protocol> {
    fn hit_test(self: Arc<Self>, results: &mut HitTestResults<PP::Canvas>) -> bool;
}

impl<
        R,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ChildRenderObjectHitTestExt<R::ParentProtocol>
    for RenderObject<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Render<RenderObject = Self>
        + SelectLayerPaint<LAYER_PAINT>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: SelectHitTestImpl<ORPHAN_LAYER>,
{
    fn hit_test(
        self: Arc<Self>,
        results: &mut HitTestResults<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        R::hit_test(self, results)
    }
}

pub trait SelectHitTestImpl<const ORPHAN_LAYER: bool>:
    TreeNode + SelectOrphanLayer<ORPHAN_LAYER>
{
    fn hit_test(
        render_object: Arc<Self::RenderObject>,
        results: &mut HitTestResults<<Self::ParentProtocol as Protocol>::Canvas>,
    ) -> bool
    where
        Self: Render + Sized;
}

impl<R, const DRY_LAYOUT: bool, const LAYER_PAINT: bool, const CACHED_COMPOSITE: bool>
    SelectHitTestImpl<false> for R
where
    R: Render<RenderObject = RenderObject<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, false>>
        + SelectLayerPaint<LAYER_PAINT>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: HitTest,
{
    fn hit_test(
        render_object: Arc<<Self as Render>::RenderObject>,
        results: &mut HitTestResults<<Self::ParentProtocol as Protocol>::Canvas>,
    ) -> bool
    where
        Self: Render + Sized,
    {
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

impl<R, const DRY_LAYOUT: bool, const LAYER_PAINT: bool, const CACHED_COMPOSITE: bool>
    SelectHitTestImpl<true> for R
where
    R: Render<RenderObject = RenderObject<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, true>>
        + SelectLayerPaint<LAYER_PAINT>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: OrphanLayer,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn hit_test(
        render_object: Arc<<Self as Render>::RenderObject>,
        results: &mut HitTestResults<<Self::ParentProtocol as Protocol>::Canvas>,
    ) -> bool
    where
        Self: Render + Sized,
    {
        false
    }
}

// impl<R, const DRY_LAYOUT: bool, const LAYER_PAINT: bool, const CACHED_COMPOSITE: bool>
//     SelectHitTestImpl<false> for R
// where
//     R: RenderNew<RenderObject = RenderObject<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, false>>
//         + SelectLayerPaint<LAYER_PAINT>
//         + SelectCachedComposite<CACHED_COMPOSITE>,
//     R: HitTest<false>
//         + SelectOrphanLayer<false, AdopterCanvas = <Self::ParentProtocol as Protocol>::Canvas>,
// {
//     fn hit_test_from_parent(
//         render_object: Arc<<Self as RenderNew>::RenderObject>,
//         results: &mut HitTestResults<<Self::ParentProtocol as Protocol>::Canvas>,
//     ) -> bool
//     where
//         Self: RenderNew + Sized,
//     {
//         // TODO: for detached layers, skip the hit test, since this method is called by its parent, not its adopter.
//         let inner = render_object.inner.lock();
//         let no_relayout_token = render_object.mark.assume_not_needing_layout(); // TODO: Do we really need to check this
//         let layout_cache = inner
//             .cache
//             .layout_cache_ref(no_relayout_token)
//             .expect("Hit test should not occur before layout");
//         let offset = layout_cache
//             .paint_offset
//             .as_ref()
//             .expect("Hit test should not occur before paint");

//         let hit_within_shape = inner.render.hit_test_self(
//             results.curr_position(),
//             &layout_cache.layout_results.size,
//             offset,
//             &layout_cache.layout_results.memo,
//         );

//         let Some(behavior) = hit_within_shape else {
//             return false;
//         };

//         let hit_children = inner.render.hit_test_children(
//             &layout_cache.layout_results.size,
//             offset,
//             &layout_cache.layout_results.memo,
//             &inner.children,
//             results,
//         );
//         drop(inner);

//         let self_has_interface = results.interface_exist_on::<R>();
//         if self_has_interface {
//             if hit_children
//                 || matches!(
//                     behavior,
//                     HitTestBehavior::Opaque | HitTestBehavior::Transparent
//                 )
//             {
//                 results.push(render_object as _);
//             }
//             return hit_children || matches!(behavior, HitTestBehavior::Opaque);
//         } else {
//             return hit_children;
//         }
//     }

//     fn hit_test_from_adopter(
//         render_object: Arc<<Self as RenderNew>::RenderObject>,
//         results: &mut HitTestResults<<Self as SelectOrphanLayer<false>>::AdopterCanvas>,
//     ) -> bool
//     where
//         Self: RenderNew + Sized,
//     {
//         R::hit_test_from_parent(render_object, results)
//     }
// }

// impl<R> SelectHitTestImpl<true> for R
// where
//     R: HitTest<true>,
// {
//     fn hit_test_from_parent(
//         render_object: Arc<<Self as RenderNew>::RenderObject>,
//         results: &mut HitTestResults<<Self::ParentProtocol as Protocol>::Canvas>,
//     ) -> bool
//     where
//         Self: RenderNew + Sized,
//     {
//         return false;
//     }

//     fn hit_test_from_adopter(
//         render_object: Arc<<Self as RenderNew>::RenderObject>,
//         results: &mut HitTestResults<<Self as SelectOrphanLayer<true>>::AdopterCanvas>,
//     ) -> bool
//     where
//         Self: RenderNew + Sized,
//     {
//         todo!()
//     }
// }

// impl<R> ChildRenderObjectHitTestExt<R::ParentProtocol> for RenderObject<R>
// where
//     R: RenderNew + SelectHitTestImpl<R::OrphanLayer>,
// {
//     fn hit_test(
//         self: Arc<Self>,
//         results: &mut HitTestResults<<R::ParentProtocol as Protocol>::Canvas>,
//     ) -> bool {
//         R::hit_test_from_parent(self, results)
//     }
// }

pub trait ChildLayerRenderObjectHitTestExt<C: Canvas> {
    // fn hit_test_layer(self: Arc<Self>, results: &mut HitTestResults<C>) -> bool;
}

// impl<R> ChildLayerRenderObjectHitTestExt<R::AdopterCanvas> for RenderObject<R>
// where
//     R: RenderNew + SelectHitTestImpl<R::OrphanLayer>,
// {
//     fn hit_test_layer(self: Arc<Self>, results: &mut HitTestResults<R::AdopterCanvas>) -> bool {
//         R::hit_test_from_adopter(self, results)
//     }
// }

impl<
        R,
        const DRY_LAYOUT: bool,
        // const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ChildLayerRenderObjectHitTestExt<<R as SelectOrphanLayer<ORPHAN_LAYER>>::AdopterCanvas>
    for RenderObject<R, DRY_LAYOUT, true, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Render<RenderObject = Self>
        + SelectLayerPaint<true>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: SelectHitTestImpl<ORPHAN_LAYER>,
{
    // fn hit_test_layer(
    //     self: Arc<Self>,
    //     results: &mut HitTestResults<<R as SelectOrphanLayer<ORPHAN_LAYER>>::AdopterCanvas>,
    // ) -> bool {
    //     R::hit_test_from_adopter(self, results)
    // }
}

pub trait ImplHitTest<R: Render> {
    fn hit_test(
        render_object: Arc<R::RenderObject>,
        results: &mut HitTestResults<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> bool;
}

impl<
        R: Render,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ImplHitTest<R> for RenderImpl<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: HitTest,
{
    fn hit_test(
        render_object: Arc<<R as Render>::RenderObject>,
        results: &mut HitTestResults<<<R>::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        // let inner = render_object.inner.lock();
        // let no_relayout_token = render_object.mark.assume_not_needing_layout(); // TODO: Do we really need to check this
        // let layout_cache = inner
        //     .cache
        //     .layout_cache_ref(no_relayout_token)
        //     .expect("Hit test should not occur before layout");
        // let offset = layout_cache
        //     .paint_offset
        //     .as_ref()
        //     .expect("Hit test should not occur before paint");

        // let hit_within_shape = inner.render.hit_test_self(
        //     results.curr_position(),
        //     &layout_cache.layout_results.size,
        //     offset,
        //     &layout_cache.layout_results.memo,
        // );

        // let Some(behavior) = hit_within_shape else {
        //     return false;
        // };

        // let hit_children = inner.render.hit_test_children(
        //     &layout_cache.layout_results.size,
        //     offset,
        //     &layout_cache.layout_results.memo,
        //     &inner.children,
        //     results,
        // );
        // drop(inner);

        // let self_has_interface = results.interface_exist_on::<R>();
        // if self_has_interface {
        //     if hit_children
        //         || matches!(
        //             behavior,
        //             HitTestBehavior::Opaque | HitTestBehavior::Transparent
        //         )
        //     {
        //         results.push(render_object as _);
        //     }
        //     return hit_children || matches!(behavior, HitTestBehavior::Opaque);
        // } else {
        //     return hit_children;
        // }
        todo!()
    }
}
