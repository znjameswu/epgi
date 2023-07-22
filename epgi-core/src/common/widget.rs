use std::{
    any::{Any, TypeId},
    fmt::Debug,
    marker::PhantomData,
    ptr::null,
    sync::Arc,
};

use crate::foundation::{AsHeapPtr, Asc, Key, Protocol};

use super::{ArcAnyElementNode, Element};

pub type ArcChildWidget<P> = Asc<dyn ChildWidget<P>>;
pub type ArcParentWidget<P> = Asc<dyn ParentWidget<ChildProtocol = P>>;
pub type ArcAnyWidget = Asc<dyn AnyWidget>;

pub trait Widget: std::fmt::Debug + 'static + Send + Sync {
    type Element: Element;

    fn key(&self) -> &dyn Key;

    fn create_element(self: Asc<Self>) -> Self::Element;

    fn into_arc_widget(self: Arc<Self>) -> <Self::Element as Element>::ArcWidget;
}

pub trait ChildWidget<SP: Protocol>:
    crate::sync::reconciler_private::ChildWidgetSyncInflateExt<SP>
    + crate::r#async::reconciler_private::ChildWidgetAsyncInflateExt<SP>
    + AnyWidget
    + 'static
    + Debug
{
    fn as_any(&self) -> &dyn Any;

    fn as_arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;

    fn as_arc_any_widget(self: Arc<Self>) -> ArcAnyWidget;

    // // These methods cannot be hidden behind a private supertrait. Since the supertrait needs associated types to be object safe and the associated types will be duplicated everywhere
    // fn spawn_inflate_with(
    //     self: Arc<Self>,
    //     reconciler: &Reconciler,
    // ) -> ArcChildElementNode<Self::SelfProtocol>;
    // fn inflate_sync<'a, 'batch>(
    //     self: Arc<Self>,
    //     parent_context: &ArcElementContextNode,
    //     job_ids: &'a SmallSet<JobId>,
    //     scope: &'a rayon::Scope<'batch>,
    //     tree_scheduler: &'batch TreeScheduler,
    // ) -> (ArcChildElementNode<SP>, SubtreeCommitResult);

    fn widget_type_id(&self) -> TypeId;
}

impl<T> ChildWidget<<T::Element as Element>::SelfProtocol> for T
where
    T: Widget,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    fn as_arc_any_widget(self: Arc<Self>) -> ArcAnyWidget {
        self
    }

    // fn inflate_sync<'a, 'batch>(
    //     self: Arc<Self>,
    //     parent_context: &ArcElementContextNode,
    //     job_ids: &'a SmallSet<JobId>,
    //     scope: &'a rayon::Scope<'batch>,
    //     tree_scheduler: &'batch TreeScheduler,
    // ) -> (
    //     ArcChildElementNode<<T::Element as Element>::SelfProtocol>,
    //     SubtreeCommitResult,
    // ) {
    //     ChildWidgetInflateExt::inflate_sync(self, parent_context, job_ids, scope, tree_scheduler)
    // }

    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl<P> dyn ChildWidget<P>
where
    P: Protocol,
{
    pub fn downcast<W: Widget>(self: Arc<Self>) -> Option<Arc<W>> {
        self.as_arc_any().downcast::<W>().ok()
    }
}

pub trait ArcChildWidgetExt {
    // fn downcast<W: Widget
}

pub trait ParentWidget {
    type ChildProtocol: Protocol;

    fn as_any(&self) -> &dyn Any;

    fn as_arc_any_widget(self: Arc<Self>) -> ArcAnyWidget;
}

impl<T> ParentWidget for T
where
    T: Widget,
{
    type ChildProtocol = <T::Element as Element>::ChildProtocol;

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_arc_any_widget(self: Arc<Self>) -> ArcAnyWidget {
        self
    }
}

pub trait AnyWidget: std::fmt::Debug + 'static + Send + Sync {
    fn key(&self) -> &dyn Key;

    fn as_any(&self) -> &dyn Any;
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
    fn as_any_child(&self) -> Box<dyn Any>;
    fn as_any_parent(&self) -> Box<dyn Any>;
    fn as_any_child_arc(self: Arc<Self>) -> Box<dyn Any>;
    fn as_any_parent_arc(self: Arc<Self>) -> Box<dyn Any>;
    fn create_element_node(
        self: Arc<Self>,
        // context: InflateContext,
    ) -> ArcAnyElementNode;
}

