use std::any::Any;

use crate::{
    foundation::{
        default_query_interface_arc, default_query_interface_box, default_query_interface_ref, Arc,
        AsAny, Aweak, Canvas, CastInterfaceByRawPtr, ContainerOf, HktContainer, LayerProtocol,
        Protocol, SyncMutex,
    },
    sync::ImplComposite,
    tree::{ElementContextNode, LayerCache, LayerMark},
};

use super::{
    ArcElementContextNode, CachedComposite, CompositionCache, FullRender, ImplMaybeLayer,
    LayerPaint, Render, RenderAction, RenderBase, RenderCache, RenderImpl, RenderMark,
};

pub type ArcChildRenderObject<P> = Arc<dyn ChildRenderObject<P>>;
pub type ArcAnyRenderObject = Arc<dyn AnyRenderObject>;
pub type AweakAnyRenderObject = Aweak<dyn AnyRenderObject>;
pub type AweakParentRenderObject<P> = Arc<dyn ParentRenderObject<P>>;
pub type ArcChildRenderObjectWithCanvas<C> = Arc<dyn ChildRenderObjectWithCanvas<C>>;

pub type ArcChildLayerRenderObject<C> = Arc<dyn ChildLayerRenderObject<C>>;
pub type AweakChildLayerRenderObject<C> = Aweak<dyn ChildLayerRenderObject<C>>;
pub type ArcAnyLayerRenderObject = Arc<dyn AnyLayerRenderObject>;
pub type AweakAnyLayerRenderObject = Aweak<dyn AnyLayerRenderObject>;

pub struct RenderObject<R>
where
    R: Render,
{
    pub(crate) element_context: ArcElementContextNode,
    pub(crate) mark: RenderMark,
    pub(crate) layer_mark: <R::Impl as ImplRenderObject<R>>::LayerMark,
    pub(crate) inner: SyncMutex<RenderObjectInner<R, <R::Impl as ImplRenderObject<R>>::LayerCache>>,
}

impl<R: Render> RenderObject<R> {
    pub(crate) fn new(
        render: R,
        children: ContainerOf<R::ChildContainer, ArcChildRenderObject<R::ChildProtocol>>,
        context: ArcElementContextNode,
    ) -> Self {
        Self {
            element_context: context,
            mark: RenderMark::new(),
            layer_mark: Default::default(),
            inner: SyncMutex::new(RenderObjectInner {
                cache: RenderCache::new(),
                render,
                children,
            }),
        }
    }

    pub fn update<T>(
        &self,
        op: impl FnOnce(
            &mut R,
            &mut ContainerOf<R::ChildContainer, ArcChildRenderObject<R::ChildProtocol>>,
        ) -> T,
    ) -> T {
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        op(&mut inner_reborrow.render, &mut inner_reborrow.children)
    }
}

pub(crate) struct RenderObjectInner<R, C>
where
    R: RenderBase,
{
    // parent: Option<AweakParentRenderObject<R::SelfProtocol>>,
    // boundaries: Option<RenderObjectBoundaries>,
    pub(crate) cache: RenderCache<R, C>,
    pub(crate) render: R,
    pub(crate) children:
        <R::ChildContainer as HktContainer>::Container<ArcChildRenderObject<R::ChildProtocol>>,
}

pub trait ImplRenderObject<R: RenderBase> {
    type LayerMark: Default + Send + Sync;
    type LayerCache: Send + Sync;
}

impl<
        R: RenderBase,
        const DRY_LAYOUT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ImplRenderObject<R> for RenderImpl<DRY_LAYOUT, false, CACHED_COMPOSITE, ORPHAN_LAYER>
{
    type LayerMark = ();
    type LayerCache = ();
}

impl<R: RenderBase, const DRY_LAYOUT: bool, const ORPHAN_LAYER: bool> ImplRenderObject<R>
    for RenderImpl<DRY_LAYOUT, true, false, ORPHAN_LAYER>
{
    type LayerMark = LayerMark;
    type LayerCache = LayerCache<<R::ChildProtocol as Protocol>::Canvas, ()>;
}

