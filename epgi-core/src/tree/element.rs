mod async_queue;
mod context;
mod mark;
mod node;
mod provider;
mod render_or_unit;
mod snapshot;

use std::marker::PhantomData;

pub use async_queue::*;
pub use context::*;
pub use mark::*;
pub use node::*;
pub use provider::*;
pub use render_or_unit::*;
pub use snapshot::*;

use crate::{
    foundation::{
        Arc, ArrayContainer, Asc, Aweak, BuildSuspendedError, HktContainer, InlinableDwsizeVec,
        Protocol, Provide, PtrEq, SyncMutex, TypeKey,
    },
    scheduler::JobId,
    sync::ImplReconcileCommit,
    tree::RenderAction,
};

use super::{
    ArcAnyRenderObject, ArcChildRenderObject, ArcChildWidget, ArcWidget, BuildContext,
    ChildElementWidgetPair, ElementWidgetPair, Render, RenderObject, TreeNode,
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

#[allow(type_alias_bounds)]
pub type ChildRenderObjectsUpdateCallbackNew<E: Element> = Box<
    dyn FnOnce(
        <E::ChildContainer as HktContainer>::Container<ArcChildRenderObject<E::ChildProtocol>>,
    ) -> <E::ChildContainer as HktContainer>::Container<
        RenderObjectSlots<E::ChildProtocol>,
    >,
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

#[allow(type_alias_bounds)]
pub type ContainerOf<E: TreeNode, T> = <E::ChildContainer as HktContainer>::Container<T>;

pub trait HasArcWidget {
    type ArcWidget: ArcWidget<Element = Self>;
}

pub trait Element: TreeNode + HasArcWidget + Clone + Sized + 'static {
    // type ElementNode: ChildElementNode<Self::ParentProtocol>
    //     + ImplElementNodeSyncReconcile<Self>
    //     + Send
    //     + Sync
    //     + 'static;

    type ElementImpl: ImplElement<Element = Self>;

    // ~~TypeId::of is not constant function so we have to work around like this.~~ Reuse Element for different widget.
    // Boxed slice generates worse code than Vec due to https://github.com/rust-lang/rust/issues/59878
    fn get_consumed_types(widget: &Self::ArcWidget) -> &[TypeKey] {
        &[]
    }

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
            Option<ChildRenderObjectsUpdateCallbackNew<Self>>,
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
}

pub trait ImplElement:
    ImplElementNode<Self::Element> + ImplProvide<Self::Element> + ImplReconcileCommit<Self::Element>
{
    type Element: Element;
}

pub struct ElementImpl<E: Element, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool>(
    PhantomData<E>,
);

impl<E: Element, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> ImplElement
    for ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    Self: ImplElementNode<E>,
    Self: ImplProvide<E>,
    Self: ImplReconcileCommit<E>,
{
    type Element = E;
}

pub trait ProvideElement: TreeNode + HasArcWidget {
    type Provided: Provide;
    fn get_provided_value(widget: &Self::ArcWidget) -> Arc<Self::Provided>;
}

pub trait RenderElement: TreeNode + HasArcWidget {
    type Render: Render<
        ParentProtocol = Self::ParentProtocol,
        ChildProtocol = Self::ChildProtocol,
        ChildContainer = Self::ChildContainer,
    >;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render;
    /// Update necessary properties of render object given by the widget
    ///
    /// Called during the commit phase, when the widget is updated.
    /// Always called after [RenderElement::try_update_render_object_children].
    /// If that call failed to update children (indicating suspense), then this call will be skipped.
    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> RenderAction;

    /// Whether [Render::update_render_object] is a no-op and always returns None
    ///
    /// When set to true, [Render::update_render_object]'s implementation will be ignored,
    /// Certain optimizations to reduce mutex usages will be applied during the commit phase.
    /// However, if [Render::update_render_object] is actually not no-op, doing this will cause unexpected behaviors.
    ///
    /// Setting to false will always guarantee the correct behavior.
    const NOOP_UPDATE_RENDER_OBJECT: bool = false;
}

pub trait ImplElementNode<E: Element> {
    type OptionArcRenderObject: Default + Clone + Send + Sync;
    fn get_current_subtree_render_object(
        render_object: &Self::OptionArcRenderObject,
        children: &ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
    ) -> Option<ArcChildRenderObject<E::ParentProtocol>>;
}

impl<E: Element, const PROVIDE_ELEMENT: bool> ImplElementNode<E>
    for ElementImpl<E, false, PROVIDE_ELEMENT>
where
    E: TreeNode<
        ChildContainer = ArrayContainer<1>,
        ChildProtocol = <E as TreeNode>::ParentProtocol,
    >,
{
    type OptionArcRenderObject = ();

    fn get_current_subtree_render_object(
        _render_object: &(),
        [child]: &[ArcChildElementNode<E::ChildProtocol>; 1],
    ) -> Option<ArcChildRenderObject<<E>::ParentProtocol>> {
        child.get_current_subtree_render_object()
    }
}

impl<E: Element, const PROVIDE_ELEMENT: bool> ImplElementNode<E>
    for ElementImpl<E, true, PROVIDE_ELEMENT>
where
    E: RenderElement,
{
    type OptionArcRenderObject = Option<Arc<RenderObject<E::Render>>>;

    fn get_current_subtree_render_object(
        render_object: &Self::OptionArcRenderObject,
        _children: &ContainerOf<E, ArcChildElementNode<<E>::ChildProtocol>>,
    ) -> Option<ArcChildRenderObject<<E>::ParentProtocol>> {
        render_object
            .as_ref()
            .map(|render_object| render_object.clone() as _)
    }
}

pub trait ImplProvide<E: Element> {
    const PROVIDE_ELEMENT: bool;
    fn option_get_provided_key_value_pair(
        widget: &E::ArcWidget,
    ) -> Option<(Arc<dyn Provide>, TypeKey)>;

    fn diff_provided_value(
        old_widget: &E::ArcWidget,
        new_widget: &E::ArcWidget,
    ) -> Option<Arc<dyn Provide>>;
}

impl<E: Element, const RENDER_ELEMENT: bool> ImplProvide<E>
    for ElementImpl<E, RENDER_ELEMENT, false>
{
    const PROVIDE_ELEMENT: bool = false;

    fn option_get_provided_key_value_pair(
        widget: &<E>::ArcWidget,
    ) -> Option<(Arc<dyn Provide>, TypeKey)> {
        None
    }

    fn diff_provided_value(
        old_widget: &<E>::ArcWidget,
        new_widget: &<E>::ArcWidget,
    ) -> Option<Arc<dyn Provide>> {
        None
    }
}

impl<E: Element, const RENDER_ELEMENT: bool> ImplProvide<E> for ElementImpl<E, RENDER_ELEMENT, true>
where
    E: ProvideElement,
{
    const PROVIDE_ELEMENT: bool = true;

    fn option_get_provided_key_value_pair(
        widget: &<E>::ArcWidget,
    ) -> Option<(Arc<dyn Provide>, TypeKey)> {
        Some((E::get_provided_value(widget), TypeKey::of::<E::Provided>()))
    }

    fn diff_provided_value(
        old_widget: &<E>::ArcWidget,
        new_widget: &<E>::ArcWidget,
    ) -> Option<Arc<dyn Provide>> {
        let old_provided_value = E::get_provided_value(&old_widget);
        let new_provided_value = E::get_provided_value(new_widget);
        if !Asc::ptr_eq(&old_provided_value, &new_provided_value)
            && !old_provided_value.eq_sized(new_provided_value.as_ref())
        {
            Some(new_provided_value)
        } else {
            None
        }
    }
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
