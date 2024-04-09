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

mod snapshot;
pub use snapshot::*;

mod r#impl;
pub use r#impl::*;

use crate::foundation::{
    Arc, Aweak, BuildSuspendedError, ContainerOf, HktContainer, InlinableDwsizeVec, Protocol,
    Provide, PtrEq, TypeKey,
};

use super::{
    ArcAnyRenderObject, ArcChildRenderObject, ArcChildWidget, ArcWidget, BuildContext,
    ChildElementWidgetPair, ElementWidgetPair, Render, RenderAction,
};

pub type ArcAnyElementNode = Arc<dyn AnyElementNode>;
pub type AweakAnyElementNode = Aweak<dyn AnyElementNode>;
pub type ArcChildElementNode<P> = Arc<dyn ChildElementNode<P>>;

pub trait ElementBase: Clone + Send + Sync + Sized + 'static {
    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;
    type ChildContainer: HktContainer;

    type ArcWidget: ArcWidget<Element = Self>;

    // ~~TypeId::of is not constant function so we have to work around like this.~~ Reuse Element for different widget.
    // Boxed slice generates worse code than Vec due to https://github.com/rust-lang/rust/issues/59878
    #[allow(unused_variables)]
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
        children: ContainerOf<Self::ChildContainer, ArcChildElementNode<Self::ChildProtocol>>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::ChildProtocol>>,
    ) -> Result<
        (
            ContainerOf<Self::ChildContainer, ElementReconcileItem<Self::ChildProtocol>>,
            Option<ChildRenderObjectsUpdateCallback<Self::ChildContainer, Self::ChildProtocol>>,
        ),
        (
            ContainerOf<Self::ChildContainer, ArcChildElementNode<Self::ChildProtocol>>,
            BuildSuspendedError,
        ),
    >;

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<
        (
            Self,
            ContainerOf<Self::ChildContainer, ArcChildWidget<Self::ChildProtocol>>,
        ),
        BuildSuspendedError,
    >;
}

// This is separated from the main Element trait to avoid inductive cycles when implementing templates.
// Otherwise, there will be something like
// impl<E> TemplateElement for E where ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>: ImplElement<Self>, //....
// The only way to break the cycle is to relocate impl bounds on "Impl*" traits from the impl block to each individual method items.
pub trait Element: ElementBase {
    type Impl: ImplElement<Element = Self>;
}

/// We assume the render has the same child container with the element,
/// ignoring the fact that Suspense may have different child containers.
///
/// However, we designate Suspense to be the only component to have different containers,
/// which will be handled by Suspense's specialized function pointers.
#[allow(type_alias_bounds)]
pub type ChildRenderObjectsUpdateCallback<C, CP> = Box<
    dyn FnOnce(ContainerOf<C, ArcChildRenderObject<CP>>) -> ContainerOf<C, RenderObjectSlots<CP>>,
>;

pub enum RenderObjectSlots<P: Protocol> {
    Inflate,
    Reuse(ArcChildRenderObject<P>),
}

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

pub trait RenderElement: ElementBase {
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

pub trait ProvideElement: ElementBase {
    type Provided: Provide;
    fn get_provided_value(widget: &Self::ArcWidget) -> Arc<Self::Provided>;
}

#[inline(always)]
pub(crate) fn no_widget_update<E: ElementBase>(
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
