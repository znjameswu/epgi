mod async_queue;
mod context;
mod provider;
mod render_element;
mod snapshot;

pub use async_queue::*;
pub use context::*;
pub use provider::*;
pub use render_element::*;
pub use snapshot::*;

use crate::{
    foundation::{
        Arc, Asc, Aweak, BuildSuspendedError, InlinableDwsizeVec, LayerProtocol, Parallel,
        Protocol, Provide, SyncMutex, TypeKey,
    },
    nodes::{RenderSuspense, Suspense, SuspenseElement},
    scheduler::JobId,
    tree::RenderObjectUpdateResult,
};

use super::{
    ArcAnyRenderObject, ArcChildRenderObject, ArcChildWidget, ArcWidget, Layer, LayerNode,
    LayerRender, ReconcileItem, Reconciler, Render, RenderCache, RenderObject, RenderObjectInner,
};

pub type ArcAnyElementNode = Arc<dyn AnyElementNode>;
pub type AweakAnyElementNode = Aweak<dyn AnyElementNode>;
pub type ArcChildElementNode<P> = Arc<dyn ChildElementNode<ParentProtocol = P>>;

pub trait Element: Send + Sync + Clone + 'static {
    type ArcWidget: ArcWidget<Element = Self>; //<Element = Self>;
    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;

    // ~~TypeId::of is not constant function so we have to work around like this.~~ Reuse Element for different widget.
    // Boxed slice generates worse code than Vec due to https://github.com/rust-lang/rust/issues/59878
    fn get_consumed_types(widget: &Self::ArcWidget) -> &[TypeKey] {
        &[]
    }

    type Provided: Provide;
    const GET_PROVIDED_VALUE: Option<fn(&Self::ArcWidget) -> Arc<Self::Provided>> = None;

    // GAT has serious lifetime bug, do not use the following GAT implementation!
    // type Yield<T>: FutureOr<T> + Send + 'static where T: Send + 'static;

    // type ReturnResults: MaybeFuture<BoxFuture<'static, BuildResults<Self>>> + Send + 'static;

    // type Return: MaybeFallible<Self, SubtreeSuspendedError>;
    /// This method does not need a GAT-specified return tpye. Since the results would have to be converted to BoxFuture<..,BoxFuture<..>> in case of interrupts.
    ///
    ///
    // SAFETY: No async path should poll or await the stashed continuation left behind by the sync build. Awaiting outside the sync build will cause child tasks to be run outside of sync build while still being the sync variant of the task.
    // Rationale for a moving self: Allows users to destructure the self without needing to fill in a placeholder value.
    fn perform_rebuild_element(
        self,
        widget: &Self::ArcWidget,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        reconciler: impl Reconciler<Self::ChildProtocol>,
    ) -> Result<Self, (Self, BuildSuspendedError)>;

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        reconciler: impl Reconciler<Self::ChildProtocol>, // TODO: A specialized reconciler for inflate, to save passing &JobIds
    ) -> Result<Self, BuildSuspendedError>;

    // Cannot use GAT due to this rustc bug https://github.com/rust-lang/rust/issues/102211
    // Choose clone semantic due to a cloned array is needed in the end, while converting from ref to cloned is not zero-cost abstraction without GAT.
    type ChildIter: Parallel<Item = ArcChildElementNode<Self::ChildProtocol>>
        + Send
        + Sync
        + 'static;
    fn children(&self) -> Self::ChildIter;

    // A workaround for specialization.
    // This is designed in such a way that we do not need to lock the mutex to know whether a render object is present.
    // Rust const is inlined, so we can safely expect that no actual function pointers will occur in binary causing indirection.
    // const GET_RENDER_OBJECT: GetRenderObject<Self>;
    type RenderOrUnit: RenderOrUnit<Self>;
}

pub trait SingleChildElement: Element<RenderOrUnit = ()> {
    fn child(&self) -> &ArcChildElementNode<Self::ParentProtocol>;
}

pub trait RenderElement<
    R: Render<ParentProtocol = Self::ParentProtocol, ChildProtocol = Self::ChildProtocol>,
