mod context;
use std::{any::Any, borrow::Cow};

pub use context::*;

mod node;
pub use node::*;

mod r#impl;
pub use r#impl::*;

mod async_queue;
pub(crate) use async_queue::*;

mod mark;
pub(crate) use mark::*;

mod provider;
pub(crate) use provider::*;

mod snapshot;
pub(crate) use snapshot::*;

mod waker;
pub(crate) use waker::*;

use crate::foundation::{
    Arc, Asc, Aweak, BuildSuspendedError, ContainerOf, HktContainer, InlinableDwsizeVec, Protocol,
    Provide, PtrEq, TypeKey, EMPTY_CONSUMED_TYPES,
};

use super::{
    ArcAnyRenderObject, ArcChildRenderObject, ArcChildWidget, ArcWidget, BuildContext,
    ChildElementWidgetPair, ElementWidgetPair, FullRender, RenderAction,
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
    fn get_consumed_types(widget: &Self::ArcWidget) -> Cow<[TypeKey]> {
        EMPTY_CONSUMED_TYPES.into()
    }

    // SAFETY: No async path should poll or await the stashed continuation left behind by the sync build. Awaiting outside the sync build will cause child tasks to be run outside of sync build while still being the sync variant of the task.
    // Rationale for a moving self: Allows users to destructure the self without needing to fill in a placeholder value.
    /// If a hook suspended, then the untouched Self should be returned along with the suspended error
    /// If nothing suspended, then the new Self should be returned.
    fn perform_rebuild_element(
        &mut self,
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
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
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<
        (
            Self,
            ContainerOf<Self::ChildContainer, ArcChildWidget<Self::ChildProtocol>>,
        ),
        BuildSuspendedError,
    >;

    /// Returns the new parent data and corresponding render action for parent
    /// if the parent data has changed
    ///
    /// It is recommended to cache the last generated parent data, and only
    /// generate parent data when the parent data needs to be changed.
    ///
    /// Will only be invoked if this element is a component element. Has no effect
    /// if implemented on other elements.
    #[allow(unused_variables)]
    #[inline(always)]
    fn generate_parent_data(
        &mut self,
        widget: &Self::ArcWidget,
    ) -> Option<(Asc<dyn Any + Send + Sync>, Option<RenderAction>)> {
        None
    }
}

// This is separated from the main ELement trait to avoid inductive cycles in ImplReconcileCommit::visit_commit
/// This is an auto-impled trait, users should not impl this trait directly.
/// Rather, they should impl [Element]
///
/// If `E: FullElement`, then `ElementNode<E>` will have full capability to reconcile, pointer cast, etc.
pub trait FullElement: Element<Impl = <Self as FullElement>::Impl> {
    type Impl: ImplFullElement<Self>;
}

// This is separated from the main ElementBase trait to avoid inductive cycles when implementing templates.
// Otherwise, there will be something like
// impl<E> TemplateElement for E where ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>: ImplElement<Self>, //....
// The only way to break the cycle is to relocate impl bounds on "Impl*" traits from the impl block to each individual method items.
// Separating Element vs ElementBase almost eliminate all possible cycles, except for ImplReconcileCommit::visit_commit
/// If `E: Element`, then `ElementNode<E>` will have a well-defined layout
pub trait Element: ElementBase {
    type Impl: ImplElementNode<Self> + ImplProvide<Self>;
}

impl<E> FullElement for E
where
    E: Element,
    E::Impl: ImplFullElement<E>,
{
    type Impl = E::Impl;
}

/// We assume the render has the same child container with the element,
/// ignoring the fact that Suspense may have different child containers.
///
/// However, we designate Suspense to be the only component to have different containers,
/// which will be handled by Suspense's specialized function pointers.
#[allow(type_alias_bounds)]
pub type ChildRenderObjectsUpdateCallback<C, CP> = Box<
    dyn FnOnce(ContainerOf<C, ArcChildRenderObject<CP>>) -> ContainerOf<C, RenderObjectSlots<CP>>
        + Send
        + Sync,
>;

#[derive(Clone)]
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
    pub fn new_update<E: FullElement<ParentProtocol = CP>>(
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
    type Render: FullRender<
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
    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction>;

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
    fn get_provided_value(widget: &Self::ArcWidget) -> &Arc<Self::Provided>;
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
