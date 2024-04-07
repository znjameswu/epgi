use crate::{
    foundation::{Arc, AsIterator, Canvas, Key, LayerProtocol, PaintContext, Protocol},
    tree::{ArcChildRenderObject, ContainerOf, PaintResults, Render},
};

pub trait HasPaintImpl<R: Render> {
    fn perform_paint(
        render: &R,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
        children: &ContainerOf<R, ArcChildRenderObject<R::ChildProtocol>>,
        paint_ctx: &mut impl PaintContext<Canvas = <R::ParentProtocol as Protocol>::Canvas>,
    );

    // fn hit_test_children(
    //     render: &R,
    //     size: &<R::ParentProtocol as Protocol>::Size,
    //     offset: &<R::ParentProtocol as Protocol>::Offset,
    //     memo: &R::LayoutMemo,
    //     children: &ContainerOf<R, ArcChildRenderObject<R::ChildProtocol>>,
    //     results: &mut HitTestResults<<R::ParentProtocol as Protocol>::Canvas>,
    // ) -> bool;

    // #[allow(unused_variables)]
    // fn hit_test_self(
    //     render: &R,
    //     position: &<<R::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
    //     size: &<R::ParentProtocol as Protocol>::Size,
    //     offset: &<R::ParentProtocol as Protocol>::Offset,
    //     memo: &R::LayoutMemo,
    // ) -> Option<HitTestBehavior> {
    //     <R::ParentProtocol as Protocol>::position_in_shape(position, offset, size)
    //         .then_some(HitTestBehavior::DeferToChild)
    // }
}

pub trait HasLayerPaintImpl<R: Render>
where
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn paint_layer(
        render: &R,
        children: &ContainerOf<R, ArcChildRenderObject<R::ChildProtocol>>,
    ) -> PaintResults<<R::ChildProtocol as Protocol>::Canvas> {
        <<R::ChildProtocol as Protocol>::Canvas as Canvas>::paint_render_objects(
            children.as_iter().cloned(),
        )
    }

    // fn transform_config(
    //     self_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
    //     child_config: &LayerCompositionConfig<<R::ChildProtocol as Protocol>::Canvas>,
    // ) -> LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>;

    fn layer_key(render: &R) -> Option<&Arc<dyn Key>> {
        None
    }
}