>: Element<RenderOrUnit = R>
{
    fn try_create_render_object(&self, widget: &Self::ArcWidget) -> Option<R>;
    /// Update necessary properties of render object given by the widget
    ///
    /// Called during the commit phase, when the widget is updated.
    /// Always called after [RenderElement::try_update_render_object_children].
    /// If that call failed to update children (indicating suspense), then this call will be skipped.
    fn update_render_object(
        render_object: &mut R,
        widget: &Self::ArcWidget,
    ) -> RenderObjectUpdateResult;

    /// Whether [Render::update_render_object] is a no-op and always returns None
    ///
    /// When set to true, [Render::update_render_object]'s implementation will be ignored,
    /// Certain optimizations to reduce mutex usages will be applied during the commit phase.
    /// However, if [Render::update_render_object] is actually not no-op, doing this will cause unexpected behaviors.
    ///
    /// Setting to false will always guarantee the correct behavior.
    const NOOP_UPDATE_RENDER_OBJECT: bool = false;

    /// Try to re-assemble the children of the givin render object.
    ///
    /// Called during the commit phase, when subtree structure has changed.
    fn try_update_render_object_children(&self, render_object: &mut R) -> Result<(), ()>;

    /// Whether [Render::try_update_render_object_children] is a no-op and always succeed
    ///
    /// When set to true, [Render::try_update_render_object_children]'s implementation will be ignored,
    /// Certain optimizations to reduce mutex usages will be applied during the commit phase.
    /// However, if [Render::try_update_render_object_children] is actually not no-op, doing this will cause unexpected behaviors.
    ///
    /// Setting to false will always guarantee the correct behavior.
    /// Leaf render objects may consider setting this to true.
    const NOOP_UPDATE_RENDER_OBJECT_CHILDREN: bool = false;

    const SUSPENSE_ELEMENT_FUNCTION_TABLE: Option<SuspenseElementFunctionTable<Self>> = None;
}

pub struct SuspenseElementFunctionTable<E: Element> {
    pub(crate) get_suspense_element_mut: fn(&mut E) -> &mut SuspenseElement<E::ChildProtocol>,
    pub(crate) get_suspense_widget_ref: fn(&E::ArcWidget) -> &Suspense<E::ParentProtocol>,
    pub(crate) get_suspense_render_object:
        fn(ArcRenderObjectOf<E>) -> Arc<RenderObject<RenderSuspense<E::ParentProtocol>>>,
    pub(crate) into_arc_render_object:
        fn(Arc<RenderObject<RenderSuspense<E::ParentProtocol>>>) -> ArcRenderObjectOf<E>,
}

pub struct ElementNode<E: Element> {
    pub context: ArcElementContextNode,
    pub(crate) snapshot: SyncMutex<ElementSnapshot<E>>,
}

pub(crate) struct ElementSnapshot<E: Element> {
    pub(crate) widget: E::ArcWidget,
    // pub(super) subtree_suspended: bool,
    pub(crate) inner: ElementSnapshotInner<E>,
}

impl<E> ElementNode<E>
where
    E: Element,
{
    pub(super) fn new(
        context: ArcElementContextNode,
        widget: E::ArcWidget,
        inner: ElementSnapshotInner<E>,
    ) -> Self {
        Self {
            context,
            snapshot: SyncMutex::new(ElementSnapshot {
                widget,
                // subtree_suspended: true,
                inner,
            }),
        }
    }
    pub fn widget(&self) -> E::ArcWidget {
        self.snapshot.lock().widget.clone()
    }
}

pub trait AnyElementNode:
    crate::sync::cancel_private::AnyElementNodeAsyncCancelExt
    + crate::sync::sync_build_private::AnyElementSyncTreeWalkExt
    + crate::sync::restart_private::AnyElementNodeRestartAsyncExt
    + crate::sync::reorder_work_private::AnyElementNodeReorderAsyncWorkExt
    + Send
    + Sync
    + 'static
{
    fn as_any_arc(self: Arc<Self>) -> ArcAnyElementNode;
    fn push_job(&self, job_id: JobId);
    fn render_object(&self) -> Result<ArcAnyRenderObject, &str>;
    // fn context(&self) -> &ArcElementContextNode;
}

pub trait ChildElementNode:
    crate::sync::sync_build_private::AnyElementSyncTreeWalkExt
//
// super::build::tree_walk_private::ElementTreeWalkExt
+ crate::sync::commit_private::ChildElementNodeCommitWalkExt
+ crate::sync::cancel_private::AnyElementNodeAsyncCancelExt
+ Send + Sync + 'static
{
    type ParentProtocol: Protocol;

    fn context(&self) -> &ElementContextNode;

    fn as_arc_any(self: Arc<Self>) -> ArcAnyElementNode;

    // Due to the limitation of both arbitrary_self_type and downcasting, we have to consume both Arc pointers
    // Which may not be a bad thing after all, considering how a fat &Arc would look like in memory layout.
    fn can_rebuild_with(
        self: Arc<Self>,
        widget: ArcChildWidget<Self::ParentProtocol>,
    ) -> Result<ReconcileItem<Self::ParentProtocol>, (ArcChildElementNode<Self::ParentProtocol>, ArcChildWidget<Self::ParentProtocol>)>;

    fn get_current_subtree_render_object(&self)
        -> Option<ArcChildRenderObject<Self::ParentProtocol>>;
}