impl<R: RenderBase, const DRY_LAYOUT: bool, const ORPHAN_LAYER: bool, CM> ImplRenderObject<R>
    for RenderImpl<DRY_LAYOUT, true, true, ORPHAN_LAYER>
where
    R: CachedComposite<CompositionMemo = CM>,
    CM: Clone + Send + Sync,
{
    type LayerMark = LayerMark;
    type LayerCache = LayerCache<
        <R::ChildProtocol as Protocol>::Canvas,
        CompositionCache<<R::ChildProtocol as Protocol>::Canvas, CM>,
    >;
}

pub trait ChildRenderObject<PP: Protocol>:
    AnyRenderObject
    + ChildRenderObjectWithCanvas<PP::Canvas>
    + crate::sync::ChildRenderObjectLayoutExt<PP>
    + crate::sync::ChildRenderObjectPaintExt<PP>
    + Send
    + Sync
{
    fn as_arc_any_render_object(self: Arc<Self>) -> ArcAnyRenderObject;
}

impl<R: FullRender> ChildRenderObject<R::ParentProtocol> for RenderObject<R> {
    fn as_arc_any_render_object(self: Arc<Self>) -> ArcAnyRenderObject {
        self
    }
}

pub trait AnyRenderObject: crate::sync::AnyRenderObjectLayoutExt + AsAny + Send + Sync {
    fn element_context(&self) -> &ElementContextNode;
    fn detach_render_object(&self);
    fn downcast_arc_any_layer_render_object(self: Arc<Self>) -> Option<ArcAnyLayerRenderObject>;

    fn mark_render_action(
        self: &Arc<Self>,
        propagated_render_action: RenderAction,
        descendant_has_action: RenderAction,
    ) -> RenderAction
    where
        Self: Sized;

    fn try_as_aweak_any_layer_render_object(
        render_object: &Arc<Self>,
    ) -> Option<AweakAnyLayerRenderObject>
    where
        Self: Sized;

    fn as_any_arc_child(self: Arc<Self>) -> Box<dyn Any>;
}

impl<R: FullRender> AnyRenderObject for RenderObject<R> {
    fn element_context(&self) -> &ElementContextNode {
        todo!()
    }

    fn detach_render_object(&self) {
        self.mark.set_is_detached();
        todo!()
    }

    fn downcast_arc_any_layer_render_object(self: Arc<Self>) -> Option<ArcAnyLayerRenderObject> {
        if <R as FullRender>::Impl::IS_LAYER {
            Some(<R as FullRender>::Impl::into_arc_any_layer_render_object(
                self,
            ))
        } else {
            None
        }
    }

    fn mark_render_action(
        self: &Arc<Self>,
        mut self_render_action: RenderAction,
        descendant_has_action: RenderAction,
    ) -> RenderAction {
        if self_render_action == RenderAction::Relayout {
            self.mark.set_self_needs_layout();
            if !self.mark.parent_use_size() {
                self_render_action = RenderAction::Repaint;
            }
        }
        if descendant_has_action == RenderAction::Relayout {
            self.mark.set_descendant_has_layout();
        }
        <R as FullRender>::Impl::maybe_layer_mark_render_action(
            self,
            self_render_action,
            descendant_has_action,
        )
    }

    fn try_as_aweak_any_layer_render_object(
        render_object: &Arc<Self>,
    ) -> Option<AweakAnyLayerRenderObject>
    where
        Self: Sized,
    {
        if <R as FullRender>::Impl::IS_LAYER {
            Some(<R as FullRender>::Impl::into_aweak_any_layer_render_object(
                Arc::downgrade(render_object),
            ))
        } else {
            None
        }
    }

    fn as_any_arc_child(self: Arc<Self>) -> Box<dyn Any> {
        Box::new(self as ArcChildRenderObject<R::ParentProtocol>)
    }
}

pub trait ArcAnyRenderObjectExt {
    fn downcast_arc_child<P: Protocol>(self) -> Option<ArcChildRenderObject<P>>;
}

impl ArcAnyRenderObjectExt for ArcAnyRenderObject {
    fn downcast_arc_child<P: Protocol>(self) -> Option<ArcChildRenderObject<P>> {
        self.as_any_arc_child()
            .downcast::<Arc<dyn ChildRenderObject<P>>>()
            .ok()
            .map(|x| *x)
    }
}

