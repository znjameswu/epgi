mod async_queue;
mod context;
mod node;
mod provider;
mod render_or_unit;
mod snapshot;

pub use async_queue::*;
pub use context::*;
pub use node::*;
pub use provider::*;
pub use render_or_unit::*;
pub use snapshot::*;

use crate::{
    foundation::{
        Arc, Asc, Aweak, BuildSuspendedError, HktContainer, InlinableDwsizeVec, LayerProtocol,
        Protocol, Provide, SyncMutex, TypeKey,
    },
    nodes::{RenderSuspense, Suspense, SuspenseElement},
    scheduler::JobId,
    tree::{LayerNode, RenderAction, RenderCache, RenderMark, RenderObjectInner},
};

use super::{
    ArcAnyRenderObject, ArcChildRenderObject, ArcChildWidget, ArcWidget, BuildContext,
    ChildElementWidgetPair, ElementWidgetPair, Layer, LayerRender, Render, RenderObject,
};

pub type ArcAnyElementNode = Arc<dyn AnyElementNode>;
pub type AweakAnyElementNode = Aweak<dyn AnyElementNode>;
pub type ArcChildElementNode<P> = Arc<dyn ChildElementNode<P>>;

/// We assume the render has the same child container with the element,
/// ignoring the fact that Suspense may have different child containers.
///
/// However, we designate Suspense to be the only component to have different containers,
/// which will be handled by Suspense's specialized function pointers.
pub type ChildRenderObjectsUpdateCallback<E: Element> = Box<
    dyn FnOnce(
        ContainerOf<E, ArcChildRenderObject<E::ChildProtocol>>,
    ) -> ContainerOf<E, RenderObjectSlots<E::ChildProtocol>>,
>;

pub enum ElementReconcileItem<P: Protocol> {
    Keep(ArcChildElementNode<P>),
    Update(Box<dyn ChildElementWidgetPair<P>>),
    Inflate(ArcChildWidget<P>),
}

impl<CP> ElementReconcileItem<CP>
where
    CP: Protocol,
{
    pub fn new_update<E: Element<ParentProtocol = CP>>(
        element: Arc<ElementNode<E>>,
        widget: E::ArcWidget,
    ) -> Self {
        Self::Update(Box::new(ElementWidgetPair { element, widget }))
    }

    pub fn new_inflate(widget: ArcChildWidget<CP>) -> Self {
        Self::Inflate(widget)
    }

    pub fn new_keep(element: ArcChildElementNode<CP>) -> Self {
        Self::Keep(element)
    }
}

pub enum RenderObjectSlots<P: Protocol> {
    Inflate,
    Reuse(ArcChildRenderObject<P>),
}

pub type ContainerOf<E: Element, T> = <E::ChildContainer as HktContainer>::Container<T>;

pub trait Element: Send + Sync + Clone + 'static {
    type ArcWidget: ArcWidget<Element = Self>; //<Element = Self>;
    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;

    type ChildContainer: HktContainer;

    // ~~TypeId::of is not constant function so we have to work around like this.~~ Reuse Element for different widget.
    // Boxed slice generates worse code than Vec due to https://github.com/rust-lang/rust/issues/59878
    fn get_consumed_types(widget: &Self::ArcWidget) -> &[TypeKey] {
        &[]
    }

    type Provided: Provide;
    const GET_PROVIDED_VALUE: Option<fn(&Self::ArcWidget) -> Arc<Self::Provided>> = None;

    // SAFETY: No async path should poll or await the stashed continuation left behind by the sync build. Awaiting outside the sync build will cause child tasks to be run outside of sync build while still being the sync variant of the task.
    // Rationale for a moving self: Allows users to destructure the self without needing to fill in a placeholder value.
    /// If a hook suspended, then the untouched Self should be returned along with the suspended error
    /// If nothing suspended, then the new Self should be returned.
    fn perform_rebuild_element(
        &mut self,
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        children: ContainerOf<Self, ArcChildElementNode<Self::ChildProtocol>>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::ChildProtocol>>,
    ) -> Result<
        (
            ContainerOf<Self, ElementReconcileItem<Self::ChildProtocol>>,
            Option<ChildRenderObjectsUpdateCallback<Self>>,
        ),
        (
            ContainerOf<Self, ArcChildElementNode<Self::ChildProtocol>>,
            BuildSuspendedError,
        ),
    >;

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(Self, ContainerOf<Self, ArcChildWidget<Self::ChildProtocol>>), BuildSuspendedError>;

    // A workaround for specialization.
    // This is designed in such a way that we do not need to lock the mutex to know whether a render object is present.
    // Rust const is inlined, so we can safely expect that no actual function pointers will occur in binary causing indirection.
    // const GET_RENDER_OBJECT: GetRenderObject<Self>;
    type RenderOrUnit: RenderOrUnit<Self>;
}

pub trait RenderElement: Element<RenderOrUnit = <Self as RenderElement>::Render> {
    type Render: Render<ParentProtocol = Self::ParentProtocol, ChildProtocol = Self::ChildProtocol>;
    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render;
    /// Update necessary properties of render object given by the widget
    ///
    /// Called during the commit phase, when the widget is updated.
    /// Always called after [RenderElement::try_update_render_object_children].
    /// If that call failed to update children (indicating suspense), then this call will be skipped.
    fn update_render(render_object: &mut Self::Render, widget: &Self::ArcWidget) -> RenderAction;