impl<E> ChildElementNode for ElementNode<E>
where
    E: Element,
{
    type ParentProtocol = E::ParentProtocol;

    fn context(&self) -> &ElementContextNode {
        self.context.as_ref()
    }

    fn as_arc_any(self: Arc<Self>) -> ArcAnyElementNode {
        self
    }

    fn can_rebuild_with(
        self: Arc<Self>,
        widget: ArcChildWidget<Self::ParentProtocol>,
    ) -> Result<
        ReconcileItem<Self::ParentProtocol>,
        (
            ArcChildElementNode<Self::ParentProtocol>,
            ArcChildWidget<Self::ParentProtocol>,
        ),
    > {
        ElementNode::<E>::can_rebuild_with(self, widget)
            .map_err(|(element, widget)| (element as _, widget))
    }

    fn get_current_subtree_render_object(
        &self,
    ) -> Option<ArcChildRenderObject<Self::ParentProtocol>> {
        todo!()
        // let snapshot = self.snapshot.lock();

        // let Some(attached_object)  = &snapshot.attached_object  else {
        //     return Err(())
        // };
        // match E::GET_RENDER_OBJECT {
        //     GetRenderObjectOrChild::Child(get_child) => match &snapshot.inner {
        //         ElementSnapshotInner::Mainline(Mainline {
        //             state: Some(MainlineState::Ready { element, .. }),
        //             ..
        //         }) => get_child(element).get_current_subtree_render_object(),
        //         _ => panic!(
        //             "This method can only be called on mounted and unsuspended element nodes."
        //         ),
        //     },
        //     GetRenderObjectOrChild::RenderObject { render_object, .. } => {
        //         Ok(render_object(attached_object).clone())
        //     }
        // }
    }
}

impl<E> AnyElementNode for ElementNode<E>
where
    E: Element,
{
    fn as_any_arc(self: Arc<Self>) -> ArcAnyElementNode {
        self
    }

    fn push_job(&self, job_id: JobId) {
        todo!()
    }

    fn render_object(&self) -> Result<ArcAnyRenderObject, &str> {
        let RenderElementFunctionTable::RenderObject {
            as_arc_child_render_object, ..
        } = render_element_function_table_of::<E>()
        else {
            return Err("Render object call should not be called on an Element type that does not associate with a render object");
        };
        let snapshot = self.snapshot.lock();
        let Some(Mainline {
            state:
                Some(MainlineState::Ready {
                    render_object: Some(render_object),
                    ..
                }),
            ..
        }) = snapshot.inner.mainline()
        else {
            return Err("Render object call should only be called on element nodes that are ready and attached");
        };
        Ok(as_arc_child_render_object(render_object.clone()).as_arc_any_render_object())
    }
}

pub fn create_root_element<E, R, L>(
    widget: E::ArcWidget,
    element: E,
    render: R,
    layer: L,
    hooks: Hooks,
    constraints: <E::ParentProtocol as Protocol>::Constraints,
) -> Arc<ElementNode<E>>
where
    E: RenderElement<R>,
    R: LayerRender<L, ParentProtocol = E::ParentProtocol, ChildProtocol = E::ChildProtocol>,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
    L: Layer<
        ParentCanvas = <R::ParentProtocol as Protocol>::Canvas,
        ChildCanvas = <R::ChildProtocol as Protocol>::Canvas,
    >,
{
    let element_node = Arc::new_cyclic(move |node| {
        let element_context = Arc::new(ElementContextNode::new_root(node.clone() as _));
        let render_context = element_context.nearest_render_context.clone();
        let layer_context = render_context.nearest_repaint_boundary.clone();
        // let render = R::try_create_render_object_from_element(&element, &widget)
        //     .expect("Root render object creation should always be successfully");
        let layer_node = Asc::new(LayerNode::new(layer_context, layer));
        let render_object = Arc::new(RenderObject {
            element_context: element_context.clone(),
            context: render_context,
            layer_node,
            inner: SyncMutex::new(RenderObjectInner {
                cache: Some(RenderCache::new(constraints, false, None)),
                render,
            }),
        });
        ElementNode {
            context: element_context,
            snapshot: SyncMutex::new(ElementSnapshot {
                widget,
                inner: ElementSnapshotInner::Mainline(Mainline {
                    state: Some(MainlineState::Ready {
                        hooks,
                        element,
                        render_object: Some(render_object),
                    }),
                    async_queue: AsyncWorkQueue::new_empty(),
                }),
            }),
        }
    });
    element_node
}