pub trait ParentRenderObject<CP: Protocol>: Send + Sync + 'static {}

pub trait ChildRenderObjectWithCanvas<C: Canvas>:
    CastInterfaceByRawPtr + crate::sync::ChildRenderObjectHitTestExt<C> + Send + Sync + 'static
{
}

impl<R: FullRender> ChildRenderObjectWithCanvas<<R::ParentProtocol as Protocol>::Canvas>
    for RenderObject<R>
{
}

impl<C: Canvas> dyn ChildRenderObjectWithCanvas<C> {
    pub fn query_interface_ref<T: ?Sized + 'static>(&self) -> Option<&T> {
        default_query_interface_ref(self)
    }

    pub fn query_interface_box<T: ?Sized + 'static>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
        default_query_interface_box(self)
    }

    pub fn query_interface_arc<T: ?Sized + 'static>(self: Arc<Self>) -> Result<Arc<T>, Arc<Self>> {
        default_query_interface_arc(self)
    }
}

pub trait AnyLayerRenderObject:
    AnyRenderObject
    + crate::sync::AnyLayerRenderObjectPaintExt
    + crate::sync::AnyLayerRenderObjectCompositeExt
    + Send
    + Sync
{
    fn mark(&self) -> &LayerMark;

    fn as_any_arc_child_layer(self: Arc<Self>) -> Box<dyn Any>;

    fn get_composited_cache_box(&self) -> Option<Box<dyn Any + Send + Sync>>;
}

impl<R: FullRender> AnyLayerRenderObject for RenderObject<R>
where
    <R as FullRender>::Impl: ImplComposite<R>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn mark(&self) -> &LayerMark {
        &self.layer_mark
    }

    fn as_any_arc_child_layer(self: Arc<Self>) -> Box<dyn std::any::Any> {
        Box::new(self as ArcChildLayerRenderObject<<R::ParentProtocol as Protocol>::Canvas>)
    }

    fn get_composited_cache_box(&self) -> Option<Box<dyn std::any::Any + Send + Sync>> {
        todo!()
    }
}

pub trait ArcAnyLayerRenderObjectExt {
    fn downcast_arc_child_layer<C: Canvas>(self) -> Option<ArcChildLayerRenderObject<C>>;

    fn downcast_arc_child<P: Protocol>(self) -> Option<ArcChildRenderObject<P>>;
    // fn downcast_arc_parent_layer<C: Canvas>(self)
    //     -> Result<ArcParentLayerNode<C>, ArcAnyLayerNode>;
}

impl ArcAnyLayerRenderObjectExt for ArcAnyLayerRenderObject {
    fn downcast_arc_child_layer<C: Canvas>(self) -> Option<ArcChildLayerRenderObject<C>> {
        self.as_any_arc_child_layer()
            .downcast::<Arc<dyn ChildLayerRenderObject<C>>>()
            .ok()
            .map(|x| *x)
    }

    fn downcast_arc_child<P: Protocol>(self) -> Option<ArcChildRenderObject<P>> {
        self.as_any_arc_child()
            .downcast::<Arc<dyn ChildRenderObject<P>>>()
            .ok()
            .map(|x| *x)
    }
    // fn downcast_arc_parent_layer<C: Canvas>(
    //     self,
    // ) -> Result<ArcParentLayerNode<C>, ArcAnyLayerNode> {
    //     todo!()
    // }
}

pub trait ChildLayerRenderObject<PC: Canvas>:
    ChildRenderObjectWithCanvas<PC> + crate::sync::ChildLayerRenderObjectCompositeExt<PC> + Send + Sync
{
    fn as_arc_any_layer_render_object(self: Arc<Self>) -> ArcAnyLayerRenderObject;
}

impl<R: FullRender> ChildLayerRenderObject<<R::ParentProtocol as Protocol>::Canvas>
    for RenderObject<R>
where
    <R as FullRender>::Impl: ImplComposite<R>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn as_arc_any_layer_render_object(self: Arc<Self>) -> ArcAnyLayerRenderObject {
        self
    }
}
