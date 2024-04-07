mod async_queue;
pub use async_queue::*;

mod context;
pub use context::*;

mod mark;
pub use mark::*;

mod node;
pub use node::*;

mod provider;
pub use provider::*;

mod render_or_unit;
pub use render_or_unit::*;
mod snapshot;

pub use snapshot::*;

mod r#impl;
pub use r#impl::*;

use crate::{
    foundation::{Arc, Aweak, Protocol, PtrEq, SyncMutex},
    scheduler::JobId,
};

use super::{
    ArcAnyRenderObject, ArcChildRenderObject, ArcChildWidget, ArcWidget, ChildElementWidgetPair,
    ContainerOf, ElementWidgetPair, TreeNode,
};

pub type ArcAnyElementNode = Arc<dyn AnyElementNode>;
pub type AweakAnyElementNode = Aweak<dyn AnyElementNode>;
pub type ArcChildElementNode<P> = Arc<dyn ChildElementNode<P>>;

/// We assume the render has the same child container with the element,
/// ignoring the fact that Suspense may have different child containers.
///
/// However, we designate Suspense to be the only component to have different containers,
/// which will be handled by Suspense's specialized function pointers.
#[allow(type_alias_bounds)]
pub type ChildRenderObjectsUpdateCallback<E: TreeNode> = Box<
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
        Self::Update(Box::new(ElementWidgetPair::<E> { element, widget }))
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

pub trait Element: TreeNode + Clone + Sized + 'static {
    type ArcWidget: ArcWidget<Element = Self>;
    type ElementImpl: ImplElement<Element = Self> + HasReconcileImpl<Self>;
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

impl<E: Element> ElementNode<E> {
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

impl<E: Element> ChildElementNode<E::ParentProtocol> for ElementNode<E> {
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
        Self::can_rebuild_with(self, widget).map_err(|(element, widget)| (element as _, widget))
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

        E::ElementImpl::get_current_subtree_render_object(render_object, children)
    }
}

impl<E: Element> AnyElementNode for ElementNode<E> {
    fn as_any_arc(self: Arc<Self>) -> ArcAnyElementNode {
        self
    }

    fn push_job(&self, job_id: JobId) {
        todo!()
    }

    fn render_object(&self) -> Result<ArcAnyRenderObject, &str> {
        todo!()
        // let RenderElementFunctionTable::RenderObject {
        //     into_arc_child_render_object,
        //     ..
        // } = render_element_function_table_of::<E>()
        // else {
        //     return Err("Render object call should not be called on an Element type that does not associate with a render object");
        // };
        // let snapshot = self.snapshot.lock();
        // let Some(Mainline {
        //     state:
        //         Some(MainlineState::Ready {
        //             render_object: Some(render_object),
        //             ..
        //         }),
        //     ..
        // }) = snapshot.inner.mainline_ref()
        // else {
        //     return Err("Render object call should only be called on element nodes that are ready and attached");
        // };
        // Ok(into_arc_child_render_object(render_object.clone()).as_arc_any_render_object())
    }
}

#[inline(always)]
pub(crate) fn no_widget_update<E: Element>(
    new_widget: Option<&E::ArcWidget>,
    old_widget: &E::ArcWidget,
) -> bool {
    if let Some(new_widget) = new_widget {
        return PtrEq(new_widget) == PtrEq(old_widget);
    }
    return true;
}

// pub fn create_root_element<E, R>(
//     widget: E::ArcWidget,
//     element: E,
//     element_children: ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
//     render: R,
//     render_children: <R::ChildContainer as HktContainer>::Container<
//         ArcChildRenderObject<E::ChildProtocol>,
//     >,
//     hooks: Hooks,
//     constraints: <E::ParentProtocol as Protocol>::Constraints,
//     offset: <E::ParentProtocol as Protocol>::Offset,
//     size: <E::ParentProtocol as Protocol>::Size,
//     layout_memo: R::LayoutMemo,
// ) -> (Arc<ElementNode<E>>, Arc<RenderObjectOld<R>>)
// where
//     E: RenderElement<Render = R>,
//     R: Render<
//         ChildContainer = E::ChildContainer,
//         ParentProtocol = E::ParentProtocol,
//         ChildProtocol = E::ChildProtocol,
//     >,
//     R: LayerRender,
//     R::ChildProtocol: LayerProtocol,
//     R::ParentProtocol: LayerProtocol,
// {
//     let mut render_object_built = None;
//     let render_object_built_mut = &mut render_object_built;
//     let element_node = Arc::new_cyclic(move |node| {
//         let element_context = Arc::new(ElementContextNode::new_root(node.clone() as _));
//         // let render = R::try_create_render_object_from_element(&element, &widget)
//         //     .expect("Root render object creation should always be successfully");
//         let render_object = Arc::new(RenderObjectOld::new(
//             render,
//             render_children,
//             element_context.clone(),
//         ));
//         *render_object_built_mut = Some(render_object.clone());
//         {
//             render_object
//                 .inner
//                 .lock()
//                 .cache
//                 .insert_layout_cache(LayoutCache::new(
//                     LayoutResults::new(constraints, size, layout_memo),
//                     Some(offset),
//                     None,
//                 ));
//         }
//         render_object.mark.set_parent_not_use_size::<R>();
//         ElementNode {
//             context: element_context,
//             snapshot: SyncMutex::new(ElementSnapshot {
//                 widget,
//                 inner: ElementSnapshotInner::Mainline(Mainline {
//                     state: Some(MainlineState::Ready {
//                         element,
//                         children: element_children,
//                         hooks,
//                         render_object: Some(render_object),
//                     }),
//                     async_queue: AsyncWorkQueue::new_empty(),
//                 }),
//             }),
//         }
//     });
//     (
//         element_node,
//         render_object_built.expect("Impossible to fail"),
//     )
// }
