mod async_queue;
mod context;
mod provider;
mod snapshot;

pub use async_queue::*;
pub use context::*;
pub use provider::*;
pub use snapshot::*;

use crate::{
    common::RenderObjectUpdateResult,
    foundation::{
        Arc, Aweak, BuildSuspendedError, InlinableDwsizeVec, Never, Parallel, Protocol, Provide,
        SyncMutex, TypeKey,
    },
    scheduler::JobId,
};

use super::{
    ArcChildRenderObject, ArcChildWidget, ArcWidget, ChildElementWidgetPair, Hook, ReconcileItem,
    Reconciler, Render, RenderCache, RenderObject, RenderObjectInner, RenderSuspense, Suspense,
    SuspenseElement,
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
    type ArcRenderObject: ArcRenderObject<Self>;
}

pub trait SingleChildElement: Element<ArcRenderObject = Never> {
    fn child(&self) -> &ArcChildElementNode<Self::ParentProtocol>;
}

// pub trait RenderElement: Element<ArcRenderObject = Arc<RenderObject<Self::Render>>> {
//     type Render: Render<Element = Self>;

//     fn try_create_render_object(
//         &self,
//         widget: &Self::ArcWidget,
//     ) -> Option<Arc<RenderObject<Self::Render>>>;
//     /// Update necessary properties of render object given by the widget
//     ///
//     /// Called during the commit phase, when the widget is updated.
//     /// Always called after [RenderElement::try_update_render_object_children].
//     /// If that call failed to update children (indicating suspense), then this call will be skipped.
//     fn update_render_object_widget(
//         widget: &Self::ArcWidget,
//         render_object: &Arc<RenderObject<Self::Render>>,
//     );
//     /// Try to re-assemble the children of the givin render object.
//     ///
//     /// Called during the commit phase, when subtree structure has changed.
//     fn try_update_render_object_children(
//         &self,
//         render_object: &Arc<RenderObject<Self::Render>>,
//     ) -> Result<(), ()>;

//     fn detach_render_object(render_object: &Arc<RenderObject<Self::Render>>);

//     const GET_SUSPENSE: Option<GetSuspense<Self>> = None;
// }

pub trait ArcRenderObject<E>: Send + Sync + 'static
where
    E: Element<ArcRenderObject = Self>,
{
    type Render;
    const GET_RENDER_OBJECT: GetRenderObject<E>;
    fn with_inner(&self, op: impl FnOnce(&mut Self::Render, &ArcElementContextNode));
}

impl<E> ArcRenderObject<E> for Never
where
    E: SingleChildElement<ArcRenderObject = Self>,
{
    type Render = Never;
    const GET_RENDER_OBJECT: GetRenderObject<E> = GetRenderObject::None(E::child);

    fn with_inner(&self, op: impl FnOnce(&mut Self::Render, &ArcElementContextNode)) {
        debug_assert!(
            false,
            "Never should never be dereferenced as ArcRenderObject"
        )
    }
}

impl<R> ArcRenderObject<R::Element> for Arc<RenderObject<R>>
where
    R: Render,
{
    type Render = R;
    const GET_RENDER_OBJECT: GetRenderObject<R::Element> = GetRenderObject::RenderObject {
        get_render_object: |x| x,
        try_create_render_object: |element, widget, element_context| {
            let render = R::try_create_render_object_from_element(element, widget)?;
            Some(Arc::new(RenderObject {
                element_context: element_context.clone(),
                inner: SyncMutex::new(RenderObjectInner {
                    cache: None,
                    render,
                }),
            }))
        },
        update_render_object: if R::NOOP_UPDATE_RENDER_OBJECT {
            None
        } else {
            Some(R::update_render_object)
        },
        try_update_render_object_children: if R::NOOP_UPDATE_RENDER_OBJECT_CHILDREN {
            None
        } else {
            Some(R::try_update_render_object_children)
        },
        detach_render_object: if R::NOOP_DETACH {
            None
        } else {
            Some(R::detach)
        },
        get_suspense: R::GET_SUSPENSE,
    };

    fn with_inner(&self, op: impl FnOnce(&mut Self::Render, &ArcElementContextNode)) {
        op(&mut self.inner.lock().render, &self.element_context)
    }
}

pub enum GetRenderObject<E: Element> {
    RenderObject {
        get_render_object: fn(E::ArcRenderObject) -> ArcChildRenderObject<E::ParentProtocol>,
        try_create_render_object:
            fn(&E, &E::ArcWidget, &ArcElementContextNode) -> Option<E::ArcRenderObject>,
        update_render_object: Option<
            fn(
                &mut <E::ArcRenderObject as ArcRenderObject<E>>::Render,
                &E::ArcWidget,
            ) -> RenderObjectUpdateResult,
        >,
        try_update_render_object_children: Option<
            fn(&mut <E::ArcRenderObject as ArcRenderObject<E>>::Render, &E) -> Result<(), ()>,
        >,
        detach_render_object: Option<fn(&mut <E::ArcRenderObject as ArcRenderObject<E>>::Render)>,
        get_suspense: Option<GetSuspense<E>>,
    },
    None(fn(&E) -> &ArcChildElementNode<E::ParentProtocol>),
}

pub struct GetSuspense<E: Element> {
    pub(crate) get_suspense_element_mut: fn(&mut E) -> &mut SuspenseElement<E::ChildProtocol>,
    pub(crate) get_suspense_widget_ref: fn(&E::ArcWidget) -> &Suspense<E::ParentProtocol>,
    pub(crate) get_suspense_render_object:
        fn(E::ArcRenderObject) -> Arc<RenderObject<RenderSuspense<E::ParentProtocol>>>,
    pub(crate) into_arc_render_object:
        fn(Arc<RenderObject<RenderSuspense<E::ParentProtocol>>>) -> E::ArcRenderObject,
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

    // fn context(&self) -> &ArcElementContextNode {
    //     &self.context
    // }
}

pub fn create_root_element<R: Render>(
    widget: <R::Element as Element>::ArcWidget,
    element: R::Element,
    hooks: Hooks,
    constraints: <<R::Element as Element>::ParentProtocol as Protocol>::Constraints,
) -> Arc<ElementNode<R::Element>> {
    let element_node = Arc::new_cyclic(move |node| {
        let element_context = Arc::new(ElementContextNode::new_no_provide(node.clone() as _, None));
        let render = R::try_create_render_object_from_element(&element, &widget)
            .expect("Root render object creation should always be successfully");
        let render_object = Arc::new(RenderObject {
            element_context: element_context.clone(),
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
