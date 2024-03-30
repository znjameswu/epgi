mod async_queue;
mod context;
mod mark;
mod node;
mod provider;
mod render_or_unit;
mod snapshot;

mod snapshot_new;
pub use snapshot_new::*;

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
        LayerProtocol, Protocol, Provide, PtrEq, SyncMutex, TypeKey,
    },
    // nodes::{RenderSuspense, Suspense, SuspenseElement},
    scheduler::JobId,
    sync::{ImplElementNodeSyncReconcile, SelectReconcileImpl},
    tree::RenderAction,
};

use super::{
    ArcAnyRenderObject, ArcChildRenderObject, ArcChildWidget, ArcWidget, BuildContext,
    ChildElementWidgetPair, ElementWidgetPair, LayerRender, LayoutCache, LayoutResults, Render,
    RenderNew, RenderObjectOld, TreeNode,
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
        element: Arc<E::ElementNode>,
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
    type ElementNode: ChildElementNode<Self::ParentProtocol>
        + ImplElementNodeSyncReconcile<Self>
        + Send
        + Sync
        + 'static;

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

pub trait ProvideElement: TreeNode + HasArcWidget {
    type Provided: Provide;
    fn get_provided_value(widget: &Self::ArcWidget) -> Arc<Self::Provided>;
}

pub trait RenderElement:
    TreeNode
    + HasArcWidget
    + SelectArcRenderObject<
        true,
        OptionArcRenderObject = Option<Arc<<Self::Render as RenderNew>::RenderObject>>,
    >
{
    type Render: RenderNew<
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

// pub trait ElementOld: Send + Sync + Clone + 'static {
//     type ArcWidget: ArcWidget<Element = Self>; //<Element = Self>;
//     type ParentProtocol: Protocol;
//     type ChildProtocol: Protocol;

//     type ChildContainer: HktContainer;

//     // ~~TypeId::of is not constant function so we have to work around like this.~~ Reuse Element for different widget.
//     // Boxed slice generates worse code than Vec due to https://github.com/rust-lang/rust/issues/59878
//     fn get_consumed_types(widget: &Self::ArcWidget) -> &[TypeKey] {
//         &[]
//     }

//     type Provided: Provide;
//     const GET_PROVIDED_VALUE: Option<fn(&Self::ArcWidget) -> Arc<Self::Provided>> = None;

//     // SAFETY: No async path should poll or await the stashed continuation left behind by the sync build. Awaiting outside the sync build will cause child tasks to be run outside of sync build while still being the sync variant of the task.
//     // Rationale for a moving self: Allows users to destructure the self without needing to fill in a placeholder value.
//     /// If a hook suspended, then the untouched Self should be returned along with the suspended error
//     /// If nothing suspended, then the new Self should be returned.
//     fn perform_rebuild_element(
//         &mut self,
//         widget: &Self::ArcWidget,
//         ctx: BuildContext<'_>,
//         provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
//         children: ContainerOf<Self, ArcChildElementNode<Self::ChildProtocol>>,
//         nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::ChildProtocol>>,
//     ) -> Result<
//         (
//             ContainerOf<Self, ElementReconcileItem<Self::ChildProtocol>>,
//             Option<ChildRenderObjectsUpdateCallback<Self>>,
//         ),
//         (
//             ContainerOf<Self, ArcChildElementNode<Self::ChildProtocol>>,
//             BuildSuspendedError,
//         ),
//     >;

//     fn perform_inflate_element(
//         widget: &Self::ArcWidget,
//         ctx: BuildContext<'_>,
//         provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
//     ) -> Result<(Self, ContainerOf<Self, ArcChildWidget<Self::ChildProtocol>>), BuildSuspendedError>;

//     // A workaround for specialization.
//     // This is designed in such a way that we do not need to lock the mutex to know whether a render object is present.
//     // Rust const is inlined, so we can safely expect that no actual function pointers will occur in binary causing indirection.
//     // const GET_RENDER_OBJECT: GetRenderObject<Self>;
//     type RenderOrUnit: RenderOrUnit<Self>;
// }

// pub trait RenderElement: Element<RenderOrUnit = <Self as RenderElement>::Render> {
//     type Render: Render<ParentProtocol = Self::ParentProtocol, ChildProtocol = Self::ChildProtocol>;
//     fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render;
//     /// Update necessary properties of render object given by the widget
//     ///
//     /// Called during the commit phase, when the widget is updated.
//     /// Always called after [RenderElement::try_update_render_object_children].
//     /// If that call failed to update children (indicating suspense), then this call will be skipped.
//     fn update_render(render_object: &mut Self::Render, widget: &Self::ArcWidget) -> RenderAction;

//     /// Whether [Render::update_render_object] is a no-op and always returns None
//     ///
//     /// When set to true, [Render::update_render_object]'s implementation will be ignored,
//     /// Certain optimizations to reduce mutex usages will be applied during the commit phase.
//     /// However, if [Render::update_render_object] is actually not no-op, doing this will cause unexpected behaviors.
//     ///
//     /// Setting to false will always guarantee the correct behavior.
//     const NOOP_UPDATE_RENDER_OBJECT: bool = false;

//     fn element_render_children_mapping<T: Send + Sync>(
//         &self,
//         element_children: <Self::ChildContainer as HktContainer>::Container<T>,
//     ) -> <<Self::Render as Render>::ChildContainer as HktContainer>::Container<T>;

//     /// BUG: Somehow rustdoc breaks on this item
//     #[doc(hidden)]
//     const SUSPENSE_ELEMENT_FUNCTION_TABLE: Option<SuspenseElementFunctionTable<Self>> = None;
// }

// pub struct SuspenseElementFunctionTable<E: Element> {
//     pub(crate) get_suspense_element_mut: fn(&mut E) -> &mut SuspenseElement<E::ChildProtocol>,
//     pub(crate) get_suspense_widget_ref: fn(&E::ArcWidget) -> &Suspense<E::ParentProtocol>,
//     pub(crate) get_suspense_render_object:
//         fn(ArcRenderObjectOf<E>) -> Arc<RenderObjectOld<RenderSuspense<E::ParentProtocol>>>,
//     pub(crate) into_arc_render_object:
//         fn(Arc<RenderObjectOld<RenderSuspense<E::ParentProtocol>>>) -> ArcRenderObjectOf<E>,
// }

pub trait SelectArcRenderObject<const RENDER_ELEMENT: bool>: TreeNode {
    type OptionArcRenderObject: Default + Clone + Send + Sync;

    fn get_current_subtree_render_object(
        render_object: &Self::OptionArcRenderObject,
        children: &ContainerOf<Self, ArcChildElementNode<Self::ChildProtocol>>,
    ) -> Option<ArcChildRenderObject<Self::ParentProtocol>>;
}

impl<E> SelectArcRenderObject<false> for E
where
    E: TreeNode<ChildContainer = ArrayContainer<1>, ChildProtocol = Self::ParentProtocol>,
{
    type OptionArcRenderObject = ();

    fn get_current_subtree_render_object(
        render_object: &Self::OptionArcRenderObject,
        [child]: &[ArcChildElementNode<Self::ChildProtocol>; 1],
    ) -> Option<ArcChildRenderObject<Self::ParentProtocol>> {
        child.get_current_subtree_render_object()
    }
}

impl<E> SelectArcRenderObject<true> for E
where
    E: RenderElement,
{
    type OptionArcRenderObject = Option<Arc<<E::Render as RenderNew>::RenderObject>>;

    fn get_current_subtree_render_object(
        render_object: &Self::OptionArcRenderObject,
        children: &ContainerOf<Self, ArcChildElementNode<Self::ChildProtocol>>,
    ) -> Option<ArcChildRenderObject<Self::ParentProtocol>> {
        render_object
            .as_ref()
            .map(|render_object| render_object.clone() as _)
    }
}

pub trait SelectProvideElement<const PROVIDE_ELEMENT: bool>: HasArcWidget {
    fn option_get_provided_key_value_pair(
        widget: &Self::ArcWidget,
    ) -> Option<(Arc<dyn Provide>, TypeKey)>;

    fn diff_provided_value(
        old_widget: &Self::ArcWidget,
        new_widget: &Self::ArcWidget,
    ) -> Option<Arc<dyn Provide>>;
}

impl<E> SelectProvideElement<false> for E
where
    E: HasArcWidget,
{
    #[inline(always)]
    fn option_get_provided_key_value_pair(
        widget: &Self::ArcWidget,
    ) -> Option<(Arc<dyn Provide>, TypeKey)> {
        None
    }

    #[inline(always)]
    fn diff_provided_value(
        old_widget: &Self::ArcWidget,
        new_widget: &Self::ArcWidget,
    ) -> Option<Arc<dyn Provide>> {
        None
    }
}

impl<E> SelectProvideElement<true> for E
where
    E: ProvideElement,
{
    #[inline(always)]
    fn option_get_provided_key_value_pair(
        widget: &Self::ArcWidget,
    ) -> Option<(Arc<dyn Provide>, TypeKey)> {
        Some((E::get_provided_value(widget), TypeKey::of::<E::Provided>()))
    }

    #[inline(always)]
    fn diff_provided_value(
        old_widget: &Self::ArcWidget,
        new_widget: &Self::ArcWidget,
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

pub struct ElementNode<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool>
where
    E: Element<ElementNode = Self> + SelectArcRenderObject<RENDER_ELEMENT>,
{
    pub context: ArcElementContextNode,
    pub(crate) snapshot: SyncMutex<ElementSnapshot<E, E::OptionArcRenderObject>>,
}

pub(crate) struct ElementSnapshot<E: Element, R> {
    pub(crate) widget: E::ArcWidget,
    // pub(super) subtree_suspended: bool,
    pub(crate) inner: ElementSnapshotInner<E, R>,
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool>
    ElementNode<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: Element<ElementNode = Self> + SelectArcRenderObject<RENDER_ELEMENT>,
{
    pub(super) fn new(
        context: ArcElementContextNode,
        widget: E::ArcWidget,
        inner: ElementSnapshotInner<E, E::OptionArcRenderObject>,
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

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> ChildElementNode<E::ParentProtocol>
    for ElementNode<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: Element<ElementNode = Self>
        + SelectArcRenderObject<RENDER_ELEMENT>
        + SelectReconcileImpl<RENDER_ELEMENT, PROVIDE_ELEMENT>
        + SelectProvideElement<PROVIDE_ELEMENT>,
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

        E::get_current_subtree_render_object(render_object, children)
    }
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> AnyElementNode
    for ElementNode<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: Element<ElementNode = Self>
        + SelectArcRenderObject<RENDER_ELEMENT>
        + SelectReconcileImpl<RENDER_ELEMENT, PROVIDE_ELEMENT>
        + SelectProvideElement<PROVIDE_ELEMENT>,
{
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