impl<T> AnyWidget for T
where
    T: Widget,
{
    fn key(&self) -> &dyn Key {
        <Self as Widget>::key(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    fn as_any_child(&self) -> Box<dyn Any> {
        let res: &dyn ChildWidget<<<Self as Widget>::Element as Element>::SelfProtocol> = self;
        Box::new(res as *const _)
    }

    fn as_any_parent(&self) -> Box<dyn Any> {
        let res: &dyn ParentWidget<
            ChildProtocol = <<Self as Widget>::Element as Element>::ChildProtocol,
        > = self;
        Box::new(res as *const _)
    }

    fn as_any_child_arc(self: Arc<Self>) -> Box<dyn Any> {
        let res: ArcChildWidget<<<Self as Widget>::Element as Element>::SelfProtocol> = self;
        Box::new(res)
    }

    fn as_any_parent_arc(self: Arc<Self>) -> Box<dyn Any> {
        let res: ArcParentWidget<<<Self as Widget>::Element as Element>::ChildProtocol> = self;
        Box::new(res)
    }

    fn create_element_node(
        self: Arc<Self>,
        // context: InflateContext,
    ) -> ArcAnyElementNode {
        todo!()
        // Arc::new(ElementNode2::<ElementInner<T>>::new(
        //     // context.provider_map,
        //     // context.suspense_boundary,
        //     self,
        //     todo!()// parent,
        // )) as ArcAnyElement
    }
}

pub trait ArcAnyWidgetExt {
    fn downcast<W: Widget>(self) -> Option<Arc<W>>;
    fn downcast_child<P: Protocol>(self) -> Option<ArcChildWidget<P>>;
    fn downcast_parent<P: Protocol>(self) -> Option<ArcParentWidget<P>>;
}

impl ArcAnyWidgetExt for ArcAnyWidget {
    fn downcast<W: Widget>(self) -> Option<Arc<W>> {
        self.as_any_arc().downcast::<W>().ok()
    }

    fn downcast_child<P: Protocol>(self) -> Option<ArcChildWidget<P>> {
        self.as_any_child_arc()
            .downcast::<ArcChildWidget<P>>()
            .ok()
            .map(|x| *x)
    }

    fn downcast_parent<P: Protocol>(self) -> Option<ArcParentWidget<P>> {
        // match self.as_any_parent_arc().downcast::<ArcParentWidget<P>>() {
        //     Ok(res) => Some(*res),
        //     Err(_) => None,
        // }
        self.as_any_parent_arc()
            .downcast::<ArcParentWidget<P>>()
            .ok()
            .map(|x| *x)
    }
}

pub trait ArcWidget: ArcRaw + AsHeapPtr + Clone + Send + Sync + 'static {
    type Element: Element;

    fn into_any_widget(self) -> ArcAnyWidget;

    fn into_child_widget(self) -> ArcChildWidget<<Self::Element as Element>::SelfProtocol>;

    fn widget_type_id(&self) -> TypeId;

    fn key(&self) -> &dyn Key;
}

pub fn try_convert_if_same_type<T: ArcWidget>(
    this: &T,
    other: ArcChildWidget<<T::Element as Element>::SelfProtocol>,
) -> Result<T, ArcChildWidget<<T::Element as Element>::SelfProtocol>> {
    if this.widget_type_id() == other.widget_type_id() {
        let raw = unsafe {
            let mut this_ptr_repr = PtrRepr::new_null();
            this_ptr_repr.const_ptr = ArcRaw::as_ptr(this);

            PtrRepr {
                components: PtrComponents {
                    data_address: PtrRepr {
                        const_ptr: Arc::into_raw(other),
                    }
                    .components
                    .data_address,
                    metadata: this_ptr_repr.components.metadata,
                    marker: PhantomData,
                },
            }
            .const_ptr
        };
        Ok(unsafe { ArcRaw::from_raw(raw) })
    } else {
        Err(other)
    }
}
pub trait ArcRaw {
    type Pointee: ?Sized;

    fn as_ptr(&self) -> *const Self::Pointee;

    unsafe fn from_raw(raw: *const Self::Pointee) -> Self;
}

impl<T> ArcRaw for Arc<T>
where
    T: ?Sized,
{
    type Pointee = T;

    fn as_ptr(&self) -> *const Self::Pointee {
        todo!()
    }

    unsafe fn from_raw(raw: *const Self::Pointee) -> Self {
        todo!()
    }
}

#[repr(C)]
union PtrRepr<T: ?Sized> {
    const_ptr: *const T,
    components: PtrComponents<T>,
}

impl<T> PtrRepr<T>
where
    T: ?Sized,
{
    fn new_null() -> Self {
        PtrRepr {
            components: PtrComponents {
                data_address: null(),
                metadata: null(),
                marker: PhantomData,
            },
        }
    }
}

#[repr(C)]
struct PtrComponents<T: ?Sized> {
    data_address: *const (),
    metadata: *const (),
    marker: PhantomData<T>,
}

// Manual impl needed to avoid `T: Copy` bound.
impl<T: ?Sized> Copy for PtrComponents<T> {}

// Manual impl needed to avoid `T: Clone` bound.
impl<T: ?Sized> Clone for PtrComponents<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<W> ArcWidget for Asc<W>
where
    W: Widget,
{
    type Element = <W as Widget>::Element;

    fn into_any_widget(self) -> ArcAnyWidget {
        self
    }

    fn into_child_widget(self) -> ArcChildWidget<<Self::Element as Element>::SelfProtocol> {
        self
    }

    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn key(&self) -> &dyn Key {
        Widget::key(self.as_ref())
    }
}