    /// Whether [Render::update_render_object] is a no-op and always returns None
    ///
    /// When set to true, [Render::update_render_object]'s implementation will be ignored,
    /// Certain optimizations to reduce mutex usages will be applied during the commit phase.
    /// However, if [Render::update_render_object] is actually not no-op, doing this will cause unexpected behaviors.
    ///
    /// Setting to false will always guarantee the correct behavior.
    const NOOP_UPDATE_RENDER_OBJECT: bool = false;

    fn element_render_children_mapping<T: Send + Sync>(
        &self,
        element_children: <Self::ChildContainer as HktContainer>::Container<T>,
    ) -> <<Self::Render as Render>::ChildContainer as HktContainer>::Container<T>;

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
    + crate::sync::sync_build_private::AnyElementSyncReconcileExt
    + crate::sync::restart_private::AnyElementNodeRestartAsyncExt
    + crate::sync::reorder_work_private::AnyElementNodeReorderAsyncWorkExt
    + crate::sync::unmount::AnyElementNodeUnmountExt
    + Send
    + Sync
    + 'static
{
    fn as_any_arc(self: Arc<Self>) -> ArcAnyElementNode;
    fn push_job(&self, job_id: JobId);
    fn render_object(&self) -> Result<ArcAnyRenderObject, &str>;
    // fn context(&self) -> &ArcElementContextNode;
}

pub trait ChildElementNode<PP: Protocol>:
    AnyElementNode
    + crate::sync::sync_build_private::ChildElementSyncReconcileExt<PP>
    + Send
    + Sync
    + 'static
{
    fn context(&self) -> &ElementContextNode;

    fn as_arc_any(self: Arc<Self>) -> ArcAnyElementNode;

    // Due to the limitation of both arbitrary_self_type and downcasting, we have to consume both Arc pointers
    // Which may not be a bad thing after all, considering how a fat &Arc would look like in memory layout.
    fn can_rebuild_with(
        self: Arc<Self>,
        widget: ArcChildWidget<PP>,
    ) -> Result<ElementReconcileItem<PP>, (ArcChildElementNode<PP>, ArcChildWidget<PP>)>;

    fn get_current_subtree_render_object(&self) -> Option<ArcChildRenderObject<PP>>;
}

impl<E> ChildElementNode<E::ParentProtocol> for ElementNode<E>
where
    E: Element,
{
    fn context(&self) -> &ElementContextNode {
        self.context.as_ref()
    }

    fn as_arc_any(self: Arc<Self>) -> ArcAnyElementNode {
        self
    }

    fn can_rebuild_with(
        self: Arc<Self>,
        widget: ArcChildWidget<E::ParentProtocol>,
    ) -> Result<
        ElementReconcileItem<E::ParentProtocol>,
        (
            ArcChildElementNode<E::ParentProtocol>,
            ArcChildWidget<E::ParentProtocol>,
        ),
    > {
        ElementNode::<E>::can_rebuild_with(self, widget)
            .map_err(|(element, widget)| (element as _, widget))
    }

    fn get_current_subtree_render_object(&self) -> Option<ArcChildRenderObject<E::ParentProtocol>> {
        let snapshot = self.snapshot.lock();

        let MainlineState::Ready {
            children,
            render_object,
            ..
        } = snapshot.inner.mainline_ref()?.state.as_ref()?
        else {
            return None;
        };

        match render_element_function_table_of::<E>() {
            RenderElementFunctionTable::RenderObject {
                into_arc_child_render_object,
                ..
            } => render_object.clone().map(into_arc_child_render_object),
            RenderElementFunctionTable::None { as_child, .. } => {
                as_child(children).get_current_subtree_render_object()
            }
        }
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
            into_arc_child_render_object,
            ..
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
        }) = snapshot.inner.mainline_ref()
        else {
            return Err("Render object call should only be called on element nodes that are ready and attached");
        };
        Ok(into_arc_child_render_object(render_object.clone()).as_arc_any_render_object())
    }
}

pub fn create_root_element<E, R, L>(
    widget: E::ArcWidget,
    element: E,
    element_children: ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
    render: R,
    render_children: <R::ChildContainer as HktContainer>::Container<
        ArcChildRenderObject<E::ChildProtocol>,
    >,
    layer: L,
    hooks: Hooks,
    constraints: <E::ParentProtocol as Protocol>::Constraints,
) -> Arc<ElementNode<E>>
where
    E: RenderElement<Render = R>,
    R: LayerRender<
        L,
        ChildContainer = E::ChildContainer,
        ParentProtocol = E::ParentProtocol,
        ChildProtocol = E::ChildProtocol,
    >,
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
                children: render_children,
            }),
            mark: RenderMark::new(),
        });
        ElementNode {
            context: element_context,
            snapshot: SyncMutex::new(ElementSnapshot {
                widget,
                inner: ElementSnapshotInner::Mainline(Mainline {
                    state: Some(MainlineState::Ready {
                        element,
                        children: element_children,
                        hooks,
                        render_object: Some(render_object),
                    }),
                    async_queue: AsyncWorkQueue::new_empty(),
                }),
            }),
        }
    });
    element_node
}
